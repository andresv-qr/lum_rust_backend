use serde::{Deserialize, Serialize};

/// Response model for a successful OCR upload and processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadOcrResponse {
    pub processing_status: String,
    pub message: String,
    pub invoice_id: Option<i64>,
    pub extracted_cufe: Option<String>,
    pub validation_details: Vec<String>,
}

/// Helpers for the Upload OCR API
pub struct UploadOcrHelpers;

impl UploadOcrHelpers {
    /// Generates a user-friendly message based on the processing result.
    pub fn generate_response_message(status: &str, details: Option<&str>) -> String {
        match status {
            "SUCCESS" => "✅ ¡Factura procesada y validada exitosamente!".to_string(),
            "PENDING_VALIDATION" => "📄 Factura recibida y en proceso de validación. Te notificaremos pronto.".to_string(),
            "PROCESSING_ERROR" => format!("❌ Error durante el procesamiento OCR. Detalles: {}", details.unwrap_or("desconocido")),
            "VALIDATION_FAILED" => format!("⚠️ La validación de la factura falló. Detalles: {}", details.unwrap_or("revisa los datos")),
            "INVALID_FILE" => "❌ El archivo proporcionado no es una imagen válida o está corrupto.".to_string(),
            _ => "ℹ️ Estado del procesamiento desconocido.".to_string(),
        }
    }
}
