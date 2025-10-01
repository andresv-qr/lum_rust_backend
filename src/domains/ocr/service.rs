use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn, error};

use crate::{
    services::{user_service, whatsapp_service, redis_service},
    services::ocr_service::{OcrService, OcrProcessRequest, OcrSource, OcrMode},
    state::AppState,
    models::user::UserState,
};

/// Main OCR processing function that replicates the Python flow_process_ocr_invoice
/// Now uses the common OCR service
pub async fn process_ocr_invoice(
    state: Arc<AppState>,
    user_ws_id: &str,
    image_bytes: &[u8],
) -> Result<()> {
    info!("🔍 Iniciando procesamiento OCR para usuario {}", user_ws_id);

    // 1. VERIFICAR ESTADO GUARDADO
    let user_state = redis_service::get_user_state(&state, user_ws_id).await?;
    match user_state {
        Some(UserState::OcrInvoice) => {
            // Estado válido, continuar con el procesamiento
        }
        _ => {
            warn!("Usuario {} no está en estado OCR válido: {:?}", user_ws_id, user_state);
            let message = "❌ Primero usa el comando /factura_sin_qr para activar el modo OCR.";
            whatsapp_service::send_text_message(&state, user_ws_id, message).await?;
            return Ok(());
        }
    };

    // 2. OBTENER USUARIO DE LA BASE DE DATOS
    let user = match user_service::get_user(&state, user_ws_id).await? {
        Some(u) => u,
        None => {
            warn!("Usuario no encontrado para procesamiento OCR: {}", user_ws_id);
            let message = "❌ Usuario no encontrado. Por favor, regístrate primero.";
            whatsapp_service::send_text_message(&state, user_ws_id, message).await?;
            return Ok(());
        }
    };

    // 3. Create OCR request using the common service (always Normal mode for WhatsApp)
    let ocr_request = OcrProcessRequest {
        user_id: user.id,
        user_identifier: user_ws_id.to_string(),
        image_bytes: image_bytes.to_vec(),
        source: OcrSource::WhatsApp,
        mode: OcrMode::Normal, // WhatsApp always uses normal mode
    };

    // 4. Process using the common OCR service
    match OcrService::process_ocr_invoice(state.clone(), ocr_request).await {
        Ok(ocr_response) => {
            // 5. Clean up state
            redis_service::delete_user_state(&state, user_ws_id).await?;

            if ocr_response.success {
                // 6. Success response
                let productos_count = ocr_response.products.as_ref().map(|p| p.len()).unwrap_or(0);
                let success_message = format!(
                    "✅ **Factura procesada exitosamente**\n\n📋 **Datos detectados:**\n🏪 Comercio: {}\n📄 Factura: {}\n💰 Total: ${}\n📦 Productos: {} artículos\n\n⏳ **Estado:** Pendiente de validación\n👥 **Proceso:** Nuestro equipo revisará la información en 24-48 horas\n📱 **Notificación:** Te avisaremos cuando esté certificada\n\n💰 **Costo:** {} Lümi procesado\n🔗 **CUFE temporal:** `{}`\n\n¡Gracias por usar nuestro servicio OCR! 🚀",
                    ocr_response.issuer_name.as_deref().unwrap_or("N/A"),
                    ocr_response.invoice_number.as_deref().unwrap_or("N/A"),
                    ocr_response.total.unwrap_or(0.0),
                    productos_count,
                    ocr_response.cost_lumis,
                    ocr_response.cufe.as_deref().unwrap_or("N/A")
                );
                whatsapp_service::send_text_message(&state, user_ws_id, &success_message).await?;
            } else {
                // 7. Error response
                let error_message = format!("❌ {}", ocr_response.message);
                whatsapp_service::send_text_message(&state, user_ws_id, &error_message).await?;
            }
        }
        Err(e) => {
            error!("💥 Error crítico en procesamiento OCR para {}: {}", user_ws_id, e);
            redis_service::delete_user_state(&state, user_ws_id).await?;
            let message = "❌ Error interno del servidor. Intenta nuevamente más tarde.";
            whatsapp_service::send_text_message(&state, user_ws_id, message).await?;
        }
    }

    info!("✅ Procesamiento OCR completado para {}", user_ws_id);
    Ok(())
}
