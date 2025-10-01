use crate::{models::user::{User, SurveyState}, services::{rewards_service, whatsapp_service}, state::AppState};
use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::{types::Json, PgPool};
use std::sync::Arc;
use tracing::{info, warn};

// Re-exporting for easy access in other modules


/// Fetches a user, checking the cache first and falling back to the database.
pub async fn get_user(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<Option<User>> {
    if let Some(user) = app_state.user_cache.get(whatsapp_id) {
        tracing::info!("Cache hit for user {}", whatsapp_id);
        return Ok(Some(user.clone()));
    }

    tracing::info!("Cache miss for user {}. Querying database.", whatsapp_id);
    let user_result = sqlx::query!(
        r#"SELECT 
            id, ws_id, name, email, date_of_birth, country_origin, 
            country_residence, telegram_id, password_hash,
            email_registration_date, ws_registration_date, telegram_registration_date,
            -- New unified auth fields with defaults
            COALESCE(auth_providers, '[]'::jsonb) as auth_providers,
            google_id, auth_metadata, email_verified_at, last_login_provider,
            COALESCE(account_status, 'active') as account_status,
            created_at, updated_at, last_login_at,
            COALESCE(is_active, true) as is_active
        FROM dim_users WHERE ws_id = $1"#, 
        whatsapp_id
    )
    .fetch_optional(&app_state.db_pool)
    .await?;

    let user = user_result.map(|row| User {
        id: row.id,
        email: row.email,
        password_hash: row.password_hash,
        name: row.name,
        avatar_url: None, // Not in current schema
        phone: None,      // Not in current schema
        country: row.country_origin.clone(), // Use country_origin as country
        is_active: row.is_active.unwrap_or(true),
        
        // Legacy fields
        ws_id: row.ws_id,
        date_of_birth: row.date_of_birth,
        country_origin: row.country_origin,
        country_residence: row.country_residence,
        telegram_id: row.telegram_id,
        email_registration_date: row.email_registration_date.map(|d| d.to_rfc3339()),
        ws_registration_date: row.ws_registration_date,
        telegram_registration_date: row.telegram_registration_date,
        
        // Unified Auth Fields (simplified)
        auth_providers: row.auth_providers.map(|v| v.to_string()),
        google_id: row.google_id,
        auth_metadata: row.auth_metadata.map(|v| v.to_string()),
        email_verified_at: row.email_verified_at.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
        last_login_provider: row.last_login_provider,
        account_status: match row.account_status.as_deref() {
            Some("active") => crate::models::user::AccountStatus::Active,
            Some("suspended") => crate::models::user::AccountStatus::Suspended,
            Some("pending_verification") => crate::models::user::AccountStatus::PendingVerification,
            Some("locked") => crate::models::user::AccountStatus::Locked,
            _ => crate::models::user::AccountStatus::Active,
        },
        
        // Timestamps
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        last_login_at: row.last_login_at,
    });

    if let Some(ref found_user) = user {
        app_state.user_cache.set(whatsapp_id.to_string(), found_user.clone());
        tracing::info!("User {} stored in cache.", whatsapp_id);
    }

    Ok(user)
}

pub async fn is_user_registered(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<bool> {
    Ok(get_user(app_state, whatsapp_id).await?.is_some())
}

pub async fn get_user_lumis_balance(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<Option<i32>> {
    if let Some(user) = get_user(app_state, whatsapp_id).await? {
        let balance = rewards_service::get_user_balance(&app_state.db_pool, user.id.try_into().unwrap()).await?;
        Ok(Some(balance))
    } else {
        Ok(None)
    }
}

pub async fn get_user_id_by_ws_id(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<Option<i32>> {
    if let Some(user) = get_user(app_state, whatsapp_id).await? {
        Ok(Some(user.id.try_into().unwrap()))
    } else {
        Ok(None)
    }
}

/// Fetches a user by email directly from the database
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    tracing::info!("Querying user by email: {}", email);
    let user_result = sqlx::query!(
        r#"SELECT 
            id, ws_id, name, email, date_of_birth, country_origin, 
            country_residence, telegram_id, password_hash,
            email_registration_date, ws_registration_date, telegram_registration_date,
            -- New unified auth fields with defaults
            COALESCE(auth_providers, '[]'::jsonb) as auth_providers,
            google_id, auth_metadata, email_verified_at, last_login_provider,
            COALESCE(account_status, 'active') as account_status,
            created_at, updated_at, last_login_at,
            COALESCE(is_active, true) as is_active
        FROM dim_users WHERE email = $1"#, 
        email
    )
    .fetch_optional(pool)
    .await?;

    let user = user_result.map(|row| User {
        id: row.id,
        email: row.email,
        password_hash: row.password_hash,
        name: row.name,
        avatar_url: None, // Not in current schema
        phone: None,      // Not in current schema
        country: row.country_origin.clone(), // Use country_origin as country
        is_active: row.is_active.unwrap_or(true),
        
        // Legacy fields
        ws_id: row.ws_id,
        date_of_birth: row.date_of_birth,
        country_origin: row.country_origin,
        country_residence: row.country_residence,
        telegram_id: row.telegram_id,
        email_registration_date: row.email_registration_date.map(|d| d.to_rfc3339()),
        ws_registration_date: row.ws_registration_date,
        telegram_registration_date: row.telegram_registration_date,
        
        // Unified Auth Fields (simplified)
        auth_providers: row.auth_providers.map(|v| v.to_string()),
        google_id: row.google_id,
        auth_metadata: row.auth_metadata.map(|v| v.to_string()),
        email_verified_at: row.email_verified_at.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
        last_login_provider: row.last_login_provider,
        account_status: match row.account_status.as_deref() {
            Some("active") => crate::models::user::AccountStatus::Active,
            Some("suspended") => crate::models::user::AccountStatus::Suspended,
            Some("pending_verification") => crate::models::user::AccountStatus::PendingVerification,
            Some("locked") => crate::models::user::AccountStatus::Locked,
            _ => crate::models::user::AccountStatus::Active,
        },
        
        // Timestamps
        created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: row.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        last_login_at: row.last_login_at,
    });

    Ok(user)
}

pub async fn is_user_subscribed(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<bool> {
    if let Some(_user) = get_user(app_state, whatsapp_id).await? {
        // For now, we consider any registered user as subscribed.
        // This can be expanded later to check a specific subscription table.
        Ok(true)
    } else {
        Ok(false)
    }
}


#[derive(Debug, Deserialize)]
pub struct TotalSummary {
    pub facturas: Option<i64>,
    pub itbms: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct LastInvoice {
    pub date: Option<String>,
    pub issuer_name: Option<String>,
    pub tot_amount: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MonthlyConsumption {
    pub mes: Option<String>,
    pub monto: Option<f64>,
    pub comercios: Option<i64>,
    pub num_facturas: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TopMerchant {
    pub issuer_name: Option<String>,
    pub visitas: Option<i64>,
    pub monto: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TopProduct {
    pub description: Option<String>,
    pub qty: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserMetrics {
    pub user_email: String,
    #[sqlx(rename = "tot_invoices")]
    pub sm_totals: Option<Json<Vec<TotalSummary>>>,
    #[sqlx(rename = "top_issuers")]
    pub sm_top_comercios: Option<Json<Vec<TopMerchant>>>,
    #[sqlx(rename = "top_products")]
    pub sm_top_productos: Option<Json<Vec<TopProduct>>>,
    #[sqlx(rename = "latest_invoice")]
    pub sm_ultima_factura: Option<Json<Vec<LastInvoice>>>,
    #[sqlx(rename = "summ_mes")]
    pub sm_consumo_6_meses: Option<Json<Vec<MonthlyConsumption>>>,
}

pub async fn get_user_summary(app_state: &Arc<AppState>, user_email: &str) -> Result<Option<UserMetrics>> {
    sqlx::query_as::<_, UserMetrics>(
        "SELECT * FROM public.vw_usr_general_metrics WHERE user_email = $1",
    )
    .bind(user_email)
    .fetch_optional(&app_state.db_pool)
    .await
    .context("Failed to fetch user summary from DB")
}

/// Finds a user by WhatsApp ID. If the user doesn't exist, it creates a new one.
pub async fn find_or_create_user(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<User> {
    if let Some(user) = get_user(app_state, whatsapp_id).await? {
        Ok(user)
    } else {
        tracing::info!("User with ws_id {} not found. Creating a new user.", whatsapp_id);
        let new_user = create_user(&app_state.db_pool, whatsapp_id, &SurveyState::default()).await?;
        // Add the new user to the cache
        app_state.user_cache.set(whatsapp_id.to_string(), new_user.clone());
        Ok(new_user)
    }
}

pub async fn create_user(pool: &PgPool, ws_id: &str, survey_data: &SurveyState) -> Result<User> {
        sqlx::query_as(
        r#"
        INSERT INTO dim_users (ws_id, name, email, date_of_birth, country_origin, country_residence)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, ws_id, name, email, date_of_birth, country_origin, country_residence, created_at, updated_at, lumis_balance, is_active, last_interaction_at, user_level, user_status, referral_code, referred_by
        "#
    )
    .bind(ws_id)
    .bind(&survey_data.name)
    .bind(&survey_data.email)
    .bind(&survey_data.birth_date)
    .bind(&survey_data.country_of_origin)
    .bind(&survey_data.country_of_residence)
    .fetch_one(pool)
    .await
    .context("Failed to create user")
}

pub async fn get_and_format_user_metrics(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    info!("Starting get_and_format_user_metrics for user: {}", whatsapp_id);
    
    if let Some(user) = get_user(app_state, whatsapp_id).await? {
        info!("Found user with email: {:?}", user.email);
        
        // Query user metrics from the same view that Python uses
        let query = r#"
            SELECT * FROM public.vw_usr_general_metrics WHERE user_email = $1
        "#;
        
        info!("Executing query for user email: {:?}", user.email);
        let metrics = sqlx::query_as::<_, UserMetrics>(query)
            .bind(&user.email)
            .fetch_optional(&app_state.db_pool)
            .await?;
        
        info!("Query executed, metrics found: {}", metrics.is_some());
        
        if let Some(user_metrics) = metrics {
            // Extract totals data
            let (total_invoices, total_itbms) = user_metrics.sm_totals
                .as_ref()
                .and_then(|json| json.0.first())
                .map(|summary| (
                    summary.facturas.unwrap_or(0),
                    summary.itbms.unwrap_or(0.0)
                ))
                .unwrap_or((0, 0.0));
            
            if total_invoices > 0 {
                // Build the complete dashboard like Python
                let mut dashboard = "📊 *Resumen de tus Facturas*\n\n".to_string();
                
                // 1. Totals section
                dashboard.push_str(&format!(
                    "🔢 *Totales:*\nTotal de facturas subidas: {} facturas\nTotal pagado en ITBMS: ${:.1}\n\n",
                    total_invoices, total_itbms
                ));
                
                // 2. Last invoice section
                if let Some(last_invoice) = user_metrics.sm_ultima_factura
                    .as_ref()
                    .and_then(|json| json.0.first()) {
                    
                    let date = last_invoice.date.as_ref().map(|d| &d[..10]).unwrap_or("N/A");
                    let issuer = last_invoice.issuer_name.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                    let amount = last_invoice.tot_amount.unwrap_or(0.0);
                    
                    dashboard.push_str(&format!(
                        "📥 *Última factura subida:*\nFecha: {}\nComercio: {}\nValor: ${:.1}\n\n",
                        date, issuer, amount
                    ));
                }
                
                // 3. 6-month consumption section
                dashboard.push_str("🛍 *Consumo últimos 6 meses:*\n");
                if let Some(consumption) = user_metrics.sm_consumo_6_meses.as_ref() {
                    // Sort by date descending (most recent first)
                    let mut sorted_months = consumption.0.clone();
                    sorted_months.sort_by(|a, b| {
                        let date_a = a.mes.as_ref().map(|s| s.as_str()).unwrap_or("1900-01-01");
                        let date_b = b.mes.as_ref().map(|s| s.as_str()).unwrap_or("1900-01-01");
                        date_b.cmp(date_a) // Reverse order for descending
                    });
                    
                    for (i, month) in sorted_months.iter().enumerate() {
                        if i >= 6 { break; } // Limit to 6 months
                        let mes = month.mes.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                        let monto = month.monto.unwrap_or(0.0);
                        let comercios = month.comercios.unwrap_or(0);
                        let facturas = month.num_facturas.unwrap_or(0);
                        
                        dashboard.push_str(&format!(
                            "{}. {} - ${:.2} - {} comercios - {}\n",
                            i + 1, mes, monto, comercios, facturas
                        ));
                    }
                }
                
                // 4. Top 5 merchants section
                dashboard.push_str("\n🏪 *Top 5 comercios (Últimos 6 meses):*\n");
                if let Some(merchants) = user_metrics.sm_top_comercios.as_ref() {
                    for (i, merchant) in merchants.0.iter().enumerate() {
                        if i >= 5 { break; } // Limit to top 5
                        let name = merchant.issuer_name.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                        let visits = merchant.visitas.unwrap_or(0);
                        let amount = merchant.monto.unwrap_or(0.0);
                        
                        dashboard.push_str(&format!(
                            "{}. {} - {} visitas - ${:.2}\n",
                            i + 1, name, visits, amount
                        ));
                    }
                }
                
                // 5. Top 10 products section
                dashboard.push_str("\n🏷️ *Top 10 Productos Más Comprados (Últimos 6 meses):*\n");
                if let Some(products) = user_metrics.sm_top_productos.as_ref() {
                    for (i, product) in products.0.iter().enumerate() {
                        if i >= 10 { break; } // Limit to top 10
                        let description = product.description.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                        let qty = product.qty.unwrap_or(0.0) as i32;
                        
                        dashboard.push_str(&format!(
                            "{}. {} - {} unidades\n",
                            i + 1, description, qty
                        ));
                    }
                }
                
                whatsapp_service::send_text_message(app_state, whatsapp_id, &dashboard).await?;
            } else {
                info!("User has 0 invoices, sending no invoices message");
                let message = format!(
                    "{}{}{}",
                    "📊 *Hola, Lümier!*\n\n",
                    "📋 **No tienes facturas registradas**\n\n",
                    "¡Sube tu primera factura para comenzar a ganar Lümis! 🎯"
                );
                
                whatsapp_service::send_text_message(app_state, whatsapp_id, &message).await?;
            }
        } else {
            warn!("No metrics found for user email: {:?}", user.email);
            whatsapp_service::send_text_message(
                app_state,
                whatsapp_id,
                "No se pudieron obtener tus datos. Inténtalo más tarde."
            ).await?;
        }
    } else {
        warn!("No user found for whatsapp_id: {}", whatsapp_id);
        whatsapp_service::send_text_message(
            app_state,
            whatsapp_id,
            "No se encontró tu perfil. Usa /registro para crear tu cuenta."
        ).await?;
    }
    Ok(())
}
