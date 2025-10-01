use crate::webhook::handlers::{doc_handler, image_handler, text_handler, interactive_handler};
use crate::state::{AppState, ProcessedMessage};
use crate::models::whatsapp::{WebhookPayload, MessageType};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Procesa el payload del webhook de forma asíncrona.
pub async fn process_message(state: Arc<AppState>, payload: WebhookPayload) {
    info!("Procesando webhook en segundo plano...");

    for entry in payload.entry {
        for change in entry.changes {
            for message in change.value.messages {
                // --- Lógica de prevención de duplicados con TTL ---
                let now = Instant::now();
                
                // Clean up old entries (older than 1 hour)
                state.processed_messages.retain(|_, processed_msg| {
                    now.duration_since(processed_msg.timestamp) < Duration::from_secs(3600)
                });
                
                // Check if message was already processed
                if state.processed_messages.contains_key(&message.id) {
                    info!("Mensaje duplicado recibido: {}. Ignorando.", message.id);
                    continue;
                }
                
                // Mark message as processed
                state.processed_messages.insert(message.id.clone(), ProcessedMessage {
                    timestamp: now,
                });

                // --- Enrutamiento basado en el tipo de mensaje ---
                match message.message_type {
                    MessageType::Text => {
                        if let Err(e) = text_handler::handle_text_message(&message, &state).await {
                            error!("Error procesando mensaje de texto: {}", e);
                        }
                    }
                    MessageType::Interactive => {
                        if let Some(ref _interactive_message) = message.interactive {
                            let _ = interactive_handler::handle_interactive_message(&message, &Arc::clone(&state)).await;
                        } else {
                            error!("Mensaje interactivo sin contenido interactivo");
                        }
                    }
                                       MessageType::Image => {
                        if let Some(ref _image_data) = message.image {
                            if let Err(e) = image_handler::handle_image_message(Arc::clone(&state), &message).await {
                                error!("Error procesando mensaje de imagen: {}", e);
                            }
                        } else {
                            error!("Mensaje de imagen sin contenido de imagen.");
                        }
                    }
                     MessageType::Document => {
                        if let Some(doc_data) = message.document {
                            if let Err(e) = doc_handler::handle_document(Arc::clone(&state), doc_data).await {
                                error!("Error procesando mensaje de documento: {}", e);
                            }
                        } else {
                            error!("Mensaje de documento sin contenido de documento.");
                        }
                    }
                    MessageType::Unsupported => {
                        warn!("Tipo de mensaje no soportado recibido: {:?}", message);
                    }
                }
            }
        }
    }
}
