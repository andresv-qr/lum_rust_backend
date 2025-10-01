use std::sync::Arc;
use anyhow::Result;

use crate::{
    models::{user::UserState, whatsapp::{Image, Message}},
    services::{redis_service, user_service, whatsapp_service},
    domains::{invoices::service as invoice_service, ocr::service::process_ocr_invoice},
    state::AppState,
};
use url::Url;
use tracing::{info, warn, error};

/// Handle image messages from WhatsApp webhook
pub async fn handle_image_message(state: Arc<AppState>, message: &Message) -> Result<()> {
    let from = &message.from;

    let image = match &message.image {
        Some(img) => img,
        None => {
            error!("Image message received without image data.");
            return Ok(());
        }
    };

    info!("Processing image message from {}", from);
    
    // Delegate to core image processing logic
    handle_image_core(state, from, image).await
}

/// The core logic for handling an incoming image message.
pub async fn handle_image_core(
    state: Arc<AppState>,
    user_ws_id: &str,
    image_message: &Image,
) -> Result<()> {
    info!("Processing image message from {}", user_ws_id);

    // Verificar que el usuario esté registrado
    let user_opt = user_service::get_user(&state, user_ws_id).await?;
    let user = match user_opt {
        Some(user) => user,
        None => {
            whatsapp_service::send_text_message(
                &state,
                user_ws_id,
                "❌ Debes estar registrado para procesar imágenes.\n\nUsa /registro para comenzar."
            ).await?;
            return Ok(());
        }
    };

    // Descargar la imagen
    let image_url = &image_message.id;
    let image_data = whatsapp_service::download_media(&state, image_url).await?;
    
    // Intentar detectar QR automáticamente en cualquier imagen
    let image = image::load_from_memory(&image_data)?;
    info!("🔍 Attempting automatic QR detection for user {}", user_ws_id);
    
    let qr_service = &state.qr_service;
    match qr_service.decode_qr(&image).await {
        Some(qr_result) => {
            let qr_data = &qr_result.content;
            info!("✅ QR detected automatically for user {}: {}", user_ws_id, qr_data);
            
            // Procesar automáticamente el QR si contiene una URL
            if qr_data.starts_with("http") {
                match Url::parse(&qr_data) {
                    Ok(url) => {
                        info!("🌐 Processing QR URL automatically: {}", url);
                        
                        // Log the final URL that will be processed (helpful for debugging redirections)
                        if let Ok(final_url) = crate::processing::web_scraping::http_client::get_final_url(&state.http_client, &url.to_string()).await {
                            if final_url != url.to_string() {
                                info!("🔄 QR URL redirection: {} → {}", url, final_url);
                            }
                        }
                        
                        // Notificar al usuario que se está procesando
                        whatsapp_service::send_text_message(
                            &state,
                            user_ws_id,
                            "🔍 **QR detectado automáticamente**\n\n⚡ Procesando factura...\n🌐 Realizando web scraping\n✅ Validando información"
                        ).await?;
                        
                        // Procesar la factura desde el QR
                        match invoice_service::process_invoice_url(state.clone(), &url.to_string(), user_ws_id, user.id as i64).await {
                            Ok(_) => {
                                info!("✅ QR invoice processed successfully for user {}", user_ws_id);
                                // El mensaje de éxito ya se envía desde process_invoice_url
                            }
                            Err(e) => {
                                warn!("❌ Error processing invoice from QR URL: {}", e);
                                whatsapp_service::send_text_message(
                                    &state,
                                    user_ws_id,
                                    "❌ **Error al procesar la factura del QR**\n\nPor favor, verifica que:\n• El QR sea válido\n• La imagen esté clara\n• La factura sea accesible"
                                ).await?;
                            }
                        }
                    }
                    Err(_) => {
                        whatsapp_service::send_text_message(
                            &state,
                            user_ws_id,
                            &format!("📱 **QR detectado:** {}\n\n⚠️ No es una URL válida de factura", qr_data)
                        ).await?;
                    }
                }
            } else {
                whatsapp_service::send_text_message(
                    &state,
                    user_ws_id,
                    &format!("📱 **QR detectado:** {}\n\n💡 Para procesar facturas, el QR debe contener una URL", qr_data)
                ).await?;
            }
            
            return Ok(());
        }
        None => {
            info!("🔍 No QR detected, checking for specific user states for OCR processing");
        }
    }

    // Si no hay QR, verificar estados específicos para OCR u otros procesamientos
    let user_state = redis_service::get_user_state(&state, user_ws_id).await?;

    match user_state {
        Some(UserState::WaitingForImage) => {
            info!("User {} was in WaitingForImage state, but QR already processed automatically", user_ws_id);
            
            // Clear user state since QR was already processed automatically above
            redis_service::delete_user_state(&state, user_ws_id).await?;
            
            // Inform user that no QR was found in their image
            whatsapp_service::send_text_message(
                &state,
                user_ws_id,
                "❌ No se detectó ningún código QR en la imagen.\n\n💡 Asegúrate de que:\n• El QR esté visible y claro\n• La imagen tenga buena iluminación\n• No haya reflejos en el QR"
            ).await?;
        }
        Some(UserState::WaitingForImageOcr) => {
            info!("User {} is in WaitingForImageOcr state, processing OCR", user_ws_id);
            
            // Download the image
            let image_url = &image_message.id;
            let image_data = whatsapp_service::download_media(&state, image_url).await?;
            
            // Process OCR
            match process_ocr_invoice(state.clone(), user_ws_id, &image_data).await {
                Ok(_) => {
                    whatsapp_service::send_text_message(
                        &state,
                        user_ws_id,
                        "✅ Imagen OCR procesada exitosamente."
                    ).await?;
                }
                Err(e) => {
                    warn!("Error processing OCR image: {}", e);
                    whatsapp_service::send_text_message(
                        &state,
                        user_ws_id,
                        "❌ Error al procesar la imagen OCR."
                    ).await?;
                }
            }
            
            // Clear user state
            redis_service::delete_user_state(&state, user_ws_id).await?;
        }
        _ => {
            info!("User {} sent image without specific state - QR already processed automatically above", user_ws_id);
            
            // QR detection was already attempted automatically above
            // If we reach here, no QR was found, so provide helpful feedback
            whatsapp_service::send_text_message(
                &state,
                user_ws_id,
                "📷 **Imagen recibida**\n\n🔍 No se detectó código QR en esta imagen.\n\n💡 **Para procesar facturas:**\n• Envía una imagen con QR visible\n• Asegúrate de buena iluminación\n• Evita reflejos en el código\n\n📋 **Otros comandos útiles:**\n• `/factura_sin_qr` - Para facturas sin QR\n• `/ayuda` - Ver todos los comandos"
            ).await?;
        }
    }

    Ok(())
}
