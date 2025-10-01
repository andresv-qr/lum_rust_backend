// Security middleware and utilities for hardening
use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::warn;

use crate::state::AppState;

// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            window_size: Duration::from_secs(60),
        }
    }
}

// Rate limiter state
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    clients: Arc<RwLock<HashMap<String, ClientState>>>,
}

#[derive(Debug, Clone)]
struct ClientState {
    requests: Vec<SystemTime>,
    last_request: SystemTime,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check_rate_limit(&self, client_id: &str) -> bool {
        let now = SystemTime::now();
        let mut clients = self.clients.write().await;
        
        let client_state = clients.entry(client_id.to_string()).or_insert_with(|| {
            ClientState {
                requests: Vec::new(),
                last_request: now,
            }
        });

        // Clean old requests outside the window
        let window_start = now - self.config.window_size;
        client_state.requests.retain(|&time| time > window_start);

        // Check if we're within limits
        if client_state.requests.len() >= self.config.requests_per_minute as usize {
            warn!("🚫 Rate limit exceeded for client: {}", client_id);
            return false;
        }

        // Add current request
        client_state.requests.push(now);
        client_state.last_request = now;
        
        true
    }
}

/// Security headers middleware - Enhanced version
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string(); // Capture path before moving request
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // Core security headers
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );
    
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );
    
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    
    // Enhanced CSP for APIs
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; font-src 'self'; object-src 'none'; media-src 'self'; frame-src 'none';"),
    );

    // Additional security headers
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=()"),
    );

    headers.insert(
        HeaderName::from_static("x-permitted-cross-domain-policies"),
        HeaderValue::from_static("none"),
    );

    // Cache control for security-sensitive responses
    if path.contains("/auth/") || path.contains("/users/profile") {
        headers.insert(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
        );
    }

    Ok(response)
}

/// Rate limiting middleware
pub async fn rate_limiting_middleware(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client identifier (IP address or user ID)
    let client_id = extract_client_id(&request);
    
    // Check rate limit (we'll implement this with a simple in-memory store for now)
    if !check_simple_rate_limit(&client_id).await {
        warn!("🚫 Rate limit exceeded for client: {}", client_id);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    let response = next.run(request).await;
    Ok(response)
}

fn extract_client_id(request: &Request) -> String {
    // Try to get real IP from headers (for reverse proxy setups)
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(ip) = forwarded_for.to_str() {
            return ip.split(',').next().unwrap_or("unknown").trim().to_string();
        }
    }
    
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.to_string();
        }
    }
    
    // Fallback to connection info (would need to be passed through state)
    "unknown".to_string()
}

// Simple in-memory rate limiter (for production, use Redis)
static RATE_LIMIT_STORE: tokio::sync::OnceCell<Arc<RwLock<HashMap<String, Vec<SystemTime>>>>> = 
    tokio::sync::OnceCell::const_new();

async fn check_simple_rate_limit(client_id: &str) -> bool {
    let store = RATE_LIMIT_STORE.get_or_init(|| async {
        Arc::new(RwLock::new(HashMap::new()))
    }).await;
    
    let now = SystemTime::now();
    let window = Duration::from_secs(60); // 1 minute window
    let max_requests = 100; // 100 requests per minute
    
    let mut clients = store.write().await;
    let requests = clients.entry(client_id.to_string()).or_insert_with(Vec::new);
    
    // Clean old requests
    let cutoff = now - window;
    requests.retain(|&time| time > cutoff);
    
    // Check limit
    if requests.len() >= max_requests {
        return false;
    }
    
    // Add current request
    requests.push(now);
    true
}

/// Input validation utilities
pub mod validation {
    use regex::Regex;
    use std::sync::OnceLock;
    
    static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
    static PHONE_REGEX: OnceLock<Regex> = OnceLock::new();
    
    pub fn is_valid_email(email: &str) -> bool {
        let regex = EMAIL_REGEX.get_or_init(|| {
            Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
        });
        
        email.len() <= 254 && regex.is_match(email)
    }
    
    pub fn is_valid_phone(phone: &str) -> bool {
        let regex = PHONE_REGEX.get_or_init(|| {
            Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap()
        });
        
        regex.is_match(phone)
    }
    
    pub fn sanitize_string(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".-_@".contains(*c))
            .collect::<String>()
            .trim()
            .to_string()
    }
    
    pub fn is_safe_filename(filename: &str) -> bool {
        !filename.contains("..") && 
        !filename.contains('/') && 
        !filename.contains('\\') &&
        filename.len() <= 255 &&
        !filename.is_empty()
    }
}

/// JWT security enhancements
pub mod jwt_security {
    use chrono::{Duration, Utc};
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TokenClaims {
        pub sub: String,      // Subject (user ID)
        pub email: String,    // User email
        pub exp: i64,         // Expiration time
        pub iat: i64,         // Issued at
        pub jti: String,      // JWT ID (for revocation)
        pub role: String,     // User role
    }
    
    impl TokenClaims {
        pub fn new(user_id: i32, email: String, role: String) -> Self {
            let now = Utc::now();
            Self {
                sub: user_id.to_string(),
                email,
                exp: (now + Duration::hours(24)).timestamp(),
                iat: now.timestamp(),
                jti: uuid::Uuid::new_v4().to_string(),
                role,
            }
        }
        
        pub fn is_expired(&self) -> bool {
            Utc::now().timestamp() > self.exp
        }
    }
}

/// CORS configuration for production
pub fn get_cors_layer() -> tower_http::cors::CorsLayer {
    use tower_http::cors::CorsLayer;
    use axum::http::Method;
    
    CorsLayer::new()
        .allow_origin([
            "https://yourdomain.com".parse().unwrap(),
            "https://app.yourdomain.com".parse().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}
