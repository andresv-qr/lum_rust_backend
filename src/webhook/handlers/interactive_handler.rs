use crate::{
    models::whatsapp::Message,
    services::{rewards_service, user_service},
    state::AppState,
};
use std::sync::Arc;
use anyhow::Result;
use tracing::info;

/// Maneja los mensajes interactivos de WhatsApp.
pub async fn handle_interactive_message(message: &Message, app_state: &Arc<AppState>) -> Result<()> {
    let user_id = &message.from;
    if let Some(interactive) = &message.interactive {
        if let Some(button_reply) = &interactive.button_reply {
            let button_id = &button_reply.id;
            info!("Button reply from {}: id='{}' title='{}'", user_id, button_id, button_reply.title);
            
            match button_id.as_str() {
                // Aquí se pueden añadir más casos de botones en el futuro
                _ => info!("Unknown button ID '{}' from user {}", button_id, user_id),
            }
        }

        if let Some(list_reply) = &interactive.list_reply {
            let list_id = &list_reply.id;
            info!("List reply from {}: id='{}' title='{}'", user_id, list_id, list_reply.title);

            if let Some(user) = user_service::get_user(app_state, user_id).await? {
                match list_id.as_str() {
                    "red_radarofertas" => {
                        // Iniciar el flujo completo de Radar de Ofertas
                        rewards_service::start_radar_ofertas_flow(app_state, user_id, user.id.try_into().unwrap()).await?
                    },
                    "red_lumiscope" => rewards_service::send_user_metrics_dashboard(app_state, user_id).await?,
                    "red_compararte" => rewards_service::send_comparison_dashboard(app_state, user_id).await?,
                    "red_giftcard" => rewards_service::send_giftcard_info(app_state, user_id).await?,
                    "red_tombola_cash" => rewards_service::send_tombola_cash_confirmation(app_state, user_id).await?,
                    "red_tombola_merch" => rewards_service::send_tombola_merch_confirmation(app_state, user_id).await?,
                    _ => info!("Unknown list ID '{}' from user {}", list_id, user_id),
                }
            } else {
                let reply = "Debes estar registrado para usar esta función. Usa /start para registrarte.";
                crate::services::whatsapp_service::send_text_message(app_state, user_id, reply).await?;
            }
        }

    } else {
        info!("Mensaje interactivo recibido sin contenido interactivo.");
    }
    Ok(())
}
