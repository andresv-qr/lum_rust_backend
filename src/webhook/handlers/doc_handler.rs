use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    models::{user::UserState, whatsapp::Document},
    services::{redis_service, whatsapp_service},
    domains::ocr::service::process_ocr_invoice,
    state::AppState,
};

pub async fn handle_document(state: Arc<AppState>, doc: Document) -> Result<()> {
    let user_ws_id = &doc.from;
    info!("Handling document from user: {}", user_ws_id);

    let user_state = redis_service::get_user_state(&state, user_ws_id).await?;

    match user_state {
        Some(UserState::OcrInvoice) => {
            info!("User {} is in OcrInvoice state. Processing document...", user_ws_id);
            let doc_bytes = whatsapp_service::download_media(&state, &doc.id).await?;
            process_ocr_invoice(state, user_ws_id, &doc_bytes).await?;
        }
        _ => {
            warn!("Received an unsolicited document from user {}. No action taken.", user_ws_id);
            let message = "No esperaba un documento en este momento. Si quieres procesar una factura sin QR, usa el comando /factura_sin_qr primero.";
            whatsapp_service::send_text_message(&state, user_ws_id, message).await?;
        }
    }

    Ok(())
}
