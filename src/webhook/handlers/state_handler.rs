use std::sync::Arc;
use anyhow::Result;
use crate::state::AppState;
use tracing::{info, warn};

/// Maneja los mensajes de los usuarios que tienen un estado activo.
pub async fn handle_user_state(_app_state: &Arc<AppState>, _whatsapp_id: &str, _text: &str, state: &str) -> Result<()> {
        info!("Processing state '{}' for user", state);

    match state {
        // Actualmente, el único estado manejado aquí es 'general', que podría tener sub-estados.
        // El estado 'encuesta' se maneja directamente en `text_handler`.
        // Este es un placeholder para lógica futura.
        _ => {
            warn!("Unhandled user state in state_handler: {}. No action taken.", state);
            // Ejemplo de cómo se podría manejar un estado futuro:
            // if state == "some_other_flow" { ... }
        }
    }
    Ok(())
}
