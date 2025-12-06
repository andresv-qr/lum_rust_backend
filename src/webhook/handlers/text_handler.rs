use crate::{
    webhook::handlers::command_handler,
    models::{user::UserState, whatsapp::Message},
    processing::flows::{product_search_flow, survey_flow},
    services::{redis_service, user_service, whatsapp_service, rewards_service},
    domains::invoices::service as invoice_service,
    state::AppState,
};
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error, warn};
use url::Url;
use tokio::spawn;
use chrono::Utc;

/// Maneja todos los mensajes de tipo texto, actuando como un enrutador basado en el estado.
pub async fn handle_text_message(message: &Message, app_state: &Arc<AppState>) -> Result<()> {
    let whatsapp_id = &message.from;
    let text_body = message.text.body.trim();
    info!("Routing text message from {}: '{}'", whatsapp_id, text_body);

    // 1. Verificar si el mensaje es una URL de factura
    if let Ok(url) = Url::parse(text_body) {
        if url.scheme() == "http" || url.scheme() == "https" {
            if let Some(user_id) = user_service::get_user_id_by_ws_id(app_state, &message.from).await? {
                whatsapp_service::send_text_message(app_state, &message.from, "Hemos recibido tu factura. La procesaremos en breve.").await?;
                // Procesar en segundo plano para no bloquear la respuesta
                let state_clone = Arc::clone(app_state);
                let url_string = url.to_string();
                let from_clone = message.from.clone();
                spawn(async move {
                                                            if let Err(e) = invoice_service::process_invoice_url(state_clone.clone(), &url_string, &from_clone, user_id.into()).await {
                        tracing::error!("Error procesando la factura desde la URL {}: {}", url_string, e);
                        // Opcional: notificar al usuario del error
                        let _ = whatsapp_service::send_text_message(&state_clone, &from_clone, "Tuvimos un problema al procesar tu factura. Por favor, inténtalo de nuevo más tarde.").await;
                    }
                });
                return Ok(());
            } else {
                whatsapp_service::send_text_message(app_state, &message.from, "Parece que aún no te has registrado. Por favor, usa el comando /registro para empezar.").await?;
                return Ok(());
            }
        }
    }

    // 2. Verificar si es un comando explícito (empieza con "/")
    if text_body.starts_with('/') {
        info!("Command detected: '{}' for user {}", text_body, whatsapp_id);
        return command_handler::handle_command(app_state, whatsapp_id, text_body).await;
    }

    // 3. Reconocimiento de frases naturales antes de verificar estados
    let normalized_text = text_body.to_lowercase();
    
    // Mapear frases naturales a comandos
    if normalized_text.contains("lumiscope") || normalized_text.contains("dashboard") || normalized_text.contains("métricas") {
        info!("Natural phrase detected for lumiscope: '{}'", text_body);
        return command_handler::handle_command(app_state, whatsapp_id, "/lumiscope").await;
    }
    
    if normalized_text.contains("ayuda") || normalized_text.contains("help") || normalized_text.contains("comandos") {
        info!("Natural phrase detected for help: '{}'", text_body);
        return command_handler::handle_command(app_state, whatsapp_id, "/ayuda").await;
    }
    
    if normalized_text.contains("saldo") || normalized_text.contains("balance") || normalized_text.contains("lumis") {
        info!("Natural phrase detected for balance: '{}'", text_body);
        return command_handler::handle_command(app_state, whatsapp_id, "/saldo").await;
    }
    
    if normalized_text.contains("buscar") || normalized_text.contains("producto") || normalized_text.contains("search") {
        info!("Natural phrase detected for search: '{}'", text_body);
        return command_handler::handle_command(app_state, whatsapp_id, "/buscar").await;
    }
    
    if normalized_text.contains("premios") || normalized_text.contains("recompensas") || normalized_text.contains("canjear") {
        info!("Natural phrase detected for rewards: '{}'", text_body);
        return command_handler::handle_command(app_state, whatsapp_id, "/premios").await;
    }
    
    if normalized_text.contains("factura sin qr") || normalized_text.contains("ocr") || normalized_text.contains("sin código") {
        info!("Natural phrase detected for OCR: '{}'", text_body);
        return command_handler::handle_command(app_state, whatsapp_id, "/factura_sin_qr").await;
    }
    
    if normalized_text.contains("ver ofertas web") || normalized_text.contains("activar radar de ofertas") || normalized_text.contains("radar ofertas") || normalized_text.contains("ofertas web") {
        info!("Natural phrase detected for offers radar: '{}'", text_body);
        return handle_offers_radar_request(app_state, whatsapp_id).await;
    }
    
    if normalized_text.contains("cancelar") || normalized_text.contains("salir") || normalized_text.contains("stop") {
        info!("Natural phrase detected for cancel: '{}'", text_body);
        return command_handler::handle_command(app_state, whatsapp_id, "/cancelar").await;
    }

    // 4. Si no es una frase natural reconocida, verificar si el usuario está en un flujo de conversación
    let user_state = redis_service::get_user_state(app_state, whatsapp_id).await?;
    match user_state {
        Some(UserState::Survey(_)) => {
            info!("Continuing survey for user {}", whatsapp_id);
            survey_flow::handle_survey_response(app_state, whatsapp_id, text_body).await
        }
        Some(UserState::ProductSearch) => {
            info!("Handling product search for user {}", whatsapp_id);
            product_search_flow::handle_product_search(app_state, whatsapp_id, text_body).await
        }
        Some(UserState::OcrInvoice) => {
            info!("User {} is in OCR mode but sent text instead of image/document", whatsapp_id);
            let response = "Estoy esperando que me envíes la imagen o PDF de tu factura. Si quieres cancelar, escribe `/cancelar`.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
        }
        Some(UserState::WaitingForImage) => {
            info!("User {} is waiting for image but sent text instead", whatsapp_id);
            let response = "Estoy esperando que me envíes una imagen para procesar el QR. Si quieres cancelar, escribe `/cancelar`.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
        }
        Some(UserState::WaitingForImageOcr) => {
            info!("User {} is waiting for OCR image but sent text instead", whatsapp_id);
            let response = "Estoy esperando que me envíes una imagen para procesar con OCR. Si quieres cancelar, escribe `/cancelar`.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
        }
        Some(UserState::OffersRadar { step, categories }) => {
            info!("Handling offers radar response for user {}: step={}", whatsapp_id, step);
            handle_offers_radar_response(app_state, whatsapp_id, text_body, &step, &categories).await
        }
        Some(UserState::PriceRange(state_json)) => {
            info!("Handling price range flow for user {}", whatsapp_id);
            handle_price_range_flow(app_state, whatsapp_id, text_body, &state_json).await
        }
        // 5. Si no hay un flujo activo y no es un comando, responder amigablemente.
        None => {
            info!("No active state and not a command for user {}. Sending default response.", whatsapp_id);
            let response = "No te he entendido. Si necesitas algo, escribe `/ayuda` para ver la lista de comandos.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
        }
    }
}

/// Handles the "ver ofertas web" request, replicating the Python logic
async fn handle_offers_radar_request(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    info!("Processing offers radar request for user {}", whatsapp_id);
    
    // 1. Check if user is registered
    let user_opt = user_service::get_user(app_state, whatsapp_id).await?;
    let user = match user_opt {
        Some(user) => user,
        None => {
            let message = "Debes estar registrado para acceder a tus retos y beneficios.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;
            return Ok(());
        }
    };
    
    // 2. Query active offers from rewards.fact_redemptions_legacy (redem_id = '0')
    // TODO: MIGRATED - Use new redemption system
    let query = r#"
        SELECT DISTINCT condition1 
        FROM rewards.fact_redemptions_legacy 
        WHERE user_id = $1 AND redem_id = '0' AND expiration_date >= $2
    "#;
    
    let current_date = Utc::now().date_naive();
    info!("Querying offers for user_id: {} with current_date: {}", user.id, current_date);
    
    let rows = sqlx::query_scalar::<_, String>(query)
        .bind(user.id as i64)
        .bind(current_date)
        .fetch_all(&app_state.db_pool)
        .await?;
    
    info!("Found {} categories for user_id: {} - {:?}", rows.len(), user.id, rows);
    
    if !rows.is_empty() {
        // 3. Show available categories
        let mut mensaje = "📋 *Categorías disponibles:*\n\n".to_string();
        for (i, categoria) in rows.iter().enumerate() {
            mensaje.push_str(&format!("{}. {}\n", i + 1, categoria));
        }
        mensaje.push_str("\n*Escribe el nombre de la categoría que te interesa*");
        
        whatsapp_service::send_text_message(app_state, whatsapp_id, &mensaje).await?;
        
        // 4. Save user state with available categories for price range flow
        let state_json = serde_json::json!({
            "step": "seleccionar_categoria",
            "categorias_disponibles": rows
        });
        
        let user_state = UserState::PriceRange(state_json.to_string());
        redis_service::save_user_state(app_state, whatsapp_id, &user_state, 600).await?; // 10 minutes TTL
        
        info!("Offers radar categories shown to user {}", whatsapp_id);
    } else {
        // No active offers, send summary (simplified version)
        let message = "No tienes ofertas activas en este momento. Te notificaremos cuando haya nuevas ofertas disponibles.";
        whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;
        warn!("No active offers found for user {}", whatsapp_id);
    }
    
    Ok(())
}

/// Handles user responses in the offers radar flow (category selection)
async fn handle_offers_radar_response(
    app_state: &Arc<AppState>, 
    whatsapp_id: &str, 
    text_body: &str, 
    step: &str, 
    categories: &[String]
) -> Result<()> {
    match step {
        "seleccionar_categoria" => {
            let selected_category = text_body.trim();
            
            // Check if the selected category is in the available categories
            let category_found = categories.iter()
                .any(|cat| cat.to_lowercase().contains(&selected_category.to_lowercase()) || 
                          selected_category.to_lowercase().contains(&cat.to_lowercase()));
            
            if category_found {
                // Category found, advance to price range step
                info!("User {} selected category: {}", whatsapp_id, selected_category);
                
                // Save state for price range step
                let new_state = serde_json::json!({
                    "step": "definir_rango",
                    "categoria_seleccionada": selected_category,
                    "categorias_disponibles": categories
                });
                
                redis_service::save_user_state(
                    app_state,
                    whatsapp_id,
                    &UserState::PriceRange(new_state.to_string()),
                    600
                ).await?;
                
                let message = format!(
                    "✅ Categoría seleccionada: *{}*\n\n📊 Ahora digita el rango de precios que te interesa\n(ejemplo: 100-200)",
                    selected_category
                );
                
                whatsapp_service::send_text_message(app_state, whatsapp_id, &message).await?;
            } else {
                // Category not found, ask again
                warn!("User {} selected invalid category: {}", whatsapp_id, selected_category);
                
                let mut mensaje = "❌ No encontré esa categoría. Las opciones disponibles son:\n\n".to_string();
                for (i, categoria) in categories.iter().enumerate() {
                    mensaje.push_str(&format!("{}. {}\n", i + 1, categoria));
                }
                mensaje.push_str("\n*Por favor, escribe el nombre exacto de una categoría de la lista.*");
                
                whatsapp_service::send_text_message(app_state, whatsapp_id, &mensaje).await?;
            }
        }
        _ => {
            warn!("Unknown offers radar step: {} for user {}", step, whatsapp_id);
            redis_service::delete_user_state(app_state, whatsapp_id).await?;
            let message = "Ha ocurrido un error. Por favor, intenta de nuevo con 'ver ofertas web'.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;
        }
    }
    
    Ok(())
}
/// Maneja el flujo de rango de precios para el Radar de Ofertas
async fn handle_price_range_flow(
    app_state: &Arc<AppState>,
    whatsapp_id: &str,
    text_body: &str,
    state_json: &str,
) -> Result<()> {
    use serde_json::Value;
    
    let state: Value = serde_json::from_str(state_json)?;
    let step = state["step"].as_str().unwrap_or("");
    
    match step {
        "seleccionar_categoria" => {
            let empty_vec = Vec::new();
            let categorias_disponibles = state["categorias_disponibles"]
                .as_array()
                .unwrap_or(&empty_vec)
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>();
            
            // Buscar si el texto coincide con alguna categoría (ignorando mayúsculas)
            let categoria_seleccionada = categorias_disponibles
                .iter()
                .find(|&&cat| cat.to_lowercase().trim() == text_body.to_lowercase().trim())
                .copied();
            
            if let Some(categoria) = categoria_seleccionada {
                // Categoría válida seleccionada
                let new_state = serde_json::json!({
                    "step": "definir_rango",
                    "categoria_seleccionada": categoria,
                    "categorias_disponibles": categorias_disponibles
                });
                
                redis_service::save_user_state(
                    app_state,
                    whatsapp_id,
                    &UserState::PriceRange(new_state.to_string()),
                    600
                ).await?;
                
                let mensaje = format!(
                    "✅ Categoría seleccionada: *{}*\n\n📊 Ahora digita el rango de precios que te interesa\n(ejemplo: 100-200)",
                    categoria
                );
                whatsapp_service::send_text_message(app_state, whatsapp_id, &mensaje).await?;
            } else {
                // Categoría no válida, mostrar opciones nuevamente
                let mut mensaje = "❌ Por favor selecciona una categoría válida de la lista:\n\n".to_string();
                for (i, cat) in categorias_disponibles.iter().enumerate() {
                    mensaje.push_str(&format!("{}. {}\n", i + 1, cat));
                }
                mensaje.push_str("\n*Escribe el nombre exacto de la categoría*");
                whatsapp_service::send_text_message(app_state, whatsapp_id, &mensaje).await?;
            }
        }
        "definir_rango" => {
            let categoria_seleccionada = state["categoria_seleccionada"].as_str().unwrap_or("");
            
            // Parsear el rango de precios
            if let Some((min_str, max_str)) = text_body.split_once('-') {
                if let (Ok(minprice), Ok(maxprice)) = (
                    min_str.trim().parse::<f64>(),
                    max_str.trim().parse::<f64>()
                ) {
                    if minprice < maxprice {
                        // Enviar mensaje de procesamiento
                        let processing_msg = format!(
                            "🔄 Analizando ofertas de *{}* en el rango ${}-${}...",
                            categoria_seleccionada, minprice, maxprice
                        );
                        whatsapp_service::send_text_message(app_state, whatsapp_id, &processing_msg).await?;
                        
                        // Buscar ofertas reales desde la base de datos
                        if let Some(user) = user_service::get_user(app_state, whatsapp_id).await? {
                            let user_id = user.id;
                            match rewards_service::search_offers_in_category(
                                &app_state.db_pool,
                                user_id,
                                categoria_seleccionada,
                                minprice,
                                maxprice
                            ).await {
                                Ok(offers) => {
                                    if offers.is_empty() {
                                        let no_offers_msg = format!(
                                            "📭 No encontramos ofertas de *{}* en el rango ${}-${}\n\n💡 *Tip*: Prueba con un rango más amplio (ej: 50-500)",
                                            categoria_seleccionada, minprice, maxprice
                                        );
                                        whatsapp_service::send_text_message(app_state, whatsapp_id, &no_offers_msg).await?;
                                    } else {
                                        // Generate visual dashboard using Python API
                                        use crate::services::visual_dashboard_service;
                                        
                                        info!("Generating visual dashboard for {} offers in category '{}'", offers.len(), categoria_seleccionada);
                                        
                                        match visual_dashboard_service::generate_offers_visual_dashboard(
                                            app_state,
                                            offers,
                                            categoria_seleccionada,
                                            whatsapp_id,
                                            Some(user_id)
                                        ).await {
                                            Ok(_) => {
                                                info!("Visual dashboard generated successfully for user {}", whatsapp_id);
                                            }
                                            Err(e) => {
                                                error!("Failed to generate visual dashboard: {}", e);
                                                // Fallback is handled inside the visual dashboard service
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Error searching offers: {:?}", e);
                                    let error_msg = "❌ Hubo un error al buscar ofertas. Inténtalo más tarde.";
                                    whatsapp_service::send_text_message(app_state, whatsapp_id, error_msg).await?;
                                }
                            }
                        } else {
                            let error_msg = "❌ Usuario no encontrado. Usa /registro para crear tu cuenta.";
                            whatsapp_service::send_text_message(app_state, whatsapp_id, error_msg).await?;
                        }
                        
                        // Limpiar estado
                        redis_service::delete_user_state(app_state, whatsapp_id).await?;
                    } else {
                        whatsapp_service::send_text_message(
                            app_state,
                            whatsapp_id,
                            "❌ El precio mínimo debe ser menor que el máximo. Intenta nuevamente (ej: 100-200)"
                        ).await?;
                    }
                } else {
                    whatsapp_service::send_text_message(
                        app_state,
                        whatsapp_id,
                        "❌ Formato de rango inválido. Usa el formato: minimo-maximo (ej: 100-200)"
                    ).await?;
                }
            } else {
                whatsapp_service::send_text_message(
                    app_state,
                    whatsapp_id,
                    "❌ Formato de rango inválido. Usa el formato: minimo-maximo (ej: 100-200)"
                ).await?;
            }
        }
        _ => {
            // Estado desconocido, limpiar y enviar mensaje de error
            redis_service::delete_user_state(app_state, whatsapp_id).await?;
            whatsapp_service::send_text_message(
                app_state,
                whatsapp_id,
                "❌ Ocurrió un error en el proceso. Por favor, intenta nuevamente con 'ver ofertas web'."
            ).await?;
        }
    }
    
    Ok(())
}
