use crate::models::whatsapp::{WebhookPayload, WebhookVerification};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use crate::processing::message_processor::process_message;

pub async fn get_webhook(
    Query(params): Query<WebhookVerification>,
) -> Response {
    info!("Received webhook verification request: {:?}", params);

    let verify_token = match std::env::var("VERIFY_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            tracing::error!("VERIFY_TOKEN environment variable not set");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("VERIFY_TOKEN is not set on the server."))
                .unwrap();
        }
    };

    info!("Comparing with expected verify_token: {}", verify_token);

    if params.hub_mode.as_deref() == Some("subscribe")
        && params.hub_verify_token.as_deref() == Some(&verify_token)
    {
        if let Some(challenge) = params.hub_challenge {
            info!("Webhook verification successful. Responding with challenge: {}", challenge);
            return Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(challenge))
                .unwrap();
        }
    }

    tracing::warn!("Webhook verification failed. Parameters did not match.");
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .body(Body::empty())
        .unwrap()
}

pub async fn post_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    // ✅ FASE 1: Respuesta inmediata HTTP 200 para prevenir retries de Facebook
    info!("📥 Webhook received, processing in background...");
    
    // Clone state for background processing
    let state_clone = state.clone();
    
    // ✅ PROCESAMIENTO ASÍNCRONO EN BACKGROUND (más eficiente que Python's BackgroundTasks)
    tokio::spawn(async move {
        if let Err(e) = process_webhook_async(state_clone, payload).await {
            error!("❌ Error in background webhook processing: {}", e);
        }
    });
    
    // Respuesta inmediata para prevenir timeouts y retries de Facebook
    StatusCode::OK
}

/// ✅ FASE 1: Procesamiento asíncrono de webhook con deduplicación
async fn process_webhook_async(
    state: Arc<AppState>, 
    payload: WebhookPayload
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Validate webhook structure
    if payload.entry.is_empty() {
        warn!("📭 Webhook received without entries");
        return Ok(());
    }
    
    info!("🔄 Processing webhook with {} entries", payload.entry.len());
    
    // Process each entry
    for entry in &payload.entry {
        for change in &entry.changes {
            let value = &change.value;
            
            if value.messages.is_empty() || value.contacts.is_empty() {
                continue;
            }
            
            // Process each message with deduplication
            for message in &value.messages {
                let message_id = &message.id;
                
                // ✅ FASE 1: DEDUPLICACIÓN DE MENSAJES (más eficiente que Python)
                if state.message_deduplicator.is_duplicate(message_id) {
                    warn!("🚫 Duplicate message detected and skipped: {}", message_id);
                    continue;
                }
                
                info!("✅ Processing new message: {} from {}", message_id, &message.from);
                
                // Process individual message
                process_message(state.clone(), payload.clone()).await;
                info!("✅ Message processed successfully: {}", message_id);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_app_router;
    use axum::{
        body::Body,
        http::{self, Request},
    };
    use tower::ServiceExt; // for `oneshot`
    use wiremock::{
        matchers::{method, path_regex, body_json_string},
        Mock, MockServer, ResponseTemplate,
    };

    async fn setup_test_app() -> axum::Router {
        // Carga las variables de entorno para la prueba
        dotenvy::dotenv().ok();
        let app_state = AppState::new().await.expect("Failed to create AppState for test");
        create_app_router(app_state.into())
    }

    #[tokio::test]
    async fn test_handle_webhook_text_message() {
        let app = setup_test_app().await;

        let message = r#"
        {
            "object": "whatsapp_business_account",
            "entry": [
                {
                    "id": "12345",
                    "changes": [
                        {
                            "value": {
                                "messaging_product": "whatsapp",
                                "metadata": {
                                    "display_phone_number": "16505551111",
                                    "phone_number_id": "1234567890"
                                },
                                "contacts": [
                                    {
                                        "profile": {
                                            "name": "John Doe"
                                        },
                                        "wa_id": "15551234567"
                                    }
                                ],
                                "messages": [
                                    {
                                        "from": "15551234567",
                                        "id": "wamid.HBgLMTU1NTEyMzQ1NjcVAgARGBJDN0U4QkZCM0E2QUQzQjgwRgA=",
                                        "timestamp": "1678901234",
                                        "text": {
                                            "body": "/echo Hello"
                                        },
                                        "type": "text"
                                    }
                                ]
                            },
                            "field": "messages"
                        }
                    ]
                }
            ]
        }
        "#;

        let response = app
            .oneshot(Request::builder()
                .method(http::Method::POST)
                .uri("/webhookws")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(message))
                .unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_handle_webhook_interactive_message() {
        let app = setup_test_app().await;

        let message = r#"
        {
            "object": "whatsapp_business_account",
            "entry": [
                {
                    "id": "12345",
                    "changes": [
                        {
                            "value": {
                                "messaging_product": "whatsapp",
                                "metadata": {
                                    "display_phone_number": "16505551111",
                                    "phone_number_id": "1234567890"
                                },
                                "contacts": [
                                    {
                                        "profile": {
                                            "name": "Jane Doe"
                                        },
                                        "wa_id": "15551234568"
                                    }
                                ],
                                "messages": [
                                    {
                                        "from": "15551234568",
                                        "id": "wamid.HBgLMTU1NTEyMzQ1NjgVAgARGBJDN0U4QkZCM0E2QUQzQjgwRgA=",
                                        "timestamp": "1678901235",
                                        "interactive": {
                                            "type": "button_reply",
                                            "button_reply": {
                                                "id": "unique-button-id-123",
                                                "title": "Yes!"
                                            }
                                        },
                                        "type": "interactive"
                                    }
                                ]
                            },
                            "field": "messages"
                        }
                    ]
                }
            ]
        }
        "#;

        let response = app
            .oneshot(Request::builder()
                .method(http::Method::POST)
                .uri("/webhookws")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(message))
                .unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_start_survey_interactive_flow() {
        // 1. Setup Wiremock server
        let mock_server = MockServer::start().await;
        let mock_uri = mock_server.uri();

        // 2. Setup AppState to use the mock server
        dotenvy::dotenv().ok();
        let mut app_state = AppState::new().await.expect("Failed to create AppState");
        app_state.whatsapp_api_base_url = mock_uri;
        let redis_client = app_state.redis_client.clone();

        let app = create_app_router(app_state.into());

        // 3. Define the mock for the WhatsApp API call
        let expected_body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": "50762122046",
            "type": "text",
            "text": {
                "body": "¡Excelente! Para comenzar, por favor dime tu nombre."
            }
        });

        Mock::given(method("POST"))
            .and(path_regex(r"/v\d+\.\d+/\d+/messages"))
            .and(body_json_string(&expected_body.to_string()))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // 4. Simulate incoming webhook with 'start_survey' button
        let test_payload = serde_json::json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "101010101010101",
                "changes": [{
                    "value": {
                        "messaging_product": "whatsapp",
                        "metadata": {
                            "display_phone_number": "15550001111",
                            "phone_number_id": "102030405060708"
                        },
                        "contacts": [{
                            "profile": {"name": "Test User"},
                            "wa_id": "50762122046"
                        }],
                        "messages": [{
                            "from": "50762122046",
                            "id": "wamid.test_start_survey_message",
                            "timestamp": "1678886400",
                            "type": "interactive",
                            "interactive": {
                                "type": "button_reply",
                                "button_reply": {
                                    "id": "start_survey",
                                    "title": "Iniciar Encuesta"
                                }
                            }
                        }]
                    },
                    "field": "messages"
                }]
            }]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/webhookws")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&test_payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // 5. Assertions
        mock_server.verify().await;

        let mut con = redis_client.get_multiplexed_async_connection().await.unwrap();
        let user_state_key = "survey_state:50762122046";
        let state_json: String = redis::cmd("GET").arg(user_state_key).query_async(&mut con).await.unwrap();
        let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();

        assert_eq!(state["step"], "awaiting_name");

    }
}
