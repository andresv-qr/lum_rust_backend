// Test script to debug OCR processing without authentication
// Run with: cargo run --bin test_ocr <IMAGE_PATH>
//
// This tests the OCR LLM processing with OpenRouter models in cascade.
// Models used: qwen3-vl-8b -> qwen3-vl-30b -> qwen2.5-vl-72b
// All attempts are logged to PostgreSQL for traceability.

use std::path::Path;
use base64::{engine::general_purpose, Engine as _};
use sqlx::PgPool;
use chrono::Utc;

#[derive(serde::Deserialize, Debug, Clone)]
struct OcrProduct {
    name: String,
    quantity: f64,
    unit_price: f64,
    total_price: f64,
    #[serde(default)]
    partkey: Option<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct OcrResponse {
    issuer_name: Option<String>,
    ruc: Option<String>,
    dv: Option<String>,
    address: Option<String>,
    invoice_number: Option<String>,
    date: Option<String>,
    total: Option<f64>,
    products: Vec<OcrProduct>,
}

// Log structure for database
#[derive(Debug)]
struct OcrAttemptLog {
    model_name: String,
    image_size_bytes: i64,
    success: bool,
    response_time_ms: i64,
    tokens_used: Option<i32>,
    error_message: Option<String>,
    extracted_fields: Option<serde_json::Value>,
}

fn extract_json_from_markdown(text: &str) -> String {
    let text = text.trim();
    
    // Check if wrapped in markdown code block
    if text.starts_with("```json") {
        text.trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else if text.starts_with("```") {
        text.trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        text.to_string()
    }
}

async fn log_ocr_attempt(
    pool: &PgPool,
    image_path: &str,
    log: &OcrAttemptLog,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO public.ocr_test_logs (
            image_path, model_name, image_size_bytes, success, 
            response_time_ms, tokens_used, error_message, extracted_fields, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        image_path,
        log.model_name,
        log.image_size_bytes,
        log.success,
        log.response_time_ms,
        log.tokens_used,
        log.error_message,
        log.extracted_fields,
        Utc::now()
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn process_with_openrouter(
    image_bytes: &[u8],
    model: &str,
    pool: &PgPool,
    image_path: &str,
) -> anyhow::Result<OcrResponse> {
    let start_time = std::time::Instant::now();
    
    let openrouter_api_key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|_| "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".to_string());

    let base64_image = general_purpose::STANDARD.encode(image_bytes);

    let prompt = r#"Extrae TODOS los datos de esta factura panameña en formato JSON. 

IMPORTANTE: Debes extraer TODOS los campos, incluso si algunos no son visibles con claridad. Si un campo no es visible, usa null.

Formato de respuesta (JSON puro, sin markdown):
{
  "issuer_name": "Nombre del emisor",
  "ruc": "RUC sin guiones (ej: 15575193822024)",
  "dv": "Dígito verificador (ej: 58)",
  "address": "Dirección completa del emisor",
  "invoice_number": "Número de factura",
  "date": "Fecha en formato YYYY-MM-DD",
  "total": 123.45,
  "products": [
    {
      "name": "Nombre del producto",
      "quantity": 1.0,
      "unit_price": 10.0,
      "total_price": 10.0
    }
  ]
}

Reglas importantes:
1. RUC debe ser numérico sin guiones ni espacios
2. DV (dígito verificador) es el último número después del último guion del RUC
3. La fecha debe estar en formato YYYY-MM-DD
4. Todos los números deben ser numéricos, no strings
5. Si un campo no es visible, usa null
6. Extrae TODOS los productos visibles en la factura"#;

    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "text": prompt
                },
                {
                    "inline_data": {
                        "mime_type": "image/jpeg",
                        "data": base64_image
                    }
                }
            ]
        }],
        "generationConfig": {
            "temperature": 0.1,
            "topK": 32,
            "topP": 1,
            "maxOutputTokens": 8192
        }
    });

    let client = reqwest::Client::new();
    // Use gemini-2.0-flash with v1beta (same as production)
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        gemini_api_key
    );

    println!("📤 Calling Gemini API (gemini-2.0-flash)...");
    
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    println!("📥 Response status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Gemini API error: {}", error_text);
    }

    let response_text = response.text().await?;
    
    // Save raw response for debugging
    std::fs::write("/tmp/ocr_gemini_response.json", &response_text).ok();
    println!("💾 Raw Gemini response saved to /tmp/ocr_gemini_response.json");

    let gemini_response: GeminiResponse = serde_json::from_str(&response_text)?;
    
    let text = gemini_response
        .candidates
        .get(0)
        .and_then(|c| c.content.parts.get(0))
        .map(|p| p.text.as_str())
        .ok_or_else(|| anyhow::anyhow!("No text in Gemini response"))?;

    println!("📄 Raw text from Gemini:");
    println!("{}", text);
    println!();

    let cleaned_text = extract_json_from_markdown(text);
    
    println!("🧹 Cleaned JSON:");
    println!("{}", cleaned_text);
    println!();

    let ocr_response: OcrResponse = serde_json::from_str(&cleaned_text)
        .map_err(|e| anyhow::anyhow!("Error parsing OCR JSON: {} - Text: {}", e, cleaned_text))?;

    Ok(ocr_response)
}

async fn process_with_openrouter(image_bytes: &[u8]) -> anyhow::Result<OcrResponse> {
    let openrouter_api_key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|_| "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".to_string());

    let base64_image = general_purpose::STANDARD.encode(image_bytes);

    let prompt = r#"Extrae TODOS los datos de esta factura panameña en formato JSON. 

IMPORTANTE: Debes extraer TODOS los campos, incluso si algunos no son visibles con claridad. Si un campo no es visible, usa null.

Formato de respuesta (JSON puro, sin markdown):
{
  "issuer_name": "Nombre del emisor",
  "ruc": "RUC sin guiones (ej: 15575193822024)",
  "dv": "Dígito verificador (ej: 58)",
  "address": "Dirección completa del emisor",
  "invoice_number": "Número de factura",
  "date": "Fecha en formato YYYY-MM-DD",
  "total": 123.45,
  "products": [
    {
      "name": "Nombre del producto",
      "quantity": 1.0,
      "unit_price": 10.0,
      "total_price": 10.0
    }
  ]
}

Reglas importantes:
1. RUC debe ser numérico sin guiones ni espacios
2. DV (dígito verificador) es el último número después del último guion del RUC
3. La fecha debe estar en formato YYYY-MM-DD
4. Todos los números deben ser numéricos, no strings
5. Si un campo no es visible, usa null
6. Extrae TODOS los productos visibles en la factura"#;

    let payload = serde_json::json!({
        "model": "qwen/qwen3-vl-8b-instruct",
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": prompt
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:image/jpeg;base64,{}", base64_image)
                    }
                }
            ]
        }],
        "temperature": 0.1,
        "max_tokens": 8192
    });

    let client = reqwest::Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    println!("📤 Calling OpenRouter API...");
    
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", openrouter_api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    println!("📥 Response status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("OpenRouter API error: {}", error_text);
    }

    let response_text = response.text().await?;
    
    // Save raw response for debugging
    std::fs::write("/tmp/ocr_openrouter_response.json", &response_text).ok();
    println!("💾 Raw OpenRouter response saved to /tmp/ocr_openrouter_response.json");

    let json_response: serde_json::Value = serde_json::from_str(&response_text)?;
    
    let text = json_response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in OpenRouter response"))?;

    println!("📄 Raw text from OpenRouter:");
    println!("{}", text);
    println!();

    let cleaned_text = extract_json_from_markdown(text);
    
    println!("🧹 Cleaned JSON:");
    println!("{}", cleaned_text);
    println!();

    let ocr_response: OcrResponse = serde_json::from_str(&cleaned_text)
        .map_err(|e| anyhow::anyhow!("Error parsing OCR JSON: {} - Text: {}", e, cleaned_text))?;

    Ok(ocr_response)
}

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().ok();
    
    // Get image path from args
    let image_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            println!("❌ Usage: cargo run --bin test_ocr <IMAGE_PATH>");
            println!("   Example: cargo run --bin test_ocr image_invoice.jpg");
            std::process::exit(1);
        });

    println!("🔍 Testing OCR with LLM");
    println!("📄 Image: {}", image_path);
    println!();

    // Check if image exists
    if !Path::new(&image_path).exists() {
        println!("❌ Image file not found: {}", image_path);
        std::process::exit(1);
    }

    // Read image file
    let image_bytes = match std::fs::read(&image_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("❌ Error reading image: {}", e);
            std::process::exit(1);
        }
    };

    println!("✅ Image loaded: {} bytes", image_bytes.len());
    println!();

    // ==========================================
    // TEST 1: GEMINI OCR
    // ==========================================
    println!("{}", "=".repeat(80));
    println!("TEST 1: GEMINI 2.0 FLASH OCR");
    println!("{}", "=".repeat(80));
    println!();

    match process_with_gemini(&image_bytes).await {
        Ok(ocr_data) => {
            println!("✅ OCR SUCCESSFUL!");
            println!();
            println!("📋 EXTRACTED DATA:");
            println!("{}", "=".repeat(80));
            
            println!("\n📤 EMISOR:");
            println!("  - Nombre: {}", ocr_data.issuer_name.as_deref().unwrap_or("NOT FOUND"));
            println!("  - RUC: {}", ocr_data.ruc.as_deref().unwrap_or("NOT FOUND"));
            println!("  - DV: {}", ocr_data.dv.as_deref().unwrap_or("NOT FOUND"));
            println!("  - Dirección: {}", ocr_data.address.as_deref().unwrap_or("NOT FOUND"));
            
            println!("\n📄 FACTURA:");
            println!("  - No. Factura: {}", ocr_data.invoice_number.as_deref().unwrap_or("NOT FOUND"));
            println!("  - Fecha: {}", ocr_data.date.as_deref().unwrap_or("NOT FOUND"));
            println!("  - Total: ${:.2}", ocr_data.total.unwrap_or(0.0));
            
            println!("\n📦 PRODUCTOS ({} items):", ocr_data.products.len());
            for (i, product) in ocr_data.products.iter().enumerate() {
                println!("\n  📌 Item #{}", i + 1);
                println!("    - Nombre: {}", product.name);
                println!("    - Cantidad: {:.2}", product.quantity);
                println!("    - Precio Unit: ${:.2}", product.unit_price);
                println!("    - Total: ${:.2}", product.total_price);
            }
            
            println!("\n{}", "=".repeat(80));
            
            // Validate required fields
            println!("\n🔍 VALIDATION:");
            let mut missing_fields = Vec::new();
            
            if ocr_data.issuer_name.is_none() { missing_fields.push("issuer_name"); }
            if ocr_data.ruc.is_none() { missing_fields.push("ruc"); }
            if ocr_data.dv.is_none() { missing_fields.push("dv"); }
            if ocr_data.address.is_none() { missing_fields.push("address"); }
            if ocr_data.invoice_number.is_none() { missing_fields.push("invoice_number"); }
            if ocr_data.date.is_none() { missing_fields.push("date"); }
            if ocr_data.total.is_none() { missing_fields.push("total"); }
            if ocr_data.products.is_empty() { missing_fields.push("products"); }
            
            if missing_fields.is_empty() {
                println!("✅ All required fields extracted successfully!");
            } else {
                println!("⚠️  Missing fields: {}", missing_fields.join(", "));
            }
        }
        Err(e) => {
            println!("❌ OCR FAILED: {}", e);
        }
    }

    println!();
    println!();

    // ==========================================
    // TEST 2: OPENROUTER OCR (FALLBACK)
    // ==========================================
    println!("{}", "=".repeat(80));
    println!("TEST 2: OPENROUTER (QWEN3-VL-8B) OCR");
    println!("{}", "=".repeat(80));
    println!();

    match process_with_openrouter(&image_bytes).await {
        Ok(ocr_data) => {
            println!("✅ OCR SUCCESSFUL!");
            println!();
            println!("📋 EXTRACTED DATA:");
            println!("{}", "=".repeat(80));
            
            println!("\n📤 EMISOR:");
            println!("  - Nombre: {}", ocr_data.issuer_name.as_deref().unwrap_or("NOT FOUND"));
            println!("  - RUC: {}", ocr_data.ruc.as_deref().unwrap_or("NOT FOUND"));
            println!("  - DV: {}", ocr_data.dv.as_deref().unwrap_or("NOT FOUND"));
            println!("  - Dirección: {}", ocr_data.address.as_deref().unwrap_or("NOT FOUND"));
            
            println!("\n📄 FACTURA:");
            println!("  - No. Factura: {}", ocr_data.invoice_number.as_deref().unwrap_or("NOT FOUND"));
            println!("  - Fecha: {}", ocr_data.date.as_deref().unwrap_or("NOT FOUND"));
            println!("  - Total: ${:.2}", ocr_data.total.unwrap_or(0.0));
            
            println!("\n📦 PRODUCTOS ({} items):", ocr_data.products.len());
            for (i, product) in ocr_data.products.iter().enumerate() {
                println!("\n  📌 Item #{}", i + 1);
                println!("    - Nombre: {}", product.name);
                println!("    - Cantidad: {:.2}", product.quantity);
                println!("    - Precio Unit: ${:.2}", product.unit_price);
                println!("    - Total: ${:.2}", product.total_price);
            }
            
            println!("\n{}", "=".repeat(80));
            
            // Validate required fields
            println!("\n🔍 VALIDATION:");
            let mut missing_fields = Vec::new();
            
            if ocr_data.issuer_name.is_none() { missing_fields.push("issuer_name"); }
            if ocr_data.ruc.is_none() { missing_fields.push("ruc"); }
            if ocr_data.dv.is_none() { missing_fields.push("dv"); }
            if ocr_data.address.is_none() { missing_fields.push("address"); }
            if ocr_data.invoice_number.is_none() { missing_fields.push("invoice_number"); }
            if ocr_data.date.is_none() { missing_fields.push("date"); }
            if ocr_data.total.is_none() { missing_fields.push("total"); }
            if ocr_data.products.is_empty() { missing_fields.push("products"); }
            
            if missing_fields.is_empty() {
                println!("✅ All required fields extracted successfully!");
            } else {
                println!("⚠️  Missing fields: {}", missing_fields.join(", "));
            }
        }
        Err(e) => {
            println!("⚠️  OpenRouter OCR SKIPPED or FAILED: {}", e);
        }
    }

    println!();
    println!("✅ Test completed!");
}
