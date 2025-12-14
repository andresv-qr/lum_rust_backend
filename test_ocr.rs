// Test script to debug OCR processing without authentication
// Run with: cargo run --bin test_ocr <IMAGE_PATH>
//
// This tests the OCR LLM processing with OpenRouter models in cascade.
// Models used: qwen3-vl-8b -> qwen3-vl-30b -> qwen2.5-vl-72b
// All attempts are logged to PostgreSQL for traceability.

use std::path::Path;
use base64::{engine::general_purpose, Engine as _};
use sqlx::PgPool;
use sqlx::types::Decimal;
use chrono::Utc;
use std::str::FromStr;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
struct OcrProduct {
    name: String,
    quantity: f64,
    unit_price: f64,
    total_price: f64,
    #[serde(default)]
    partkey: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
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
    user_id: Option<i32>,
    image_path: String,
    image_size_bytes: i64,
    model_name: String,
    provider: String,
    success: bool,
    response_time_ms: i64,
    error_message: Option<String>,
    tokens_prompt: Option<i32>,
    tokens_completion: Option<i32>,
    tokens_total: Option<i32>,
    cost_prompt_usd: Option<Decimal>,
    cost_completion_usd: Option<Decimal>,
    cost_total_usd: Option<Decimal>,
    generation_id: Option<String>,
    model_used: Option<String>,
    finish_reason: Option<String>,
    extracted_fields: Option<serde_json::Value>,
    raw_response: Option<serde_json::Value>,
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
    log: &OcrAttemptLog,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO public.ocr_test_logs (
            user_id, image_path, image_size_bytes, model_name, provider,
            success, response_time_ms, error_message,
            tokens_prompt, tokens_completion, tokens_total,
            cost_prompt_usd, cost_completion_usd, cost_total_usd,
            generation_id, model_used, finish_reason,
            extracted_fields, raw_response, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
        "#,
        log.user_id,
        log.image_path,
        log.image_size_bytes,
        log.model_name,
        log.provider,
        log.success,
        log.response_time_ms,
        log.error_message,
        log.tokens_prompt,
        log.tokens_completion,
        log.tokens_total,
        log.cost_prompt_usd,
        log.cost_completion_usd,
        log.cost_total_usd,
        log.generation_id,
        log.model_used,
        log.finish_reason,
        log.extracted_fields,
        log.raw_response,
        Utc::now()
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn process_with_openrouter(
    image_bytes: &[u8],
    model: &str,
    pool: Option<&PgPool>,
    image_path: &str,
) -> anyhow::Result<OcrResponse> {
    let start_time = std::time::Instant::now();
    
    let openrouter_api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set in environment variables");

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
        "model": model,
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

    println!("📤 Calling OpenRouter API with model: {}", model);
    
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", openrouter_api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await;

    let response_time_ms = start_time.elapsed().as_millis() as i64;

    let response = match response {
        Ok(r) => r,
        Err(e) => {
            let error_msg = format!("Request error: {}", e);
            println!("❌ {}", error_msg);
            
            // Log failed attempt
            if let Some(pool) = pool {
                let log = OcrAttemptLog {
                    user_id: None,
                    image_path: image_path.to_string(),
                    image_size_bytes: image_bytes.len() as i64,
                    model_name: model.to_string(),
                    provider: "openrouter".to_string(),
                    success: false,
                    response_time_ms,
                    error_message: Some(error_msg.clone()),
                    tokens_prompt: None,
                    tokens_completion: None,
                    tokens_total: None,
                    cost_prompt_usd: None,
                    cost_completion_usd: None,
                    cost_total_usd: None,
                    generation_id: None,
                    model_used: None,
                    finish_reason: None,
                    extracted_fields: None,
                    raw_response: None,
                };
                log_ocr_attempt(pool, &log).await.ok();
            }
            
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    let status = response.status();
    println!("📥 Response status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        let error_msg = format!("OpenRouter API error: {}", error_text);
        
        // Log failed attempt
        if let Some(pool) = pool {
            let log = OcrAttemptLog {
                user_id: None,
                image_path: image_path.to_string(),
                image_size_bytes: image_bytes.len() as i64,
                model_name: model.to_string(),
                provider: "openrouter".to_string(),
                success: false,
                response_time_ms,
                error_message: Some(error_msg.clone()),
                tokens_prompt: None,
                tokens_completion: None,
                tokens_total: None,
                cost_prompt_usd: None,
                cost_completion_usd: None,
                cost_total_usd: None,
                generation_id: None,
                model_used: None,
                finish_reason: None,
                extracted_fields: None,
                raw_response: None,
            };
            log_ocr_attempt(pool, &log).await.ok();
        }
        
        return Err(anyhow::anyhow!(error_msg));
    }

    let response_text = response.text().await?;
    
    // Save raw response for debugging
    std::fs::write(format!("/tmp/ocr_{}_response.json", model.replace("/", "_")), &response_text).ok();
    println!("💾 Raw response saved to /tmp/ocr_{}_response.json", model.replace("/", "_"));

    let json_response: serde_json::Value = serde_json::from_str(&response_text)?;
    
    // Extract detailed token usage from OpenRouter response
    let tokens_prompt = json_response["usage"]["prompt_tokens"].as_i64().map(|t| t as i32);
    let tokens_completion = json_response["usage"]["completion_tokens"].as_i64().map(|t| t as i32);
    let tokens_total = json_response["usage"]["total_tokens"].as_i64().map(|t| t as i32);
    
    // Extract cost information (OpenRouter specific)
    // OpenRouter provides: usage.cost (total), usage.cost_details.upstream_inference_prompt_cost, usage.cost_details.upstream_inference_completions_cost
    let cost_total_usd = json_response["usage"]["cost"].as_f64()
        .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
    let cost_prompt_usd = json_response["usage"]["cost_details"]["upstream_inference_prompt_cost"].as_f64()
        .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
    let cost_completion_usd = json_response["usage"]["cost_details"]["upstream_inference_completions_cost"].as_f64()
        .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
    
    // Extract metadata
    let generation_id = json_response["id"].as_str().map(|s| s.to_string());
    let model_used = json_response["model"].as_str().map(|s| s.to_string());
    let finish_reason = json_response["choices"][0]["finish_reason"].as_str().map(|s| s.to_string());
    
    // Log token usage
    if let Some(tokens) = tokens_total {
        println!("🎫 Tokens: {} total (prompt: {}, completion: {})", 
                 tokens, 
                 tokens_prompt.unwrap_or(0), 
                 tokens_completion.unwrap_or(0));
    }
    if let Some(cost) = &cost_total_usd {
        println!("💰 Cost: ${} USD (prompt: ${}, completion: ${})", 
                 cost,
                 cost_prompt_usd.as_ref().map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()),
                 cost_completion_usd.as_ref().map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()));
    }
    
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

    let ocr_response: OcrResponse = match serde_json::from_str(&cleaned_text) {
        Ok(r) => r,
        Err(e) => {
            let error_msg = format!("Error parsing OCR JSON: {} - Text: {}", e, cleaned_text);
            
            // Log failed attempt
            if let Some(pool) = pool {
                let log = OcrAttemptLog {
                    user_id: None,
                    image_path: image_path.to_string(),
                    image_size_bytes: image_bytes.len() as i64,
                    model_name: model.to_string(),
                    provider: "openrouter".to_string(),
                    success: false,
                    response_time_ms,
                    error_message: Some(error_msg.clone()),
                    tokens_prompt,
                    tokens_completion,
                    tokens_total,
                    cost_prompt_usd,
                    cost_completion_usd,
                    cost_total_usd,
                    generation_id: generation_id.clone(),
                    model_used: model_used.clone(),
                    finish_reason: finish_reason.clone(),
                    extracted_fields: None,
                    raw_response: Some(json_response.clone()),
                };
                log_ocr_attempt(pool, &log).await.ok();
            }
            
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    // Log successful attempt
    if let Some(pool) = pool {
        let extracted_json = serde_json::to_value(&ocr_response).ok();
        let log = OcrAttemptLog {
            user_id: None,
            image_path: image_path.to_string(),
            image_size_bytes: image_bytes.len() as i64,
            model_name: model.to_string(),
            provider: "openrouter".to_string(),
            success: true,
            response_time_ms,
            error_message: None,
            tokens_prompt,
            tokens_completion,
            tokens_total,
            cost_prompt_usd,
            cost_completion_usd,
            cost_total_usd,
            generation_id,
            model_used,
            finish_reason,
            extracted_fields: extracted_json,
            raw_response: Some(json_response),
        };
        log_ocr_attempt(pool, &log).await.ok();
    }

    Ok(ocr_response)
}

fn print_ocr_results(ocr_data: &OcrResponse) {
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

    println!("🔍 Testing OCR with OpenRouter Models in Cascade");
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

    // Connect to database for logging
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = match PgPool::connect(&database_url).await {
        Ok(p) => {
            println!("✅ Connected to database for logging");
            println!();
            Some(p)
        }
        Err(e) => {
            println!("⚠️  Could not connect to database: {} - Continuing without logging", e);
            println!();
            None
        }
    };

    // Models to try in cascade
    let models = vec![
        ("qwen/qwen3-vl-8b-instruct", "QWEN3-VL-8B (Primary)"),
        ("qwen/qwen3-vl-30b-a3b-instruct", "QWEN3-VL-30B (Fallback 1)"),
        ("qwen/qwen2.5-vl-72b-instruct", "QWEN2.5-VL-72B (Fallback 2)"),
    ];

    let mut success = false;
    let mut ocr_result = None;

    for (i, (model, label)) in models.iter().enumerate() {
        println!("{}", "=".repeat(80));
        println!("TEST {}: {} - {}", i + 1, label, model);
        println!("{}", "=".repeat(80));
        println!();

        match process_with_openrouter(&image_bytes, model, pool.as_ref(), &image_path).await {
            Ok(ocr_data) => {
                print_ocr_results(&ocr_data);
                success = true;
                ocr_result = Some(ocr_data);
                break; // Success! No need to try other models
            }
            Err(e) => {
                println!("❌ OCR FAILED: {}", e);
                if i < models.len() - 1 {
                    println!("\n⚠️  Trying next model in cascade...\n");
                }
            }
        }

        println!();
        println!();
    }

    // Final summary
    println!("{}", "=".repeat(80));
    println!("FINAL RESULT");
    println!("{}", "=".repeat(80));
    
    if success {
        println!("✅ OCR completed successfully!");
        if let Some(data) = ocr_result {
            println!("\n📊 Summary:");
            println!("  - Issuer: {}", data.issuer_name.as_deref().unwrap_or("N/A"));
            println!("  - Invoice: {}", data.invoice_number.as_deref().unwrap_or("N/A"));
            println!("  - Total: ${:.2}", data.total.unwrap_or(0.0));
            println!("  - Products: {} items", data.products.len());
        }
    } else {
        println!("❌ All OCR attempts failed!");
        println!("   Tried all {} models in cascade.", models.len());
    }
    
    if pool.is_some() {
        println!("\n💾 All attempts have been logged to PostgreSQL table: public.ocr_test_logs");
    }
    
    println!();
}
