use crate::{
    models::invoice::{InvoiceHeader, InvoiceDetail, InvoicePayment, MefPending},
    processing::web_scraping::{data_parser, http_client, ocr_extractor},
    shared::database as db_service,
    shared::whatsapp as whatsapp_service,
    AppState,
};
use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{warn, info, error};

pub async fn process_invoice_url(
    state: Arc<AppState>,
    url: &str,
    ws_id: &str,
    user_id: i64,
) -> Result<()> {
    info!("🔄 Iniciando procesamiento de factura desde URL: {}", url);
    
    // PASO 1: Web Scraping - Extraer datos de la URL
    info!("📡 Realizando web scraping de la URL...");
    let scraping_result = try_process_invoice(&state, url).await;
    
    match scraping_result {
        Ok((mut header, details, payments)) => {
            let cufe = &header.cufe;
            info!("✅ Web scraping exitoso. CUFE extraído: {}", cufe);
            
            // PASO 2: Validar si CUFE ya existe
            info!("🔍 Validando si CUFE ya existe en la base de datos...");
            let cufe_exists = db_service::validate_cufe_exists(&state.db_pool, cufe).await
                .context("Failed to validate CUFE existence")?;
            
            if cufe_exists {
                // PASO 3A: CUFE ya existe - Responder al usuario
                let duplicate_message = "¡Estos Lümis ya están en tu cuenta! 🔍 ¿Probamos con otra factura para ganar más Lümis? 💰";
                whatsapp_service::send_text_message(&state, ws_id, duplicate_message).await?;
                info!("📋 Factura duplicada detectada para CUFE: {}", cufe);
                return Ok(());
            }
            
            // PASO 3B: CUFE es nuevo - Proceder con guardado en tablas principales
            info!("💾 CUFE es nuevo, guardando en tablas principales...");
            header.user_id = user_id;
            
            let mut tx = state.db_pool.begin().await.context("Failed to start transaction")?;
            
            match db_service::save_invoice_data(&mut tx, &header, &details, &payments).await {
                Ok(()) => {
                    // PASO 4A: Guardado exitoso en tablas principales
                    tx.commit().await.context("Failed to commit transaction")?;
                    
                    let success_message = format!(
                        "✅ ¡Factura procesada exitosamente!\n\n📋 **Detalles:**\n🏪 Emisor: {}\n📄 Número: {}\n💰 Total: ${}\n\n🎉 ¡Lümis agregados a tu cuenta!",
                        &header.issuer_name,
                        &header.no,
                        header.tot_amount
                    );
                    whatsapp_service::send_text_message(&state, ws_id, &success_message).await?;
                    info!("🎉 Factura procesada exitosamente para CUFE: {}", cufe);
                }
                Err(save_error) => {
                    // PASO 4B: Error al guardar en tablas principales - Fallback a mef_pending
                    error!("❌ Error guardando en tablas principales: {}. Usando fallback a mef_pending.", save_error);
                    
                    let pending_entry = MefPending {
                        id: 0,
                        url: Some(url.to_string()),
                        chat_id: Some(ws_id.to_string()),
                        reception_date: Some(chrono::Utc::now()),
                        message_id: None,
                        type_document: Some("QR_INVOICE".to_string()),
                        user_email: None,
                        user_id: Some(user_id),
                        error_message: Some(format!("Save error: {}", save_error)),
                        origin: Some("WHATSAPP_RUST".to_string()),
                        ws_id: Some(ws_id.to_string()),
                    };
                    
                    db_service::save_to_mef_pending(&mut tx, &pending_entry)
                        .await
                        .context("Failed to save to mef_pending as fallback")?;
                    
                    tx.commit().await.context("Failed to commit mef_pending transaction")?;
                    
                    let fallback_message = "📝 Hemos recibido tu factura. Nuestro equipo la revisará y te confirmaremos cuando esté procesada. ¡Gracias por tu paciencia!";
                    whatsapp_service::send_text_message(&state, ws_id, fallback_message).await?;
                    warn!("⚠️ Factura guardada en mef_pending como fallback para CUFE: {}", cufe);
                }
            }
        }
        Err(scraping_error) => {
            // PASO 5: Error en web scraping - Fallback a mef_pending
            error!("❌ Error en web scraping: {}. Guardando en mef_pending.", scraping_error);
            
            let mut tx = state.db_pool.begin().await.context("Failed to start transaction")?;
            
            let pending_entry = MefPending {
                id: 0,
                url: Some(url.to_string()),
                chat_id: Some(ws_id.to_string()),
                reception_date: Some(chrono::Utc::now()),
                message_id: None,
                type_document: Some("QR_INVOICE".to_string()),
                user_email: None,
                user_id: Some(user_id),
                error_message: Some(format!("Scraping error: {}", scraping_error)),
                origin: Some("WHATSAPP_RUST".to_string()),
                ws_id: Some(ws_id.to_string()),
            };
            
            db_service::save_to_mef_pending(&mut tx, &pending_entry)
                .await
                .context("Failed to save to mef_pending after scraping error")?;
            
            tx.commit().await.context("Failed to commit mef_pending transaction")?;
            
            let error_message = "🔧 No pudimos procesar la factura automáticamente. Nuestro equipo la revisará manualmente. Te notificaremos cuando esté lista.";
            whatsapp_service::send_text_message(&state, ws_id, error_message).await?;
            warn!("⚠️ Error de scraping, factura guardada en mef_pending para URL: {}", url);
        }
    }
    
    Ok(())
}

async fn try_process_invoice(
    state: &AppState,
    url: &str,
) -> Result<(InvoiceHeader, Vec<InvoiceDetail>, Vec<InvoicePayment>)> {
    // First, get the final URL after following any redirections
    info!("🔍 Resolving final URL for: {}", url);
    let (html_content, final_url) = http_client::fetch_url_content_with_final_url(&state.http_client, url).await
        .context("Failed to fetch URL content with final URL")?;
    
    if final_url != url {
        info!("✅ Successfully resolved URL: {} → {}", url, final_url);
    } else {
        info!("✅ URL resolved without redirection: {}", url);
    }
    
    let extracted_data = ocr_extractor::extract_main_info(&html_content)?;
    
    // Use the final URL for data parsing instead of the original URL
    let (header, details, payments) = data_parser::parse_invoice_data(&extracted_data, &final_url)?;
    Ok((header, details, payments))
}

pub async fn process_invoice(_state: Arc<AppState>, _ws_id: &str, _cufe: &str) -> Result<()> {
    // This function is not implemented in the provided code
    unimplemented!()
}
