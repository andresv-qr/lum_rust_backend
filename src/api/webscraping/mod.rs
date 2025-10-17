use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};
use crate::processing::web_scraping::ocr_extractor::{extract_main_info, ExtractedData};

// ============================================================================
// DATA STRUCTURES matching REAL invoice_header table schema
// CORRECTED: 2024-10-01 - Fixed field names and types to match PostgreSQL
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceHeader {
    // Core invoice fields (campos extraídos del HTML DGI)
    pub cufe: String, // CUFE del invoice
    pub no: Option<String>, // número de factura (NOT numero_factura)
    pub date: Option<String>, // fecha emisión como String DD/MM/YYYY HH:MM:SS (NOT fecha_emision)
    pub auth_date: Option<String>, // protocolo de autorización
    pub tot_amount: Option<f64>, // CHANGED: f64 instead of Decimal (matches DOUBLE PRECISION)
    pub tot_itbms: Option<f64>, // CHANGED: f64 instead of Decimal (matches DOUBLE PRECISION)
    
    // Issuer (Emisor/Proveedor) fields - ALL THESE ARE CORRECT
    pub issuer_name: Option<String>, // nombre del emisor (NOT proveedor_nombre)
    pub issuer_ruc: Option<String>, // RUC del emisor
    pub issuer_dv: Option<String>, // dígito verificador del emisor
    pub issuer_address: Option<String>, // dirección del emisor
    pub issuer_phone: Option<String>, // teléfono del emisor
    
    // Receptor (Cliente) fields - ALL THESE ARE CORRECT
    pub receptor_name: Option<String>, // nombre del receptor (NOT cliente_nombre)
    pub receptor_id: Option<String>, // ID/RUC del receptor (NOT cliente_ruc)
    pub receptor_dv: Option<String>, // dígito verificador del receptor
    pub receptor_address: Option<String>, // dirección del receptor
    pub receptor_phone: Option<String>, // teléfono del receptor
    
    // User and processing fields (campos del usuario y sistema)
    pub user_id: i64, // CHANGED: i64 to match BIGINT in PostgreSQL
    pub user_email: Option<String>, // email del usuario
    pub user_phone_number: Option<String>, // teléfono del usuario
    pub user_telegram_id: Option<String>, // telegram del usuario
    pub user_ws: Option<String>, // workspace del usuario
    
    // Processing metadata (metadatos de procesamiento)
    pub origin: String, // "app", "whatsapp", "telegram", etc.
    pub type_field: String, // "QR" o "CUFE" (usando type_field porque type es palabra reservada)
    pub url: String, // URL de la factura (NOT source_url)
    pub process_date: chrono::DateTime<chrono::Utc>, // fecha de procesamiento
    pub reception_date: chrono::DateTime<chrono::Utc>, // fecha de recepción
    pub time: Option<String>, // campo time adicional (HH:MM:SS)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceDetail {
    // CORRECTED: Removed invoice_header_id (doesn't exist, relation is by CUFE)
    // CORRECTED: Changed all Decimal to String to match TEXT fields in PostgreSQL
    pub cufe: String, // CUFE del invoice (FK)
    pub partkey: Option<String>, // ADDED: llave de partición (cufe|linea)
    pub date: Option<String>, // ADDED: fecha de emisión
    pub quantity: Option<String>, // CHANGED: String instead of Decimal (NOT cantidad)
    pub code: Option<String>, // ADDED: código del producto
    pub description: Option<String>, // descripción del ítem (NOT descripcion)
    pub unit_discount: Option<String>, // ADDED: descuento unitario
    pub unit_price: Option<String>, // CHANGED: String instead of Decimal (NOT precio_unitario)
    pub itbms: Option<String>, // CHANGED: String instead of Decimal (NOT impuesto_monto)
    pub amount: Option<String>, // CHANGED: String instead of Decimal (NOT subtotal)
    pub total: Option<String>, // CHANGED: String instead of Decimal
    pub information_of_interest: Option<String>, // ADDED: información adicional
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePayment {
    // CORRECTED: Removed invoice_header_id (doesn't exist, relation is by CUFE)
    // CORRECTED: Changed all Decimal to String to match TEXT fields in PostgreSQL
    pub cufe: String, // CUFE del invoice (FK)
    pub forma_de_pago: Option<String>, // CHANGED: forma_de_pago instead of metodo_pago
    pub forma_de_pago_otro: Option<String>, // ADDED: otra forma de pago
    pub valor_pago: Option<String>, // CHANGED: String instead of Decimal (NOT monto)
    pub efectivo: Option<String>, // ADDED: monto en efectivo
    pub tarjeta_debito: Option<String>, // ADDED: monto en tarjeta débito
    pub tarjeta_credito: Option<String>, // ADDED: monto en tarjeta crédito
    pub tarjeta_clave_banistmo: Option<String>, // ADDED: tarjeta clave Banistmo
    pub vuelto: Option<String>, // ADDED: vuelto dado
    pub total_pagado: Option<String>, // ADDED: total pagado
    pub descuentos: Option<String>, // ADDED: descuentos aplicados
    pub merged: Option<serde_json::Value>, // ADDED: datos JSON adicionales
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapingResult {
    pub success: bool,
    pub header: Option<InvoiceHeader>,
    pub details: Vec<InvoiceDetail>,
    pub payments: Vec<InvoicePayment>,
    pub error_message: Option<String>,
}

// ============================================================================
// SCRAPING FUNCTIONS
// ============================================================================

pub async fn scrape_invoice(
    client: &reqwest::Client,
    url: &str,
    user_id: i64,
) -> Result<ScrapingResult, String> {
    info!("Starting to scrape invoice from URL: {}", url);
    
    // Fetch the HTML content and get final URL after redirections
    let (html_content, final_url) = match fetch_html_with_final_url(client, url).await {
        Ok((content, final_url)) => {
            if final_url != url {
                info!("🔄 URL redirection in scraping: {} → {}", url, final_url);
            }
            (content, final_url)
        },
        Err(e) => {
            error!("Failed to fetch HTML: {}", e);
            return Ok(ScrapingResult {
                success: false,
                header: None,
                details: Vec::new(),
                payments: Vec::new(),
                error_message: Some(format!("Failed to fetch HTML: {}", e)),
            });
        }
    };

    // Parse HTML and extract data
    let document = Html::parse_document(&html_content);
    
    info!("🔍 DEBUG - HTML content length: {} bytes", html_content.len());
    info!("🔍 DEBUG - HTML first 500 chars: {}", &html_content.chars().take(500).collect::<String>());
    
    // 🔍 DEBUG: Guardar HTML en archivo para inspección
    if let Err(e) = std::fs::write("/tmp/scraped_invoice.html", &html_content) {
        warn!("Failed to save HTML debug file: {}", e);
    } else {
        info!("🔍 DEBUG - HTML saved to /tmp/scraped_invoice.html");
    }
    
    // Extract CUFE from final URL or page
    let cufe = extract_cufe_from_url(&final_url).unwrap_or_else(|| "UNKNOWN".to_string());
    
    // Try to extract basic invoice information
    let mut header = extract_invoice_header(&document, &cufe, user_id);
    
    // Set the FINAL URL in the header if extraction was successful
    if let Some(ref mut h) = header {
        h.url = final_url;  // Use final URL instead of original URL
    }
    
    let details = extract_invoice_details(&document, &cufe, user_id);
    let payments = extract_invoice_payments(&document, &cufe, user_id);

    Ok(ScrapingResult {
        success: true, // Always successful since we always return a header with at least CUFE
        header,
        details,
        payments,
        error_message: None,
    })
}

async fn fetch_html_with_final_url(client: &Client, url: &str) -> Result<(String, String), String> {
    info!("Fetching HTML with final URL tracking from: {}", url);
    
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "es-ES,es;q=0.8,en;q=0.5")
        .header("Accept-Encoding", "gzip, deflate")
        .header("Connection", "keep-alive")
        .header("Upgrade-Insecure-Requests", "1")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let final_url = response.url().to_string();

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

    info!("Successfully fetched HTML content ({} bytes) from final URL: {}", html.len(), final_url);
    Ok((html, final_url))
}

fn extract_cufe_from_url(url: &str) -> Option<String> {
    // Extract chFE parameter from URL
    if let Some(start) = url.find("chFE=") {
        let start = start + 5; // Skip "chFE="
        if let Some(end) = url[start..].find('&') {
            return Some(url[start..start + end].to_string());
        } else {
            return Some(url[start..].to_string());
        }
    }
    None
}



// CORRECTED: Changed to return f64 instead of Decimal to match DOUBLE PRECISION in PostgreSQL
fn parse_amount_from_text(text: &str) -> Option<f64> {
    // Remove common currency symbols and formatting
    let binding = text
        .replace("B/.", "")
        .replace("$", "")
        .replace(",", "")
        .replace(" ", "");
    let cleaned = binding.trim();
    
    // Try to parse as f64
    cleaned.parse::<f64>().ok()
}



fn extract_invoice_header(document: &Html, cufe: &str, user_id: i64) -> Option<InvoiceHeader> {
    info!("Extracting invoice header from document using ocr_extractor");

    // The new ocr_extractor is the single source of truth.
    // The old selector/regex logic has been removed to fix warnings and simplify.

    let now_utc = chrono::Utc::now();
    
    // Extract invoice data using our unified extractor
    let html_str = document.html();
    
    info!("🔍 DEBUG - Calling extract_main_info with HTML length: {}", html_str.len());
    let extracted_data = match extract_main_info(&html_str) {
        Ok(data) => {
            info!("🔍 DEBUG - extract_main_info SUCCESS, header keys: {:?}", data.header.keys().collect::<Vec<_>>());
            data
        },
        Err(e) => {
            error!("🔍 DEBUG - extract_main_info ERROR: {}", e);
            ExtractedData::default()
        }
    };
    
    // Map extracted header data
    let header_data = extracted_data.header;
    let invoice_date = header_data.get("date").cloned();
    let invoice_no = header_data.get("no").cloned();
    let issuer_name = header_data.get("emisor_name").cloned();
    let issuer_ruc = header_data.get("emisor_ruc").cloned();
    let tot_amount_str = header_data.get("tot_amount").cloned();
    let tot_itbms_str = header_data.get("tot_itbms").cloned();

    info!("🔍 DEBUG - Raw header_data keys: {:?}", header_data.keys().collect::<Vec<_>>());
    info!("🔍 DEBUG - emisor_name value: {:?}", header_data.get("emisor_name"));
    info!("🔍 DEBUG - tot_amount_str value: {:?}", &tot_amount_str);
    
    let tot_amount = tot_amount_str.and_then(|s| parse_amount_from_text(&s));
    let tot_itbms = tot_itbms_str.and_then(|s| parse_amount_from_text(&s));
    
    info!("🔍 DEBUG - Extracted data - RUC: {:?}, Nombre: {:?}, Total: {:?}, ITBMS: {:?}", 
          issuer_ruc, issuer_name, tot_amount, tot_itbms);

    // Return a header if we found at least something meaningful
    if issuer_ruc.is_some() || issuer_name.is_some() || tot_amount.is_some() || !cufe.is_empty() {
        Some(InvoiceHeader {
            cufe: cufe.to_string(),
            no: invoice_no,
            date: invoice_date,
            auth_date: None, // This is not extracted currently
            tot_amount,
            tot_itbms,
            
            // Issuer fields
            issuer_name,
            issuer_ruc,
            issuer_dv: header_data.get("emisor_dv").cloned(),
            issuer_address: header_data.get("emisor_address").cloned(),
            issuer_phone: header_data.get("emisor_phone").cloned(),
            
            // Receptor fields  
            receptor_name: header_data.get("receptor_name").cloned(),
            receptor_id: header_data.get("receptor_ruc").cloned(),
            receptor_dv: header_data.get("receptor_dv").cloned(),
            receptor_address: header_data.get("receptor_address").cloned(),
            receptor_phone: header_data.get("receptor_phone").cloned(),
            
            // User fields (can be provided by API)
            user_id,
            user_email: None,
            user_phone_number: None,
            user_telegram_id: None,
            user_ws: None,
            
            // Processing metadata
            origin: "app".to_string(),
            type_field: "QR".to_string(),
            url: "".to_string(), // Will be set by caller
            process_date: now_utc,
            reception_date: now_utc,
            time: None,
        })
    } else {
        // Return header with minimal data even if extraction failed
        Some(InvoiceHeader {
            cufe: cufe.to_string(),
            no: Some(cufe.to_string()),
            date: Some(chrono::Utc::now().format("%d/%m/%Y %H:%M:%S").to_string()),
            auth_date: None,
            tot_amount: None,
            tot_itbms: None,
            issuer_name: None,
            issuer_ruc: None,
            issuer_dv: None,
            issuer_address: None,
            issuer_phone: None,
            receptor_name: None,
            receptor_id: None,
            receptor_dv: None,
            receptor_address: None,
            receptor_phone: None,
            user_id,
            user_email: None,
            user_phone_number: None,
            user_telegram_id: None,
            user_ws: None,
            origin: "app".to_string(),
            type_field: "QR".to_string(),
            url: "".to_string(),
            process_date: now_utc,
            reception_date: now_utc,
            time: None,
        })
    }
}

fn extract_invoice_details(document: &Html, cufe: &str, _user_id: i64) -> Vec<InvoiceDetail> {
    // Try to find table rows or detail sections
    // TODO: Implement real extraction from HTML table
    let selectors = ["tr", ".detail-row", ".item-row", "tbody tr"];
    
    for selector_str in &selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            let elements: Vec<_> = document.select(&selector).collect();
            if elements.len() > 1 {
                info!("Found {} potential detail rows", elements.len());
                
                // CORRECTED: Return mock detail with correct field names and types (TEXT)
                return vec![InvoiceDetail {
                    cufe: cufe.to_string(),
                    partkey: Some(format!("{}|1", cufe)), // cufe|linea
                    date: Some(chrono::Utc::now().format("%d/%m/%Y").to_string()),
                    quantity: Some("1.00".to_string()), // TEXT not Decimal
                    code: Some("PROD-001".to_string()),
                    description: Some("Extracted item".to_string()),
                    unit_discount: Some("0.00".to_string()),
                    unit_price: Some("100.00".to_string()), // TEXT not Decimal
                    itbms: Some("7.00".to_string()), // TEXT not Decimal
                    amount: Some("100.00".to_string()), // TEXT not Decimal
                    total: Some("107.00".to_string()), // TEXT not Decimal
                    information_of_interest: None,
                }];
            }
        }
    }

    warn!("Could not extract invoice details");
    Vec::new()
}

fn extract_invoice_payments(document: &Html, cufe: &str, _user_id: i64) -> Vec<InvoicePayment> {
    // Extract payment information from tfoot using text-relative strategy
    let tfoot_selector = match Selector::parse("tfoot") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let tr_selector = match Selector::parse("tr") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let td_selector = match Selector::parse("td") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let div_selector = match Selector::parse("div") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut forma_de_pago: Option<String> = None;
    let mut valor_pago: Option<String> = None;
    let mut efectivo: Option<String> = None;
    let mut tarjeta_credito: Option<String> = None;
    let mut tarjeta_debito: Option<String> = None;
    let mut tarjeta_clave_banistmo: Option<String> = None;
    let mut forma_de_pago_otro: Option<String> = None;
    let mut vuelto: Option<String> = None;
    let mut total_pagado: Option<String> = None;

    for tfoot in document.select(&tfoot_selector) {
        for tr in tfoot.select(&tr_selector) {
            if let Some(td) = tr.select(&td_selector).next() {
                let td_text = td.text().collect::<String>();
                let td_upper = td_text.to_uppercase();
                
                // Extract div value
                let value = if let Some(div) = td.select(&div_selector).next() {
                    div.text().collect::<String>().trim().to_string()
                } else {
                    String::new()
                };
                
                // Skip if no value or value is empty
                if value.is_empty() {
                    continue;
                }
                
                // Match payment types using text patterns
                if td_upper.contains("EFECTIVO:") {
                    efectivo = Some(value);
                    if forma_de_pago.is_none() {
                        forma_de_pago = Some("Efectivo".to_string());
                    }
                } else if (td_upper.contains("TARJETA") && td_upper.contains("CRÉDITO")) || td_upper.contains("CREDITO") {
                    tarjeta_credito = Some(value);
                    if forma_de_pago.is_none() {
                        forma_de_pago = Some("Tarjeta Crédito".to_string());
                    }
                } else if (td_upper.contains("TARJETA") && td_upper.contains("DÉBITO")) || td_upper.contains("DEBITO") {
                    tarjeta_debito = Some(value);
                    if forma_de_pago.is_none() {
                        forma_de_pago = Some("Tarjeta Débito".to_string());
                    }
                } else if td_upper.contains("TARJETA CLAVE") && td_upper.contains("BANISTMO") {
                    tarjeta_clave_banistmo = Some(value);
                    if forma_de_pago.is_none() {
                        forma_de_pago = Some("Tarjeta Clave Banistmo".to_string());
                    }
                } else if td_upper.contains("CHEQUE:") {
                    forma_de_pago_otro = Some(value);
                    if forma_de_pago.is_none() {
                        forma_de_pago = Some("Cheque".to_string());
                    }
                } else if td_upper.contains("TRANSFERENCIA:") {
                    forma_de_pago_otro = Some(value);
                    if forma_de_pago.is_none() {
                        forma_de_pago = Some("Transferencia".to_string());
                    }
                } else if td_upper.contains("ACH:") {
                    forma_de_pago_otro = Some(value);
                    if forma_de_pago.is_none() {
                        forma_de_pago = Some("ACH".to_string());
                    }
                } else if td_upper.contains("TOTAL PAGADO:") {
                    total_pagado = Some(value.clone());
                    valor_pago = Some(value);
                } else if td_upper.contains("VUELTO:") {
                    vuelto = Some(value);
                }
            }
        }
    }

    // Return payment record if any payment data was found
    if forma_de_pago.is_some() || total_pagado.is_some() {
        info!("Payment information extracted: forma_de_pago={:?}, total_pagado={:?}", forma_de_pago, total_pagado);
        vec![InvoicePayment {
            cufe: cufe.to_string(),
            forma_de_pago,
            forma_de_pago_otro,
            valor_pago,
            efectivo,
            tarjeta_debito,
            tarjeta_credito,
            tarjeta_clave_banistmo,
            vuelto,
            total_pagado,
            descuentos: None,
            merged: None,
        }]
    } else {
        warn!("Could not extract payment information from tfoot");
        Vec::new()
    }
}
