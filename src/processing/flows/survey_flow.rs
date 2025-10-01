use crate::{
    models::user::SurveyState,
    services::{whatsapp_service, user_service},
    state::AppState
};
use redis::AsyncCommands;

use tracing::{info, error};
use chrono::NaiveDate;
use regex::Regex;
use std::sync::Arc;
use anyhow::Result;

const SURVEY_STATE_KEY_PREFIX: &str = "survey_state:";
const SURVEY_EXPIRATION_SECONDS: u64 = 1800; // 30 minutos



/// Inicia el flujo de la encuesta para un nuevo usuario.
pub async fn start_survey(app_state: &Arc<AppState>, ws_id: &str) -> Result<()> {
    info!("Iniciando encuesta para el usuario {}", ws_id);

    let mut survey_state = SurveyState::default();
    survey_state.step = "awaiting_name".to_string();

    let mut redis_conn = app_state.redis_client.get_multiplexed_async_connection().await?;
    let key = format!("{}{}", SURVEY_STATE_KEY_PREFIX, ws_id);
    let survey_state_json = serde_json::to_string(&survey_state)?;
    let _: () = redis_conn.set_ex(key, survey_state_json, SURVEY_EXPIRATION_SECONDS).await?;

    ask_question(app_state, ws_id, "¡Excelente! Para comenzar, por favor dime tu nombre.").await
}

/// Maneja las respuestas del usuario durante el flujo de la encuesta.
pub async fn handle_survey_response(app_state: &Arc<AppState>, ws_id: &str, response: &str) -> Result<()> {
    let mut redis_conn = app_state.redis_client.get_multiplexed_async_connection().await?;
    let key = format!("{}{}", SURVEY_STATE_KEY_PREFIX, ws_id);
    let survey_state_json: Option<String> = redis_conn.get(&key).await?;

    if let Some(json) = survey_state_json {
        let mut survey_state: SurveyState = serde_json::from_str(&json)?;

        match survey_state.step.as_str() {
            "awaiting_name" => handle_name_response(app_state, &mut survey_state, ws_id, response).await?,
            "awaiting_birth_date" => handle_birth_date_response(app_state, &mut survey_state, ws_id, response).await?,
            "awaiting_origin_country" => handle_origin_country_response(app_state, &mut survey_state, ws_id, response).await?,
            "awaiting_residence_country" => handle_residence_country_response(app_state, &mut survey_state, ws_id, response).await?,
            "awaiting_email" => handle_email_response(app_state, &mut survey_state, ws_id, response).await?,
            "awaiting_email_confirmation" => handle_email_confirmation_response(app_state, &mut survey_state, ws_id, response).await?,
            _ => {
                error!("Estado de encuesta desconocido '{}' para el usuario {}", survey_state.step, ws_id);
                whatsapp_service::send_text_message(app_state, ws_id, "Lo siento, hubo un error. Por favor, intenta de nuevo.").await?;
            }
        };

        // Si el flujo no ha terminado, guarda el estado actualizado
        if survey_state.step != "completed" {
            let updated_json = serde_json::to_string(&survey_state)?;
            let _: () = redis_conn.set_ex(key, updated_json, SURVEY_EXPIRATION_SECONDS).await?;
        }
    } else {
        info!("No se encontró estado de encuesta para {}, iniciando de nuevo.", ws_id);
        start_survey(app_state, ws_id).await?;
    }

    Ok(())
}

async fn ask_question(app_state: &Arc<AppState>, ws_id: &str, question: &str) -> Result<()> {
    whatsapp_service::send_text_message(app_state, ws_id, question).await
}

async fn handle_name_response(app_state: &Arc<AppState>, survey_state: &mut crate::models::user::SurveyState, ws_id: &str, name: &str) -> Result<()> {
    survey_state.name = Some(name.to_string());
    survey_state.step = "awaiting_birth_date".to_string();
    ask_question(app_state, ws_id, "Gracias. ¿Cuál es tu fecha de nacimiento? (DD-MM-AAAA)").await
}

async fn handle_birth_date_response(app_state: &Arc<AppState>, survey_state: &mut crate::models::user::SurveyState, ws_id: &str, birth_date: &str) -> Result<()> {
    match NaiveDate::parse_from_str(birth_date, "%d-%m-%Y") {
        Ok(_) => {
            survey_state.birth_date = Some(birth_date.to_string());
            survey_state.step = "awaiting_origin_country".to_string();
            ask_question(app_state, ws_id, "Entendido. ¿De qué país eres?").await
        }
        Err(_) => {
            let error_message = "El formato de la fecha no es válido. Por favor, usa el formato DD-MM-AAAA.";
            whatsapp_service::send_text_message(app_state, ws_id, error_message).await?;
            // Keep the user at the same step
            let updated_json = serde_json::to_string(survey_state)?;
            let key = format!("{}{}", SURVEY_STATE_KEY_PREFIX, ws_id);
            let mut redis_conn = app_state.redis_client.get_multiplexed_async_connection().await?;
            let _: () = redis_conn.set_ex(key, updated_json, SURVEY_EXPIRATION_SECONDS).await?;
            Ok(())
        }
    }
}

async fn handle_origin_country_response(app_state: &Arc<AppState>, survey_state: &mut crate::models::user::SurveyState, ws_id: &str, country: &str) -> Result<()> {
    survey_state.country_of_origin = Some(country.to_string());
    survey_state.step = "awaiting_residence_country".to_string();
    ask_question(app_state, ws_id, "¿Y en qué país resides actualmente?").await
}

async fn handle_residence_country_response(app_state: &Arc<AppState>, survey_state: &mut crate::models::user::SurveyState, ws_id: &str, country: &str) -> Result<()> {
    survey_state.country_of_residence = Some(country.to_string());
    survey_state.step = "awaiting_email".to_string();
    ask_question(app_state, ws_id, "¡Ya casi terminamos! Por favor, dime tu correo electrónico.").await
}

async fn handle_email_response(app_state: &Arc<AppState>, survey_state: &mut crate::models::user::SurveyState, ws_id: &str, email: &str) -> Result<()> {
    let email_regex = Regex::new(r"^([a-zA-Z0-9_\-\.]+)@([a-zA-Z0-9_\-\.]+)\.([a-zA-Z]{2,5})$").unwrap();
    if email_regex.is_match(email) {
        survey_state.email = Some(email.to_string());
        survey_state.step = "awaiting_email_confirmation".to_string();
        ask_question(app_state, ws_id, &format!("Has introducido {}. ¿Es correcto? (Sí/No)", email)).await
    } else {
        let error_message = "El formato del correo electrónico no es válido. Por favor, introduce una dirección de correo válida (por ejemplo, tu@email.com).";
        whatsapp_service::send_text_message(app_state, ws_id, error_message).await?;
        // Keep the user at the same step
        let updated_json = serde_json::to_string(survey_state)?;
        let key = format!("{}{}", SURVEY_STATE_KEY_PREFIX, ws_id);
        let mut redis_conn = app_state.redis_client.get_multiplexed_async_connection().await?;
        let _: () = redis_conn.set_ex(key, updated_json, SURVEY_EXPIRATION_SECONDS).await?;
        Ok(())
    }
}

async fn handle_email_confirmation_response(app_state: &Arc<AppState>, survey_state: &mut crate::models::user::SurveyState, ws_id: &str, confirmation: &str) -> Result<()> {
    if confirmation.trim().eq_ignore_ascii_case("sí") || confirmation.trim().eq_ignore_ascii_case("si") {
        survey_state.step = "completed".to_string();
        handle_final_response(app_state, survey_state, ws_id).await
    } else {
        survey_state.step = "awaiting_email".to_string();
        ask_question(app_state, ws_id, "Entendido. Por favor, ingresa tu correo electrónico de nuevo.").await
    }
}

async fn handle_final_response(app_state: &Arc<AppState>, survey_state: &crate::models::user::SurveyState, ws_id: &str) -> Result<()> {
    match user_service::create_user(&app_state.db_pool, ws_id, survey_state).await {
        Ok(_) => {
            whatsapp_service::send_text_message(app_state, ws_id, "¡Gracias por completar la encuesta! Tu perfil ha sido creado.").await?;
            let mut redis_conn = app_state.redis_client.get_multiplexed_async_connection().await?;
            let key = format!("{}{}", SURVEY_STATE_KEY_PREFIX, ws_id);
            let _: () = redis_conn.del(key).await?;
        }
        Err(e) => {
            error!("Error al crear el usuario: {}", e);
            whatsapp_service::send_text_message(app_state, ws_id, "Lo siento, hubo un error al crear tu perfil. Por favor, intenta de nuevo.").await?;
        }
    }

    Ok(())
}
