//! Helpers y queries para sincronización incremental
//! 
//! Este módulo proporciona funciones utilitarias para:
//! - Calcular checksums de datos
//! - Obtener versiones de datasets
//! - Query de items eliminados
//! - Validaciones de integridad

use sqlx::PgPool;
use sha2::{Sha256, Digest};
use serde::Serialize;

use super::sync_types::DeletedItem;
// use super::sync_types::VersionResponse;  // DEPRECATED

/// Calcular checksum SHA256 de datos serializados
/// 
/// # Arguments
/// * `data` - Referencia a cualquier tipo serializable
/// 
/// # Returns
/// String en formato "sha256:hexadecimal"
/// 
/// # Example
/// ```
/// let products = vec![product1, product2];
/// let checksum = calculate_checksum(&products)?;
/// // "sha256:a1b2c3d4..."
/// ```
pub fn calculate_checksum<T: Serialize>(data: &T) -> Result<String, serde_json::Error> {
    let data_json = serde_json::to_string(data)?;
    
    let mut hasher = Sha256::new();
    hasher.update(data_json.as_bytes());
    let result = hasher.finalize();
    
    Ok(format!("sha256:{:x}", result))
}

/// Obtener versión actual de un dataset
/// 
/// # Arguments
/// * `pool` - Pool de conexiones PostgreSQL
/// * `table_name` - Nombre de la tabla (dim_product, dim_issuer, invoice_header, invoice_detail)
/// 
/// # Returns
/// Número de versión actual (i64) o 0 si no existe
/// 
/// # Example
/// ```
/// let version = get_dataset_version(&pool, "dim_product").await?;
/// println!("Current version: {}", version);
/// ```
/// 
/// DEPRECATED: No longer needed - dataset_versions removed for multi-user scenario
/*
pub async fn get_dataset_version(
    pool: &PgPool,
    table_name: &str,
) -> Result<i64, sqlx::Error> {
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT version FROM public.dataset_versions WHERE table_name = $1"
    )
    .bind(table_name)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    Ok(version)
}
*/

/// Obtener información completa de versión de un dataset
/// 
/// # Arguments
/// * `pool` - Pool de conexiones PostgreSQL
/// * `table_name` - Nombre de la tabla
/// 
/// # Returns
/// VersionResponse con version, last_modified, server_timestamp
/// 
/// DEPRECATED: No longer needed - dataset_versions removed for multi-user scenario
/*
pub async fn get_version_info(
    pool: &PgPool,
    table_name: &str,
) -> Result<VersionResponse, sqlx::Error> {
    let row = sqlx::query(
        "SELECT version, last_modified FROM public.dataset_versions WHERE table_name = $1"
    )
    .bind(table_name)
    .fetch_one(pool)
    .await?;
    
    let version: i64 = row.try_get("version")?;
    let last_modified: chrono::NaiveDateTime = row.try_get("last_modified")?;
    
    Ok(VersionResponse {
        dataset_version: version,
        last_modified,
        server_timestamp: chrono::Utc::now().naive_utc(),
        total_records: None,
    })
}
*/

/// Query template para obtener items eliminados desde una fecha
/// 
/// # Arguments
/// * `table_name` - Nombre de la tabla
/// * `id_column` - Nombre de la columna que contiene el ID (ej: "code", "cufe", "issuer_ruc")
/// 
/// # Returns
/// String SQL query listo para usar con bind()
pub fn get_deleted_items_query(table_name: &str, id_column: &str) -> String {
    format!(
        r#"
        SELECT 
            {} as id,
            deleted_at
        FROM public.{}
        WHERE is_deleted = TRUE
          AND deleted_at >= $1
        ORDER BY deleted_at DESC
        LIMIT 1000
        "#,
        id_column, table_name
    )
}

/// Obtener items eliminados desde una fecha específica
/// 
/// # Arguments
/// * `pool` - Pool de conexiones PostgreSQL
/// * `table_name` - Nombre de la tabla
/// * `id_column` - Columna de ID
/// * `since` - Timestamp desde el cual buscar eliminaciones
/// 
/// # Returns
/// Vector de DeletedItem o vector vacío si hay error
pub async fn get_deleted_items_since(
    pool: &PgPool,
    table_name: &str,
    id_column: &str,
    since: &str,
) -> Vec<DeletedItem> {
    let query = get_deleted_items_query(table_name, id_column);
    
    sqlx::query_as::<_, DeletedItem>(&query)
        .bind(since)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

/// Obtener total de registros activos (no eliminados) en una tabla
/// 
/// # Arguments
/// * `pool` - Pool de conexiones PostgreSQL
/// * `table_name` - Nombre de la tabla
/// 
/// # Returns
/// Número total de registros activos
pub async fn get_total_active_records(
    pool: &PgPool,
    table_name: &str,
) -> Result<i64, sqlx::Error> {
    let query = format!(
        "SELECT COUNT(*) as total FROM public.{} WHERE is_deleted = FALSE",
        table_name
    );
    
    let total = sqlx::query_scalar::<_, i64>(&query)
        .fetch_one(pool)
        .await?;
    
    Ok(total)
}

/// Validar formato de fecha para update_date_from parameter
/// 
/// # Arguments
/// * `date_str` - String de fecha a validar
/// 
/// # Returns
/// Ok(date_str) si es válida, Err con mensaje de error si no
/// 
/// # Formatos aceptados
/// - ISO 8601 con timezone: "2025-11-07T10:30:45Z"
/// - ISO 8601 con millis: "2025-11-07T10:30:45.123Z"
/// - Solo fecha: "2025-11-07" (asume 00:00:00)
/// - DateTime sin timezone: "2025-11-07T10:30:45" (asume UTC)
pub fn validate_date_format(date_str: &str) -> Result<String, String> {
    // Intentar parsear como DateTime con timezone (RFC3339)
    if chrono::DateTime::parse_from_rfc3339(date_str).is_ok() {
        return Ok(date_str.to_string());
    }
    
    // Intentar parsear como NaiveDateTime (sin timezone) - formato ISO 8601 local
    // Formatos: "2025-11-07T10:30:45" o "2025-11-07T10:30:45.123"
    if chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S").is_ok() {
        return Ok(date_str.to_string());
    }
    if chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%.f").is_ok() {
        return Ok(date_str.to_string());
    }
    
    // Intentar parsear como fecha sola (YYYY-MM-DD)
    if chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok() {
        return Ok(date_str.to_string());
    }
    
    Err(format!(
        "Invalid date format '{}'. Use ISO 8601 format (e.g., 2025-11-07T10:30:45Z, 2025-11-07T10:30:45, or 2025-11-07)",
        date_str
    ))
}

/// Parsear string de fecha a NaiveDateTime para usar con SQLx/PostgreSQL
/// 
/// # Arguments
/// * `date_str` - String de fecha en formato ISO 8601
/// 
/// # Returns
/// Ok(NaiveDateTime) si se parsea correctamente, Err con mensaje de error si no
/// 
/// # Formatos aceptados
/// - ISO 8601 con timezone: "2025-11-07T10:30:45Z" o "2025-11-07T10:30:45.123456Z"
/// - DateTime sin timezone: "2025-11-07T10:30:45"
/// - Solo fecha: "2025-11-07" (asume 00:00:00)
pub fn parse_date_to_naive(date_str: &str) -> Result<chrono::NaiveDateTime, String> {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
    
    // Intentar parsear como DateTime con timezone (RFC3339) y convertir a NaiveDateTime
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.naive_utc());
    }
    
    // Intentar parsear como NaiveDateTime con microsegundos
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(naive_dt);
    }
    
    // Intentar parsear como NaiveDateTime sin fracciones de segundo
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
        return Ok(naive_dt);
    }
    
    // Intentar parsear como fecha sola (YYYY-MM-DD) - asumir 00:00:00
    if let Ok(naive_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Ok(NaiveDateTime::new(naive_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
    }
    
    Err(format!(
        "Invalid date format '{}'. Use ISO 8601 format (e.g., 2025-11-07T10:30:45Z, 2025-11-07T10:30:45, or 2025-11-07)",
        date_str
    ))
}

/// Extraer max update_date de un vector de items
/// 
/// Este es un helper genérico que funciona con cualquier tipo que tenga
/// un campo update_date: Option<chrono::NaiveDateTime>
/// 
/// # Type Parameters
/// * `T` - Tipo que implementa el trait HasUpdateDate
/// 
/// # Arguments
/// * `items` - Slice de items a analizar
/// 
/// # Returns
/// Option con el timestamp más reciente, o None si no hay items o ninguno tiene update_date
pub fn extract_max_update_date<T>(items: &[T]) -> Option<chrono::NaiveDateTime>
where
    T: HasUpdateDate,
{
    items
        .iter()
        .filter_map(|item| item.get_update_date())
        .max()
}

/// Trait para tipos que tienen campo update_date
/// 
/// Implementar este trait permite usar extract_max_update_date
pub trait HasUpdateDate {
    fn get_update_date(&self) -> Option<chrono::NaiveDateTime>;
}

/// Extraer IDs de un vector de items
/// 
/// Helper genérico para construir la lista de record_ids en sync_metadata
/// 
/// # Type Parameters
/// * `T` - Tipo que implementa el trait HasId
/// 
/// # Arguments
/// * `items` - Slice de items
/// 
/// # Returns
/// Vector de Strings con los IDs
pub fn extract_record_ids<T>(items: &[T]) -> Vec<String>
where
    T: HasId,
{
    items
        .iter()
        .filter_map(|item| item.get_id())
        .collect()
}

/// Trait para tipos que tienen un ID único
pub trait HasId {
    fn get_id(&self) -> Option<String>;
}

/// Query template para count con date filter
pub fn get_count_query_with_filter(table_name: &str) -> String {
    format!(
        r#"
        SELECT COUNT(*) as total
        FROM public.{}
        WHERE is_deleted = FALSE
          AND update_date >= $1
        "#,
        table_name
    )
}

/// Query template para count sin filter
pub fn get_count_query(table_name: &str) -> String {
    format!(
        r#"
        SELECT COUNT(*) as total
        FROM public.{}
        WHERE is_deleted = FALSE
        "#,
        table_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    
    #[derive(Serialize)]
    struct TestData {
        id: i32,
        name: String,
    }
    
    #[test]
    fn test_calculate_checksum() {
        let data = vec![
            TestData { id: 1, name: "Test 1".to_string() },
            TestData { id: 2, name: "Test 2".to_string() },
        ];
        
        let checksum = calculate_checksum(&data).unwrap();
        assert!(checksum.starts_with("sha256:"));
        assert!(checksum.len() > 10);
    }
    
    #[test]
    fn test_calculate_checksum_deterministic() {
        let data1 = vec![
            TestData { id: 1, name: "Test".to_string() },
        ];
        let data2 = vec![
            TestData { id: 1, name: "Test".to_string() },
        ];
        
        let checksum1 = calculate_checksum(&data1).unwrap();
        let checksum2 = calculate_checksum(&data2).unwrap();
        
        assert_eq!(checksum1, checksum2);
    }
    
    #[test]
    fn test_validate_date_format_iso8601() {
        assert!(validate_date_format("2025-11-07T10:30:45Z").is_ok());
        assert!(validate_date_format("2025-11-07T10:30:45.123Z").is_ok());
        assert!(validate_date_format("2025-11-07T10:30:45-05:00").is_ok());
    }
    
    #[test]
    fn test_validate_date_format_date_only() {
        assert!(validate_date_format("2025-11-07").is_ok());
        assert!(validate_date_format("2024-01-15").is_ok());
    }
    
    #[test]
    fn test_validate_date_format_invalid() {
        assert!(validate_date_format("invalid-date").is_err());
        assert!(validate_date_format("2025/11/07").is_err());
        assert!(validate_date_format("11-07-2025").is_err());
    }
    
    #[test]
    fn test_get_deleted_items_query() {
        let query = get_deleted_items_query("dim_product", "code");
        assert!(query.contains("code as id"));
        assert!(query.contains("dim_product"));
        assert!(query.contains("is_deleted = TRUE"));
    }
    
    #[test]
    fn test_get_count_query() {
        let query = get_count_query("dim_product");
        assert!(query.contains("COUNT(*) as total"));
        assert!(query.contains("dim_product"));
        assert!(query.contains("is_deleted = FALSE"));
    }
}
