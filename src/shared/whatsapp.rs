use crate::{
    models::whatsapp::{
        Action, InteractiveBody, InteractiveMessage, InteractiveMessageRequest, Section, Text,
        TextMessageRequest, ImageMessageRequest, ImageMedia,
    },
    state::AppState,
};
use anyhow::{bail, Result};

use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

/// Envía un mensaje de texto a través de la API de WhatsApp.
pub async fn send_text_message(app_state: &Arc<AppState>, to: &str, body: &str) -> Result<()> {
    let whatsapp_token = &app_state.whatsapp_token;
    let phone_number_id = &app_state.phone_number_id;

    let request_body = TextMessageRequest {
        messaging_product: "whatsapp".to_string(),
        to: to.to_string(),
        message_type: "text".to_string(),
        text: Text { body: body.to_string() },
    };

    let url = format!("{}/{}/messages", app_state.whatsapp_api_base_url, phone_number_id);

    let response = app_state
        .http_client
        .post(&url)
        .bearer_auth(whatsapp_token)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_body = response.text().await?;
        bail!("Failed to send text message: {}", error_body);
    }

    info!("Successfully sent text message to {}", to);
    Ok(())
}


/// Envía un mensaje de lista interactiva a través de la API de WhatsApp.
/// Envía un mensaje interactivo de botones a través de la API de WhatsApp.
pub async fn send_interactive_button_message(
    app_state: &Arc<AppState>,
    to: &str,
    interactive: InteractiveMessage,
) -> Result<()> {
    let whatsapp_token = &app_state.whatsapp_token;
    let phone_number_id = &app_state.phone_number_id;

    let request_body = InteractiveMessageRequest {
        messaging_product: "whatsapp".to_string(),
        to: to.to_string(),
        message_type: "interactive".to_string(),
        interactive,
    };

    let url = format!("{}/{}/messages", app_state.whatsapp_api_base_url, phone_number_id);

    let response = app_state
        .http_client
        .post(&url)
        .bearer_auth(whatsapp_token)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_body = response.text().await?;
        bail!("Failed to send interactive button message: {}", error_body);
    }

    info!("Successfully sent interactive button message to {}", to);
    Ok(())
}


/// Envía un mensaje de lista interactiva a través de la API de WhatsApp.
pub async fn send_interactive_list_message(
    app_state: &Arc<AppState>,
    to: &str,
    body_text: &str,
    button_text: &str,
    sections: Vec<Section>,
) -> Result<()> {
    let whatsapp_token = &app_state.whatsapp_token;
    let phone_number_id = &app_state.phone_number_id;

    let body = InteractiveBody::new(body_text);
    let action = Action::new_for_list(button_text, sections);
    let interactive = InteractiveMessage::new_for_list(body, action, None);

    let request_body = InteractiveMessageRequest {
        messaging_product: "whatsapp".to_string(),
        to: to.to_string(),
        message_type: "interactive".to_string(),
        interactive,
    };

    let url = format!("{}/{}/messages", app_state.whatsapp_api_base_url, phone_number_id);

    let response = app_state
        .http_client
        .post(&url)
        .bearer_auth(whatsapp_token)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_body = response.text().await?;
        bail!("Failed to send interactive list message: {}", error_body);
    }

    info!("Successfully sent interactive list message to {}", to);
    Ok(())
}

#[derive(Deserialize)]
struct MediaUrlResponse {
    url: String,
}

/// Downloads a media file from WhatsApp's servers.
pub async fn download_media(app_state: &Arc<AppState>, media_id: &str) -> Result<Vec<u8>> {
    let whatsapp_token = &app_state.whatsapp_token;

    // Step 1: Get the media URL
    let url_lookup_url = format!("{}/{}", app_state.whatsapp_api_base_url, media_id);
    let url_response = app_state
        .http_client
        .get(&url_lookup_url)
        .bearer_auth(whatsapp_token)
        .send()
        .await?;

    if !url_response.status().is_success() {
        let error_body = url_response.text().await?;
        bail!("Failed to get media URL: {}", error_body);
    }

    let media_url_data = url_response.json::<MediaUrlResponse>().await?;

    // Step 2: Download the media file from the obtained URL
    let media_response = app_state
        .http_client
        .get(&media_url_data.url)
        .bearer_auth(whatsapp_token)
        .send()
        .await?;

    if !media_response.status().is_success() {
        let error_body = media_response.text().await?;
        bail!("Failed to download media file: {}", error_body);
    }

    let media_bytes = media_response.bytes().await?.to_vec();
    info!("Successfully downloaded media file with ID: {}", media_id);

    Ok(media_bytes)
}

/// Envía una imagen a través de la API de WhatsApp.
pub async fn send_image_message(app_state: &Arc<AppState>, to: &str, image_url: &str, caption: Option<&str>) -> Result<()> {
    let whatsapp_token = &app_state.whatsapp_token;
    let phone_number_id = &app_state.phone_number_id;

    let request_body = ImageMessageRequest {
        messaging_product: "whatsapp".to_string(),
        to: to.to_string(),
        message_type: "image".to_string(),
        image: ImageMedia {
            link: image_url.to_string(),
            caption: caption.map(|c| c.to_string()),
        },
    };

    let url = format!("{}/{}/messages", app_state.whatsapp_api_base_url, phone_number_id);

    let response = app_state
        .http_client
        .post(&url)
        .bearer_auth(whatsapp_token)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_body = response.text().await?;
        bail!("Failed to send image message: {}", error_body);
    }

    info!("Successfully sent image message to {}", to);
    Ok(())
}
