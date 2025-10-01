use crate::{
    models::user::UserState,
    processing::flows::product_search_flow,
    services::{redis_service, user_service, whatsapp_service, rewards_service},
    state::AppState,
};
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

/// Maneja los comandos de texto enviados por el usuario.
pub async fn handle_command(app_state: &Arc<AppState>, whatsapp_id: &str, text: &str) -> Result<()> {
    info!("Processing command '{}' for user {}", text, whatsapp_id);
    let command = text.split_whitespace().next().unwrap_or("").to_lowercase();

    match command.as_str() {
        "/start" | "/registro" => handle_registration_command(app_state, whatsapp_id).await,
        "/ayuda" => handle_help_command(app_state, whatsapp_id).await,
        "/lumis" | "/saldo" | "/mis_lumis" => handle_lumis_balance_command(app_state, whatsapp_id).await,
        "/resumen" | "/movimientos" | "/resumen_movimientos" => handle_movements_summary_command(app_state, whatsapp_id).await,
        "/buscar" => handle_product_search_command(app_state, whatsapp_id).await,
        "/premios" | "/retos" | "/misiones" => handle_rewards_command(app_state, whatsapp_id).await,
        "/historial" => handle_history_command(app_state, whatsapp_id).await,
        "/cancelar" | "/salir" => handle_cancel_command(app_state, whatsapp_id).await,
        "/perfil" => handle_profile_command(app_state, whatsapp_id).await,
        "/factura" => handle_qr_invoice_command(app_state, whatsapp_id).await,
        "/qr" => handle_qr_invoice_command(app_state, whatsapp_id).await,
        "/privacidad" => handle_data_protection_command(app_state, whatsapp_id).await,
        "/feedback" | "/sugerencia" => handle_feedback_command(app_state, whatsapp_id).await,
        "/trivias" => handle_trivia_command(app_state, whatsapp_id).await,
        "/factura_sin_qr" => handle_ocr_invoice_command(app_state, whatsapp_id).await,
        _ => {
            let response_text = "No he reconocido ese comando. Escribe */ayuda* para ver la lista de opciones disponibles.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, response_text).await
        }
    }
}

async fn handle_cancel_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    redis_service::delete_user_state(app_state, whatsapp_id).await?;
    let message = "Tu operación ha sido cancelada. Puedes empezar de nuevo cuando quieras.";
    whatsapp_service::send_text_message(app_state, whatsapp_id, message).await
}

async fn handle_registration_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let response = "🎉 *¡Bienvenido a Lüm!*\n\nPara completar tu registro y desbloquear todos los beneficios, necesitamos conocerte mejor.\n\n¡Empecemos con una breve encuesta!";
    whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
}

async fn handle_help_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let user_state = redis_service::get_user_state(app_state, whatsapp_id).await?;

    let help_message = match user_state {
        Some(UserState::Survey(state)) => match state.step.as_str() {
            "awaiting_name" => "Parece que estás en medio del registro. Por favor, escribe tu nombre completo para continuar, o `/cancelar` para salir.",
            "awaiting_birth_date" => "Ahora necesitamos tu fecha de nacimiento (DD/MM/AAAA). O escribe `/cancelar` para salir.",
            "awaiting_country" => "¿En qué país naciste? Escríbelo para continuar, o `/cancelar` para salir.",
            "awaiting_residence_country" => "¿Y en qué país vives actualmente? Escríbelo para continuar, o `/cancelar` para salir.",
            "awaiting_email" => "Por favor, introduce tu correo electrónico. O escribe `/cancelar` para salir.",
            "awaiting_email_confirmation" => "Re-escribe tu correo para confirmarlo. O escribe `/cancelar` para salir.",
            _ => "Estás en medio de un proceso. Por favor, sigue las instrucciones o escribe `/cancelar` para empezar de nuevo.",
        },
        Some(UserState::ProductSearch) => "Estás buscando un producto. Escribe el nombre del producto que buscas, o `/cancelar` para salir.",
        Some(UserState::OcrInvoice) => "Estoy esperando que me envíes la imagen o el PDF de tu factura. Si no quieres continuar, escribe `/cancelar`.",
        Some(UserState::WaitingForImage) => "Estoy esperando que me envíes una imagen para procesar el QR. Si no quieres continuar, escribe `/cancelar`.",
        Some(UserState::WaitingForImageOcr) => "Estoy esperando que me envíes una imagen para procesar con OCR. Si no quieres continuar, escribe `/cancelar`.",
        Some(UserState::OffersRadar { .. }) => "Estás seleccionando una categoría de ofertas. Escribe el nombre de la categoría que te interesa, o `/cancelar` para salir.",
        None => "Aquí tienes la lista de comandos disponibles:\n\n*COMANDOS PRINCIPALES*\n`/registro` - Inicia tu registro en Lüm.\n`/saldo` - Consulta tu balance de Lümis.\n`/movimientos` - Muestra tus últimos movimientos.\n`/buscar` - Busca productos en nuestra base de datos.\n`/premios` - Descubre los premios que puedes canjear.\n`/historial` - Revisa tu historial de canjes.\n`/factura_sin_qr` - Procesa una factura sin código QR.\n\n*OTROS COMANDOS*\n`/ayuda` - Muestra este mensaje de ayuda.\n`/perfil` - (Próximamente) Gestiona tu perfil.\n`/factura` - Ayuda para subir facturas.\n`/privacidad` - Información sobre protección de datos.\n`/feedback` - Envíanos tus sugerencias.\n`/trivias` - (Próximamente) Juega y gana Lümis.\n`/cancelar` - Cancela la operación actual.",
        Some(UserState::PriceRange(_)) => "Estás en el proceso de selección de ofertas. Escribe el nombre de una categoría o un rango de precios según el paso actual. Usa `/cancelar` para salir.",
    };

    whatsapp_service::send_text_message(app_state, whatsapp_id, help_message).await
}

async fn handle_lumis_balance_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    if let Some(balance) = user_service::get_user_lumis_balance(app_state, whatsapp_id).await? {
        let response = format!("Tienes un saldo de *{} Lümis*.", balance);
        whatsapp_service::send_text_message(app_state, whatsapp_id, &response).await
    } else {
        let response = "No hemos podido encontrar tu saldo. ¿Te has registrado ya? Usa el comando `/registro`.";
        whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
    }
}

async fn handle_movements_summary_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    user_service::get_and_format_user_metrics(app_state, whatsapp_id).await
}

async fn handle_product_search_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    if user_service::is_user_subscribed(app_state, whatsapp_id).await? {
        product_search_flow::start_product_search(app_state, whatsapp_id).await
    } else {
        let message = "Esta es una función para usuarios registrados. \nUsa el comando `/registro` para darte de alta.";
        whatsapp_service::send_text_message(app_state, whatsapp_id, message).await
    }
}

async fn handle_rewards_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let response = "🏆 *Premios, Retos y Misiones*\n\n¡Aquí podrás ver todas las formas de ganar Lümis y los premios que puedes canjear!\n\nEsta sección estará disponible muy pronto. ¡Mantente atento! ✨";
    whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
}

async fn handle_history_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    if let Some(user) = user_service::get_user(app_state, whatsapp_id).await? {
        let history = rewards_service::get_user_redemption_history(&app_state.db_pool, user.id.into(), 5).await?;
        let mut response = String::from("📜 *Tu Historial de Canjes (últimos 5)*");

        if history.is_empty() {
            response.push_str("\n\nNo has canjeado ningún premio todavía. ¡Anímate a explorar nuestro catálogo de `premios`!");
        } else {
            for item in history {
                let description = item.redem_id.as_deref().unwrap_or("Redención");
                let cost = item.quantity.unwrap_or(0);
                let date_str = item.date
                    .map(|d| d.format("%d/%m/%Y").to_string())
                    .unwrap_or_else(|| "Fecha no disponible".to_string());
                response.push_str(&format!("\n• *{}* ({} Lümis) - {}", description, cost, date_str));
            }
        }
        whatsapp_service::send_text_message(app_state, whatsapp_id, &response).await
    } else {
        whatsapp_service::send_text_message(app_state, whatsapp_id, "Debes estar registrado para ver tu historial. Usa `/registro` para registrarte.").await
    }
}

async fn handle_profile_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let response = "👤 *Tu Perfil*\n\nEsta funcionalidad estará disponible pronto.\n\nPodrás ver y editar:\n• Información personal\n• Preferencias de notificaciones\n• Historial de actividad\n• Configuración de privacidad\n\n¡Mantente atento a las actualizaciones!";
    whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
}

// async fn handle_invoice_upload_help_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> { // Commented out - dead code
//     let response = "📷 *¿Cómo subir facturas?*\n\n*Método 1: Foto del QR*\n• Toma una foto clara del código QR\n• Asegúrate que esté bien enfocado\n• Evita reflejos y sombras\n\n*Método 2: Foto de la factura completa*\n• Toma foto de toda la factura\n• Debe ser legible y clara\n• Incluye todos los datos fiscales\n\n*Tips importantes:*
// ✅ Buena iluminación
// ✅ Imagen nítida y clara
// ✅ QR completo y visible
// ❌ Evita fotos borrosas
// ❌ No cortes el QR
// ❌ Evita reflejos\n\n¡Envía tu factura ahora mismo! 📸";
//     whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
// }

async fn handle_data_protection_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let response = "🔒 *Protección de Datos*\n\n*Tu privacidad es nuestra prioridad*\n\n🛡️ *Qué protegemos:*
• Información personal\n• Datos de facturas\n• Historial de compras\n• Preferencias de usuario\n\n🔐 *Cómo lo hacemos:*
• Encriptación de datos\n• Servidores seguros\n• Acceso restringido\n• Cumplimiento legal\n\n📋 *Tus derechos:*
• Acceso a tus datos\n• Corrección de información\n• Eliminación de cuenta\n• Portabilidad de datos\n\n📄 Para más detalles, consulta nuestra política de privacidad completa.\n\n¿Tienes dudas? Escribe /feedback";
    whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
}

async fn handle_feedback_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let response = "📝 *¡Tu opinión es un tesoro!* ✨\n\nNos ayuda a mejorar Lüm para ti.\n\n💭 *¿Tienes alguna sugerencia, idea o comentario?*\n
👉 Escríbelo aquí: https://docs.google.com/forms/d/e/1FAIpQLScU7ZuYIFznCbwXT80ns3wBOhrbjz3iQ8zdI2-EmZnYziIv3A/viewform\n\n¡Cada comentario cuenta y lo guardaremos como un tesoro! 💎";
    whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
}

async fn handle_qr_invoice_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    info!("Processing /qr or /factura command for user {}", whatsapp_id);
    
    // 1. Verificar que el usuario esté registrado
    let user_opt = user_service::get_user(app_state, whatsapp_id).await?;
    let _user = match user_opt {
        Some(user) => user,
        None => {
            let message = "❌ Debes estar registrado para usar esta función.\n\nUsa /registro para comenzar.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;
            return Ok(());
        }
    };
    
    // 2. Establecer estado WaitingForImage
    let qr_state = UserState::WaitingForImage;
    redis_service::save_user_state(app_state, whatsapp_id, &qr_state, 1800).await?; // 30 minutos TTL
    
    // 3. Enviar mensaje de instrucciones
    let mensaje = "📱 **Procesamiento de Facturas con QR**\n\n\
        🔍 Envía una foto clara de tu factura con código QR\n\
        ⚡ Detectaremos automáticamente el QR\n\
        🌐 Haremos web scraping de la URL\n\
        ✅ Validaremos si ya está registrada\n\
        💾 Guardaremos los datos en tu cuenta\n\n\
        📋 **Instrucciones:**\n\
        • Asegúrate de que el QR sea visible\n\
        • La imagen debe estar bien iluminada\n\
        • Evita reflejos en el QR\n\n\
        ⏰ Tienes 30 minutos para enviar la imagen.\n\
        Escribe /cancelar si cambias de opinión.";
    
    whatsapp_service::send_text_message(app_state, whatsapp_id, &mensaje).await?;
    
    info!("QR Command activated - User {} is now in WaitingForImage state", whatsapp_id);
    
    Ok(())
}

async fn handle_ocr_invoice_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    info!("Processing /factura_sin_qr command for user {}", whatsapp_id);
    
    // 1. Verificar que el usuario esté registrado
    let user_opt = user_service::get_user(app_state, whatsapp_id).await?;
    let user = match user_opt {
        Some(user) => user,
        None => {
            let message = "❌ Debes estar registrado para usar esta función.\n\nUsa /registro para comenzar.";
            whatsapp_service::send_text_message(app_state, whatsapp_id, message).await?;
            return Ok(());
        }
    };
    
    // 2. Verificar rate limits usando el sistema avanzado
    let (rate_allowed, rate_message) = redis_service::check_advanced_ocr_rate_limit(app_state, whatsapp_id).await?;
    if !rate_allowed {
        let message = format!(
            "{}

⏰ Intenta más tarde o usa facturas con QR para incrementar tu límite.",
            rate_message
        );
        whatsapp_service::send_text_message(app_state, whatsapp_id, &message).await?;
        return Ok(());
    }
    
    // 3. Obtener límites del usuario, trust score y balance
    let user_limits = redis_service::get_user_ocr_limits(app_state, whatsapp_id).await?;
    let trust_score = redis_service::get_user_trust_score(app_state, whatsapp_id).await?;
    let balance = rewards_service::get_user_balance(&app_state.db_pool, user.id as i64).await?;
    
    // 4. Verificar balance solo si hay costo (actualmente 0 para pruebas)
    let cost_lumis = user_limits.cost_lumis.unwrap_or(0);
    if cost_lumis > 0 && balance < cost_lumis {
        let message = format!(
            "❌ Balance insuficiente.

💰 Necesitas: {} Lümis
💳 Tu balance: {} Lümis",
            cost_lumis, balance
        );
        whatsapp_service::send_text_message(app_state, whatsapp_id, &message).await?;
        return Ok(());
    }
    
    // 5. Crear mensaje personalizado según el costo
    let costo_texto = if cost_lumis == 0 {
        "🆓 **GRATUITO** (período de prueba)".to_string()
    } else {
        format!("💰 **Costo:** {} Lümis", cost_lumis)
    };
    
    let mensaje = format!(
        "🤖 **Procesamiento de Facturas sin QR**

\
        📷 Sube una foto clara de tu factura
\
        🔍 La procesaremos con inteligencia artificial
\
        ✅ Validaremos todos los campos obligatorios
\
        👥 Nuestro equipo verificará la información

\
        {}
\
        📊 **Tu nivel de confianza:** {}/50
\
        ⏱️ **Límites:** {}/hora, {}/día
\
        📋 **Requisitos:** Comercio, fecha, número, total y productos claramente visibles

\
        ⚠️ **Importante:** Solo sube facturas reales. El mal uso puede resultar en restricciones.

\
        ¿Estás listo? Envía la foto de tu factura.",
        costo_texto,
        trust_score,
        10, // per_hour default
        user_limits.max_daily
    );
    
    // 6. Guardar estado OCR con contexto completo
    let ocr_state = UserState::OcrInvoice;
    redis_service::save_user_state(app_state, whatsapp_id, &ocr_state, 1800).await?; // 30 minutos TTL
    
    // 7. Enviar mensaje al usuario
    whatsapp_service::send_text_message(app_state, whatsapp_id, &mensaje).await?;
    
    info!("OCR Command Debug - Chat: {}, Cost: {}, Trust: {}", 
          whatsapp_id, cost_lumis, trust_score);
    
    Ok(())
}

async fn handle_trivia_command(app_state: &Arc<AppState>, whatsapp_id: &str) -> Result<()> {
    let response = "🧠 *¡Trivias Lüm!* 🎯\n\n*¡Pon a prueba tus conocimientos y gana Lümis!*\n\n🎮 *¿Cómo funciona?*\n• Responde preguntas de cultura general\n• Cada respuesta correcta suma Lümis\n• Nuevas trivias cada día\n\n🏆 *Premios:*
• 5 Lümis por respuesta correcta\n• Bonos especiales por rachas\n• Trivias temáticas con premios extra\n
⏰ *Próximamente:*
Esta funcionalidad estará disponible muy pronto.\n\n¡Mantente atento para ser el primero en participar! 🚀";
    whatsapp_service::send_text_message(app_state, whatsapp_id, response).await
}
