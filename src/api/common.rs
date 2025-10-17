use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, error};
use uuid::Uuid;
use sqlx::PgPool;
use serde_json::Value;

use crate::state::AppState;


/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub request_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub execution_time_ms: Option<u64>,
    pub cached: bool,
}

/// Standard API error structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Database query with caching configuration
#[derive(Debug, Clone)]
pub struct CachedQuery {
    pub sql: String,
    pub cache_key_prefix: String,
    pub cache_ttl_seconds: u64,
    pub invalidate_on_write: bool,
}

/// Query parameters for database operations
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryParams {
    pub filters: Option<Value>,
    pub pagination: Option<PaginationParams>,
    pub sorting: Option<SortingParams>,
}

/// Pagination parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: u32,
    pub limit: u32,
}

/// Sorting parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct SortingParams {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T, request_id: String, execution_time_ms: Option<u64>, cached: bool) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id,
            timestamp: chrono::Utc::now(),
            execution_time_ms,
            cached,
        }
    }

    pub fn error(error: ApiError, request_id: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
            request_id,
            timestamp: chrono::Utc::now(),
            execution_time_ms: None,
            cached: false,
        }
    }
}

/// Helper struct for simplified responses (daily game endpoints)
#[derive(Debug, Serialize)]
pub struct SimpleApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SimpleApiError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SimpleApiError {
    pub code: String,
    pub message: String,
}

impl<T> SimpleApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }
    
    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: Some(message),
        }
    }
    
    pub fn error(message: &str) -> SimpleApiResponse<()> {
        SimpleApiResponse {
            success: false,
            data: None,
            error: Some(SimpleApiError {
                code: "ERROR".to_string(),
                message: message.to_string(),
            }),
            message: None,
        }
    }
    
    pub fn error_with_code(code: &str, message: &str) -> SimpleApiResponse<()> {
        SimpleApiResponse {
            success: false,
            data: None,
            error: Some(SimpleApiError {
                code: code.to_string(),
                message: message.to_string(),
            }),
            message: None,
        }
    }
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }

    pub fn validation_error(message: &str) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    pub fn database_error(message: &str) -> Self {
        Self::new("DATABASE_ERROR", message)
    }

    pub fn not_found(resource: &str) -> Self {
        Self::new("NOT_FOUND", &format!("{} not found", resource))
    }

    pub fn cache_error(message: &str) -> Self {
        Self::new("CACHE_ERROR", message)
    }

    pub fn bad_request(message: &str) -> Self {
        Self::new("BAD_REQUEST", message)
    }

    pub fn internal_server_error(message: &str) -> Self {
        Self::new("INTERNAL_SERVER_ERROR", message)
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::new("UNAUTHORIZED", message)
    }

    pub fn too_many_requests(message: &str) -> Self {
        Self::new("TOO_MANY_REQUESTS", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "VALIDATION_ERROR" | "BAD_REQUEST" => StatusCode::BAD_REQUEST,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "TOO_MANY_REQUESTS" => StatusCode::TOO_MANY_REQUESTS,
            "DATABASE_ERROR" | "CACHE_ERROR" | "INTERNAL_SERVER_ERROR" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let response = ApiResponse::<()>::error(self, Uuid::new_v4().to_string());
        (status, Json(response)).into_response()
    }
}

/// Database service with intelligent caching
pub struct DatabaseService {
    pub pool: PgPool,
    pub user_cache: crate::cache::UserCache,
}

impl DatabaseService {
    pub fn new(pool: PgPool, user_cache: crate::cache::UserCache) -> Self {
        Self { pool, user_cache }
    }

    /// Execute a simple cached query (simplified version)
    pub async fn execute_simple_query<T>(
        &self,
        sql: &str,
        cache_key: &str,
    ) -> Result<(Vec<T>, bool), ApiError>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Serialize + for<'de> Deserialize<'de> + Clone + Send + Unpin,
    {
        // Try to get from cache first (simplified for now)
        // TODO: Implement generic caching when user_cache supports generic types
        // if let Some(cached_data) = self.user_cache.get(cache_key) {
        //     info!("Cache hit for key: {}", cache_key);
        //     return Ok((cached_data, true));
        // }
        
        info!("Cache miss for key: {}, executing database query", cache_key);
        
        // Execute database query
        let start_time = Instant::now();
        let rows = sqlx::query_as::<_, T>(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Database query failed: {}", e);
                ApiError::database_error(&format!("Query execution failed: {}", e))
            })?;

        let query_time = start_time.elapsed();
        info!("Database query completed in {:?}", query_time);

        // TODO: Store in cache for future requests
        // self.user_cache.set(cache_key, &rows, cache_ttl).await;

        Ok((rows, false))
    }

    /// Execute a cached query with parameters (for user-specific queries with user_id and offset)
    pub async fn execute_query_with_params<T>(
        &self,
        sql: &str,
        user_id: i64,
        offset: i64,
        cache_key: &str,
    ) -> Result<(Vec<T>, bool), ApiError>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Serialize + for<'de> Deserialize<'de> + Clone + Send + Unpin,
    {
        info!("Cache miss for key: {}, executing database query with user_id: {} and offset: {}", cache_key, user_id, offset);
        
        // Execute database query with parameters
        let start_time = Instant::now();
        
        let rows = sqlx::query_as::<_, T>(sql)
            .bind(user_id)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Database query with parameters failed: {}", e);
                ApiError::database_error(&format!("Query execution failed: {}", e))
            })?;

        let query_time = start_time.elapsed();
        info!("Database query with parameters completed in {:?}", query_time);

        // Cache the results (simplified for now)
        // TODO: Implement generic caching when user_cache supports generic types
        // if let Err(e) = self.user_cache.set(cache_key.to_string(), &rows) {
        //     error!("Failed to cache results for key {}: {}", cache_key, e);
        // } else {
            info!("Would cache {} results for key: {}", rows.len(), cache_key);
        // }

        Ok((rows, false))
    }

    /// Execute a single row query with ID parameter
    pub async fn execute_single_query_with_id<T>(
        &self,
        sql: &str,
        id: i64,
        cache_key: &str,
    ) -> Result<(Option<T>, bool), ApiError>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Serialize + for<'de> Deserialize<'de> + Clone + Send + Unpin,
    {
        // Try to get from cache first (simplified for now)
        // TODO: Implement generic caching when user_cache supports generic types
        // if let Some(cached_data) = self.user_cache.get(cache_key) {
        //     info!("Cache hit for single row key: {}", cache_key);
        //     return Ok((cached_data, true));
        // }
        
        info!("Cache miss for single row key: {}, executing database query", cache_key);
        
        // Execute database query
        let row = sqlx::query_as::<_, T>(sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Database query failed: {}", e);
                ApiError::database_error(&format!("Query execution failed: {}", e))
            })?;

        // Cache the result (simplified for now)
        // TODO: Implement generic caching when user_cache supports generic types
        // if let Err(e) = self.user_cache.set(cache_key.to_string(), &row) {
        //     error!("Failed to cache single row result for key {}: {}", cache_key, e);
        // } else {
            info!("Would cache single row result for key: {}", cache_key);
        // }

        Ok((row, false))
    }



    /// Execute a simple write operation
    pub async fn execute_write_with_params<P1, P2>(
        &self,
        sql: &str,
        param1: P1,
        param2: P2,
    ) -> Result<u64, ApiError>
    where
        P1: for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send,
        P2: for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send,
    {
        info!("Executing write operation");
        
        let result = sqlx::query(sql)
            .bind(param1)
            .bind(param2)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Write operation failed: {}", e);
                ApiError::database_error(&format!("Write operation failed: {}", e))
            })?;

        info!("Write operation completed, affected rows: {}", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// Execute a simple write operation with three parameters
    pub async fn execute_write_with_three_params<P1, P2, P3>(
        &self,
        sql: &str,
        param1: P1,
        param2: P2,
        param3: P3,
    ) -> Result<u64, ApiError>
    where
        P1: for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send,
        P2: for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send,
        P3: for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send,
    {
        info!("Executing write operation with three params");
        
        let result = sqlx::query(sql)
            .bind(param1)
            .bind(param2)
            .bind(param3)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Write operation failed: {}", e);
                ApiError::database_error(&format!("Write operation failed: {}", e))
            })?;

        info!("Write operation completed, affected rows: {}", result.rows_affected());
        Ok(result.rows_affected())
    }

}

/// Middleware for automatic request logging and metrics
pub async fn request_logging_middleware(
    State(_state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Add request ID to headers for downstream handlers
    request.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );

    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "API request started"
    );

    // Update performance metrics
    // TODO: Add performance metrics update through public methods
    info!("Request started - would update performance metrics");

    let response = next.run(request).await;
    let execution_time = start_time.elapsed();

    // Update performance metrics after request
    // TODO: Add performance metrics update through public methods
    info!("Request completed in {:?} - would update performance metrics", execution_time);

    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        status = %response.status(),
        execution_time_ms = execution_time.as_millis(),
        "API request completed"
    );

    response
}

/// Helper macro for creating simple database API handlers
#[macro_export]
macro_rules! simple_query_handler {
    ($handler_name:ident, $response_type:ty, $sql:expr) => {
        pub async fn $handler_name(
            State(state): State<Arc<AppState>>,
            headers: HeaderMap,
            Json(_params): Json<QueryParams>,
        ) -> Result<Json<ApiResponse<Vec<$response_type>>>, ApiError> {
            let request_id = headers
                .get("x-request-id")
                .and_then(|h| h.to_str().ok())
                .unwrap_or(&uuid::Uuid::new_v4().to_string())
                .to_string();

            let start_time = std::time::Instant::now();
            let db_service = DatabaseService::new(
                state.db_pool.clone(),
                state.user_cache.clone()
            );

            let cache_key = format!("{}_{}", stringify!($handler_name), "list");
            let (data, cached) = db_service
                .execute_simple_query::<$response_type>($sql, &cache_key)
                .await?;

            let execution_time = start_time.elapsed().as_millis() as u64;
            Ok(Json(ApiResponse::success(data, request_id, Some(execution_time), cached)))
        }
    };
}

/// Helper macro for creating single row database API handlers
#[macro_export]
macro_rules! simple_single_query_handler {
    ($handler_name:ident, $response_type:ty, $sql:expr) => {
        pub async fn $handler_name(
            State(state): State<Arc<AppState>>,
            headers: HeaderMap,
            axum::extract::Path(id): axum::extract::Path<i64>,
        ) -> Result<Json<ApiResponse<Option<$response_type>>>, ApiError> {
            let request_id = headers
                .get("x-request-id")
                .and_then(|h| h.to_str().ok())
                .unwrap_or(&uuid::Uuid::new_v4().to_string())
                .to_string();

            let start_time = std::time::Instant::now();
            let db_service = DatabaseService::new(
                state.db_pool.clone(),
                state.user_cache.clone()
            );

            let cache_key = format!("{}_{}_{}", stringify!($handler_name), "single", id);
            let (data, cached) = db_service
                .execute_single_query_with_id::<$response_type>($sql, id, &cache_key)
                .await?;

            let execution_time = start_time.elapsed().as_millis() as u64;
            Ok(Json(ApiResponse::success(data, request_id, Some(execution_time), cached)))
        }
    };
}
