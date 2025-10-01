use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};

use crate::{
    models::ocr::*,
    services::redis_service,
    state::AppState,
};

/// Servicio para manejar sesiones OCR iterativas
pub struct OcrSessionService;

impl OcrSessionService {
    /// Crear nueva sesión OCR
    pub async fn create_session(
        state: &Arc<AppState>,
        user_id: i64,
    ) -> Result<OcrSession> {
        let session = OcrSession::new(user_id);
        
        // Guardar en Redis con TTL de 30 minutos
        let session_key = format!("ocr_session:{}", session.session_id);
        let session_data = serde_json::to_string(&session)?;
        
        redis_service::set_with_ttl(
            &state.redis_pool,
            &session_key,
            &session_data,
            1800, // 30 minutos
        ).await?;
        
        info!("Nueva sesión OCR creada: {} para usuario {}", session.session_id, user_id);
        Ok(session)
    }
    
    /// Obtener sesión existente
    pub async fn get_session(
        state: &Arc<AppState>,
        session_id: &str,
    ) -> Result<Option<OcrSession>> {
        let session_key = format!("ocr_session:{}", session_id);
        
        match crate::shared::redis_compat::get::<String>(&state.redis_pool, &session_key).await? {
            Some(session_data) => {
                match serde_json::from_str::<OcrSession>(&session_data) {
                    Ok(session) => Ok(Some(session)),
                    Err(e) => {
                        warn!("Error deserializando sesión {}: {}", session_id, e);
                        Ok(None)
                    }
                }
            }
            None => Ok(None),
        }
    }
    
    /// Actualizar sesión existente
    pub async fn update_session(
        state: &Arc<AppState>,
        session: &OcrSession,
    ) -> Result<()> {
        let session_key = format!("ocr_session:{}", session.session_id);
        let session_data = serde_json::to_string(session)?;
        
        redis_service::set_with_ttl(
            &state.redis_pool,
            &session_key,
            &session_data,
            1800, // Renovar TTL
        ).await?;
        
        Ok(())
    }
    
    /// Eliminar sesión
    pub async fn delete_session(
        state: &Arc<AppState>,
        session_id: &str,
    ) -> Result<()> {
        let session_key = format!("ocr_session:{}", session_id);
        redis_service::delete(&state.redis_pool, &session_key).await?;
        Ok(())
    }
    
    /// Agregar nueva imagen y datos detectados a la sesión
    pub async fn add_attempt(
        state: &Arc<AppState>,
        session_id: &str,
        image_bytes: &[u8],
        mime_type: &str,
        detected_data: InvoiceData,
        focus_fields: Option<Vec<String>>,
    ) -> Result<OcrSession> {
        let mut session = Self::get_session(state, session_id).await?
            .ok_or_else(|| anyhow!("Sesión no encontrada"))?;
        
        // Verificar que no se haya excedido el límite
        if session.attempt_count >= session.max_attempts {
            return Err(anyhow!("Límite de intentos alcanzado"));
        }
        
        // Crear datos de imagen
        let image_data = OcrImageData {
            image_id: Uuid::new_v4().to_string(),
            attempt_number: session.attempt_count + 1,
            image_data: general_purpose::STANDARD.encode(image_bytes),
            mime_type: mime_type.to_string(),
            file_size: image_bytes.len() as u64,
            focus_fields,
            uploaded_at: chrono::Utc::now(),
        };
        
        // Agregar intento a la sesión
        session.add_attempt(image_data, detected_data);
        
        // Guardar sesión actualizada
        Self::update_session(state, &session).await?;
        
        info!(
            "Intento {} agregado a sesión {}, estado: {:?}",
            session.attempt_count, session_id, session.state
        );
        
        Ok(session)
    }
    
    /// Consolidar todas las imágenes de la sesión en una sola
    pub async fn consolidate_images(
        state: &Arc<AppState>,
        session_id: &str,
    ) -> Result<String> {
        let mut session = Self::get_session(state, session_id).await?
            .ok_or_else(|| anyhow!("Sesión no encontrada"))?;
        
        if session.images.is_empty() {
            return Err(anyhow!("No hay imágenes para consolidar"));
        }
        
        // TODO: Implementar consolidación real de imágenes
        // Por ahora, usar la imagen más reciente como consolidada
        let latest_image = session.images.last().unwrap();
        let consolidated = latest_image.image_data.clone();
        
        session.consolidated_image = Some(consolidated.clone());
        Self::update_session(state, &session).await?;
        
        info!("Imágenes consolidadas para sesión {}", session_id);
        Ok(consolidated)
    }
    
    /// Limpiar sesiones expiradas (task de mantenimiento)
    pub async fn cleanup_expired_sessions(_state: &Arc<AppState>) -> Result<()> {
        // Redis maneja esto automáticamente con TTL, pero podemos loggear
        info!("Limpieza de sesiones OCR completada");
        Ok(())
    }
    
    /// Obtener estadísticas de sesiones activas
    pub async fn get_session_stats(_state: &Arc<AppState>) -> Result<SessionStats> {
        // TODO: Implementar conteo de sesiones activas en Redis
        Ok(SessionStats {
            active_sessions: 0,
            total_attempts_today: 0,
            success_rate: 0.0,
        })
    }
}

/// Estadísticas de sesiones OCR
#[derive(Debug)]
pub struct SessionStats {
    pub active_sessions: u32,
    pub total_attempts_today: u32,
    pub success_rate: f64,
}

/// Servicio para consolidación de imágenes
pub struct ImageConsolidationService;

impl ImageConsolidationService {
    /// Consolidar múltiples imágenes en una sola optimizada
    pub async fn consolidate_images(images: &[OcrImageData]) -> Result<String> {
        if images.is_empty() {
            return Err(anyhow!("No hay imágenes para consolidar"));
        }
        
        // TODO: Implementar consolidación real usando imageproc o similar
        // Por ahora, devolver la imagen más reciente
        let latest = images.last().unwrap();
        Ok(latest.image_data.clone())
    }
    
    /// Optimizar imagen para mejor calidad OCR
    pub async fn optimize_for_ocr(image_data: &str) -> Result<String> {
        // TODO: Implementar optimización (contraste, resolución, etc.)
        Ok(image_data.to_string())
    }
    
    /// Generar preview de imagen consolidada
    pub async fn generate_preview(consolidated_image: &str) -> Result<String> {
        // TODO: Generar thumbnail o preview de menor calidad
        Ok(consolidated_image.to_string())
    }
}

/// Generador de prompts específicos para OCR
pub struct OcrPromptGenerator;

impl OcrPromptGenerator {
    /// Generar prompt base para primer intento
    pub fn generate_initial_prompt() -> String {
        r#"Analiza esta imagen de factura y extrae la siguiente información en formato JSON:
{
  "issuer_name": "Nombre del comercio/empresa",
  "invoice_number": "Número de factura",
  "date": "Fecha en formato YYYY-MM-DD",
  "total": 0.0,
  "products": [
    {
      "name": "Nombre del producto",
      "quantity": 1.0,
      "unit_price": 0.0,
      "total_price": 0.0
    }
  ],
  "rif": "RIF/NIT si está visible",
  "address": "Dirección si está visible",
  "subtotal": 0.0,
  "tax": 0.0
}

Extrae TODOS los datos visibles. Si un campo no está claro, usa null."#.to_string()
    }
    
    /// Generar prompt enfocado para campos específicos
    pub fn generate_focused_prompt(
        missing_fields: &[String],
        existing_data: &InvoiceData,
    ) -> String {
        let fields_description = missing_fields.iter()
            .map(|field| Self::get_field_description(field))
            .collect::<Vec<_>>()
            .join(", ");
        
        let existing_summary = Self::summarize_existing_data(existing_data);
        
        format!(
            r#"Esta es una imagen adicional de una factura. ENFÓCATE ESPECÍFICAMENTE en detectar:
{}

Información ya detectada en intentos anteriores:
{}

Solo actualiza o agrega los campos faltantes. No cambies los datos ya detectados.
Responde en el mismo formato JSON anterior."#,
            fields_description,
            existing_summary
        )
    }
    
    fn get_field_description(field: &str) -> String {
        match field {
            "issuer_name" => "- NOMBRE DEL COMERCIO: Busca el nombre de la empresa en la parte superior".to_string(),
            "invoice_number" => "- NÚMERO DE FACTURA: Busca el número único de la factura (ej: F001-123456)".to_string(),
            "date" => "- FECHA: Busca la fecha de emisión".to_string(),
            "total" => "- TOTAL: Busca el monto total final a pagar".to_string(),
            "products" => "- PRODUCTOS: Busca la lista detallada de artículos con precios".to_string(),
            _ => format!("- {}: Campo requerido", field.to_uppercase()),
        }
    }
    
    fn summarize_existing_data(data: &InvoiceData) -> String {
        let mut summary = Vec::new();
        
        if let Some(name) = &data.issuer_name {
            summary.push(format!("Comercio: {}", name));
        }
        if let Some(number) = &data.invoice_number {
            summary.push(format!("Número: {}", number));
        }
        if let Some(date) = &data.date {
            summary.push(format!("Fecha: {}", date));
        }
        if let Some(total) = data.total {
            summary.push(format!("Total: ${}", total));
        }
        if !data.products.is_empty() {
            summary.push(format!("{} productos detectados", data.products.len()));
        }
        
        if summary.is_empty() {
            "Ningún dato detectado aún".to_string()
        } else {
            summary.join(", ")
        }
    }
}
