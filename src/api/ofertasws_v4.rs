use axum::{
    extract::State,
    http::{header, StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use sqlx::types::Decimal;
use redis::AsyncCommands;
use flate2::{write::GzEncoder, Compression};
use std::io::Write;
use std::sync::Arc;
use chrono::Timelike; // Para .hour()

use crate::api::common::SimpleApiResponse;
use crate::state::AppState as GlobalAppState;

// ============================================================================
// STRUCTS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Oferta {
    pub comercio: String,
    pub producto: String,
    pub codigo: Option<String>,
    pub precio_actual: f64,                      // double precision
    pub fecha_actual: String,
    pub dias_con_precio_actual: Option<i64>,     // bigint (puede ser NULL)
    pub precio_anterior: f64,                    // double precision
    pub fecha_anterior: String,
    pub precio_minimo_60d: Option<f64>,          // double precision
    pub precio_maximo_60d: Option<f64>,          // double precision
    pub precio_promedio_60d: Option<f64>,        // numeric (pero puede tener NaN, usar f64)
    pub es_precio_mas_bajo: bool,                // boolean
    #[serde(serialize_with = "serialize_decimal_as_f64")]
    pub porc: Decimal,                           // numeric
    #[serde(serialize_with = "serialize_decimal_as_f64")]
    pub diferencia: Decimal,                     // numeric
    pub link: Option<String>,
    pub imagen: Option<String>,
}

// Helper functions for serializing Decimal as f64 for JSON
fn serialize_decimal_as_f64<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use std::str::FromStr;
    let float_val = value.to_string().parse::<f64>().unwrap_or(0.0);
    serializer.serialize_f64(float_val)
}

fn serialize_option_decimal_as_f64<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(v) => {
            let float_val = v.to_string().parse::<f64>().unwrap_or(0.0);
            serializer.serialize_some(&float_val)
        }
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Serialize)]
pub struct OfertasWsMetadata {
    pub total_count: usize,
    pub generated_at: String, // ISO timestamp
    pub next_update: String, // Next scheduled update
    pub version: String, // Cache version identifier
}

#[derive(Debug, Serialize)]
pub struct OfertasWsResponse {
    pub ofertasws: Vec<Oferta>,
    pub metadata: OfertasWsMetadata,
}

pub struct AppState {
    pub ws_pool: PgPool,
    pub redis_client: deadpool_redis::Pool,
}

// ============================================================================
// CACHE HELPERS
// ============================================================================

/// Genera la key de Redis basada en el timestamp slot
fn get_cache_key() -> String {
    let now = chrono::Utc::now().with_timezone(&chrono_tz::America::Panama);
    
    // Determinar slot: si hora < 15 (3pm), usar 10am, sino usar 3pm
    let slot_hour = if now.hour() < 15 { 10 } else { 15 };
    
    format!(
        "ofertasws:cache:{}:{:02}:00",
        now.format("%Y-%m-%d"),
        slot_hour
    )
}

/// Genera E-Tag desde la cache key
fn get_etag() -> String {
    format!("\"{}\"", get_cache_key().replace("ofertasws:cache:", "ofertas-"))
}

/// Calcula el próximo update timestamp
fn get_next_update() -> String {
    use chrono::TimeZone;
    
    let now = chrono::Utc::now().with_timezone(&chrono_tz::America::Panama);
    
    let next_update = if now.hour() < 10 {
        // Antes de 10am → próximo update hoy 10am
        now.date_naive().and_hms_opt(10, 0, 0).unwrap()
    } else if now.hour() < 15 {
        // Entre 10am y 3pm → próximo update hoy 3pm
        now.date_naive().and_hms_opt(15, 0, 0).unwrap()
    } else {
        // Después de 3pm → próximo update mañana 10am
        (now.date_naive() + chrono::Days::new(1))
            .and_hms_opt(10, 0, 0)
            .unwrap()
    };
    
    chrono_tz::America::Panama
        .from_local_datetime(&next_update)
        .unwrap()
        .with_timezone(&chrono::Utc)
        .to_rfc3339()
}

/// Comprime JSON con GZIP
fn compress_json(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

/// Descomprime GZIP a JSON
/// Nota: Función mantenida para compatibilidad futura, actualmente no usada
/// debido a optimización que evita descompresión innecesaria
#[allow(dead_code)]
fn decompress_json(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

// ============================================================================
// DATABASE QUERIES
// ============================================================================

/// Query a PostgreSQL para obtener ofertas
async fn fetch_ofertasws_from_db(pool: &PgPool) -> Result<Vec<Oferta>, sqlx::Error> {
    // Usar query sin macro para evitar errores en compile-time cuando tabla no existe
    let rows = sqlx::query(
        r#"
        SELECT 
            comercio,
            producto,
            codigo,
            precio_actual,
            fecha_actual::text as fecha_actual,
            dias_con_precio_actual,
            precio_anterior,
            fecha_anterior::text as fecha_anterior,
            precio_minimo_60d,
            precio_maximo_60d,
            precio_promedio_60d::double precision as precio_promedio_60d,
            COALESCE(es_precio_mas_bajo, false) as es_precio_mas_bajo,
            porc,
            diferencia,
            link,
            imagen
        FROM wsf_consolidado
        WHERE abs(precio_anterior - precio_actual) > 3
          AND precio_actual IS NOT NULL 
          AND precio_anterior IS NOT NULL
          AND precio_actual <> precio_anterior
          AND NOT (precio_actual = 0 OR precio_anterior = 0)
        ORDER BY diferencia DESC
        LIMIT 7000
        "#
    )
    .fetch_all(pool)
    .await?;

    let ofertas = rows
        .into_iter()
        .map(|row| Oferta {
            comercio: row.get("comercio"),
            producto: row.get("producto"),
            codigo: row.get("codigo"),
            precio_actual: row.get("precio_actual"),
            fecha_actual: row.get("fecha_actual"),
            dias_con_precio_actual: row.get("dias_con_precio_actual"),
            precio_anterior: row.get("precio_anterior"),
            fecha_anterior: row.get("fecha_anterior"),
            precio_minimo_60d: row.get("precio_minimo_60d"),
            precio_maximo_60d: row.get("precio_maximo_60d"),
            precio_promedio_60d: row.get("precio_promedio_60d"),
            es_precio_mas_bajo: row.get("es_precio_mas_bajo"),
            porc: row.get("porc"),
            diferencia: row.get("diferencia"),
            link: row.get("link"),
            imagen: row.get("imagen"),
        })
        .collect();

    Ok(ofertas)
}

/// Registra log de ejecución en PostgreSQL
pub async fn log_refresh_execution(
    pool: &PgPool,
    status: &str,
    records_count: Option<i32>,
    execution_time_ms: i32,
    request_size_kb: Option<i32>,
    error_message: Option<&str>,
    redis_key: &str,
) -> Result<(), sqlx::Error> {
    // Usar query sin macro para evitar errores en compile-time cuando tabla no existe
    sqlx::query(
        r#"
        INSERT INTO ofertasws_cache_refresh_log 
            (executed_at, status, records_count, execution_time_ms, request_size_kb, error_message, redis_key)
        VALUES 
            (NOW(), $1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(status)
    .bind(records_count)
    .bind(execution_time_ms)
    .bind(request_size_kb)
    .bind(error_message)
    .bind(redis_key)
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// CACHE LAYER
// ============================================================================

/// Obtiene ofertas desde cache o DB, con logging
/// Devuelve: (compressed_data, etag, records_count)
pub async fn get_ofertasws_cached(
    ws_pool: &PgPool,
    redis_pool: &deadpool_redis::Pool,
) -> Result<(Vec<u8>, String, usize), String> {
    let cache_key = get_cache_key();
    let etag = get_etag();
    
    // Intentar obtener desde Redis
    let mut redis_conn = redis_pool
        .get()
        .await
        .map_err(|e| format!("Redis connection error: {}", e))?;
    
    let cached: Option<Vec<u8>> = redis_conn
        .get(&cache_key)
        .await
        .map_err(|e| format!("Redis GET error: {}", e))?;
    
    if let Some(compressed_data) = cached {
        tracing::info!("✅ Cache HIT for key: {}", cache_key);
        // En cache hit, extraer count del metadata (evita guardar separadamente)
        // Por ahora retornamos 0, el llamador puede ignorarlo en cache hit
        return Ok((compressed_data, etag, 0));
    }
    
    tracing::warn!("⚠️ Cache MISS for key: {}", cache_key);
    
    // Cache miss: fetch from DB
    let start = std::time::Instant::now();
    
    let ofertas = fetch_ofertasws_from_db(ws_pool)
        .await
        .map_err(|e| format!("Database query error: {}", e))?;
    
    // Guardar count antes de mover ofertas (optimización: evita clone de ~1.4 MB)
    let ofertas_count = ofertas.len();
    
    let response = OfertasWsResponse {
        ofertasws: ofertas,  // Move, no clone
        metadata: OfertasWsMetadata {
            total_count: ofertas_count,
            generated_at: chrono::Utc::now().to_rfc3339(),
            next_update: get_next_update(),
            version: cache_key.clone(),
        },
    };
    
    let json_data = serde_json::to_vec(&SimpleApiResponse::success(response))
        .map_err(|e| format!("JSON serialization error: {}", e))?;
    
    let compressed = compress_json(&json_data)
        .map_err(|e| format!("Compression error: {}", e))?;
    
    // Guardar en Redis con TTL de 12 horas
    let _: () = redis_conn
        .set_ex(&cache_key, &compressed, 12 * 3600)
        .await
        .map_err(|e| format!("Redis SET error: {}", e))?;
    
    let execution_time = start.elapsed().as_millis() as i32;
    
    // Log exitoso: calcular tamaño en KB (redondeado)
    let request_size_kb = ((compressed.len() as f64) / 1024.0).ceil() as i32;
    if let Err(e) = log_refresh_execution(
        ws_pool,
        "success",
        Some(ofertas_count as i32),
        execution_time,
        Some(request_size_kb),
        None,
        &cache_key,
    )
    .await
    {
        tracing::error!("Failed to log refresh execution: {}", e);
    }
    
    tracing::info!(
        "💾 Cache STORED: {} bytes compressed ({} ofertas) in {}ms",
        compressed.len(),
        ofertas_count,
        execution_time
    );
    
    Ok((compressed, etag, ofertas_count))
}

// ============================================================================
// API ENDPOINTS
// ============================================================================

/// GET /api/v4/ofertas
/// Devuelve ofertas con cache + E-Tag + GZIP
pub async fn get_ofertasws(
    State(state): State<Arc<GlobalAppState>>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, Json<SimpleApiResponse<()>>)> {
    // Verificar que WS pool esté disponible
    let ws_pool = match &state.ws_pool {
        Some(pool) => pool,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(SimpleApiResponse::<()>::error("WS database not configured. Ofertas API is unavailable.")),
            ));
        }
    };
    
    let current_etag = get_etag();
    
    // Check If-None-Match header
    if let Some(client_etag) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(client_etag_str) = client_etag.to_str() {
            if client_etag_str == current_etag {
                tracing::info!("📭 E-Tag match: returning 304 Not Modified");
                return Ok((
                    StatusCode::NOT_MODIFIED,
                    [(header::ETAG, current_etag)],
                ).into_response());
            }
        }
    }
    
    // Obtener ofertas (cache o DB)
    match get_ofertasws_cached(ws_pool, &state.redis_pool).await {
        Ok((compressed_data, etag, _count)) => {
            tracing::info!(
                "📦 Serving ofertasws: {} bytes compressed",
                compressed_data.len()
            );
            
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/json"),
                    (header::CONTENT_ENCODING, "gzip"),
                    (header::ETAG, etag.as_str()),
                    (header::CACHE_CONTROL, "public, max-age=18000"), // 5 horas
                ],
                compressed_data,
            ).into_response())
        }
        Err(e) => {
            tracing::error!("❌ Error fetching ofertasws: {}", e);
            
            // Log error (si ws_pool está disponible)
            if let Some(ref ws_pool) = state.ws_pool {
                let _ = log_refresh_execution(
                    ws_pool,
                    "error",
                    None,
                    0,
                    None,
                    Some(&e),
                    &get_cache_key(),
                )
                .await;
            }
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SimpleApiResponse::<()>::error(&format!("Failed to fetch ofertasws: {}", e))),
            ))
        }
    }
}

/// POST /api/v4/ofertas/refresh
/// Invalida cache y fuerza refresh (protegido por JWT middleware)
pub async fn refresh_ofertasws_cache(
    State(state): State<Arc<GlobalAppState>>,
) -> Result<Json<SimpleApiResponse<serde_json::Value>>, (StatusCode, Json<SimpleApiResponse<()>>)> {
    // Verificar que WS pool esté disponible
    let ws_pool = match &state.ws_pool {
        Some(pool) => pool,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(SimpleApiResponse::<()>::error("WS database not configured. Ofertas API is unavailable.")),
            ));
        }
    };
    
    let start = std::time::Instant::now();
    let cache_key = get_cache_key();
    
    tracing::info!("🔄 Manual cache refresh requested for key: {}", cache_key);
    
    // Invalidar cache actual
    let mut redis_conn = state
        .redis_pool
        .get()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SimpleApiResponse::<()>::error(&format!("Redis connection error: {}", e))),
            )
        })?;
    
    let _: () = redis_conn
        .del(&cache_key)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SimpleApiResponse::<()>::error(&format!("Redis DEL error: {}", e))),
            )
        })?;
    
    // Forzar regeneración
    match get_ofertasws_cached(ws_pool, &state.redis_pool).await {
        Ok((compressed_data, _, count)) => {
            let execution_time = start.elapsed().as_millis() as i32;
            
            // Ya tenemos el count del resultado - no necesitamos descomprimir (optimización: -120ms)
            
            tracing::info!(
                "✅ Cache refreshed: {} ofertas, {} bytes, {}ms",
                count,
                compressed_data.len(),
                execution_time
            );
            
            Ok(Json(SimpleApiResponse::success(serde_json::json!({
                "message": "Cache refreshed successfully",
                "records_count": count as i32,
                "compressed_size_bytes": compressed_data.len(),
                "execution_time_ms": execution_time,
                "cache_key": cache_key,
            }))))
        }
        Err(e) => {
            let execution_time = start.elapsed().as_millis() as i32;
            
            // Log error (si ws_pool está disponible)
            if let Some(ref ws_pool) = state.ws_pool {
                let _ = log_refresh_execution(
                    ws_pool,
                    "error",
                    None,
                    execution_time,
                    None,
                    Some(&e),
                    &cache_key,
                )
                .await;
            }
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SimpleApiResponse::<()>::error(&format!("Cache refresh failed: {}", e))),
            ))
        }
    }
}
