use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Legacy structs (mantener compatibilidad)
#[derive(Debug, Serialize, Deserialize)]
pub struct OcrResult {
    pub success: bool,
    pub message: String,
    pub data: Option<LegacyInvoiceData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyInvoiceData {
    pub cufe: String,
    pub total_amount: f64,
    // Add other relevant fields from the invoice
}

/// Estados de una sesión OCR iterativa
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OcrSessionState {
    Initial,
    Processing,
    NeedsRetry { missing_fields: Vec<String> },
    Complete,
    ManualReview,
    Failed,
}

/// Sesión OCR que mantiene el estado entre llamadas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrSession {
    pub session_id: String,
    pub user_id: i64,
    pub attempt_count: u8,
    pub max_attempts: u8,
    pub state: OcrSessionState,
    pub detected_fields: InvoiceData,
    pub missing_fields: Vec<String>,
    pub images: Vec<OcrImageData>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub consolidated_image: Option<String>, // Base64
}

impl OcrSession {
    pub fn new(user_id: i64) -> Self {
        let now = chrono::Utc::now();
        Self {
            session_id: format!("ocr_sess_{}", &Uuid::new_v4().to_string()[..8]),
            user_id,
            attempt_count: 0,
            max_attempts: 5,
            state: OcrSessionState::Initial,
            detected_fields: InvoiceData::empty(),
            missing_fields: vec![
                "issuer_name".to_string(),
                "invoice_number".to_string(),
                "date".to_string(),
                "total".to_string(),
                "products".to_string(),
            ],
            images: Vec::new(),
            created_at: now,
            updated_at: now,
            consolidated_image: None,
        }
    }

    pub fn add_attempt(&mut self, image_data: OcrImageData, detected: InvoiceData) {
        self.attempt_count += 1;
        self.images.push(image_data);
        self.detected_fields.merge_with(detected);
        self.missing_fields = self.detected_fields.get_missing_fields();
        self.updated_at = chrono::Utc::now();

        // Actualizar estado
        if self.missing_fields.is_empty() {
            self.state = OcrSessionState::Complete;
        } else if self.attempt_count >= self.max_attempts {
            self.state = OcrSessionState::ManualReview;
        } else {
            self.state = OcrSessionState::NeedsRetry {
                missing_fields: self.missing_fields.clone(),
            };
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.state, OcrSessionState::Complete)
    }

    pub fn needs_manual_review(&self) -> bool {
        matches!(self.state, OcrSessionState::ManualReview)
    }
}

/// Datos de una imagen subida
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrImageData {
    pub image_id: String,
    pub attempt_number: u8,
    pub image_data: String, // Base64
    pub mime_type: String,
    pub file_size: u64,
    pub focus_fields: Option<Vec<String>>, // Campos que se pedía enfocar
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

/// Datos de factura detectados por OCR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceData {
    pub issuer_name: Option<String>,
    pub invoice_number: Option<String>,
    pub date: Option<String>,
    pub total: Option<f64>,
    pub products: Vec<ProductData>,
    
    // Campos adicionales
    pub rif: Option<String>,
    pub address: Option<String>,
    pub subtotal: Option<f64>,
    pub tax: Option<f64>,
}

impl InvoiceData {
    pub fn empty() -> Self {
        Self {
            issuer_name: None,
            invoice_number: None,
            date: None,
            total: None,
            products: Vec::new(),
            rif: None,
            address: None,
            subtotal: None,
            tax: None,
        }
    }

    pub fn merge_with(&mut self, other: InvoiceData) {
        // Combinar datos, priorizando los más recientes (other) si no son None
        if other.issuer_name.is_some() {
            self.issuer_name = other.issuer_name;
        }
        if other.invoice_number.is_some() {
            self.invoice_number = other.invoice_number;
        }
        if other.date.is_some() {
            self.date = other.date;
        }
        if other.total.is_some() {
            self.total = other.total;
        }
        if !other.products.is_empty() {
            self.products = other.products;
        }
        if other.rif.is_some() {
            self.rif = other.rif;
        }
        if other.address.is_some() {
            self.address = other.address;
        }
        if other.subtotal.is_some() {
            self.subtotal = other.subtotal;
        }
        if other.tax.is_some() {
            self.tax = other.tax;
        }
    }

    pub fn get_missing_fields(&self) -> Vec<String> {
        let mut missing = Vec::new();
        
        if self.issuer_name.is_none() || self.issuer_name.as_ref().unwrap().trim().is_empty() {
            missing.push("issuer_name".to_string());
        }
        if self.invoice_number.is_none() || self.invoice_number.as_ref().unwrap().trim().is_empty() {
            missing.push("invoice_number".to_string());
        }
        if self.date.is_none() || self.date.as_ref().unwrap().trim().is_empty() {
            missing.push("date".to_string());
        }
        if self.total.is_none() || self.total.unwrap_or(0.0) <= 0.0 {
            missing.push("total".to_string());
        }
        if self.products.is_empty() {
            missing.push("products".to_string());
        }
        
        missing
    }

    pub fn is_complete(&self) -> bool {
        self.get_missing_fields().is_empty()
    }
}

/// Producto detectado por OCR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductData {
    pub name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total_price: f64,
    pub line_number: Option<i32>,
}

/// Request para procesar OCR
#[derive(Debug, Deserialize)]
pub struct OcrProcessRequest {
    pub session_id: Option<String>,
    pub action: OcrAction,
    pub missing_fields: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OcrAction {
    Initial,
    Retry,
    Consolidate,
}

/// Response del procesamiento OCR
#[derive(Debug, Serialize)]
pub struct OcrProcessResponse {
    pub success: bool,
    pub session_id: String,
    pub attempt_count: u8,
    pub max_attempts: u8,
    pub status: String, // "processing" | "complete" | "needs_retry" | "manual_review" | "failed"
    pub detected_fields: InvoiceData,
    pub missing_fields: Vec<String>,
    pub consolidated_image: Option<String>,
    pub message: String,
    pub cost: OcrCostInfo,
}

/// Información de costos del procesamiento
#[derive(Debug, Serialize)]
pub struct OcrCostInfo {
    pub lumis_used: i32,
    pub tokens_used: i32,
}

/// Request para guardar factura OCR
#[derive(Debug, Deserialize)]
pub struct SaveOcrRequest {
    pub session_id: String,
    pub invoice_data: InvoiceData,
    pub consolidated_image: String,
    pub validation_status: ValidationStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Complete,
    ManualReview,
}

/// Response del guardado de factura
#[derive(Debug, Serialize)]
pub struct SaveOcrResponse {
    pub success: bool,
    pub invoice_id: Option<i64>,
    pub cufe: Option<String>,
    pub status: String,
    pub message: String,
    pub rewards: Option<RewardsInfo>,
    pub next_steps: Vec<String>,
}

/// Información de recompensas
#[derive(Debug, Serialize)]
pub struct RewardsInfo {
    pub lumis_earned: i32,
    pub xp_earned: i32,
    pub note: String,
}

/// Errores específicos del sistema OCR
#[derive(Debug)]
pub enum OcrSessionError {
    SessionNotFound(String),
    SessionExpired(String), 
    MaxAttemptsReached(u32),
    InvalidImage(String),
    InvalidInvoiceData(String),
    DuplicateInvoice(String),
    ProcessingError(String),
    DatabaseError(String),
}

/// Método de procesamiento de facturas
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "processing_method", rename_all = "snake_case")]
pub enum ProcessingMethod {
    QrCode,
    OcrSingle,
    OcrIterative,
    ManualEntry,
}

impl std::fmt::Display for ProcessingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingMethod::QrCode => write!(f, "qr_code"),
            ProcessingMethod::OcrSingle => write!(f, "ocr_single"),
            ProcessingMethod::OcrIterative => write!(f, "ocr_iterative"),
            ProcessingMethod::ManualEntry => write!(f, "manual_entry"),
        }
    }
}
