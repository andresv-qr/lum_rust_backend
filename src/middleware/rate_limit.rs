use axum::{extract::Request, middleware::Next, response::Response, http::{StatusCode, HeaderValue}};
use chrono::{Utc, Datelike, Timelike};
use redis::AsyncCommands;
use tracing::{warn, debug};
use std::sync::Arc;

use crate::state::AppState;
use crate::middleware::CurrentUser;
use crate::api::models::ErrorResponse;

#[derive(Clone, Copy, Debug)]
pub struct RateLimits { pub per_hour: u32, pub per_day: u32 }

#[derive(Clone, Debug)]
pub struct RateLimitInfo {
    pub limits: RateLimits,
    pub hour_count: u32,
    pub day_count: u32,
    pub hour_remaining: u32,
    pub day_remaining: u32,
}

// Rate limiting policies by endpoint
fn get_endpoint_limits(path: &str, score: i32) -> RateLimits {
    match path {
        p if p.contains("/auth/login") => RateLimits { per_hour: 3, per_day: 10 }, // Strict for login
        p if p.contains("/auth/register") => RateLimits { per_hour: 5, per_day: 20 },
        p if p.contains("/qr/detect") => RateLimits { per_hour: 30, per_day: 300 },
        p if p.contains("/invoices/process-from-url") => limits_for_score(score), // Dynamic by trust
        p if p.contains("/invoices/upload-ocr") => limits_for_score(score), // Dynamic by trust
        _ => RateLimits { per_hour: 100, per_day: 1000 }, // Default generous for GET endpoints
    }
}

fn limits_for_score(score: i32) -> RateLimits {
    if score >= 40 { RateLimits { per_hour: 5, per_day: 20 } }
    else if score >= 25 { RateLimits { per_hour: 3, per_day: 12 } }
    else if score >= 10 { RateLimits { per_hour: 2, per_day: 8 } }
    else { RateLimits { per_hour: 1, per_day: 3 } }
}

async fn fetch_trust_score(user_id: i64, state: &AppState) -> i32 {
    // Placeholder simple heuristic: later replace with DB table user_trust_scores
    // Attempt fetch from cache, else default
    let key = format!("trust:score:{}", user_id);
        if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(score_str)) = redis::cmd("GET").arg(&key).query_async::<Option<String>>(&mut conn).await {
            if let Ok(v) = score_str.parse::<i32>() { return v; }
        }
    }
    10
}

fn window_keys_generic(user_id: &str, scope: &str) -> (String, String, u64, u64) {
    let now = Utc::now();
    let hour_key = format!("rl:{}:u:{}:h:{}{:02}{:02}{:02}", scope, user_id, now.year(), now.month(), now.day(), now.hour());
    let day_key = format!("rl:{}:u:{}:d:{}{:02}{:02}", scope, user_id, now.year(), now.month(), now.day());
    let secs_hour = 3600 - (now.minute()*60 + now.second()) as u64;
    let secs_day = 86400 - (now.num_seconds_from_midnight() as u64);
    (hour_key, day_key, secs_hour, secs_day)
}

fn extract_client_ip(req: &Request) -> Option<String> {
    // Try X-Forwarded-For first, then X-Real-IP
    req.headers().get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("unknown").trim().to_string())
        .or_else(|| {
            req.headers().get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
}

pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<ErrorResponse>)> {
    let path = req.uri().path();
    
    // Determine scope based on path for Redis keys
    let scope = if path.contains("process-from-url") { "invoice_proc" } 
        else if path.contains("upload-ocr") { "upload_ocr" }
        else if path.contains("/auth/login") { "auth_login" }
        else if path.contains("/auth/register") { "auth_register" }
        else if path.contains("/qr/detect") { "qr_detect" }
        else { return Ok(next.run(req).await) }; // Skip rate limiting for other endpoints

    // For auth endpoints, use IP-based limiting (no CurrentUser yet)
    let user_id = if scope.starts_with("auth") {
        // Extract IP or use placeholder for auth endpoints
        extract_client_ip(&req).unwrap_or_else(|| "unknown".to_string())
    } else {
        // For protected endpoints, require CurrentUser
        match req.extensions().get::<CurrentUser>() {
            Some(u) => u.user_id.to_string(),
            None => return Err((StatusCode::UNAUTHORIZED, axum::Json(ErrorResponse { 
                error: "AUTH_REQUIRED".into(), 
                message: "Authentication required before rate limiting.".into(), 
                details: None 
            })))
        }
    };

    let state_arc = match req.extensions().get::<Arc<AppState>>() { 
        Some(s) => s.clone(), 
        None => return Err((StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ErrorResponse { 
            error: "STATE_MISSING".into(), 
            message: "App state missing".into(), 
            details: None 
        })))
    };

    // Get limits (for auth endpoints, use default score=10)
    let score = if scope.starts_with("auth") { 10 } else { 
        fetch_trust_score(user_id.parse().unwrap_or(1), &state_arc).await 
    };
    let limits = get_endpoint_limits(path, score);
    let (hour_key, day_key, ttl_hour, ttl_day) = window_keys_generic(&user_id, scope);

    if let Ok(mut conn) = state_arc.redis_client.get_multiplexed_async_connection().await {
        let hour_count: i64 = conn.incr(&hour_key, 1).await.unwrap_or(1);
        if hour_count == 1 { let _: () = conn.expire(&hour_key, ttl_hour as i64).await.unwrap_or(()); }
        let day_count: i64 = conn.incr(&day_key, 1).await.unwrap_or(1);
        if day_count == 1 { let _: () = conn.expire(&day_key, ttl_day as i64).await.unwrap_or(()); }

        debug!(user_id=%user_id, scope, hour_count, day_count, limits=?limits, "rate-limit counters");

        if hour_count as u32 > limits.per_hour {
            warn!(user_id=%user_id, scope, "Hourly rate limit exceeded");
            return Err((StatusCode::TOO_MANY_REQUESTS, axum::Json(ErrorResponse { 
                error: "RATE_LIMIT_HOURLY".into(), 
                message: format!("Hourly limit {} exceeded", limits.per_hour), 
                details: Some(format!("retry_after={}s", ttl_hour)) 
            })));
        }
        if day_count as u32 > limits.per_day {
            warn!(user_id=%user_id, scope, "Daily rate limit exceeded");
            return Err((StatusCode::TOO_MANY_REQUESTS, axum::Json(ErrorResponse { 
                error: "RATE_LIMIT_DAILY".into(), 
                message: format!("Daily limit {} exceeded", limits.per_day), 
                details: Some(format!("retry_after={}s", ttl_day)) 
            })));
        }
        
        // Store rate limit info for response headers
        let rate_info = RateLimitInfo {
            limits,
            hour_count: hour_count as u32,
            day_count: day_count as u32,
            hour_remaining: (limits.per_hour as u32).saturating_sub(hour_count as u32),
            day_remaining: (limits.per_day as u32).saturating_sub(day_count as u32),
        };
    
        // Process request and add headers to response
        let mut response = next.run(req).await;
        
        // Add rate limit headers to all responses (success and error)
        let headers = response.headers_mut();
        if let Ok(value) = HeaderValue::from_str(&rate_info.limits.per_hour.to_string()) {
            headers.insert("X-RateLimit-Limit-Hour", value);
        }
        if let Ok(value) = HeaderValue::from_str(&rate_info.hour_remaining.to_string()) {
            headers.insert("X-RateLimit-Remaining-Hour", value);
        }
        if let Ok(value) = HeaderValue::from_str(&rate_info.limits.per_day.to_string()) {
            headers.insert("X-RateLimit-Limit-Day", value);
        }
        if let Ok(value) = HeaderValue::from_str(&rate_info.day_remaining.to_string()) {
            headers.insert("X-RateLimit-Remaining-Day", value);
        }
        
        Ok(response)
    } else {
        // No rate limiting applied (health checks, etc.)
        Ok(next.run(req).await)
    }
}
