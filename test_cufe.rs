// Test script to debug CUFE API without authentication
// Run with: cargo run --bin test_cufe [CUFE]
//
// This tests the DGI API directly and extracts invoice data from the HTML response.

use scraper::{Html, Selector};
use std::collections::HashMap;
use regex::Regex;

#[derive(serde::Deserialize, Debug)]
struct DgiCufeResponse {
    #[serde(rename = "CUFE")]
    cufe: Option<String>,
    #[serde(rename = "FacturaHTML")]
    factura_html: Option<String>,
    #[serde(rename = "Error")]
    error: Option<String>,
    #[serde(rename = "Mensaje")]
    mensaje: Option<String>,
}

#[tokio::main]
async fn main() {
    // Get CUFE from args or use default
    let cufe = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            "FE012000000630-483-123250-163OC72025113000005879620040310542847129".to_string()
        });

    // Get captcha token from args or env
    let captcha_token = std::env::args()
        .nth(2)
        .or_else(|| std::env::var("DGI_CAPTCHA_TOKEN").ok())
        .unwrap_or_else(|| {
            println!("⚠️  No captcha token provided. Use: cargo run --bin test_cufe <CUFE> <CAPTCHA_TOKEN>");
            println!("   Or set DGI_CAPTCHA_TOKEN environment variable");
            std::process::exit(1);
        });

    println!("🔍 Testing DGI CUFE API");
    println!("📄 CUFE: {}", cufe);
    println!("🔑 Captcha token: {} chars", captcha_token.len());
    println!();

    // Call DGI API
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let url = "https://dgi-fep.mef.gob.pa/Consultas/ConsultarFacturasPorCUFE?Length=9";
    
    let form_data = [
        ("CUFE", cufe.as_str()),
        ("g-recaptcha-response", captcha_token.as_str()),
        ("X-Requested-With", "XMLHttpRequest"),
    ];

    println!("🔗 Calling DGI API...");
    
    let response = client
        .post(url)
        .header("accept", "*/*")
        .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
        .header("origin", "https://dgi-fep.mef.gob.pa")
        .header("referer", "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorCUFE")
        .header("x-requested-with", "XMLHttpRequest")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .form(&form_data)
        .send()
        .await;

    let response = match response {
        Ok(r) => r,
        Err(e) => {
            println!("❌ Connection error: {}", e);
            return;
        }
    };

    println!("📥 Response status: {}", response.status());
    
    let response_text = response.text().await.unwrap();
    println!("📥 Response length: {} bytes", response_text.len());
    
    // Save raw response
    std::fs::write("/tmp/cufe_raw_response.json", &response_text).ok();
    println!("💾 Raw response saved to /tmp/cufe_raw_response.json");
    println!();

    // Parse JSON response
    let dgi_response: DgiCufeResponse = match serde_json::from_str(&response_text) {
        Ok(r) => r,
        Err(e) => {
            println!("❌ JSON parse error: {}", e);
            println!("📄 Response (first 500 chars): {}", &response_text[..response_text.len().min(500)]);
            return;
        }
    };

    println!("📋 DGI RESPONSE:");
    println!("{}", "=".repeat(80));
    println!("  CUFE: {:?}", dgi_response.cufe);
    println!("  Error: {:?}", dgi_response.error);
    println!("  Mensaje: {:?}", dgi_response.mensaje);
    println!("  FacturaHTML: {} chars", dgi_response.factura_html.as_ref().map(|s| s.len()).unwrap_or(0));
    println!();

    // Check for errors
    if let Some(ref error) = dgi_response.error {
        if !error.is_empty() {
            println!("❌ DGI Error: {}", error);
            return;
        }
    }

    // Extract HTML content
    let html_content = match dgi_response.factura_html {
        Some(html) if !html.is_empty() => html,
        _ => {
            println!("❌ No HTML content in response");
            return;
        }
    };

    // Save HTML for inspection
    std::fs::write("/tmp/cufe_invoice.html", &html_content).ok();
    println!("💾 HTML saved to /tmp/cufe_invoice.html");
    println!();

    // Parse and extract data
    let document = Html::parse_document(&html_content);

    // ==========================================
    // EXTRACT INVOICE DATA
    // ==========================================
    println!("📋 EXTRACTED INVOICE DATA:");
    println!("{}", "=".repeat(80));

    // Method 1: Using scraper (dt/dd structure)
    println!("\n🔍 Method 1: Scraper (dt/dd):");
    let data_scraper = extract_with_scraper(&document);
    for (key, value) in &data_scraper {
        println!("  {}: {}", key, value);
    }

    // Method 2: Using regex on raw HTML
    println!("\n🔍 Method 2: Regex patterns:");
    let data_regex = extract_with_regex(&html_content);
    for (key, value) in &data_regex {
        println!("  {}: {}", key, value);
    }

    // ==========================================
    // PRODUCT DETAILS
    // ==========================================
    println!("\n");
    println!("📦 PRODUCT DETAILS:");
    println!("{}", "=".repeat(80));
    
    let products = extract_products(&document);
    println!("  Total products: {}", products.len());
    
    for (i, product) in products.iter().enumerate() {
        println!("\n  📌 Item #{}", i + 1);
        println!("    - Línea: {}", product.get("line").unwrap_or(&"".to_string()));
        println!("    - Código: {}", product.get("code").unwrap_or(&"".to_string()));
        println!("    - Descripción: {}", product.get("description").unwrap_or(&"".to_string()));
        println!("    - Cantidad: {}", product.get("quantity").unwrap_or(&"".to_string()));
        println!("    - Precio Unitario: {}", product.get("unit_price").unwrap_or(&"".to_string()));
        println!("    - Descuento: {}", product.get("unit_discount").unwrap_or(&"".to_string()));
        println!("    - Monto: {}", product.get("amount").unwrap_or(&"".to_string()));
        println!("    - ITBMS: {}", product.get("itbms").unwrap_or(&"".to_string()));
        println!("    - Total: {}", product.get("total").unwrap_or(&"".to_string()));
    }

    // ==========================================
    // SUMMARY
    // ==========================================
    println!("\n");
    println!("📊 SUMMARY:");
    println!("{}", "=".repeat(80));
    
    let emisor_ruc = data_scraper.get("emisor_ruc").or(data_regex.get("emisor_ruc"));
    let emisor_name = data_scraper.get("emisor_name").or(data_regex.get("emisor_name"));
    let tot_amount = data_scraper.get("tot_amount").or(data_regex.get("tot_amount"));
    let tot_itbms = data_scraper.get("tot_itbms").or(data_regex.get("tot_itbms"));
    let invoice_date = data_scraper.get("date").or(data_regex.get("date"));
    
    println!("  ✓ Emisor RUC: {}", emisor_ruc.unwrap_or(&"NOT FOUND".to_string()));
    println!("  ✓ Emisor Name: {}", emisor_name.unwrap_or(&"NOT FOUND".to_string()));
    println!("  ✓ Total Amount: {}", tot_amount.unwrap_or(&"NOT FOUND".to_string()));
    println!("  ✓ Total ITBMS: {}", tot_itbms.unwrap_or(&"NOT FOUND".to_string()));
    println!("  ✓ Date: {}", invoice_date.unwrap_or(&"NOT FOUND".to_string()));
    println!("  ✓ Products: {}", products.len());
    
    println!("\n✅ Test completed!");
}

/// Extract data using scraper crate (dt/dd structure)
fn extract_with_scraper(document: &Html) -> HashMap<String, String> {
    let mut data = HashMap::new();
    
    // Find all dt/dd pairs
    let dt_selector = Selector::parse("dt").unwrap();
    let dd_selector = Selector::parse("dd").unwrap();
    
    let dts: Vec<_> = document.select(&dt_selector).collect();
    let dds: Vec<_> = document.select(&dd_selector).collect();
    
    println!("    Found {} dt elements, {} dd elements", dts.len(), dds.len());
    
    for (dt, dd) in dts.iter().zip(dds.iter()) {
        let key = dt.text().collect::<String>().trim().to_lowercase();
        let value = dd.text().collect::<String>().trim().to_string();
        
        // Map known fields
        match key.as_str() {
            "ruc" => {
                // Check if we're in EMISOR or RECEPTOR context
                if data.get("emisor_ruc").is_none() {
                    data.insert("emisor_ruc".to_string(), value);
                } else if data.get("receptor_ruc").is_none() {
                    data.insert("receptor_ruc".to_string(), value);
                }
            }
            "dv" => {
                if data.get("emisor_dv").is_none() {
                    data.insert("emisor_dv".to_string(), value);
                } else if data.get("receptor_dv").is_none() {
                    data.insert("receptor_dv".to_string(), value);
                }
            }
            "nombre" => {
                if data.get("emisor_name").is_none() {
                    data.insert("emisor_name".to_string(), value);
                } else if data.get("receptor_name").is_none() {
                    data.insert("receptor_name".to_string(), value);
                }
            }
            "dirección" | "direccion" => {
                if data.get("emisor_address").is_none() {
                    data.insert("emisor_address".to_string(), value);
                } else if data.get("receptor_address").is_none() {
                    data.insert("receptor_address".to_string(), value);
                }
            }
            "teléfono" | "telefono" => {
                if data.get("emisor_phone").is_none() {
                    data.insert("emisor_phone".to_string(), value);
                } else if data.get("receptor_phone").is_none() {
                    data.insert("receptor_phone".to_string(), value);
                }
            }
            _ => {}
        }
    }
    
    // Extract totals from tfoot
    let tfoot_selector = Selector::parse("tfoot tr").unwrap();
    for row in document.select(&tfoot_selector) {
        let text = row.text().collect::<String>();
        let text_lower = text.to_lowercase();
        
        if text_lower.contains("valor total") {
            if let Some(amount) = extract_amount_from_text(&text) {
                data.insert("tot_amount".to_string(), amount);
            }
        }
        if text_lower.contains("itbms total") {
            if let Some(amount) = extract_amount_from_text(&text) {
                data.insert("tot_itbms".to_string(), amount);
            }
        }
        if text_lower.contains("total pagado") {
            if let Some(amount) = extract_amount_from_text(&text) {
                data.insert("total_pagado".to_string(), amount);
            }
        }
    }
    
    // Extract date from events table
    let td_selector = Selector::parse("td").unwrap();
    for td in document.select(&td_selector) {
        let text = td.text().collect::<String>();
        if text.contains("/") && text.contains(":") {
            // Looks like a date
            if let Ok(re) = Regex::new(r"(\d{2}/\d{2}/\d{4}\s+\d{2}:\d{2}:\d{2})") {
                if let Some(caps) = re.captures(&text) {
                    if let Some(date) = caps.get(1) {
                        data.insert("date".to_string(), date.as_str().to_string());
                        break;
                    }
                }
            }
        }
    }
    
    data
}

/// Extract amount from text like "Valor Total: 14.10"
fn extract_amount_from_text(text: &str) -> Option<String> {
    let re = Regex::new(r"[\d,.]+").ok()?;
    let amounts: Vec<_> = re.find_iter(text)
        .map(|m| m.as_str())
        .filter(|s| s.contains('.') || s.len() > 2)
        .collect();
    amounts.last().map(|s| s.to_string())
}

/// Extract data using regex patterns on raw HTML
fn extract_with_regex(html: &str) -> HashMap<String, String> {
    let mut data = HashMap::new();
    
    // Extract RUC - pattern: <dt>RUC</dt><dd>630-483-123250</dd>
    if let Ok(re) = Regex::new(r"(?i)<dt>\s*RUC\s*</dt>\s*<dd>([^<]+)</dd>") {
        let matches: Vec<_> = re.captures_iter(html).collect();
        if let Some(caps) = matches.get(0) {
            if let Some(m) = caps.get(1) {
                data.insert("emisor_ruc".to_string(), m.as_str().trim().to_string());
            }
        }
        if let Some(caps) = matches.get(1) {
            if let Some(m) = caps.get(1) {
                data.insert("receptor_ruc".to_string(), m.as_str().trim().to_string());
            }
        }
    }
    
    // Extract DV
    if let Ok(re) = Regex::new(r"(?i)<dt>\s*DV\s*</dt>\s*<dd>([^<]+)</dd>") {
        let matches: Vec<_> = re.captures_iter(html).collect();
        if let Some(caps) = matches.get(0) {
            if let Some(m) = caps.get(1) {
                data.insert("emisor_dv".to_string(), m.as_str().trim().to_string());
            }
        }
        if let Some(caps) = matches.get(1) {
            if let Some(m) = caps.get(1) {
                data.insert("receptor_dv".to_string(), m.as_str().trim().to_string());
            }
        }
    }
    
    // Extract NOMBRE
    if let Ok(re) = Regex::new(r"(?i)<dt>\s*NOMBRE\s*</dt>\s*<dd>([^<]+)</dd>") {
        let matches: Vec<_> = re.captures_iter(html).collect();
        if let Some(caps) = matches.get(0) {
            if let Some(m) = caps.get(1) {
                data.insert("emisor_name".to_string(), m.as_str().trim().to_string());
            }
        }
        if let Some(caps) = matches.get(1) {
            if let Some(m) = caps.get(1) {
                data.insert("receptor_name".to_string(), m.as_str().trim().to_string());
            }
        }
    }
    
    // Extract totals
    // Pattern: Valor Total:</td><td class="text-right">14.10</td>
    if let Ok(re) = Regex::new(r"(?i)Valor\s+Total:?\s*</td>\s*<td[^>]*>([\d,.]+)</td>") {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                data.insert("tot_amount".to_string(), m.as_str().trim().to_string());
            }
        }
    }
    
    if let Ok(re) = Regex::new(r"(?i)ITBMS\s+Total:?\s*</td>\s*<td[^>]*>([\d,.]+)</td>") {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                data.insert("tot_itbms".to_string(), m.as_str().trim().to_string());
            }
        }
    }
    
    // Extract date from Autorización event
    if let Ok(re) = Regex::new(r"(\d{2}/\d{2}/\d{4}\s+\d{2}:\d{2}:\d{2})[^<]*</td>\s*<td[^>]*>\s*Autorización") {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                data.insert("date".to_string(), m.as_str().trim().to_string());
            }
        }
    }
    
    // Extract CUFE
    if let Ok(re) = Regex::new(r"(FE\d{2}[A-Z0-9-]{50,70})") {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                data.insert("cufe".to_string(), m.as_str().trim().to_string());
            }
        }
    }
    
    data
}

/// Extract product details from tbody
fn extract_products(document: &Html) -> Vec<HashMap<String, String>> {
    let mut products = Vec::new();
    
    let tbody_selector = Selector::parse("tbody").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    
    for tbody in document.select(&tbody_selector) {
        for tr in tbody.select(&tr_selector) {
            let cells: Vec<_> = tr.select(&td_selector).collect();
            
            // Product rows have at least 8 columns
            if cells.len() >= 8 {
                let mut product = HashMap::new();
                
                // Based on HTML structure:
                // 0: Linea, 1: Código, 2: Descripción, 3: Info interés, 4: Cantidad,
                // 5: Precio Unitario, 6: Descuento Unitario, 7: Monto, 8: ITBMS, 
                // 9: ISC, 10: Acarreo, 11: Seguro, 12: Total
                
                product.insert("line".to_string(), cells[0].text().collect::<String>().trim().to_string());
                product.insert("code".to_string(), cells[1].text().collect::<String>().trim().to_string());
                product.insert("description".to_string(), cells[2].text().collect::<String>().trim().to_string());
                product.insert("info".to_string(), cells[3].text().collect::<String>().trim().to_string());
                product.insert("quantity".to_string(), cells[4].text().collect::<String>().trim().to_string());
                product.insert("unit_price".to_string(), cells[5].text().collect::<String>().trim().to_string());
                product.insert("unit_discount".to_string(), cells[6].text().collect::<String>().trim().to_string());
                product.insert("amount".to_string(), cells[7].text().collect::<String>().trim().to_string());
                
                if cells.len() > 8 {
                    product.insert("itbms".to_string(), cells[8].text().collect::<String>().trim().to_string());
                }
                if cells.len() > 12 {
                    product.insert("total".to_string(), cells[12].text().collect::<String>().trim().to_string());
                }
                
                // Only add if it looks like a valid product (has a line number)
                if let Ok(_) = product.get("line").unwrap_or(&"".to_string()).parse::<i32>() {
                    products.push(product);
                }
            }
        }
    }
    
    products
}
