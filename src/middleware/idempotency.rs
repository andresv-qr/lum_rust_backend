use axum::{extract::Request, middleware::Next, response::{Response, IntoResponse}, http::{StatusCode, HeaderMap}};
use redis::AsyncCommands;
use tracing::debug;
use crate::{state::AppState, api::models::ErrorResponse};
use std::sync::Arc;

const IDEMPOTENCY_HEADER: &str = "Idempotency-Key";
const IDEMPOTENCY_TTL_SECS: u64 = 60 * 60 * 24; // 24h

pub async fn idempotency_middleware(
    req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = req.uri().path();
    // Apply only to mutating invoice endpoints (extendable)
    if !(path.contains("/invoices/process-from-url") || path.contains("/invoices/upload-ocr")) {
        return Ok(next.run(req).await);
    }

    let headers: &HeaderMap = req.headers();
    let key = match headers.get(IDEMPOTENCY_HEADER).and_then(|h| h.to_str().ok()) {
        Some(k) if !k.is_empty() => k.trim(),
        _ => {
            return Err((StatusCode::BAD_REQUEST, axum::Json(ErrorResponse {
                error: "IDEMPOTENCY_KEY_REQUIRED".into(),
                message: format!("{} header required for this endpoint", IDEMPOTENCY_HEADER),
                details: None,
            })))
        }
    };

    // Compose redis key (include path to avoid collisions across endpoints)
    let redis_key = format!("idem:{}:{}", path, key);

    let state_arc = match req.extensions().get::<Arc<AppState>>() { Some(s) => s.clone(), None => {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ErrorResponse {
            error: "STATE_MISSING".into(),
            message: "App state missing".into(),
            details: None,
        })));
    }};

    if let Ok(mut conn) = state_arc.redis_client.get_multiplexed_async_connection().await {
        // Check if key exists
        if let Ok(existing) = conn.get::<_, Option<String>>(&redis_key).await {
            if let Some(stored_status) = existing {
                debug!(idempotency_key=%key, path, "Replaying stored idempotent response");
                // Stored format: status_code|json_body
                if let Some((code_part, body_part)) = stored_status.split_once('|') {
                    if let Ok(code_num) = code_part.parse::<u16>() {
                        let status = StatusCode::from_u16(code_num).unwrap_or(StatusCode::OK);
                        let body = body_part.to_string();
                        let resp = Response::builder()
                            .status(status)
                            .header("X-Idempotent-Replay", "true")
                            .body(body.into())
                            .unwrap();
                        return Ok(resp);
                    }
                }
            }
        }
    }

    let response = next.run(req).await;
    let status = response.status();
    let body_bytes = match axum::body::to_bytes(response.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(_) => return Ok(Response::builder().status(status).body("".into()).unwrap()),
    };
    let body_clone = body_bytes.clone();

    // Store response (status + body) for idempotent replay
    if let Ok(mut conn) = state_arc.redis_client.get_multiplexed_async_connection().await {
        let value = format!("{}|{}", status.as_u16(), String::from_utf8_lossy(&body_bytes));
        let _: () = conn.set_ex(&redis_key, value, IDEMPOTENCY_TTL_SECS).await.unwrap_or(());
    }

    let resp = Response::builder()
        .status(status)
        .header("X-Idempotent-Replay", "false")
        .body(body_clone.into())
        .unwrap();

    Ok(resp)
}
