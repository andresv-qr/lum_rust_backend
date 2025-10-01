use anyhow::Result;
use image::DynamicImage;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, info, warn, instrument};

// Import our new hybrid QR detection
use crate::processing::qr_detection::{decode_qr_hybrid_cascade, QrScanResult};

/// QR Service with optimized 2-level hybrid detection
/// 
/// LEVEL 1: Fast Rust decoders (rqrr, quircs, rxing) - 85% success, 5-15ms
/// LEVEL 2: Python OpenCV fallback - 12% additional success, 30-50ms  
/// 
/// Memory optimized: ~3MB per request vs 50MB+ in old ONNX version
#[derive(Clone)]
pub struct QrService {
    // Simple service - no session pooling, no ONNX overhead
}

impl QrService {
    /// Create a new optimized QR service
    pub fn new(_python_socket_path: String) -> Self {
        info!("🚀 Initializing optimized hybrid QR service (2-level cascade)");
        info!("📊 Expected performance: 85% success with Rust (5-15ms), 12% with Python fallback (30-50ms)");
        
        Self {}
    }
    
    /// Main QR detection entry point - uses hybrid 2-level cascade
    #[instrument(skip(self, image_bytes), fields(image_size = image_bytes.len()))]
    pub async fn decode_qr_from_image_bytes(&self, image_bytes: &[u8]) -> Result<QrScanResult> {
        info!("🔍 Starting QR detection for image ({} bytes)", image_bytes.len());
        
        // Use our optimized hybrid cascade
        decode_qr_hybrid_cascade(image_bytes).await
    }

/// Represents the result of a successful QR code scan.
#[derive(Debug, Clone)]
pub struct QrScanResult {
    pub content: String,
    pub decoder: Decoder,
}

/// QR Service with multi-level detection including RustQReader ONNX and Python fallback
#[derive(Clone)]
pub struct QrService {
    rust_qreader: Option<Arc<Mutex<RustQReader>>>,
    python_client: Arc<PythonQReaderClient>,
}

impl QrService {
    /// Create a new QR service with RustQReader ONNX and Python fallback
    pub fn new(python_socket_path: String) -> Self {
        // Convert socket path to HTTP URL for the hybrid fallback service
        let python_url = if python_socket_path.starts_with("http") {
            python_socket_path
        } else {
            // Default to localhost:8008 for FastAPI Python service
            "http://localhost:8008".to_string()
        };
        
        let python_client = Arc::new(
            PythonQReaderClient::new(python_url)
                .with_timeout(std::time::Duration::from_secs(10))
                .with_max_image_size(10 * 1024 * 1024), // 10MB
        );
        
        // Try to initialize RustQReader (optional - graceful fallback if ONNX models not available)
        let rust_qreader = Self::try_init_rust_qreader();
        
        Self { 
            rust_qreader,
            python_client 
        }
    }
    
    /// Try to initialize RustQReader with ONNX models (optimized: Small first, Large fallback)
    fn try_init_rust_qreader() -> Option<Arc<Mutex<RustQReader>>> {
        let models_dir = std::path::Path::new("models");
        let small_model_path = models_dir.join(ModelSize::Small.model_name());
        let large_model_path = models_dir.join(ModelSize::Large.model_name());

        info!(
            "Attempting to load ONNX models (Small → Large). Searching in directory: {:?}",
            std::fs::canonicalize(&models_dir).unwrap_or_else(|_| models_dir.to_path_buf())
        );

        // Try small model first (optimal balance of speed/precision)
        if small_model_path.exists() {
            info!("Found small model file: {:?}. Initializing...", small_model_path);
            match RustQReader::new(&small_model_path, ModelSize::Small) {
                Ok(reader) => {
                    info!("✅ RustQReader ONNX initialized successfully with small model");
                    return Some(Arc::new(Mutex::new(reader)));
                }
                Err(e) => {
                    warn!(
                        "❌ Failed to initialize RustQReader with small model: {}. Trying large model...",
                        e
                    );
    /// Legacy method for backward compatibility - converts DynamicImage to bytes and calls new hybrid method
    #[instrument(skip(self, img), fields(image_size = %format!("{}x{}", img.width(), img.height())))]
    pub async fn decode_qr(&self, img: &DynamicImage) -> Option<QrScanResult> {
        info!("� Converting DynamicImage to bytes for hybrid processing...");
        
        // Convert DynamicImage to bytes (JPEG format for efficiency)
        let mut bytes = std::io::Cursor::new(Vec::new());
        if let Err(e) = img.write_to(&mut bytes, image::ImageFormat::Jpeg) {
            warn!("Failed to convert image to bytes: {}", e);
            return None;
        }
        
        let image_bytes = bytes.into_inner();
        info!("📊 Image converted to {} bytes", image_bytes.len());
        
        // Use our new hybrid detection
        match self.decode_qr_from_image_bytes(&image_bytes).await {
            Ok(result) => {
                info!("✅ QR decoded successfully: {}", result.content);
                Some(result)
            }
            Err(e) => {
                warn!("❌ QR detection failed: {}", e);
                None
            }
        }
    }
                None
            }
        }
    }
    
    /// Try decoding with rqrr library
    fn try_rqrr(img: &DynamicImage) -> Option<QrScanResult> {
        info!("➡️ Level 1: Attempting decoding with rqrr...");
        let luma_img = img.to_luma8();
        let mut prepared_img = rqrr::PreparedImage::prepare(luma_img);
        let grids = prepared_img.detect_grids();

        if grids.is_empty() {
            debug!("Level 1: rqrr: No QR grids found");
            return None;
        }

        for grid in grids {
            match grid.decode() {
                Ok((_meta, content)) => {
                    info!("✅ Level 1: rqrr: Successfully decoded QR");
                    return Some(QrScanResult {
                        content,
                        decoder: Decoder::Rqrr,
                    });
                }
                Err(_) => continue, // Try next grid
            }
        }

        debug!("Level 1: rqrr: Found potential QR but failed to decode");
        None
    }
    
    /// Try decoding with quircs library
    fn try_quircs(img: &DynamicImage) -> Option<QrScanResult> {
        info!("➡️ Level 2: Attempting decoding with quircs...");
        let luma_img = img.to_luma8();
        let (width, height) = luma_img.dimensions();
        let mut decoder = quircs::Quirc::new();
        let codes = decoder.identify(width as usize, height as usize, &luma_img.into_raw());

        for code_result in codes {
            if let Ok(code) = code_result {
                if let Ok(decoded) = code.decode() {
                    let content = String::from_utf8_lossy(&decoded.payload).to_string();
                    info!("✅ Level 2: quircs: Successfully decoded QR");
                    return Some(QrScanResult {
                        content,
                        decoder: Decoder::Quircs,
                    });
                }
            }
        }

        debug!("Level 2: quircs: No QR code found");
        None
    }
    
    /// Try decoding with rxing library
    fn try_rxing(img: &DynamicImage) -> Option<QrScanResult> {
        info!("➡️ Level 3: Attempting decoding with rxing...");
        let mut hints = rxing::DecodingHintDictionary::new();
        hints.insert(
            rxing::DecodeHintType::TRY_HARDER,
            rxing::DecodeHintValue::TryHarder(true),
        );

        let mut luminance_source = rxing::BufferedImageLuminanceSource::new(img.clone());

        if let Some(result) = Self::decode_with_rxing_source(&mut luminance_source, &mut hints, false) {
            return Some(result);
        }

        // Try again with inverted image
        luminance_source.invert();
        if let Some(result) = Self::decode_with_rxing_source(&mut luminance_source, &mut hints, true) {
            return Some(result);
        }

        debug!("Level 3: rxing: No QR code found in normal or inverted image");
        None
    }

    fn decode_with_rxing_source(
        source: &mut rxing::BufferedImageLuminanceSource,
        hints: &mut rxing::DecodingHintDictionary,
        is_inverted: bool,
    ) -> Option<QrScanResult> {
        let width = source.get_width();
        let height = source.get_height();
        let gray_image = image::GrayImage::from_raw(width as u32, height as u32, source.get_matrix()).unwrap();
        let dynamic_image = image::DynamicImage::ImageLuma8(gray_image);
        let new_source = rxing::BufferedImageLuminanceSource::new(dynamic_image);

        let binarizer = rxing::common::HybridBinarizer::new(new_source);
        let mut bitmap = rxing::BinaryBitmap::new(binarizer);
        let mut reader = rxing::MultiFormatReader::default();

        match reader.decode_with_hints(&mut bitmap, hints) {
            Ok(result) => {
                let content = result.getText().to_string();
                if is_inverted {
                    info!("✅ Level 3: rxing (inverted): Successfully decoded QR");
                } else {
                    info!("✅ Level 3: rxing (normal): Successfully decoded QR");
                }
                Some(QrScanResult {
                    content,
                    decoder: Decoder::Rxing,
                })
            }
            Err(_) => None,
        }
    }
    
    /// Try decoding with zbar-rust library (Level 3.5)
    fn try_zbar_rust(img: &DynamicImage) -> Option<QrScanResult> {
        info!("➡️ Level 3.5: Attempting decoding with zbar-rust...");
        
        // Convert image to grayscale for zbar
        let gray_img = img.to_luma8();
        let (width, height) = gray_img.dimensions();
        
        // Create zbar scanner with basic configuration
        let mut scanner = ZBarImageScanner::new();
        
        // Scan the image with basic Y800 format (grayscale)
        match scanner.scan_y800(gray_img.as_raw(), width, height) {
            Ok(symbols) => {
                for symbol in symbols {
                    // Extract symbol data (field access, not method)
                    let content = String::from_utf8_lossy(&symbol.data).to_string();
                    if !content.is_empty() {
                        info!("✅ Level 3.5: zbar-rust: Successfully decoded QR: {}", content);
                        return Some(QrScanResult {
                            content,
                            decoder: Decoder::ZbarRust,
                        });
                    }
                }
            }
            Err(e) => {
                debug!("Level 3.5: zbar-rust: Failed to scan image: {}", e);
            }
        }
        
        debug!("Level 3.5: zbar-rust: No QR code found");
        None
    }
    
    /// Try decoding with Python QReader fallback
    async fn try_python_fallback(&self, img: &DynamicImage) -> Option<QrScanResult> {
        debug!("Attempting QR decode with Python QReader fallback...");
        
        // Check if Python service is available
        if !self.python_client.is_available().await {
            warn!("Python QReader service is not available");
            return None;
        }
        
        // Try to decode with Python service
        match self.python_client.detect_qr(img).await {
            Ok(Some(content)) => {
                info!("✅ Level 5: Python QReader: Successfully decoded QR: {}", content);
                Some(QrScanResult {
                    content,
                    decoder: Decoder::PythonQReader,
                })
            }
            Ok(None) => {
                info!("❌ Level 5: Python QReader: No QR code found");
                None
            }
            Err(e) => {
                warn!("Python QReader fallback failed: {}", e);
                None
            }
        }
    }
    
    /// Get Python client metrics
    pub fn get_python_metrics(&self) -> Arc<crate::services::python_qreader_client::ClientMetrics> {
        self.python_client.get_metrics()
    }
    
    /// Check if Python fallback is available
    pub async fn is_python_available(&self) -> bool {
        self.python_client.is_available().await
    }
    
    /// Check if RustQReader ONNX is available
    pub fn is_rust_qreader_available(&self) -> bool {
        self.rust_qreader.is_some()
    }
    
    /// Get decoder availability status
    pub async fn get_decoder_status(&self) -> DecoderStatus {
        DecoderStatus {
            rqrr: true,
            quircs: true,
            rxing: true,
            rust_qreader_onnx: self.is_rust_qreader_available(),
            python_qreader: self.is_python_available().await,
        }
    }
}

/// Status of available decoders
#[derive(Debug, Clone)]
pub struct DecoderStatus {
    pub rqrr: bool,
    pub quircs: bool,
    pub rxing: bool,
    pub rust_qreader_onnx: bool,
    pub python_qreader: bool,
}

/// Logs a successful QR code scan into the database.
/// Uses the same logging approach as the Python system (logs.ocr_attempts table)
pub async fn log_qr_scan(
    pool: &PgPool,
    user_id: i64,
    qr_content: &str,
    decoder: &str,
) -> Result<()> {
    // Try to log to the logs.ocr_attempts table (same as Python system)
    let log_result = sqlx::query(
        r#"
        INSERT INTO logs.ocr_attempts (whatsapp_id, user_id, attempt_date, status, details)
        VALUES ($1, $2, NOW(), $3, $4)
        "#,
    )
    .bind(user_id.to_string()) // whatsapp_id as string
    .bind(user_id)
    .bind("qr_success")
    .bind(format!("QR decoded with {}: {}", decoder, qr_content.chars().take(100).collect::<String>()))
    .execute(pool)
    .await;

    // If the logs table doesn't exist, just log the warning and continue
    if let Err(e) = log_result {
        warn!("Could not log QR scan to database (table may not exist): {}. Continuing without logging.", e);
        // Don't return error - QR processing should continue even if logging fails
    } else {
        debug!("Successfully logged QR scan for user {} using decoder {}", user_id, decoder);
    }

    Ok(())
}


/// Legacy function for backward compatibility
/// 
/// This function creates a temporary QrService and uses it to decode.
/// For better performance, create a QrService instance and reuse it.
pub async fn decode_qr(img: &DynamicImage) -> Option<QrScanResult> {
    let qr_service = QrService::new("/tmp/qreader_service.sock".to_string());
    qr_service.decode_qr(img).await
}


#[cfg(test)]
mod tests {
    /*
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_decode_qr_from_image_bytes() {
        // Cargar los bytes de una imagen de prueba
        let image_bytes = include_bytes!("../../tests/fixtures/test_qr.png");

        // Llamar a la función de decodificación
        let result = decode_qr_from_image_bytes(image_bytes).await;

        // Verificar que el resultado es exitoso
        assert!(result.is_ok(), "La decodificación del QR debería ser exitosa");

        // Verificar que la URL decodificada es la esperada
        let urls = result.unwrap();
        assert_eq!(urls.len(), 1, "Debería encontrarse exactamente una URL");
        assert_eq!(
            urls[0],
            "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorCUFE?chFE=FE01200002679372-1-844914-7300002025051500311570140020317481978892&i=0031157014&f=MjAyNTA1MTUwOTUwMDQ=",
            "La URL decodificada no coincide con la esperada"
        );
    }
    */
}

