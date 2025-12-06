// ============================================================================
// MIDDLEWARE PARA MÉTRICAS AUTOMÁTICAS
// ============================================================================

use axum::{
    // body::Body, // Unused
    extract::Request,
    // http::StatusCode, // Unused
    middleware::Next,
    response::Response,
};
use std::time::Instant;

use crate::observability::record_http_request;

/// Middleware que automáticamente registra métricas de todas las requests HTTP
pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    
    // Normalizar el path para agrupar rutas similares
    let normalized_path = normalize_path(&path);
    
    // Procesar la request
    let response = next.run(req).await;
    
    // Calcular duración
    let duration = start.elapsed().as_secs_f64();
    
    // Obtener status code
    let status = response.status().as_u16();
    
    // Estimar tamaño de respuesta (aproximado)
    let response_size = estimate_response_size(&response);
    
    // Registrar métricas
    record_http_request(&method, &normalized_path, status, duration, response_size);
    
    response
}

/// Normaliza paths para agrupar rutas con parámetros dinámicos
fn normalize_path(path: &str) -> String {
    // Reemplazar UUIDs, números largos, etc. por placeholders
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = segments
        .iter()
        .map(|seg| {
            if seg.len() == 36 && seg.contains('-') {
                // UUID
                ":id".to_string()
            } else if seg.parse::<i64>().is_ok() {
                // Número (ID)
                ":id".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect();
    
    normalized.join("/")
}

/// Estima el tamaño de la respuesta basado en headers
fn estimate_response_size(response: &Response) -> usize {
    response
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
}
