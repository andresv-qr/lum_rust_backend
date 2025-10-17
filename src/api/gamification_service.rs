use sqlx::{PgPool, Row};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Estructura simplificada para la respuesta de Lumis
#[derive(Debug, Serialize, Deserialize)]
pub struct LumisResult {
    pub lumis_earned: i32,
    pub lumis_balance: i32,
}

/// Acredita Lumis al usuario después de procesar una factura exitosamente
/// 
/// Este flujo replica EXACTAMENTE la implementación de Python/WhatsApp:
/// 1. Consulta la regla de acumulación activa (id=0) desde rewards.dim_accumulations
/// 2. Inserta registro en rewards.fact_accumulations (trigger automático actualiza balance)
/// 3. Consulta el balance actualizado desde rewards.fact_balance_points
pub async fn credit_lumis_for_invoice(
    pool: &PgPool,
    user_id: i64,
    cufe: &str,
) -> Result<LumisResult, sqlx::Error> {
    // 1. Consultar la regla de acumulación activa (ID = 0)
    let rule = sqlx::query_as::<_, (i32, String, i32)>(
        r#"
        SELECT id, name, points
        FROM rewards.dim_accumulations
        WHERE id = 0 
        AND valid_from <= NOW() 
        AND valid_to >= NOW()
        "#
    )
    .fetch_optional(pool)
    .await?;
    
    let (rule_id, rule_name, points) = match rule {
        Some((id, name, pts)) => {
            tracing::info!("�� Found accumulation rule: '{}' (id={}) with {} Lumis", name, id, pts);
            (id, name, pts)
        },
        None => {
            tracing::warn!("⚠️ No active accumulation rule found (id=0)");
            return Err(sqlx::Error::RowNotFound);
        }
    };
    
    // 2. Registrar acumulación en rewards.fact_accumulations
    // El TRIGGER de PostgreSQL actualizará automáticamente rewards.fact_balance_points
    let current_time = Utc::now().naive_utc();
    
    sqlx::query(
        r#"
        INSERT INTO rewards.fact_accumulations 
        (user_id, accum_type, accum_key, dtype, quantity, date, accum_id)
        VALUES ($1, $2, $3, 'points', $4, $5, $6)
        "#
    )
    .bind(user_id)
    .bind(&rule_name)
    .bind(cufe)
    .bind(points)
    .bind(current_time)
    .bind(rule_id)
    .execute(pool)
    .await?;
    
    tracing::info!("✅ Recorded accumulation for user {} - {} Lumis (CUFE: {})", user_id, points, cufe);
    
    // 3. Consultar el balance actualizado (el trigger ya lo actualizó)
    let new_balance = get_user_balance(pool, user_id).await?;
    
    tracing::info!("💰 New balance for user {}: {} Lumis", user_id, new_balance);
    
    Ok(LumisResult {
        lumis_earned: points,
        lumis_balance: new_balance,
    })
}

/// Obtiene el balance actual de Lumis del usuario desde rewards.fact_balance_points
pub async fn get_user_balance(pool: &PgPool, user_id: i64) -> Result<i32, sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT COALESCE(balance, 0)::INTEGER as balance
        FROM rewards.fact_balance_points
        WHERE user_id = $1
        "#
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    
    if let Some(row) = result {
        let balance: i32 = row.get("balance");
        Ok(balance)
    } else {
        // Usuario nuevo sin balance aún
        Ok(0)
    }
}
