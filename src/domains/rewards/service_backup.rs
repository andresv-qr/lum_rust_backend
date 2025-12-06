use crate::{
    models::whatsapp::{Row, Section},
    services::user_service,
    services::whatsapp_service,
    state::AppState,
};
use anyhow::Result;
use sqlx::types::Decimal;
use chrono::{DateTime, Utc, Duration};
use sqlx::{types::Json, PgPool};
use std::sync::Arc;

#[derive(Debug, sqlx::FromRow)]
pub struct Redemption {
    pub redem_id: Option<String>,
    pub quantity: Option<i32>,
    pub date: Option<DateTime<Utc>>,
    #[sqlx(rename = "condition1")]
    pub condition: Option<String>,
    pub async fn get_user_summary(
        &self,
        user_id: i32,
        query_params: Option<UserSummaryQuery>,
    ) -> Result<UserSummaryResponse> {

#[derive(Debug, sqlx::FromRow)]
pub struct Reward {
    pub id: i32,
    pub name: Option<String>,
    pub points: Option<i32>,
}

pub async fn get_reward_by_id(pool: &PgPool, redemption_id: i32) -> Result<Option<Reward>> {
    let reward = sqlx::query_as::<_, Reward>(
        "SELECT id, name, points FROM rewards.dim_redemptions WHERE id = $1"
    )
    .bind(redemption_id)
    .fetch_optional(pool)
    .await?;
    Ok(reward)
}

pub async fn redeem_reward(pool: &PgPool, user_id: i64, reward: &Reward) -> Result<()> {
    let mut tx = pool.begin().await?;

    let balance_record = sqlx::query!(
        "SELECT balance::integer FROM rewards.fact_balance_points WHERE user_id = $1 FOR UPDATE",
        user_id as i32
    )
    .fetch_optional(&mut *tx)
    .await?;

    let current_balance = balance_record.map_or(0, |r| r.balance.unwrap_or(0));
    let points_cost = reward.points.unwrap_or(0);

    if current_balance < points_cost {
        return Err(anyhow::anyhow!(format!("Saldo insuficiente. Tienes {} Lümis y necesitas {}.", current_balance, points_cost)));
    }

    let new_balance = current_balance - points_cost;
    sqlx::query!(
        "UPDATE rewards.fact_balance_points SET balance = $1 WHERE user_id = $2",
        new_balance as i32,
        user_id as i32
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO rewards.fact_redemptions_legacy (user_id, redem_id, quantity, date, condition1)
        VALUES ($1, $2, $3, NOW(), $4)
        "#,
        user_id as i32,
        reward.id.to_string(),
        Decimal::from(reward.points.unwrap_or(0)),
        "Canjeado"
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn get_user_balance(pool: &PgPool, user_id: i64) -> Result<i32> {
    let result = sqlx::query!(
        "SELECT balance::integer FROM rewards.fact_balance_points WHERE user_id = $1",
        user_id as i32
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map_or(0, |row| row.balance.unwrap_or(0)))
}

pub async fn get_user_redemption_history(pool: &PgPool, user_id: i64, limit: i64) -> Result<Vec<Redemption>> {
    let rows = sqlx::query_as::<_, Redemption>(
        "SELECT redem_id, quantity::integer, date, condition1 FROM rewards.fact_redemptions_legacy WHERE user_id = $1 ORDER BY date DESC LIMIT $2",
    )
    .bind(user_id as i32)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn activate_radar_ofertas(pool: &PgPool, user_id: i64) -> Result<()> {
    let expiration_date = Utc::now() + Duration::days(365 * 10);
    sqlx::query!(
        r#"
        INSERT INTO rewards.fact_redemptions_legacy (user_id, redem_id, date, expiration_date, quantity, condition1)
        VALUES ($1, 'red_radarofertas', NOW(), $2, 1, 'active')
        ON CONFLICT (user_id, redem_id) DO UPDATE
        SET expiration_date = $2, date = NOW();
        "#,
        user_id as i32,
        expiration_date
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Inicia el flujo completo de Radar de Ofertas replicando el comportamiento de Python
pub async fn start_radar_ofertas_flow(app_state: &Arc<AppState>, whatsapp_id: &str, user_id: i64) -> Result<()> {
    use crate::services::{redis_service, whatsapp_service};
    use crate::models::user::UserState;
    use tracing::info;
    
    info!("Starting radar ofertas flow for whatsapp_id: {} with user_id: {}", whatsapp_id, user_id);
    
    // Consultar las categorías disponibles para el usuario usando la función dedicada
    let available_categories = get_available_offer_categories(&app_state.db_pool, user_id).await?;

    if available_categories.is_empty() {
        // No hay categorías disponibles, enviar resumen como en Python
        let message = "📭 No tienes categorías de ofertas activas disponibles en este momento.\n\n💡 Te notificaremos cuando tengamos ofertas disponibles para ti.";
        whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;
        return Ok(());
    }

    // Construir mensaje con categorías disponibles
    let mut message = "📋 *Categorías disponibles:*\n\n".to_string();
    
    for (i, category) in available_categories.iter().enumerate() {
        message.push_str(&format!("{}. {}\n", i + 1, category));
    }
    
    message.push_str("\n*Escribe el nombre de la categoría que te interesa*");
    
    // Enviar mensaje con categorías
    whatsapp_service::send_text_message(app_state, whatsapp_id, &message).await?;
    
    // Guardar estado del usuario para el flujo de selección de categoría
    let price_range_state = serde_json::json!({
        "step": "seleccionar_categoria",
        "categorias_disponibles": available_categories
    });
    
    redis_service::save_user_state(
        app_state,
        whatsapp_id,
        &UserState::PriceRange(price_range_state.to_string()),
        600 // 10 minutos TTL
    ).await?;
    
    Ok(())
}

pub async fn has_active_product_search_subscription(pool: &PgPool, user_id: i64) -> Result<bool> {
    let subscription = sqlx::query!(
        "SELECT id FROM rewards.fact_redemptions_legacy WHERE user_id = $1 AND redem_id = '2' AND expiration_date >= NOW()",
        user_id as i32
    )
    .fetch_optional(pool)
    .await?;

    Ok(subscription.is_some())
}

/// Get available offer categories for a user
pub async fn get_available_offer_categories(pool: &PgPool, user_id: i64) -> Result<Vec<String>> {
    use tracing::{info, warn};
    
    info!("Searching for offer categories for user_id: {}", user_id);
    
    let categories = sqlx::query!(
        "SELECT DISTINCT condition1 FROM rewards.fact_redemptions_legacy WHERE user_id = $1 AND redem_id = '0' AND expiration_date >= CURRENT_DATE",
        user_id as i32
    )
    .fetch_all(pool)
    .await?;

    info!("Found {} raw category rows for user_id: {}", categories.len(), user_id);

    let mut result = Vec::new();
    for (i, row) in categories.iter().enumerate() {
        info!("Category row {}: condition1 = {:?}", i, row.condition1);
        if let Some(category) = &row.condition1 {
            if !category.trim().is_empty() {
                info!("Adding valid category: '{}'", category);
                result.push(category.clone());
            } else {
                warn!("Skipping empty category for user_id: {}", user_id);
            }
        } else {
            warn!("Skipping null category for user_id: {}", user_id);
        }
    }
    
    info!("Final result: {} categories for user_id: {} - {:?}", result.len(), user_id, result);
    Ok(result)
}

/// Search for offers in a specific category and price range
pub async fn search_offers_in_category(
    pool: &PgPool, 
    user_id: i64, 
    category: &str, 
    min_price: f64, 
    max_price: f64
) -> Result<Vec<OfferResult>> {
    use tracing::info;
    
    info!("Searching offers for user_id: {}, category: '{}', price_range: {}-{}", 
          user_id, category, min_price, max_price);
    
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    
    let min_decimal = Decimal::from_f64(min_price).unwrap_or_default();
    let max_decimal = Decimal::from_f64(max_price).unwrap_or_default();
    
    let offers = sqlx::query_as!(
        OfferResult,
        r#"
        SELECT 
            product_name as category,
            product_name as offer_title,
            CONCAT('Precio: $', current_price::text, ' (Descuento: ', descu_perc::text, '%)') as offer_description,
            NULL::timestamptz as expiration_date
        FROM rewards.ws_offers 
        WHERE LOWER(product_name) LIKE LOWER('%' || $1 || '%')
            AND current_price BETWEEN $2 AND $3
        ORDER BY days_in_row ASC
        LIMIT 10
        "#,
        category,
        min_decimal,
        max_decimal
    )
    .fetch_all(pool)
    .await?;

    info!("Found {} offers for user_id: {} in category '{}'", offers.len(), user_id, category);
    
    for (i, offer) in offers.iter().enumerate() {
        info!("Offer {}: category={:?}, title={:?}, description={:?}, expiration={:?}", 
              i+1, offer.category, offer.offer_title, offer.offer_description, offer.expiration_date);
    }

    Ok(offers)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OfferResult {
    pub category: Option<String>,
    pub offer_title: Option<String>,
    pub offer_description: Option<String>,
    pub expiration_date: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn send_user_metrics_dashboard(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    if let Some(user) = user_service::get_user(app_state, ws_id).await? {
        if user.email.is_some() {
            match user_service::get_user_summary(app_state, &user.email.unwrap()).await {
                Ok(Some(metrics)) => {
                    let mut dashboard = format!("📊 *Hola, Lümier!* 📊\n\nEste es tu resumen de movimientos:\n\n");

                    if let Some(Json(totals)) = metrics.sm_totals {
                        if let Some(total) = totals.get(0) {
                            dashboard.push_str(&format!("🧾 Total Facturas: {}\n", total.facturas.unwrap_or(0)));
                            dashboard.push_str(&format!("💳 Total Pagado en ITBMS: ${:.2}\n\n", total.itbms.unwrap_or(0.0)));
                        }
                    }

                    if let Some(Json(last_invoices)) = metrics.sm_ultima_factura {
                        if let Some(last_invoice) = last_invoices.get(0) {
                            dashboard.push_str(&format!("📥 *Última factura subida:*\n"));
                            dashboard.push_str(&format!("Fecha: {}\n", last_invoice.date.as_deref().unwrap_or("N/A")));
                            dashboard.push_str(&format!("Comercio: {}\n", last_invoice.issuer_name.as_deref().unwrap_or("N/A")));
                            dashboard.push_str(&format!("Valor: ${:.1}\n\n", last_invoice.tot_amount.unwrap_or(0.0)));
                        }
                    }

                    if let Some(Json(mut consumption)) = metrics.sm_consumo_6_meses {
                        dashboard.push_str("🛒 *Consumo últimos 6 meses:*\n");
                        consumption.sort_by(|a, b| b.mes.cmp(&a.mes));
                        for (i, month) in consumption.iter().enumerate() {
                            dashboard.push_str(&format!("{}. {} - ${:.2} - {} comercios - {} facturas\n", i + 1, month.mes.as_deref().unwrap_or("N/A"), month.monto.unwrap_or(0.0), month.comercios.unwrap_or(0), month.num_facturas.unwrap_or(0)));
                        }
                        dashboard.push_str("\n");
                    }

                    if let Some(Json(merchants)) = metrics.sm_top_comercios {
                        dashboard.push_str("🏪 *Top 5 comercios (Últimos 6 meses):*\n");
                        for (i, merchant) in merchants.iter().enumerate() {
                            dashboard.push_str(&format!("{}. {} - {} visitas - ${:.2}\n", i + 1, merchant.issuer_name.as_deref().unwrap_or("N/A"), merchant.visitas.unwrap_or(0), merchant.monto.unwrap_or(0.0)));
                        }
                        dashboard.push_str("\n");
                    }

                    if let Some(Json(products)) = metrics.sm_top_productos {
                        dashboard.push_str("🏷️ *Top 10 Productos Más Comprados (Últimos 6 meses):*\n");
                        for (i, product) in products.iter().enumerate() {
                            dashboard.push_str(&format!("{}. {} - {:.0} unidades\n", i + 1, product.description.as_deref().unwrap_or("N/A"), product.qty.unwrap_or(0.0)));
                        }
                    }

                    whatsapp_service::send_text_message(app_state, ws_id, &dashboard).await?;
                },
                Ok(None) => {
                    whatsapp_service::send_text_message(app_state, ws_id, "No encontramos datos de movimientos para tu usuario.").await?;
                },
                Err(e) => {
                    tracing::error!("Error fetching user metrics: {}", e);
                    whatsapp_service::send_text_message(app_state, ws_id, "Tuvimos un problema al consultar tu resumen. Por favor, intenta de nuevo más tarde.").await?;
                }
            }
        } else {
            whatsapp_service::send_text_message(app_state, ws_id, "Necesitas tener un email registrado para ver tu resumen.").await?;
        }
    } else {
        whatsapp_service::send_text_message(app_state, ws_id, "Debes estar registrado para ver tu resumen. Usa /start para registrarte.").await?;
    }
    Ok(())
}

pub async fn send_comparison_dashboard(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    if user_service::get_user(app_state, ws_id).await?.is_some() {
        let reply = "🧬 *Compararte*\n\n¿Quieres saber cómo se comparan tus hábitos con los de otros usuarios? ¡Estamos procesando los datos para mostrarte una comparativa fascinante! Esta función estará disponible pronto.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    } else {
        let reply = "Debes estar registrado para usar esta función. Usa /start para registrarte.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    }
    Ok(())
}

pub async fn send_giftcard_info(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    if user_service::get_user(app_state, ws_id).await?.is_some() {
        let reply = "🎁 *Giftcard* 🎁\n\n¡Pronto podrás canjear tus Lumis por giftcards de tus comercios favoritos! Estamos trabajando para que esta opción esté disponible lo antes posible.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    } else {
        let reply = "Debes estar registrado para usar esta función. Usa /start para registrarte.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    }
    Ok(())
}

pub async fn send_prizes_info(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    if user_service::get_user(app_state, ws_id).await?.is_some() {
        let reply = "🏆 *Premios* 🏆\n\n¡Consulta nuestro catálogo de premios! Estamos añadiendo nuevas y emocionantes recompensas constantemente. ¡No te las pierdas!";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    } else {
        let reply = "Debes estar registrado para usar esta función. Usa /start para registrarte.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    }
    Ok(())
}

pub async fn send_challenges_info(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    if user_service::get_user(app_state, ws_id).await?.is_some() {
        let reply = "🎯 *Retos y Misiones* 🎯\n\n¡Completa retos y misiones para ganar más Lumis! Nuevos desafíos te esperan cada semana. ¿Estás listo?";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    } else {
        let reply = "Debes estar registrado para usar esta función. Usa /start para registrarte.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    }
    Ok(())
}

pub async fn send_tombola_cash_confirmation(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    if user_service::get_user(app_state, ws_id).await?.is_some() {
        let reply = "💸 *Tómbola de Cash*\n\n¡Tu participación en la tómbola de dinero ha sido registrada! Te notificaremos si resultas ganador. ¡Mucha suerte!";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    } else {
        let reply = "Debes estar registrado para usar esta función. Usa /start para registrarte.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    }
    Ok(())
}

pub async fn send_tombola_merch_confirmation(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    if user_service::get_user(app_state, ws_id).await?.is_some() {
        let reply = "🧢 *Tómbola de Merch*\n\n¡Tu participación en la tómbola de merch ha sido registrada! Te notificaremos si resultas ganador. ¡Mucha suerte!";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    } else {
        let reply = "Debes estar registrado para usar esta función. Usa /start para registrarte.";
        whatsapp_service::send_text_message(app_state, ws_id, reply).await?;
    }
    Ok(())
}

pub async fn deduct_lumis_for_ocr(pool: &PgPool, user_id: i64, cost: i32) -> Result<()> {
    let cost_decimal = Decimal::from(-cost);
    sqlx::query!(
        r#"
        INSERT INTO rewards.fact_redemptions_legacy (user_id, redem_id, quantity, date, condition1)
        VALUES ($1, 'ocr_service_cost', $2, NOW(), 'deducted')
        "#,
        user_id as i32,
        cost_decimal
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refund_lumis_for_ocr(pool: &PgPool, user_id: i64, cost: i32) -> Result<()> {
    let cost_decimal = Decimal::from(cost);
    sqlx::query!(
        r#"
        INSERT INTO rewards.fact_redemptions_legacy (user_id, redem_id, quantity, date, condition1)
        VALUES ($1, 'ocr_service_refund', $2, NOW(), 'refunded')
        "#,
        user_id as i32,
        cost_decimal
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn send_rewards_categories(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    let rows = vec![
        Row {
            id: "red_radarofertas".to_string(),
            title: "🔎 Radar de Ofertas".to_string(),
            description: Some("Encuentra las mejores ofertas del mercado".to_string()),
        },
        Row {
            id: "red_lumiscope".to_string(),
            title: "🧠 Lümiscope Premium".to_string(),
            description: Some("Dashboard visual de tus hábitos".to_string()),
        },
        Row {
            id: "red_compararte".to_string(),
            title: "🧬 Compararte".to_string(),
            description: Some("Compara tus hábitos con otros usuarios".to_string()),
        },
        Row {
            id: "red_giftcard".to_string(),
            title: "🎁 Giftcard Digital".to_string(),
            description: Some("Canjea por consumo real en tiendas".to_string()),
        },
        Row {
            id: "red_tombola_cash".to_string(),
            title: "💸 Tómbola de Cash".to_string(),
            description: Some("Participa por dinero real".to_string()),
        },
        Row {
            id: "red_tombola_merch".to_string(),
            title: "🧢 Tómbola de Merch".to_string(),
            description: Some("Participa por productos Lüm".to_string()),
        },
    ];

    let body_text = "Selecciona una categoría de recompensas:";
    let button_text = "Ver opciones";
    let section_title = "Opciones disponibles";
    let sections = vec![Section {
        title: section_title.to_string(),
        rows,
    }];

    whatsapp_service::send_interactive_list_message(
        app_state,
        ws_id,
        body_text,
        button_text,
        sections,
    )
    .await
}

// ===== NUEVA FUNCIONALIDAD: USER INVOICE SUMMARY API =====

use crate::models::rewards::{UserInvoiceSummary, UserSummaryResponse, UserSummaryQuery, PerformanceMetrics, TrendAnalysis};
use crate::api::common::ApiError;
use anyhow::Result;

pub struct UserSummaryService {
    pool: PgPool,
}

impl UserSummaryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Obtener resumen de facturas del usuario con métricas avanzadas
    pub async fn get_user_summary(
        &self,
        user_id: i32,
        query: Option<UserSummaryQuery>,
    ) -> Result<UserSummaryResponse, AppError> {
        let start_time = std::time::Instant::now();
        
        tracing::info!("Fetching user invoice summary for user_id: {}", user_id);

        // Obtener datos básicos de la tabla
        let summary = self.get_base_summary(user_id).await?;
        
        let query = query.unwrap_or_default();
        
        // Calcular métricas de rendimiento
        let performance_metrics = if query.include_trends.unwrap_or(true) {
            self.calculate_performance_metrics(&summary).await?
        } else {
            PerformanceMetrics::default()
        };

        // Análisis de tendencias
        let trends = if query.include_projections.unwrap_or(true) {
            self.calculate_trend_analysis(&summary).await?
        } else {
            TrendAnalysis::default()
        };

        let elapsed = start_time.elapsed();
        tracing::info!("User summary retrieved in {:?}ms for user {}", elapsed.as_millis(), user_id);

        Ok(UserSummaryResponse {
            summary,
            performance_metrics,
            trends,
        })
    }

    /// Obtener datos base de user_invoice_summary
    async fn get_base_summary(&self, user_id: i32) -> Result<UserInvoiceSummary> {
        let query = r#"
            SELECT 
                user_id, total_facturas, total_monto, total_items, 
                n_descuentos, total_descuento, top_emisores, 
                top_categorias, serie_mensual, updated_at, 
                comparativo_categoria
            FROM rewards.user_invoice_summary 
            WHERE user_id = $1
        "#;

        match sqlx::query_as::<_, UserInvoiceSummary>(query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(summary)) => {
                tracing::info!("Found summary for user {}: {} facturas, ${:.2} total monto", 
                      user_id, 
                      summary.total_facturas.unwrap_or(0), 
                      summary.total_monto.unwrap_or(0.0));
                Ok(summary)
            },
            Ok(None) => {
                tracing::warn!("No summary found for user {}, returning empty summary", user_id);
                Ok(self.create_empty_summary(user_id))
            },
            Err(e) => {
                tracing::error!("Database error fetching summary for user {}: {}", user_id, e);
                Err(AppError::database_connection(e.to_string()))
            }
        }
    }

    /// Crear resumen vacío para usuarios sin datos
    fn create_empty_summary(&self, user_id: i32) -> UserInvoiceSummary {
        let now = Utc::now();
        UserInvoiceSummary {
            user_id,
            total_facturas: Some(0),
            total_monto: Some(0.0),
            total_items: Some(0),
            n_descuentos: Some(0),
            total_descuento: Some(0.0),
            top_emisores: None,
            top_categorias: None,
            serie_mensual: None,
            updated_at: Some(now),
            comparativo_categoria: None,
        }
    }

    /// Calcular métricas de rendimiento
    async fn calculate_performance_metrics(&self, summary: &UserInvoiceSummary) -> Result<PerformanceMetrics> {
        // Crecimiento mes a mes - calculado desde serie_mensual si está disponible
        let month_over_month_growth = if let Some(_serie) = &summary.serie_mensual {
            // Intentar extraer crecimiento de la serie mensual
            // Por ahora, usar un cálculo simple basado en el número de facturas
            if let Some(total_facturas) = summary.total_facturas {
                if total_facturas > 0 {
                    15.0 // Placeholder, se puede mejorar analizando serie_mensual
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Score de frecuencia de facturas
        let invoice_frequency_score = if let Some(total_facturas) = summary.total_facturas {
            // Normalizar el número de facturas a un score de 0-100
            match total_facturas {
                x if x >= 100 => 100.0,
                x if x >= 50 => 80.0,
                x if x >= 20 => 60.0,
                x if x >= 10 => 40.0,
                x if x > 0 => 20.0,
                _ => 0.0,
            }
        } else {
            0.0
        };

        // Tier de gasto basado en total_monto
        let spending_tier = match summary.total_monto.unwrap_or(0.0) {
            x if x >= 10000.0 => "Premium".to_string(),
            x if x >= 5000.0 => "Gold".to_string(),
            x if x >= 1000.0 => "Silver".to_string(),
            x if x > 0.0 => "Bronze".to_string(),
            _ => "New".to_string(),
        };

        // Eficiencia de Lümis (placeholder - se puede calcular desde datos reales)
        let lumis_efficiency = if summary.total_monto.unwrap_or(0.0) > 0.0 {
            // Asumir 1 Lümi por cada $10 gastados como baseline
            let _expected_lumis = summary.total_monto.unwrap_or(0.0) / 10.0;
            85.0 // Placeholder efficiency score
        } else {
            100.0
        };

        Ok(PerformanceMetrics {
            month_over_month_growth,
            invoice_frequency_score,
            spending_tier,
            lumis_efficiency,
        })
    }

    /// Análisis de tendencias
    async fn calculate_trend_analysis(&self, summary: &UserInvoiceSummary) -> Result<TrendAnalysis, AppError> {
        // Tendencia mensual basada en la serie_mensual si está disponible
        let monthly_trend = if let Some(_serie) = &summary.serie_mensual {
            "increasing".to_string() // Placeholder - se puede analizar la serie real
        } else {
            "stable".to_string()
        };

        // Promedio mensual de facturas
        let avg_monthly_invoices = if let Some(total_facturas) = summary.total_facturas {
            // Asumir distribución a lo largo de 12 meses
            total_facturas as f64 / 12.0
        } else {
            0.0
        };

        // Patrón estacional (placeholder)
        let seasonal_pattern = if summary.total_facturas.unwrap_or(0) > 50 {
            "Q4 peak".to_string()
        } else {
            "stable".to_string()
        };

        // Proyección del próximo mes
        let projected_next_month = avg_monthly_invoices * 1.1; // 10% de crecimiento proyectado

        Ok(TrendAnalysis {
            monthly_trend,
            avg_monthly_invoices,
            seasonal_pattern,
            projected_next_month,
        })
    }

    /// Verificar si el usuario tiene datos de rewards
    pub async fn user_has_summary(&self, user_id: i32) -> Result<bool, AppError> {
        let query = "SELECT EXISTS(SELECT 1 FROM rewards.user_invoice_summary WHERE user_id = $1)";
        
        match sqlx::query_scalar::<_, bool>(query)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
        {
            Ok(exists) => Ok(exists),
            Err(e) => {
                tracing::error!("Error checking if user {} has summary: {}", user_id, e);
                Err(AppError::database_connection(e.to_string()))
            }
        }
    }
}
