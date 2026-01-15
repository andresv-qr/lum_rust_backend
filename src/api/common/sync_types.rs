//! Tipos comunes para sincronización incremental
//! 
//! Este módulo define las estructuras de datos para el sistema de sincronización
//! incremental Nivel 2, que garantiza integridad de datos entre backend y frontend.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Response estructura para endpoints de sincronización incremental
/// 
/// Esta estructura unifica la respuesta de todos los endpoints que soportan
/// sincronización incremental (products, issuers, headers, details).
/// 
/// # Type Parameters
/// * `T` - El tipo de datos específico (UserProductsResponse, UserIssuersResponse, etc.)
/// 
/// # Fields
/// * `data` - Array de items retornados (nuevos/modificados desde last sync)
/// * `pagination` - Información de paginación completa
/// * `sync_metadata` - Metadata para validación de integridad y próximo sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalSyncResponse<T> {
    /// Datos retornados (nuevos/modificados)
    pub data: Vec<T>,
    
    /// Información de paginación
    pub pagination: PaginationInfo,
    
    /// Metadata de sincronización para integridad
    pub sync_metadata: SyncMetadata,
}

/// Información de paginación detallada
/// 
/// Proporciona toda la información necesaria para navegar entre páginas
/// y validar que se recibieron todos los datos esperados.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// Total de registros en el dataset completo (sin filtrar por fecha)
    pub total_records: i64,
    
    /// Número de registros retornados en esta respuesta
    pub returned_records: usize,
    
    /// Límite aplicado en esta query
    pub limit: i64,
    
    /// Offset aplicado en esta query
    pub offset: i64,
    
    /// Indica si hay más páginas disponibles
    pub has_more: bool,
    
    /// Total de páginas con el límite actual
    pub total_pages: i64,
    
    /// Página actual (basada en offset/limit)
    pub current_page: i64,
}

/// Metadata de sincronización para validación de integridad
/// 
/// Esta estructura contiene toda la información necesaria para que el frontend:
/// 1. Valide la integridad de los datos recibidos (checksums, conteos)
/// 2. Detecte cambios globales en el dataset (version tracking)
/// 3. Sepa cuándo hacer el próximo sync incremental (max_update_date)
/// 4. Aplique eliminaciones (deleted_since)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// Timestamp del registro MÁS RECIENTE en esta respuesta
    /// 
    /// El frontend debe guardar este valor y usarlo como `update_date_from`
    /// en la próxima petición de sync incremental.
    /// 
    /// Esto resuelve race conditions: si un registro se modifica DESPUÉS
    /// de que el servidor generó esta respuesta, será capturado en el
    /// próximo sync.
    pub max_update_date: Option<DateTime<Utc>>,
    
    /// Timestamp del servidor al momento de generar esta respuesta
    /// 
    /// Útil para detectar desfases de reloj entre cliente y servidor.
    pub server_timestamp: DateTime<Utc>,
    
    /// SHA256 checksum de los datos retornados
    /// 
    /// El frontend puede calcular el checksum localmente y compararlo
    /// con este valor para detectar corrupción de datos en tránsito.
    /// 
    /// Formato: "sha256:abc123def456..."
    pub data_checksum: String,
    
    /// Lista de IDs de los registros retornados
    /// 
    /// Útil para:
    /// 1. Validar que record_ids.length == data.length
    /// 2. Debugging (ver exactamente qué IDs se retornaron)
    /// 3. Deduplicación en el frontend
    pub record_ids: Vec<String>,
    
    /// Número de registros retornados (duplicado para validación)
    /// 
    /// El frontend debe verificar:
    /// - returned_records == data.length
    /// - returned_records == record_ids.length
    /// - returned_records == pagination.returned_records
    pub returned_records: usize,
    
    /// Items eliminados desde el último sync
    /// 
    /// Solo se llena si el cliente proveyó `update_date_from`.
    /// Contiene los IDs de registros que fueron soft-deleted desde esa fecha.
    pub deleted_since: DeletedItems,
}

/// Información de items eliminados (soft delete tracking)
/// 
/// Permite al frontend eliminar registros de su base de datos local
/// que fueron marcados como eliminados en el backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedItems {
    /// Indica si el tracking de deletes está habilitado
    /// 
    /// Siempre true en Nivel 2. Útil para feature flags.
    pub enabled: bool,
    
    /// Número de items eliminados desde last sync
    pub count: usize,
    
    /// Lista de items eliminados con sus IDs y timestamps
    pub items: Vec<DeletedItem>,
}

/// Información de un item individual eliminado
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeletedItem {
    /// ID del item eliminado (puede ser code, cufe, ruc, etc.)
    pub id: String,
    
    /// Timestamp cuando fue marcado como eliminado
    pub deleted_at: DateTime<Utc>,
}

/// Response para endpoints de version check (/api/v4/invoices/{resource}/version)
/// 
/// Este endpoint ligero permite al frontend verificar si el dataset cambió
/// sin necesidad de descargar datos.
/// 
/// # Uso típico:
/// ```javascript
/// // Frontend hace polling cada 5 minutos
/// const response = await fetch('/api/v4/invoices/products/version');
/// if (response.dataset_version > localVersion) {
///     // Hay cambios, hacer sync incremental
///     await syncProductsIncremental();
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    /// Versión actual del dataset
    pub dataset_version: i64,
    
    /// Timestamp de la última modificación al dataset
    pub last_modified: DateTime<Utc>,
    
    /// Timestamp del servidor al generar esta respuesta
    pub server_timestamp: DateTime<Utc>,
    
    /// Total de registros en el dataset (opcional, puede ser costoso de calcular)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<i64>,
}

/// Request parameters comunes para endpoints de sync incremental
#[derive(Debug, Clone, Deserialize)]
pub struct IncrementalSyncRequest {
    /// Filtrar registros actualizados desde esta fecha (ISO 8601)
    /// 
    /// Formato: "2025-11-07T10:30:45.123Z"
    /// 
    /// Si se omite, se retornan todos los registros (primera carga).
    /// Si se provee, solo se retornan registros con update_date >= esta fecha.
    #[serde(default)]
    pub update_date_from: Option<String>,
    
    /// Número máximo de items a retornar (default: 20, max: 100)
    #[serde(default)]
    pub limit: Option<i64>,
    
    /// Número de items a omitir para paginación (default: 0)
    #[serde(default)]
    pub offset: Option<i64>,
}

impl IncrementalSyncRequest {
    /// Obtener limit con default y validación
    pub fn get_limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100).max(1)
    }
    
    /// Obtener offset con default y validación
    pub fn get_offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

/*
/// Helper para construir IncrementalSyncResponse
/// 
/// Simplifica la construcción de responses en los handlers.
/// 
/// DEPRECATED: Not used - handlers build responses manually
pub struct IncrementalSyncResponseBuilder<T> {
    data: Vec<T>,
    total_records: i64,
    limit: i64,
    offset: i64,
    dataset_version: i64,
    deleted_items: Vec<DeletedItem>,
}

impl<T: Clone> IncrementalSyncResponseBuilder<T> {
    pub fn new(data: Vec<T>, total_records: i64, limit: i64, offset: i64) -> Self {
        Self {
            data,
            total_records,
            limit,
            offset,
            dataset_version: 0,
            deleted_items: vec![],
        }
    }
    
    pub fn dataset_version(mut self, version: i64) -> Self {
        self.dataset_version = version;
        self
    }
    
    pub fn deleted_items(mut self, items: Vec<DeletedItem>) -> Self {
        self.deleted_items = items;
        self
    }
    
    pub fn build(self, data_checksum: String, record_ids: Vec<String>) -> IncrementalSyncResponse<T>
    where
        T: serde::Serialize,
    {
        let returned_records = self.data.len();
        let total_pages = (self.total_records as f64 / self.limit as f64).ceil() as i64;
        let current_page = (self.offset / self.limit) + 1;
        let has_more = (self.offset + self.limit) < self.total_records;
        
        // Encontrar max_update_date si T tiene campo update_date
        // (esto requeriría trait specialization, por ahora lo dejamos None
        // y lo calculamos en cada handler específico)
        let max_update_date = None;
        
        IncrementalSyncResponse {
            data: self.data,
            pagination: PaginationInfo {
                total_records: self.total_records,
                returned_records,
                limit: self.limit,
                offset: self.offset,
                has_more,
                total_pages,
                current_page,
            },
            sync_metadata: SyncMetadata {
                max_update_date,
                server_timestamp: chrono::Utc::now().naive_utc(),
                data_checksum,
                record_ids,
                returned_records,
                dataset_version: self.dataset_version,
                deleted_since: DeletedItems {
                    enabled: true,
                    count: self.deleted_items.len(),
                    items: self.deleted_items,
                },
            },
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_sync_request_defaults() {
        let req = IncrementalSyncRequest {
            update_date_from: None,
            limit: None,
            offset: None,
        };
        
        assert_eq!(req.get_limit(), 20);
        assert_eq!(req.get_offset(), 0);
    }
    
    #[test]
    fn test_incremental_sync_request_limit_validation() {
        let req = IncrementalSyncRequest {
            update_date_from: None,
            limit: Some(200), // Excede max
            offset: None,
        };
        
        assert_eq!(req.get_limit(), 100); // Limitado a max
    }
    
    #[test]
    fn test_incremental_sync_request_offset_validation() {
        let req = IncrementalSyncRequest {
            update_date_from: None,
            limit: None,
            offset: Some(-5), // Negativo
        };
        
        assert_eq!(req.get_offset(), 0); // Limitado a min 0
    }
}
