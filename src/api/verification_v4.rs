use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{post},
    Router,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use redis::AsyncCommands;
use chrono::{Utc, Duration};
use uuid;

use crate::state::AppState;
use crate::api::templates::verification_templates::{
    SendVerificationRequest, SendVerificationResponse,
    VerifyAccountRequest, VerifyAccountResponse,
    VerificationQueries, VerificationCode
};
// use crate::utils::request_id::get_request_id; // Not needed anymore

// ============================================================================
// VERIFICATION CODE UTILITIES
// ============================================================================

fn generate_verification_code() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // Generate pseudo-random 6-digit code using timestamp and hash
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let mut hasher = DefaultHasher::new();
    now.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Convert to 6-digit code
    let code = ((hash % 900000) + 100000) as u32;
    format!("{:06}", code)
}

async fn store_verification_code(
    state: &AppState,
    email: &str,
    code: &str,
    code_type: &str,
) -> Result<(), String> {
    let redis_key = format!("verification:{}:{}", email.to_lowercase(), code_type);
    let code_data = serde_json::json!({
        "code": code,
        "email": email.to_lowercase(),
        "code_type": code_type,
        "created_at": Utc::now().to_rfc3339(),
        "attempts": 0
    });
    
    let mut conn = state.redis_client.get_multiplexed_async_connection().await
        .map_err(|e| {
            error!("Redis connection error: {}", e);
            "Failed to connect to Redis".to_string()
        })?;
    
    match conn.set_ex::<&str, String, ()>(&redis_key, code_data.to_string(), 3600).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to store verification code in Redis: {}", e);
            Err("Failed to store verification code".to_string())
        }
    }
}

async fn get_verification_code(
    state: &AppState,
    email: &str,
    code_type: &str,
) -> Result<Option<VerificationCode>, String> {
    let redis_key = format!("verification:{}:{}", email.to_lowercase(), code_type);
    
    let mut conn = state.redis_client.get_multiplexed_async_connection().await
        .map_err(|e| format!("Redis connection error: {}", e))?;
    
    match conn.get::<&str, Option<String>>(&redis_key).await {
        Ok(Some(data)) => {
            match serde_json::from_str::<serde_json::Value>(&data) {
                Ok(json_data) => {
                    let code = VerificationCode {
                        code: json_data["code"].as_str().unwrap_or("").to_string(),
                        email: json_data["email"].as_str().unwrap_or("").to_string(),
                        code_type: json_data["code_type"].as_str().unwrap_or("").to_string(),
                        expires_at: Utc::now() + Duration::hours(1), // Redis TTL handles expiration
                        attempts: json_data["attempts"].as_i64().unwrap_or(0) as i32,
                    };
                    Ok(Some(code))
                }
                Err(e) => {
                    error!("Failed to parse verification code from Redis: {}", e);
                    Err("Invalid verification code data".to_string())
                }
            }
        }
        Ok(None) => Ok(None),
        Err(e) => {
            error!("Failed to get verification code from Redis: {}", e);
            Err("Failed to retrieve verification code".to_string())
        }
    }
}

async fn increment_verification_attempts(
    state: &AppState,
    email: &str,
    code_type: &str,
) -> Result<i32, String> {
    let redis_key = format!("verification:{}:{}", email.to_lowercase(), code_type);
    
    let mut conn = state.redis_client.get_multiplexed_async_connection().await
        .map_err(|e| format!("Redis connection error: {}", e))?;
    
    match conn.get::<&str, Option<String>>(&redis_key).await {
        Ok(Some(data)) => {
            match serde_json::from_str::<serde_json::Value>(&data) {
                Ok(mut json_data) => {
                    let current_attempts = json_data["attempts"].as_i64().unwrap_or(0) as i32;
                    let new_attempts = current_attempts + 1;
                    json_data["attempts"] = serde_json::Value::Number(serde_json::Number::from(new_attempts));
                    
                    match conn.set_ex::<&str, String, ()>(&redis_key, json_data.to_string(), 3600).await {
                        Ok(_) => Ok(new_attempts),
                        Err(e) => {
                            error!("Failed to update verification attempts: {}", e);
                            Err("Failed to update attempts".to_string())
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse verification data for attempts update: {}", e);
                    Err("Invalid verification data".to_string())
                }
            }
        }
        Ok(None) => Err("Verification code not found".to_string()),
        Err(e) => {
            error!("Failed to get verification code for attempts update: {}", e);
            Err("Failed to retrieve verification code".to_string())
        }
    }
}

async fn delete_verification_code(
    state: &AppState,
    email: &str,
    code_type: &str,
) -> Result<(), String> {
    let redis_key = format!("verification:{}:{}", email.to_lowercase(), code_type);
    
    let mut conn = state.redis_client.get_multiplexed_async_connection().await
        .map_err(|e| format!("Redis connection error: {}", e))?;
    
    match conn.del::<&str, ()>(&redis_key).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to delete verification code: {}", e);
            Err("Failed to delete verification code".to_string())
        }
    }
}

// ============================================================================
// API HANDLERS
// ============================================================================

/// Send verification code to user
pub async fn send_verification_code(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendVerificationRequest>,
) -> Result<Json<SendVerificationResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    info!("Request {}: Send verification code to: {}", request_id, req.email);
    
    // Validate request
    if let Err(error) = req.validate() {
        warn!("Request {}: Validation error: {}", request_id, error);
        return Ok(Json(SendVerificationResponse::error(&error)));
    }
    
    let email = req.email.to_lowercase().trim().to_string();
    
    // Check if user exists
    let user_query = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, Option<String>, Option<String>)>(
        VerificationQueries::GET_USER_FOR_VERIFICATION
    )
    .bind(&email)
    .fetch_optional(&state.db_pool)
    .await;
    
    let user = match user_query {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!("Request {}: User not found: {}", request_id, email);
            return Ok(Json(SendVerificationResponse::error("User not found")));
        }
        Err(e) => {
            error!("Request {}: Database error: {}", request_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    let (_user_id, _email, password_hash, ws_id, _telegram_id, _name) = user;
    
    // Generate verification code
    let code = generate_verification_code();
    
    // Determine code type based on whether user has password
    let code_type = if password_hash.is_some() {
        "reset_password"
    } else {
        "set_password"
    };
    
    // Store code in Redis
    if let Err(error) = store_verification_code(&state, &email, &code, code_type).await {
        error!("Request {}: Failed to store verification code: {}", request_id, error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    // Determine sending method based on user preference and availability
    let method = match req.method.as_deref() {
        Some("email") => "email",
        Some("whatsapp") => {
            if ws_id.is_some() {
                "whatsapp"
            } else {
                warn!("Request {}: WhatsApp requested but user has no WhatsApp ID, falling back to email", request_id);
                "email"
            }
        }
        _ => {
            // Default behavior: prefer WhatsApp if available, otherwise email
            if ws_id.is_some() {
                "whatsapp"
            } else {
                "email"
            }
        }
    };
    
    // Send verification code
    let actual_method = match method {
        "email" => {
            if let Err(e) = send_email_verification(&email, &code, &request_id).await {
                error!("Request {}: Failed to send email: {}", request_id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            "email"
        },
        "whatsapp" => {
            // TODO: Implement WhatsApp sending
            info!("Request {}: WhatsApp sending not implemented yet for: {}", request_id, ws_id.as_ref().unwrap());
            warn!("Request {}: WhatsApp not implemented, falling back to email", request_id);
            
            // Fallback to email
            if let Err(e) = send_email_verification(&email, &code, &request_id).await {
                error!("Request {}: Failed to send fallback email: {}", request_id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            "email" // Return actual method used
        },
        _ => {
            error!("Request {}: Unknown sending method: {}", request_id, method);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    info!("Request {}: Verification code sent successfully to: {} via {}", request_id, email, actual_method);
    Ok(Json(SendVerificationResponse::success(actual_method)))
}

/// Verify account with verification code
pub async fn verify_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyAccountRequest>,
) -> Result<Json<VerifyAccountResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    info!("Request {}: Verify account for: {}", request_id, req.email);
    
    // Validate request
    if let Err(error) = req.validate() {
        warn!("Request {}: Validation error: {}", request_id, error);
        return Ok(Json(VerifyAccountResponse::error(&error)));
    }
    
    let email = req.email.to_lowercase().trim().to_string();
    
    // Get verification code from Redis
    let stored_code = match get_verification_code(&state, &email, "verify_account").await {
        Ok(Some(code)) => code,
        Ok(None) => {
            warn!("Request {}: No verification code found for: {}", request_id, email);
            return Ok(Json(VerifyAccountResponse::error("Verification code not found or expired")));
        }
        Err(error) => {
            error!("Request {}: Failed to get verification code: {}", request_id, error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    // Check attempts limit
    if stored_code.attempts >= 3 {
        warn!("Request {}: Too many verification attempts for: {}", request_id, email);
        return Ok(Json(VerifyAccountResponse::error("Too many failed attempts. Please request a new code")));
    }
    
    // Verify code
    if stored_code.code != req.verification_code {
        // Increment attempts
        if let Err(error) = increment_verification_attempts(&state, &email, "verify_account").await {
            error!("Request {}: Failed to increment attempts: {}", request_id, error);
        }
        
        warn!("Request {}: Invalid verification code for: {}", request_id, email);
        return Ok(Json(VerifyAccountResponse::error("Invalid verification code")));
    }
    
    // Get user ID
    let user_query = sqlx::query_as::<_, (i32,)>("SELECT id FROM public.dim_users WHERE LOWER(email) = LOWER($1)")
        .bind(&email)
        .fetch_optional(&state.db_pool)
        .await;
    
    let user_id = match user_query {
        Ok(Some((id,))) => id,
        Ok(None) => {
            warn!("Request {}: User not found during verification: {}", request_id, email);
            return Ok(Json(VerifyAccountResponse::error("User not found")));
        }
        Err(e) => {
            error!("Request {}: Database error during verification: {}", request_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    // Delete verification code
    if let Err(error) = delete_verification_code(&state, &email, "verify_account").await {
        error!("Request {}: Failed to delete verification code: {}", request_id, error);
    }
    
    info!("Request {}: Account verified successfully for user: {}", request_id, user_id);
    Ok(Json(VerifyAccountResponse::success(user_id)))
}

// ============================================================================
// ROUTER CREATION
// ============================================================================

pub fn create_verification_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/send-verification", post(send_verification_code))
        .route("/verify-account", post(verify_account))
        .route("/test-email", post(test_email_sending))  // Test endpoint
        .route("/test-smtp", post(test_smtp_configuration))  // SMTP test endpoint
}

// ============================================================================
// EMAIL SENDING FUNCTION
// ============================================================================

// Function to load and process the HTML email template
pub fn load_email_template(code: &str, user_name: &str, expiry_time: &str) -> Result<String, String> {
    let template_content = include_str!("../templates/email_verification.html");
    
    let html_body = template_content
        .replace("{{VERIFICATION_CODE}}", code)
        .replace("{{USER_NAME}}", user_name)
        .replace("{{EXPIRY_TIME}}", expiry_time);
    
    Ok(html_body)
}

// Function to create plain text fallback
pub fn create_plain_text_body(code: &str, user_name: &str, expiry_time: &str) -> String {
    format!(
        "¡Hola {}!\n\nPara completar tu verificación en Lüm, usa el siguiente código de 6 dígitos:\n\n{} \n\n⏰ Importante: Este código expira en {} por tu seguridad.\n\nSi no solicitaste este código, puedes ignorar este mensaje de forma segura.\n\n¡Gracias por ser parte del universo Lüm! 🌟\n\nLüm - Tu app de recompensas\n¿Necesitas ayuda? Contáctanos en soporte@lumapp.org\n\nEste es un correo automático, por favor no respondas a esta dirección.",
        user_name, code, expiry_time
    )
}

pub async fn send_email_verification(email: &str, code: &str, request_id: &str) -> Result<(), String> {
    // Email content
    let subject = "Código de Verificación - Lüm";
    let user_name = "Usuario"; // Default, could be extracted from email or database
    let expiry_time = "1 hora";
    
    // Create both HTML and plain text versions
    let html_body = load_email_template(code, user_name, expiry_time)
        .map_err(|e| format!("Failed to load email template: {}", e))?;
    let plain_body = create_plain_text_body(code, user_name, expiry_time);
    
    // Log email content for debugging
    info!(
        "📧 EMAIL CONTENT for {} (Request: {})\n📬 To: {}\n📋 Subject: {}\n📝 Format: HTML + Plain Text\n📝 Code: {}",
        email, request_id, email, subject, code
    );
    
    // Try SendGrid API first (easiest to configure)
    if let Ok(sendgrid_api_key) = std::env::var("SENDGRID_API_KEY") {
        if !sendgrid_api_key.is_empty() {
            return send_via_sendgrid_html(email, &subject, &html_body, &plain_body, &sendgrid_api_key, request_id).await;
        }
    }
    
    // Try SMTP if configured
    if let (Ok(smtp_server), Ok(smtp_username), Ok(smtp_password)) = (
        std::env::var("SMTP_SERVER"),
        std::env::var("SMTP_USERNAME"), 
        std::env::var("SMTP_PASSWORD")
    ) {
        if !smtp_server.is_empty() && !smtp_username.is_empty() && !smtp_password.is_empty() {
            return send_via_smtp_html(email, &subject, &html_body, &plain_body, &smtp_server, &smtp_username, &smtp_password, request_id).await;
        }
    }
    
    // Fallback: simulation with detailed logging
    warn!("⚠️ No email service configured (SENDGRID_API_KEY or SMTP_* variables), using simulation");
    info!("🚀 SIMULATING EMAIL SEND");
    info!("📬 From: info@lumapp.org");
    info!("📬 To: {}", email);
    info!("📬 Subject: {}", subject);
    info!("📬 Code: {}", code);
    info!("📬 Request: {}", request_id);
    
    info!("✅ Email simulated successfully to: {} (Request: {})", email, request_id);
    Ok(())
}

// SendGrid API implementation with HTML support
pub async fn send_via_sendgrid_html(email: &str, subject: &str, html_body: &str, plain_body: &str, api_key: &str, request_id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    
    let email_payload = serde_json::json!({
        "personalizations": [{
            "to": [{"email": email}]
        }],
        "from": {"email": "info@lumapp.org", "name": "Lüm Team"},
        "subject": subject,
        "content": [
            {
                "type": "text/plain",
                "value": plain_body
            },
            {
                "type": "text/html",
                "value": html_body
            }
        ]
    });
    
    info!("🚀 SENDING HTML EMAIL via SendGrid API to: {}", email);
    
    let response = client
        .post("https://api.sendgrid.com/v3/mail/send")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&email_payload)
        .send()
        .await
        .map_err(|e| format!("SendGrid request failed: {}", e))?;
    
    if response.status().is_success() {
        info!("✅ HTML Email sent successfully via SendGrid to: {} (Request: {})", email, request_id);
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        error!("❌ SendGrid API error {}: {}", status, error_text);
        Err(format!("SendGrid error {}: {}", status, error_text))
    }
}

// SMTP implementation with HTML support
pub async fn send_via_smtp_html(email: &str, subject: &str, html_body: &str, plain_body: &str, smtp_server: &str, username: &str, password: &str, request_id: &str) -> Result<(), String> {
    use lettre::{
        message::{header::ContentType, MultiPart, SinglePart},
        transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };
    
    info!("🚀 SENDING HTML EMAIL via SMTP {} to: {}", smtp_server, email);
    
    // Use the same email as username for the From field (this should match the SMTP account)
    let from_email = username; // info@lumapp.org
    
    // Build multipart email message with both HTML and plain text
    let email_message = Message::builder()
        .from(from_email.parse().map_err(|e| format!("Invalid from address: {}", e))?)
        .to(email.parse().map_err(|e| format!("Invalid to address: {}", e))?)
        .subject(subject)
        .multipart(
            MultiPart::alternative() // This allows email clients to choose between HTML and plain text
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(plain_body.to_string())
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body.to_string())
                )
        )
        .map_err(|e| format!("Failed to build email: {}", e))?;
    
    // Create SMTP transport
    let creds = Credentials::new(username.to_string(), password.to_string());
    
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_server)
        .map_err(|e| format!("SMTP relay error: {}", e))?
        .credentials(creds)
        .build();
    
    // Send email
    match mailer.send(email_message).await {
        Ok(response) => {
            info!("✅ HTML Email sent successfully via SMTP to: {} (Request: {}) - Response: {:?}", email, request_id, response);
            Ok(())
        },
        Err(e) => {
            error!("❌ SMTP send failed: {}", e);
            Err(format!("SMTP send failed: {}", e))
        }
    }
}

// ============================================================================
// TEST EMAIL ENDPOINT
// ============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct TestEmailRequest {
    pub email: String,
    pub message: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct TestEmailResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
}

/// Test endpoint to send email directly
pub async fn test_email_sending(
    Json(req): Json<TestEmailRequest>,
) -> Result<Json<TestEmailResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let test_code = generate_verification_code();
    
    info!("🧪 TEST EMAIL REQUEST: {} to {}", request_id, req.email);
    
    // Send test email
    match send_email_verification(&req.email, &test_code, &request_id).await {
        Ok(_) => {
            Ok(Json(TestEmailResponse {
                success: true,
                message: format!("Test email sent successfully! Code: {}", test_code),
                email: req.email,
            }))
        },
        Err(e) => {
            error!("🧪 TEST EMAIL FAILED: {}", e);
            Ok(Json(TestEmailResponse {
                success: false,
                message: format!("Failed to send test email: {}", e),
                email: req.email,
            }))
        }
    }
}

// ============================================================================
// SMTP CONFIGURATION TEST ENDPOINT
// ============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct TestSmtpRequest {
    pub email: String,
    pub smtp_server: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_port: Option<u16>,
}

#[derive(Debug, serde::Serialize)]
pub struct TestSmtpResponse {
    pub success: bool,
    pub message: String,
    pub email: String,
    pub smtp_server: String,
}

/// Test SMTP configuration directly
pub async fn test_smtp_configuration(
    Json(req): Json<TestSmtpRequest>,
) -> Result<Json<TestSmtpResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let test_code = generate_verification_code();
    
    info!("🧪 SMTP TEST REQUEST: {} to {} via {}", request_id, req.email, req.smtp_server);
    
    // Test SMTP sending with HTML
    let subject = "Test de Configuración SMTP - Lüm";
    let user_name = "Usuario de Prueba";
    let expiry_time = "10 minutos";
    
    let html_body = match load_email_template(&test_code, user_name, expiry_time) {
        Ok(html) => html,
        Err(e) => {
            error!("🧪 SMTP TEST FAILED - Template error: {}", e);
            return Ok(Json(TestSmtpResponse {
                success: false,
                message: format!("Template error: {}", e),
                email: req.email,
                smtp_server: req.smtp_server,
            }));
        }
    };
    let plain_body = create_plain_text_body(&test_code, user_name, expiry_time);
    
    match send_via_smtp_html(&req.email, &subject, &html_body, &plain_body, &req.smtp_server, &req.smtp_username, &req.smtp_password, &request_id).await {
        Ok(_) => {
            info!("🧪 SMTP TEST SUCCESS: {} via {}", req.email, req.smtp_server);
            Ok(Json(TestSmtpResponse {
                success: true,
                message: format!("SMTP test successful! Email sent with code: {}", test_code),
                email: req.email,
                smtp_server: req.smtp_server,
            }))
        },
        Err(e) => {
            error!("🧪 SMTP TEST FAILED: {} via {} - Error: {}", req.email, req.smtp_server, e);
            Ok(Json(TestSmtpResponse {
                success: false,
                message: format!("SMTP test failed: {}", e),
                email: req.email,
                smtp_server: req.smtp_server,
            }))
        }
    }
}
