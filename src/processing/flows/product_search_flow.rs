use crate::{models::user::UserState, services::{redis_service, whatsapp_service, user_service}, state::AppState};
use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info};

pub async fn start_product_search(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let state = UserState::ProductSearch;
    redis_service::save_user_state(app_state, whatsapp_id, &state, 600).await?;

    let message = "✅ ¡Modo de búsqueda activado!\n\nAhora dime, ¿qué producto quieres buscar?";
    whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;

    Ok(())
}

pub async fn handle_product_search(app_state: &Arc<AppState>, whatsapp_id: &str, text: &str) -> Result<()> {
    info!("Searching for products matching '{}' for user {}", text, whatsapp_id);

    // Basic input validation
    if text.trim().len() < 2 {
        let message = "Por favor, escribe un término de búsqueda más largo (al menos 2 caracteres).";
        whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;
        return Ok(());
    }

    // Get user information first
    let user_result = user_service::get_user(app_state, whatsapp_id).await;
    
    let response_message = match user_result {
        Ok(Some(user)) => {
            // Get user metrics which may contain product information
                        let metrics_result = user_service::get_user_summary(app_state, &user.email.as_ref().map(|s| s.as_str()).unwrap_or("")).await;
            
            match metrics_result {
                Ok(Some(metrics)) => {
                    // Check if user has top products data
                    if let Some(top_products) = &metrics.sm_top_productos {
                        let search_term_lower = text.to_lowercase();
                        let mut found_products = Vec::new();
                        
                        // Search through top products
                        for product in top_products.0.iter() {
                            if let Some(description) = &product.description {
                                if description.to_lowercase().contains(&search_term_lower) {
                                    found_products.push(product);
                                }
                            }
                        }
                        
                        if !found_products.is_empty() {
                            let mut results_text = "🔎 *Productos encontrados en tu historial:*\n\n".to_string();
                            for product in found_products.iter().take(5) {
                                results_text.push_str(&format!(
                                    "• *{}* - Cantidad: {:.1}\n",
                                    product.description.as_deref().unwrap_or("Producto"),
                                    product.qty.unwrap_or(0.0)
                                ));
                            }
                            results_text.push_str("\n🔍 Puedes realizar otra búsqueda o escribir /cancelar para salir.");
                            results_text
                        } else {
                            format!(
                                "🔍 No encontré '{}' en tu historial de productos más comprados.\n\n✨ *Sugerencias:*\n• Verifica la ortografía\n• Intenta con palabras más generales\n• Este producto podría no estar entre tus más comprados\n\n¿Quieres intentar otra búsqueda?", 
                                text
                            )
                        }
                    } else {
                        "📊 Aún no tienes suficiente historial de compras para realizar búsquedas.\n\n¡Sigue escaneando facturas para construir tu historial de productos!".to_string()
                    }
                }
                Ok(None) => {
                    "Para usar la búsqueda de productos, necesitas tener un historial de compras. ¡Escanea algunas facturas primero!".to_string()
                }
                Err(e) => {
                    error!("Error fetching user metrics for product search: {}", e);
                    "⚠️ Tuvimos un problema al acceder a tu historial. Por favor, intenta más tarde.".to_string()
                }
            }
        }
        Ok(None) => {
            "Debes estar registrado para usar la búsqueda de productos. Usa /registro para empezar.".to_string()
        }
        Err(e) => {
            error!("Database error during product search for user {}: {}", whatsapp_id, e);
            "⚠️ Tuvimos un problema al buscar tus productos. Por favor, intenta más tarde.\n\nSi el problema persiste, escribe /feedback para reportarlo.".to_string()
        }
    };

    whatsapp_service::send_text_message(app_state, whatsapp_id, &response_message).await?;

    Ok(())
}
