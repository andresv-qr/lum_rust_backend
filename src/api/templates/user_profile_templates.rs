use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Response model for user profile query from dim_users table
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct UserProfileResponse {
    pub id: i64,
    pub email: String,
    pub ws_id: Option<String>,
    pub telegram_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub ws_registration_date: Option<DateTime<Utc>>,
    pub telegram_registration_date: Option<DateTime<Utc>>,
    pub name: Option<String>,
    pub date_of_birth: Option<String>,
    pub country_origin: Option<String>,
    pub country_residence: Option<String>,
    pub password_hash: Option<String>,
    pub segment_activity: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Sanitized response model (without sensitive data like password)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfileSafeResponse {
    pub id: i64,
    pub email: String,
    pub ws_id: Option<String>,
    pub telegram_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub ws_registration_date: Option<DateTime<Utc>>,
    pub telegram_registration_date: Option<DateTime<Utc>>,
    pub name: Option<String>,
    pub date_of_birth: Option<String>,
    pub country_origin: Option<String>,
    pub country_residence: Option<String>,
    pub segment_activity: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub has_password: bool,
    pub registration_platforms: Vec<String>,
}

impl From<UserProfileResponse> for UserProfileSafeResponse {
    fn from(user: UserProfileResponse) -> Self {
        let mut platforms = Vec::new();
        
        if user.created_at.is_some() {
            platforms.push("email".to_string());
        }
        if user.ws_registration_date.is_some() {
            platforms.push("whatsapp".to_string());
        }
        if user.telegram_registration_date.is_some() {
            platforms.push("telegram".to_string());
        }

        Self {
            id: user.id,
            email: user.email,
            ws_id: user.ws_id,
            telegram_id: user.telegram_id,
            created_at: user.created_at,
            ws_registration_date: user.ws_registration_date,
            telegram_registration_date: user.telegram_registration_date,
            name: user.name,
            date_of_birth: user.date_of_birth,
            country_origin: user.country_origin,
            country_residence: user.country_residence,
            segment_activity: user.segment_activity,
            updated_at: user.updated_at,
            has_password: user.password_hash.is_some() && !user.password_hash.as_ref().unwrap().is_empty(),
            registration_platforms: platforms,
        }
    }
}

/// Query templates for user profile operations
pub struct UserProfileQueryTemplates;

impl UserProfileQueryTemplates {
    /// Get user profile by email query
    pub fn get_user_by_email_query() -> &'static str {
        "SELECT id, email, ws_id, telegram_id, created_at, created_at as ws_registration_date, created_at as telegram_registration_date, name, date_of_birth, country_origin, country_residence, password_hash, segment_activity, updated_at FROM public.dim_users WHERE email = $1"
    }

    /// Get user profile by ID query
    pub fn get_user_by_id_query() -> &'static str {
        "SELECT id, email, ws_id, telegram_id, created_at, created_at as ws_registration_date, created_at as telegram_registration_date, name, date_of_birth, country_origin, country_residence, password_hash, segment_activity, updated_at FROM public.dim_users WHERE id = $1"
    }

    /// Search users by name or email query
    pub fn search_users_query() -> &'static str {
        "SELECT id, email, ws_id, telegram_id, created_at, created_at as ws_registration_date, created_at as telegram_registration_date, name, date_of_birth, country_origin, country_residence, password_hash, segment_activity, updated_at FROM public.dim_users WHERE email ILIKE $1 OR name ILIKE $1 ORDER BY created_at DESC LIMIT 50"
    }

    /// Cache key prefix for user by email
    pub fn get_user_by_email_cache_key_prefix() -> &'static str {
        "user_profile_email"
    }

    /// Cache key prefix for user by ID
    pub fn get_user_by_id_cache_key_prefix() -> &'static str {
        "user_profile_id"
    }

    /// Cache key prefix for user search
    pub fn get_user_search_cache_key_prefix() -> &'static str {
        "user_profile_search"
    }
}

/// Helper functions for user profile operations
pub struct UserProfileHelpers;

impl UserProfileHelpers {
    /// Generate user summary message
    pub fn generate_user_summary(user: &UserProfileSafeResponse) -> String {
        let name = user.name.as_deref().unwrap_or("Usuario");
        let platforms_count = user.registration_platforms.len();
        let platforms_text = user.registration_platforms.join(", ");
        
        match platforms_count {
            0 => format!("👤 {}: Usuario sin plataformas registradas", name),
            1 => format!("📱 {}: Registrado en {}", name, platforms_text),
            _ => format!("🌐 {}: Multi-plataforma ({})", name, platforms_text),
        }
    }

    /// Check if user profile is complete
    pub fn is_profile_complete(user: &UserProfileSafeResponse) -> bool {
        user.name.is_some() && 
        user.date_of_birth.is_some() && 
        user.country_origin.is_some() && 
        user.country_residence.is_some()
    }

    /// Get profile completion percentage
    pub fn get_profile_completion_percentage(user: &UserProfileSafeResponse) -> u8 {
        let mut completed_fields = 0;
        let total_fields = 7; // name, date_of_birth, country_origin, country_residence, ws_id, telegram_id, segment_activity

        if user.name.is_some() { completed_fields += 1; }
        if user.date_of_birth.is_some() { completed_fields += 1; }
        if user.country_origin.is_some() { completed_fields += 1; }
        if user.country_residence.is_some() { completed_fields += 1; }
        if user.ws_id.is_some() { completed_fields += 1; }
        if user.telegram_id.is_some() { completed_fields += 1; }
        if user.segment_activity.is_some() { completed_fields += 1; }

        ((completed_fields as f32 / total_fields as f32) * 100.0) as u8
    }

    /// Sanitize email for cache key
    pub fn sanitize_email_for_cache(email: &str) -> String {
        email.to_lowercase().replace("@", "_at_").replace(".", "_dot_")
    }

    /// Check if user data is stale based on updated_at
    pub fn is_user_data_stale(updated_at: Option<DateTime<Utc>>, max_age_hours: i64) -> bool {
        match updated_at {
            Some(date) => {
                let max_age = chrono::Duration::hours(max_age_hours);
                let cutoff_time = Utc::now() - max_age;
                date < cutoff_time
            },
            None => true, // No update date means stale
        }
    }

    /// Generate cache key with updated_at for versioning
    pub fn generate_cache_key_with_version(prefix: &str, identifier: &str, updated_at: Option<DateTime<Utc>>) -> String {
        match updated_at {
            Some(date) => format!("{}_{}_{}", prefix, identifier, date.timestamp()),
            None => format!("{}_{}_{}", prefix, identifier, "no_date"),
        }
    }

    /// Format updated_at for user-friendly display
    pub fn format_last_update(updated_at: Option<DateTime<Utc>>) -> String {
        match updated_at {
            Some(date) => {
                let now = Utc::now();
                let duration = now.signed_duration_since(date);
                
                if duration.num_days() > 0 {
                    format!("Actualizado hace {} días", duration.num_days())
                } else if duration.num_hours() > 0 {
                    format!("Actualizado hace {} horas", duration.num_hours())
                } else if duration.num_minutes() > 0 {
                    format!("Actualizado hace {} minutos", duration.num_minutes())
                } else {
                    "Actualizado recientemente".to_string()
                }
            },
            None => "Fecha de actualización no disponible".to_string(),
        }
    }
}
