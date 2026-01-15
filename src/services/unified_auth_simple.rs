// ============================================================================
// SIMPLIFIED UNIFIED AUTHENTICATION SERVICE
// ============================================================================
// Date: September 19, 2025
// Purpose: Simplified version using direct SQL queries
// ============================================================================


use sqlx::PgPool;
use tracing::{info, error};
use uuid::Uuid;

use crate::{
    models::{
        unified_auth::{
            UnifiedAuthRequest, UnifiedAuthResponse, AuthResult, AuthMetadata,
            ProviderData, AuthenticatedUser,
        },
        user::AccountStatus,
    },
    services::{
        google_service::GoogleService,
        token_service::TokenService,
    },
};
use ipnetwork::IpNetwork;

// ============================================================================
// ERRORS
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum SimpleAuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Provider error: {0}")]
    ProviderError(String),
}

// ============================================================================
// SIMPLE UNIFIED AUTH SERVICE
// ============================================================================

pub struct SimpleUnifiedAuthService {
    pub db_pool: PgPool,
    pub google: GoogleService,
    pub token: TokenService,
}

impl SimpleUnifiedAuthService {
    pub fn new(
        db_pool: PgPool,
        google: GoogleService,
        token: TokenService,
    ) -> Self {
        Self {
            db_pool,
            google,
            token,
        }
    }

    /// Log authentication event to audit table
    async fn log_auth_event(
        &self,
        user_id: Option<i32>,
        event_type: &str,
        provider: &str,
        success: bool,
        error_code: Option<&str>,
        error_message: Option<&str>,
        request_id: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) {
        let metadata = serde_json::json!({
            "request_id": request_id,
            "timestamp": chrono::Utc::now(),
            "provider": provider
        });

        let ip_addr: Option<IpNetwork> = ip_address.and_then(|ip| {
            ip.parse::<std::net::IpAddr>()
                .ok()
                .and_then(|addr| IpNetwork::new(addr, if addr.is_ipv4() { 32 } else { 128 }).ok())
        });

        let result = sqlx::query!(
            r#"SELECT public.log_auth_event(
                $1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, $10
            )"#,
            user_id,
            event_type,
            provider,
            ip_addr,
            user_agent,
            success,
            error_code,
            error_message,
            metadata,
            request_id
        )
        .execute(&self.db_pool)
        .await;

        if let Err(e) = result {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to log auth event to database"
            );
        } else {
            info!(
                request_id = %request_id,
                event_type = %event_type,
                success = %success,
                "📝 Auth event logged to database"
            );
        }
    }

    /// Main unified authentication method
    pub async fn authenticate(
        &self,
        request: &UnifiedAuthRequest,
    ) -> Result<UnifiedAuthResponse, SimpleAuthError> {
        self.authenticate_with_client_info(request, None, None).await
    }

    /// Main unified authentication method with client info for audit logging
    pub async fn authenticate_with_client_info(
        &self,
        request: &UnifiedAuthRequest,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<UnifiedAuthResponse, SimpleAuthError> {
        let start_time = std::time::Instant::now();
        let request_id = Uuid::new_v4().to_string();
        
        info!(
            request_id = %request_id,
            provider = ?request.provider_data,
            "🚀 Starting unified authentication"
        );

        let result = match &request.provider_data {
            ProviderData::Email { email, password, name } => {
                if request.create_if_not_exists && name.is_some() {
                    self.register_email_user(email, password, name.as_deref().unwrap_or(""), &request_id, ip_address, user_agent).await
                } else {
                    self.authenticate_email_user(email, password, &request_id, ip_address, user_agent).await
                }
            }
            ProviderData::Google { id_token, .. } => {
                self.authenticate_google_user(id_token, &request_id, ip_address, user_agent).await
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(mut response) => {
                response.metadata.execution_time_ms = execution_time;
                info!(
                    request_id = %request_id,
                    execution_time_ms = %execution_time,
                    "✅ Authentication successful"
                );
                Ok(response)
            }
            Err(e) => {
                error!(
                    request_id = %request_id,
                    execution_time_ms = %execution_time,
                    error = %e,
                    "❌ Authentication failed"
                );
                Err(e)
            }
        }
    }

    /// Authenticate with email/password
    async fn authenticate_email_user(
        &self,
        email: &str,
        password: &str,
        request_id: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<UnifiedAuthResponse, SimpleAuthError> {
        // Normalize email to lowercase for case-insensitive comparison
        let email_normalized = email.to_lowercase();
        info!(email = %email_normalized, "🔑 Email authentication");

        // Log login attempt
        self.log_auth_event(
            None, // We don't have user_id yet
            "login_attempt",
            "email",
            true, // Attempt itself is successful (we'll log result separately)
            None,
            None,
            request_id,
            ip_address,
            user_agent,
        ).await;

        let row = sqlx::query!(
            r#"SELECT 
                id, email, password_hash, name, is_active,
                ws_id, telegram_id, country_residence, country_origin,
                auth_providers, google_id, last_login_provider,
                email_verified_at, created_at, updated_at,
                date_of_birth, genre, trust_score, last_login_at
            FROM dim_users 
            WHERE LOWER(email) = $1 AND is_active = true"#,
            email_normalized
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| SimpleAuthError::DatabaseError(e.to_string()))?;

        let row = match row {
            Some(row) => row,
            None => {
                // Log failed login - user not found
                self.log_auth_event(
                    None,
                    "login_failure",
                    "email",
                    false,
                    Some("USER_NOT_FOUND"),
                    Some("User not found or inactive"),
                    request_id,
                    ip_address,
                    user_agent,
                ).await;
                return Err(SimpleAuthError::InvalidCredentials);
            }
        };

        // Verify password
        let password_hash = row.password_hash.as_ref().ok_or(SimpleAuthError::InvalidCredentials)?;
        let is_valid = bcrypt::verify(password, password_hash)
            .map_err(|e| SimpleAuthError::InternalError(format!("Password verify: {}", e)))?;

        if !is_valid {
            // Log failed login - invalid password
            self.log_auth_event(
                Some(row.id as i32),
                "login_failure",
                "email",
                false,
                Some("INVALID_PASSWORD"),
                Some("Invalid password"),
                request_id,
                ip_address,
                user_agent,
            ).await;
            return Err(SimpleAuthError::InvalidCredentials);
        }

        // Generate token
        let token = self.token.generate_access_token(row.id, email).await
            .map_err(|e| SimpleAuthError::InternalError(format!("Token generation: {}", e)))?;

        // Log successful login
        self.log_auth_event(
            Some(row.id as i32),
            "login_success",
            "email",
            true,
            None,
            None,
            request_id,
            ip_address,
            user_agent,
        ).await;

        Ok(UnifiedAuthResponse {
            result: AuthResult::Success {
                user: AuthenticatedUser {
                    id: row.id,
                    email: row.email.clone(),
                    name: row.name.clone(),
                    avatar_url: None, // La tabla no tiene avatar_url, se puede implementar después
                    providers: row.auth_providers.as_ref()
                        .and_then(|json| serde_json::from_value::<Vec<String>>(json.clone()).ok())
                        .unwrap_or_else(|| vec!["email".to_string()]),
                    primary_provider: row.last_login_provider.clone().unwrap_or_else(|| "email".to_string()),
                    email_verified: row.email_verified_at.is_some(),
                    account_status: AccountStatus::Active, // Inferimos que está activo porque is_active = true
                    created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
                    last_login_at: row.last_login_at,
                },
                token,
                expires_at: chrono::Utc::now() + chrono::Duration::days(90),
            },
            metadata: AuthMetadata {
                request_id: request_id.to_string(),
                provider_used: "email".to_string(),
                is_new_user: false,
                linking_performed: false,
                execution_time_ms: 0,
                timestamp: chrono::Utc::now(),
            },
        })
    }

    /// Register new email user
    async fn register_email_user(
        &self,
        email: &str,
        password: &str,
        name: &str,
        request_id: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<UnifiedAuthResponse, SimpleAuthError> {
        info!(email = %email, name = %name, "📝 Email registration");

        // Log registration attempt
        self.log_auth_event(
            None,
            "register_attempt",
            "email",
            true,
            None,
            None,
            request_id,
            ip_address,
            user_agent,
        ).await;

        // Hash password
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| SimpleAuthError::InternalError(format!("Password hash: {}", e)))?;

        // Insert user
        let row = sqlx::query!(
            r#"INSERT INTO dim_users (
                email, password_hash, name, auth_providers, last_login_provider,
                account_status, created_at, updated_at, is_active
            ) VALUES (
                $1, $2, $3, '["email"]', 'email',
                'active', NOW(), NOW(), true
            ) RETURNING id, email, name"#,
            email, password_hash, name
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| SimpleAuthError::DatabaseError(e.to_string()))?;

        // Generate token
        let token = self.token.generate_access_token(row.id, email).await
            .map_err(|e| SimpleAuthError::InternalError(format!("Token generation: {}", e)))?;

        // Log successful registration
        self.log_auth_event(
            Some(row.id as i32),
            "register_success",
            "email",
            true,
            None,
            None,
            request_id,
            ip_address,
            user_agent,
        ).await;

        Ok(UnifiedAuthResponse {
            result: AuthResult::Success {
                user: AuthenticatedUser {
                    id: row.id,
                    email: row.email.clone(),
                    name: row.name.clone(),
                    avatar_url: None, // Nuevo usuario no tiene avatar inicialmente
                    providers: vec!["email".to_string()],
                    primary_provider: "email".to_string(),
                    email_verified: false, // Nuevo usuario necesita verificar email
                    account_status: AccountStatus::Active,
                    created_at: chrono::Utc::now(),
                    last_login_at: None,
                },
                token,
                expires_at: chrono::Utc::now() + chrono::Duration::days(90),
            },
            metadata: AuthMetadata {
                request_id: request_id.to_string(),
                provider_used: "email".to_string(),
                is_new_user: true,
                linking_performed: false,
                execution_time_ms: 0,
                timestamp: chrono::Utc::now(),
            },
        })
    }

    /// Authenticate with Google
    async fn authenticate_google_user(
        &self,
        id_token: &str,
        request_id: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<UnifiedAuthResponse, SimpleAuthError> {
        info!("🔑 Google authentication");

        // Log Google auth attempt
        self.log_auth_event(
            None,
            "google_auth",
            "google",
            true,
            None,
            None,
            request_id,
            ip_address,
            user_agent,
        ).await;

        // Validate Google token
        let google_user = match self.google.validate_id_token(id_token).await {
            Ok(user) => user,
            Err(e) => {
                // Log Google auth failure
                self.log_auth_event(
                    None,
                    "login_failure",
                    "google",
                    false,
                    Some("GOOGLE_TOKEN_INVALID"),
                    Some(&format!("Google token validation failed: {}", e)),
                    request_id,
                    ip_address,
                    user_agent,
                ).await;
                return Err(SimpleAuthError::ProviderError(format!("Google validation: {}", e)));
            }
        };

        // Check if user exists
        let row = sqlx::query!(
            "SELECT id, email, name FROM dim_users WHERE google_id = $1 OR email = $2",
            google_user.id, google_user.email
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| SimpleAuthError::DatabaseError(e.to_string()))?;

        let (user_id, email, name, is_new_user) = if let Some(row) = row {
            // Update existing user
            sqlx::query!(
                "UPDATE dim_users SET google_id = $2, last_login_provider = 'google', updated_at = NOW() WHERE id = $1",
                row.id, google_user.id
            )
            .execute(&self.db_pool)
            .await
            .map_err(|e| SimpleAuthError::DatabaseError(e.to_string()))?;

            (row.id, row.email, row.name, false)
        } else {
            // Create new user
            let row = sqlx::query!(
                r#"INSERT INTO dim_users (
                    email, name, google_id, auth_providers, last_login_provider,
                    email_verified_at, account_status, created_at, updated_at, is_active
                ) VALUES (
                    $1, $2, $3, '["google"]', 'google',
                    NOW(), 'active', NOW(), NOW(), true
                ) RETURNING id, email, name"#,
                google_user.email, google_user.name, google_user.id
            )
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| SimpleAuthError::DatabaseError(e.to_string()))?;

            (row.id, row.email, row.name, true)
        };

        // Generate token
        let token = self.token.generate_access_token(user_id, &email.as_ref().unwrap_or(&String::new())).await
            .map_err(|e| SimpleAuthError::InternalError(format!("Token generation: {}", e)))?;

        // Log successful Google auth
        let event_type = if is_new_user { "register_success" } else { "login_success" };
        self.log_auth_event(
            Some(user_id as i32),
            event_type,
            "google",
            true,
            None,
            None,
            request_id,
            ip_address,
            user_agent,
        ).await;

        Ok(UnifiedAuthResponse {
            result: AuthResult::Success {
                user: AuthenticatedUser {
                    id: user_id,
                    email,
                    name,
                    avatar_url: google_user.picture,
                    providers: vec!["google".to_string()],
                    primary_provider: "google".to_string(),
                    email_verified: true,
                    account_status: AccountStatus::Active,
                    created_at: chrono::Utc::now(),
                    last_login_at: None,
                },
                token,
                expires_at: chrono::Utc::now() + chrono::Duration::days(90),
            },
            metadata: AuthMetadata {
                request_id: request_id.to_string(),
                provider_used: "google".to_string(),
                is_new_user,
                linking_performed: false,
                execution_time_ms: 0,
                timestamp: chrono::Utc::now(),
            },
        })
    }
}