// ============================================================================
// PROMETHEUS METRICS - Sistema de Observabilidad
// ============================================================================
// Métricas para monitoreo en tiempo real con Prometheus/Grafana
// ============================================================================

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_int_counter_vec,
    register_int_gauge_vec, CounterVec, HistogramVec, IntCounterVec, IntGaugeVec,
};

lazy_static! {
    // ========================================================================
    // HTTP REQUEST METRICS
    // ========================================================================
    
    /// Total de requests HTTP por método, endpoint y status
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["method", "endpoint", "status"]
    )
    .unwrap();

    /// Duración de requests HTTP en segundos
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "endpoint"],
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0]
    )
    .unwrap();

    /// Tamaño de respuestas HTTP en bytes
    pub static ref HTTP_RESPONSE_SIZE_BYTES: HistogramVec = register_histogram_vec!(
        "http_response_size_bytes",
        "HTTP response size in bytes",
        &["method", "endpoint"],
        vec![100.0, 1000.0, 5000.0, 10000.0, 50000.0, 100000.0, 500000.0, 1000000.0]
    )
    .unwrap();

    // ========================================================================
    // DATABASE METRICS
    // ========================================================================
    
    /// Total de queries ejecutadas
    pub static ref DB_QUERIES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "db_queries_total",
        "Total number of database queries",
        &["query_type", "table", "status"]
    )
    .unwrap();

    /// Duración de queries en segundos
    pub static ref DB_QUERY_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "db_query_duration_seconds",
        "Database query duration in seconds",
        &["query_type", "table"],
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0]
    )
    .unwrap();

    /// Conexiones activas del pool
    pub static ref DB_CONNECTIONS_ACTIVE: IntGaugeVec = register_int_gauge_vec!(
        "db_connections_active",
        "Number of active database connections",
        &["pool"]
    )
    .unwrap();

    /// Conexiones idle del pool
    pub static ref DB_CONNECTIONS_IDLE: IntGaugeVec = register_int_gauge_vec!(
        "db_connections_idle",
        "Number of idle database connections",
        &["pool"]
    )
    .unwrap();

    // ========================================================================
    // CACHE METRICS
    // ========================================================================
    
    /// Cache hits
    pub static ref CACHE_HITS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_hits_total",
        "Total number of cache hits",
        &["cache_type", "cache_name"]
    )
    .unwrap();

    /// Cache misses
    pub static ref CACHE_MISSES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_misses_total",
        "Total number of cache misses",
        &["cache_type", "cache_name"]
    )
    .unwrap();

    /// Tamaño actual del cache
    pub static ref CACHE_SIZE: IntGaugeVec = register_int_gauge_vec!(
        "cache_size",
        "Current cache size (number of entries)",
        &["cache_type", "cache_name"]
    )
    .unwrap();

    // ========================================================================
    // AUTHENTICATION METRICS
    // ========================================================================
    
    /// Total de intentos de login
    pub static ref AUTH_ATTEMPTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "auth_attempts_total",
        "Total authentication attempts",
        &["auth_type", "status"]
    )
    .unwrap();

    /// JWT tokens generados
    pub static ref JWT_TOKENS_ISSUED: IntCounterVec = register_int_counter_vec!(
        "jwt_tokens_issued",
        "Total JWT tokens issued",
        &["token_type"]
    )
    .unwrap();

    /// JWT tokens validados
    pub static ref JWT_TOKENS_VALIDATED: IntCounterVec = register_int_counter_vec!(
        "jwt_tokens_validated",
        "Total JWT tokens validated",
        &["status"]
    )
    .unwrap();

    // ========================================================================
    // INVOICE PROCESSING METRICS
    // ========================================================================
    
    /// Facturas procesadas
    pub static ref INVOICES_PROCESSED: IntCounterVec = register_int_counter_vec!(
        "invoices_processed_total",
        "Total invoices processed",
        &["source_type", "status"]
    )
    .unwrap();

    /// Tiempo de procesamiento de facturas
    pub static ref INVOICE_PROCESSING_DURATION: HistogramVec = register_histogram_vec!(
        "invoice_processing_duration_seconds",
        "Invoice processing duration in seconds",
        &["source_type"],
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]
    )
    .unwrap();

    // ========================================================================
    // QR DETECTION METRICS
    // ========================================================================
    
    /// QR codes detectados
    pub static ref QR_DETECTIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "qr_detections_total",
        "Total QR codes detected",
        &["detector", "status"]
    )
    .unwrap();

    /// Tiempo de detección QR
    pub static ref QR_DETECTION_DURATION: HistogramVec = register_histogram_vec!(
        "qr_detection_duration_seconds",
        "QR detection duration in seconds",
        &["detector"],
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500]
    )
    .unwrap();

    // ========================================================================
    // OCR PROCESSING METRICS
    // ========================================================================
    
    /// OCR procesados
    pub static ref OCR_PROCESSED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "ocr_processed_total",
        "Total OCR processing attempts",
        &["mode", "status"]
    )
    .unwrap();

    /// Tiempo de procesamiento OCR
    pub static ref OCR_PROCESSING_DURATION: HistogramVec = register_histogram_vec!(
        "ocr_processing_duration_seconds",
        "OCR processing duration in seconds",
        &["mode"],
        vec![0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
    )
    .unwrap();

    // ========================================================================
    // RATE LIMITING METRICS
    // ========================================================================
    
    /// Requests bloqueados por rate limit
    pub static ref RATE_LIMIT_EXCEEDED: IntCounterVec = register_int_counter_vec!(
        "rate_limit_exceeded_total",
        "Total requests blocked by rate limiting",
        &["endpoint", "limit_type"]
    )
    .unwrap();

    // ========================================================================
    // ERROR METRICS
    // ========================================================================
    
    /// Errores por tipo
    pub static ref ERRORS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "errors_total",
        "Total errors by type",
        &["error_type", "component"]
    )
    .unwrap();

    // ========================================================================
    // BUSINESS METRICS
    // ========================================================================
    
    /// Usuarios activos (últimos 24h)
    pub static ref ACTIVE_USERS: IntGaugeVec = register_int_gauge_vec!(
        "active_users",
        "Number of active users",
        &["time_window"]
    )
    .unwrap();

    /// Recompensas acumuladas
    pub static ref REWARDS_ACCUMULATED: CounterVec = register_counter_vec!(
        "rewards_accumulated_total",
        "Total rewards accumulated (in Lümis)",
        &["reward_type"]
    )
    .unwrap();

    // ========================================================================
    // REDEMPTION METRICS (Sistema de Redención de Lümis)
    // ========================================================================
    
    /// Redenciones creadas
    pub static ref REDEMPTIONS_CREATED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "redemptions_created_total",
        "Total redemptions created",
        &["offer_type", "status"]
    )
    .unwrap();

    /// Redenciones confirmadas por merchants
    pub static ref REDEMPTIONS_CONFIRMED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "redemptions_confirmed_total",
        "Total redemptions confirmed by merchants",
        &["merchant_id", "offer_type"]
    )
    .unwrap();

    /// Redenciones expiradas
    pub static ref REDEMPTIONS_EXPIRED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "redemptions_expired_total",
        "Total redemptions expired",
        &["offer_type"]
    )
    .unwrap();

    /// Redenciones canceladas
    pub static ref REDEMPTIONS_CANCELLED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "redemptions_cancelled_total",
        "Total redemptions cancelled",
        &["reason"]
    )
    .unwrap();

    /// Balance updates (por triggers)
    pub static ref BALANCE_UPDATES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "balance_updates_total",
        "Total balance updates",
        &["update_type"]
    )
    .unwrap();

    /// Logins de merchants
    pub static ref MERCHANT_LOGINS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "merchant_logins_total",
        "Total merchant logins",
        &["merchant_id", "status"]
    )
    .unwrap();

    /// Validaciones de merchants
    pub static ref MERCHANT_VALIDATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "merchant_validations_total",
        "Total redemption validations by merchants",
        &["merchant_id", "status"]
    )
    .unwrap();

    /// Lümis gastados en redenciones
    pub static ref LUMIS_SPENT_TOTAL: CounterVec = register_counter_vec!(
        "lumis_spent_total",
        "Total Lümis spent on redemptions",
        &["offer_type"]
    )
    .unwrap();

    /// Duración de procesamiento de redenciones
    pub static ref REDEMPTION_PROCESSING_DURATION: HistogramVec = register_histogram_vec!(
        "redemption_processing_duration_seconds",
        "Redemption processing duration in seconds",
        &["operation"],
        vec![0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.0]
    )
    .unwrap();

    /// QR codes generados para redenciones
    pub static ref QR_CODES_GENERATED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "qr_codes_generated_total",
        "Total QR codes generated for redemptions",
        &["format"]
    )
    .unwrap();

    /// Webhooks enviados a merchants
    pub static ref WEBHOOKS_SENT_TOTAL: IntCounterVec = register_int_counter_vec!(
        "webhooks_sent_total",
        "Total webhooks sent to merchants",
        &["event_type", "status"]
    )
    .unwrap();

    /// Push notifications enviadas
    pub static ref PUSH_NOTIFICATIONS_SENT_TOTAL: IntCounterVec = register_int_counter_vec!(
        "push_notifications_sent_total",
        "Total push notifications sent",
        &["notification_type", "status"]
    )
    .unwrap();

    /// Notification queue processing metrics
    pub static ref NOTIFICATION_QUEUE_PROCESSED: IntCounterVec = register_int_counter_vec!(
        "notification_queue_processed_total",
        "Total notifications processed from queue",
        &["status"]  // sent, failed, skipped, invalid_token
    )
    .unwrap();

    /// In-app notifications created
    pub static ref NOTIFICATIONS_CREATED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "notifications_created_total",
        "Total in-app notifications created",
        &["notification_type"]
    )
    .unwrap();

    /// Notification API requests
    pub static ref NOTIFICATION_API_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "notification_api_requests_total",
        "Total notification API requests",
        &["endpoint", "status"]
    )
    .unwrap();
}

/// Helper para registrar una request HTTP
pub fn record_http_request(method: &str, endpoint: &str, status: u16, duration_secs: f64, response_size: usize) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, endpoint, &status.to_string()])
        .inc();
    
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[method, endpoint])
        .observe(duration_secs);
    
    HTTP_RESPONSE_SIZE_BYTES
        .with_label_values(&[method, endpoint])
        .observe(response_size as f64);
}

/// Helper para registrar una query de base de datos
pub fn record_db_query(query_type: &str, table: &str, duration_secs: f64, success: bool) {
    let status = if success { "success" } else { "error" };
    
    DB_QUERIES_TOTAL
        .with_label_values(&[query_type, table, status])
        .inc();
    
    DB_QUERY_DURATION_SECONDS
        .with_label_values(&[query_type, table])
        .observe(duration_secs);
}

/// Helper para registrar cache hit/miss
pub fn record_cache_access(cache_type: &str, cache_name: &str, hit: bool) {
    if hit {
        CACHE_HITS_TOTAL
            .with_label_values(&[cache_type, cache_name])
            .inc();
    } else {
        CACHE_MISSES_TOTAL
            .with_label_values(&[cache_type, cache_name])
            .inc();
    }
}

/// Helper para actualizar tamaño de cache
pub fn update_cache_size(cache_type: &str, cache_name: &str, size: i64) {
    CACHE_SIZE
        .with_label_values(&[cache_type, cache_name])
        .set(size);
}

/// Helper para registrar autenticación
pub fn record_auth_attempt(auth_type: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    AUTH_ATTEMPTS_TOTAL
        .with_label_values(&[auth_type, status])
        .inc();
}

// ============================================================================
// REDEMPTION METRICS HELPERS
// ============================================================================

/// Helper para registrar creación de redención
pub fn record_redemption_created(offer_type: &str, success: bool, lumis_cost: f64) {
    let status = if success { "success" } else { "error" };
    REDEMPTIONS_CREATED_TOTAL
        .with_label_values(&[offer_type, status])
        .inc();
    
    if success {
        LUMIS_SPENT_TOTAL
            .with_label_values(&[offer_type])
            .inc_by(lumis_cost);
    }
}

/// Helper para registrar confirmación de redención
pub fn record_redemption_confirmed(merchant_id: &str, offer_type: &str) {
    REDEMPTIONS_CONFIRMED_TOTAL
        .with_label_values(&[merchant_id, offer_type])
        .inc();
}

/// Helper para registrar expiración de redención
pub fn record_redemption_expired(offer_type: &str) {
    REDEMPTIONS_EXPIRED_TOTAL
        .with_label_values(&[offer_type])
        .inc();
}

/// Helper para registrar cancelación de redención
pub fn record_redemption_cancelled(reason: &str) {
    REDEMPTIONS_CANCELLED_TOTAL
        .with_label_values(&[reason])
        .inc();
}

/// Helper para registrar login de merchant
pub fn record_merchant_login(merchant_id: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    MERCHANT_LOGINS_TOTAL
        .with_label_values(&[merchant_id, status])
        .inc();
}

/// Helper para registrar validación de merchant
pub fn record_merchant_validation(merchant_id: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    MERCHANT_VALIDATIONS_TOTAL
        .with_label_values(&[merchant_id, status])
        .inc();
}

/// Helper para registrar generación de QR
pub fn record_qr_generated(format: &str) {
    QR_CODES_GENERATED_TOTAL
        .with_label_values(&[format])
        .inc();
}

/// Helper para registrar webhook enviado
pub fn record_webhook_sent(event_type: &str, success: bool) {
    let status = if success { "success" } else { "error" };
    WEBHOOKS_SENT_TOTAL
        .with_label_values(&[event_type, status])
        .inc();
}

/// Helper para registrar push notification
pub fn record_push_notification(notification_type: &str, success: bool) {
    let status = if success { "success" } else { "error" };
    PUSH_NOTIFICATIONS_SENT_TOTAL
        .with_label_values(&[notification_type, status])
        .inc();
}

/// Helper para registrar procesamiento de cola de notificaciones
pub fn record_notification_queue_processed(sent: usize, failed: usize, skipped: usize, invalid_tokens: usize) {
    if sent > 0 {
        NOTIFICATION_QUEUE_PROCESSED.with_label_values(&["sent"]).inc_by(sent as u64);
    }
    if failed > 0 {
        NOTIFICATION_QUEUE_PROCESSED.with_label_values(&["failed"]).inc_by(failed as u64);
    }
    if skipped > 0 {
        NOTIFICATION_QUEUE_PROCESSED.with_label_values(&["skipped"]).inc_by(skipped as u64);
    }
    if invalid_tokens > 0 {
        NOTIFICATION_QUEUE_PROCESSED.with_label_values(&["invalid_token"]).inc_by(invalid_tokens as u64);
    }
}

/// Helper para registrar creación de notificación in-app
pub fn record_notification_created(notification_type: &str) {
    NOTIFICATIONS_CREATED_TOTAL.with_label_values(&[notification_type]).inc();
}

/// Helper para registrar request a API de notificaciones
pub fn record_notification_api_request(endpoint: &str, success: bool) {
    let status = if success { "success" } else { "error" };
    NOTIFICATION_API_REQUESTS.with_label_values(&[endpoint, status]).inc();
}

/// Helper para registrar procesamiento de factura
pub fn record_invoice_processing(source_type: &str, duration_secs: f64, success: bool) {
    let status = if success { "success" } else { "error" };
    
    INVOICES_PROCESSED
        .with_label_values(&[source_type, status])
        .inc();
    
    INVOICE_PROCESSING_DURATION
        .with_label_values(&[source_type])
        .observe(duration_secs);
}

/// Helper para registrar detección QR
pub fn record_qr_detection(detector: &str, duration_secs: f64, success: bool) {
    let status = if success { "success" } else { "failure" };
    
    QR_DETECTIONS_TOTAL
        .with_label_values(&[detector, status])
        .inc();
    
    QR_DETECTION_DURATION
        .with_label_values(&[detector])
        .observe(duration_secs);
}

/// Helper para registrar error
pub fn record_error(error_type: &str, component: &str) {
    ERRORS_TOTAL
        .with_label_values(&[error_type, component])
        .inc();
}
