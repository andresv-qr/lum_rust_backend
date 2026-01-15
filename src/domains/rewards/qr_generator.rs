use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use image::{imageops, DynamicImage, ImageBuffer, Rgba};
use jsonwebtoken::{encode, decode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use qrcode::QrCode;
use rand::Rng;
use sha2::{Sha256, Digest};
use std::io::Cursor;
use uuid::Uuid;

/// Configuración del QR
pub struct QrConfig {
    /// Tamaño del QR en píxeles
    pub size: u32,
    /// Porcentaje del logo (0.0 - 1.0)
    pub logo_percentage: f32,
    /// Path del logo
    pub logo_path: String,
    /// Base URL para landing pages
    pub landing_base_url: String,
    /// Días hasta expiración del código
    pub expiration_days: i64,
    /// Segundos hasta expiración del token JWT
    pub token_expiration_seconds: i64,
}

impl Default for QrConfig {
    fn default() -> Self {
        Self {
            size: 800,
            logo_percentage: 0.15,
            logo_path: "assets/logoqr.png".to_string(),
            landing_base_url: "https://lumis.pa".to_string(),
            expiration_days: 30,
            // 30 días en segundos - el código es de UN SOLO USO, screenshot no es riesgo
            token_expiration_seconds: 2_592_000,
        }
    }
}

/// Generador de códigos QR con logo
pub struct QrGenerator {
    pub config: QrConfig,
}

impl QrGenerator {
    pub fn new(config: QrConfig) -> Self {
        Self { config }
    }

    /// Genera un código de redención único con alta entropía
    pub fn generate_redemption_code(&self) -> String {
        let mut rng = rand::thread_rng();
        
        // SEGURIDAD: Usar componentes completamente random para máxima entropía
        // 4 segmentos de 16 bits = 64 bits de entropía (vs 16 bits anterior)
        let hex1 = format!("{:04X}", rng.gen::<u16>());
        let hex2 = format!("{:04X}", rng.gen::<u16>());
        let hex3 = format!("{:04X}", rng.gen::<u16>());
        let hex4 = format!("{:04X}", rng.gen::<u16>());
        
        // Formato: LUMS-XXXX-XXXX-XXXX-XXXX (19 caracteres + prefijo)
        format!("LUMS-{}-{}-{}-{}", hex1, hex2, hex3, hex4)
    }

    /// Genera un código corto legible para humanos (ej: "K9P2X5")
    pub fn generate_short_code(&self) -> String {
        let mut rng = rand::thread_rng();
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Excluye I, 1, O, 0
        
        (0..6)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Genera QR code con logo overlay
    pub async fn generate_qr_with_logo(
        &self,
        redemption_code: &str,
        validation_token: &str,
    ) -> Result<Vec<u8>> {
        // 1. Construir URL del QR
        let qr_url = format!(
            "{}/r/{}?t={}",
            self.config.landing_base_url, redemption_code, validation_token
        );

        // 2. Generar QR base
        let qr = QrCode::new(qr_url.as_bytes())
            .context("Error al crear QR code")?;

        let qr_image = qr
            .render::<Rgba<u8>>()
            .max_dimensions(self.config.size, self.config.size)
            .build();

        // 3. Cargar y procesar logo
        let logo = self.load_and_prepare_logo(&qr_image)?;

        // 4. Overlay logo en centro
        let final_image = self.overlay_logo(qr_image, logo)?;

        // 5. Convertir a PNG bytes
        let mut buffer = Cursor::new(Vec::new());
        final_image
            .write_to(&mut buffer, image::ImageFormat::Png)
            .context("Error al escribir imagen PNG")?;

        Ok(buffer.into_inner())
    }

    /// Carga y prepara el logo (redimensiona y agrega margen blanco)
    fn load_and_prepare_logo(&self, qr_image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<DynamicImage> {
        // Cargar logo
        let logo = image::open(&self.config.logo_path)
            .context(format!("Error al cargar logo desde {}", self.config.logo_path))?;

        // Calcular tamaño del logo (porcentaje del QR)
        let logo_size = (qr_image.width() as f32 * self.config.logo_percentage) as u32;

        // Redimensionar logo con filtro de alta calidad
        let logo_resized = logo.resize(
            logo_size,
            logo_size,
            imageops::FilterType::Lanczos3,
        );

        // Crear margen blanco alrededor del logo (mejor legibilidad)
        let margin = (logo_size as f32 * 0.1) as u32;
        let logo_with_margin = self.add_white_margin(logo_resized, margin)?;

        Ok(logo_with_margin)
    }

    /// Agrega margen blanco alrededor de la imagen
    fn add_white_margin(&self, img: DynamicImage, margin: u32) -> Result<DynamicImage> {
        let new_width = img.width() + (margin * 2);
        let new_height = img.height() + (margin * 2);

        let mut canvas = ImageBuffer::from_pixel(new_width, new_height, Rgba([255, 255, 255, 255]));
        imageops::overlay(&mut canvas, &img.to_rgba8(), margin as i64, margin as i64);

        Ok(DynamicImage::ImageRgba8(canvas))
    }

    /// Superpone el logo en el centro del QR
    fn overlay_logo(
        &self,
        qr_image: ImageBuffer<Rgba<u8>, Vec<u8>>,
        logo: DynamicImage,
    ) -> Result<DynamicImage> {
        let mut final_image = qr_image.clone();

        // Calcular posición central
        let x_offset = (qr_image.width() - logo.width()) / 2;
        let y_offset = (qr_image.height() - logo.height()) / 2;

        // Overlay
        imageops::overlay(
            &mut final_image,
            &logo.to_rgba8(),
            x_offset as i64,
            y_offset as i64,
        );

        Ok(DynamicImage::ImageRgba8(final_image))
    }

    /// Calcula fecha de expiración del código
    pub fn calculate_code_expiration(&self) -> chrono::DateTime<Utc> {
        Utc::now() + Duration::days(self.config.expiration_days)
    }

    /// Calcula fecha de expiración del token de validación
    pub fn calculate_token_expiration(&self) -> chrono::DateTime<Utc> {
        Utc::now() + Duration::seconds(self.config.token_expiration_seconds)
    }

    /// Genera URL de landing page
    pub fn generate_landing_url(&self, redemption_code: &str, token: Option<&str>) -> String {
        match token {
            Some(t) => format!("{}/r/{}?t={}", self.config.landing_base_url, redemption_code, t),
            None => format!("{}/r/{}", self.config.landing_base_url, redemption_code),
        }
    }

    /// Genera un QR simple sin logo (fallback si no hay logo disponible)
    pub fn generate_qr_simple(&self, redemption_code: &str) -> Result<Vec<u8>> {
        let qr_url = format!("{}/r/{}", self.config.landing_base_url, redemption_code);
        
        let qr = QrCode::new(qr_url.as_bytes())
            .context("Error al crear QR code")?;

        let qr_image = qr
            .render::<Rgba<u8>>()
            .max_dimensions(self.config.size, self.config.size)
            .build();

        let mut buffer = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(qr_image)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .context("Error al escribir imagen PNG")?;

        Ok(buffer.into_inner())
    }

    /// Genera token JWT de validación para el QR
    pub fn generate_validation_token(
        &self,
        redemption_code: &str,
        user_id: i32,
        redemption_id: &Uuid,
    ) -> Result<String> {
        let secret = std::env::var("JWT_SECRET")
            .context("CRITICAL: JWT_SECRET environment variable must be set for QR token generation")?;
        
        let claims = ValidationTokenClaims::new(
            redemption_code.to_string(),
            user_id,
            self.config.token_expiration_seconds,
        );
        
        // Agregar redemption_id al jti para mayor trazabilidad
        let claims_with_rid = ValidationTokenClaimsExtended {
            redemption_code: claims.redemption_code,
            user_id: claims.user_id,
            redemption_id: *redemption_id,
            exp: claims.exp,
            jti: claims.jti,
        };
        
        encode(
            &Header::default(),
            &claims_with_rid,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .context("Error al generar token de validación")
    }

    /// Verifica un token de validación JWT
    pub fn verify_validation_token(&self, token: &str) -> Result<ValidationTokenClaimsExtended> {
        let secret = std::env::var("JWT_SECRET")
            .context("CRITICAL: JWT_SECRET environment variable must be set for token verification")?;
        
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        
        let token_data = decode::<ValidationTokenClaimsExtended>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .context("Token de validación inválido o expirado")?;
        
        Ok(token_data.claims)
    }

    /// Genera un hash del token para almacenar en DB (no el token completo)
    pub fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Claims del JWT de validación
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ValidationTokenClaims {
    pub redemption_code: String,
    pub user_id: i32,
    pub exp: i64,        // Timestamp de expiración
    pub jti: String,     // JWT ID único (previene replay)
}

/// Claims extendidos con redemption_id
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationTokenClaimsExtended {
    pub redemption_code: String,
    pub user_id: i32,
    pub redemption_id: Uuid,
    pub exp: i64,
    pub jti: String,
}

impl ValidationTokenClaims {
    pub fn new(redemption_code: String, user_id: i32, exp_seconds: i64) -> Self {
        Self {
            redemption_code,
            user_id,
            exp: (Utc::now() + Duration::seconds(exp_seconds)).timestamp(),
            jti: Uuid::new_v4().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_redemption_code() {
        let generator = QrGenerator::new(QrConfig::default());
        let code = generator.generate_redemption_code();
        
        // Debe tener formato LUMS-XXXX-XXXX-XXXX
        assert!(code.starts_with("LUMS-"));
        assert_eq!(code.len(), 19); // LUMS-XXXX-XXXX-XXXX
        
        // Debe tener 3 guiones
        assert_eq!(code.matches('-').count(), 3);
    }

    #[test]
    fn test_code_uniqueness() {
        let generator = QrGenerator::new(QrConfig::default());
        let code1 = generator.generate_redemption_code();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let code2 = generator.generate_redemption_code();
        
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_landing_url_generation() {
        let generator = QrGenerator::new(QrConfig::default());
        
        let url_without_token = generator.generate_landing_url("LUMS-A7F2-9K3X-B5Y1", None);
        assert_eq!(url_without_token, "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1");
        
        let url_with_token = generator.generate_landing_url("LUMS-A7F2-9K3X-B5Y1", Some("token123"));
        assert_eq!(url_with_token, "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1?t=token123");
    }

    #[test]
    fn test_validation_token_claims() {
        let claims = ValidationTokenClaims::new("LUMS-TEST".to_string(), 123, 60);
        
        assert_eq!(claims.redemption_code, "LUMS-TEST");
        assert_eq!(claims.user_id, 123);
        assert!(!claims.jti.is_empty());
        assert!(claims.exp > Utc::now().timestamp());
    }
}
