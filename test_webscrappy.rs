// Simple test script to debug web scraping without authentication
// Run with: cargo run --bin test_webscrappy

use scraper::{Html, Selector, ElementRef};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=FE0120000155631118-2-2016-5800002025100100001813560010310796964284&iAmb=1&digestValue=Hc0Xd/keq229i/8c7Ge8aOE6jsZm4XVGfQ2C7SW4//Y=&jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJjaEZFICI6IkZFMDEyMDAwMDE1NTYzMTExOC0yLTIwMTYtNTgwMDAwMjAyNTEwMDEwMDAwMTgxMzU2MDAxMDMxMDc5Njk2NDI4NCIsImlBbWIiOiIxIiwiZGlnZXN0VmFsdWUiOiJIYzBYZC9rZXEyMjlpLzhjN0dlOGFPRTZqc1ptNFhWR2ZRMkM3U1c0Ly9ZPSJ9.dZvtG-ytUFVSIcOFgVFlj-DeKM96Qw2kXKxOuA1pfws".to_string()
        });

    println!("🔍 Testing Web Scraper");
    println!("📄 URL: {}", url);
    println!();

    // Fetch HTML
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .unwrap();

    let response = client.get(&url).send().await.unwrap();
    let html_content = response.text().await.unwrap();
    
    println!("✅ HTML Downloaded: {} bytes", html_content.len());
    println!();

    let document = Html::parse_document(&html_content);

    // ==========================================
    // TABLA 1: INVOICE HEADER
    // ==========================================
    println!("📋 TABLA 1: INVOICE HEADER");
    println!("{}", "=".repeat(80));

    // Extract CUFE
    let cufe = extract_cufe(&document);
    println!("✓ CUFE: {}", cufe.as_ref().unwrap_or(&"NOT FOUND".to_string()));

    // Extract Invoice Number and Date
    let (invoice_no, invoice_date) = extract_invoice_info(&document);
    println!("✓ No. Factura: {}", invoice_no.as_ref().unwrap_or(&"NOT FOUND".to_string()));
    println!("✓ Fecha: {}", invoice_date.as_ref().unwrap_or(&"NOT FOUND".to_string()));

    // Extract EMISOR data
    let emisor = extract_panel_data(&document, "EMISOR");
    println!("\n📤 EMISOR:");
    println!("  - RUC: {}", emisor.get("ruc").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - DV: {}", emisor.get("dv").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - Nombre: {}", emisor.get("nombre").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - Dirección: {}", emisor.get("dirección").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - Teléfono: {}", emisor.get("teléfono").unwrap_or(&"NOT FOUND".to_string()));

    // Extract RECEPTOR data
    let receptor = extract_panel_data(&document, "RECEPTOR");
    println!("\n📥 RECEPTOR:");
    println!("  - RUC: {}", receptor.get("ruc").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - DV: {}", receptor.get("dv").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - Nombre: {}", receptor.get("nombre").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - Dirección: {}", receptor.get("dirección").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - Teléfono: {}", receptor.get("teléfono").unwrap_or(&"NOT FOUND".to_string()));

    // Extract Totals
    let totals = extract_totals(&document);
    println!("\n💰 TOTALES:");
    println!("  - Valor Total: {}", totals.get("tot_amount").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - ITBMS Total: {}", totals.get("tot_itbms").unwrap_or(&"NOT FOUND".to_string()));
    println!("  - Total Pagado: {}", totals.get("total_pagado").unwrap_or(&"NOT FOUND".to_string()));

    // ==========================================
    // TABLA 2: INVOICE DETAILS
    // ==========================================
    println!("\n");
    println!("📦 TABLA 2: INVOICE DETAILS");
    println!("{}", "=".repeat(80));

    let details = extract_details(&document);
    println!("✓ Total items: {}", details.len());
    
    for (i, detail) in details.iter().enumerate() {
        println!("\n  📌 Item #{}", i + 1);
        println!("    - Código: {}", detail.get("code").unwrap_or(&"".to_string()));
        println!("    - Descripción: {}", detail.get("description").unwrap_or(&"".to_string()));
        println!("    - Cantidad: {}", detail.get("quantity").unwrap_or(&"".to_string()));
        println!("    - Precio Unitario: {}", detail.get("unit_price").unwrap_or(&"".to_string()));
        println!("    - Descuento: {}", detail.get("unit_discount").unwrap_or(&"".to_string()));
        println!("    - Monto: {}", detail.get("amount").unwrap_or(&"".to_string()));
        println!("    - ITBMS: {}", detail.get("itbms").unwrap_or(&"".to_string()));
        println!("    - Total: {}", detail.get("total").unwrap_or(&"".to_string()));
    }

    // ==========================================
    // TABLA 3: INVOICE PAYMENTS
    // ==========================================
    println!("\n");
    println!("💳 TABLA 3: INVOICE PAYMENTS");
    println!("{}", "=".repeat(80));

    let payments = extract_payments(&document);
    if payments.is_empty() {
        println!("⚠️  No payment information found");
    } else {
        for (i, payment) in payments.iter().enumerate() {
            println!("\n  💵 Payment #{}", i + 1);
            println!("    - Forma de Pago: {}", payment.get("forma_de_pago").unwrap_or(&"NULL".to_string()));
            println!("    - Valor Pago (Total Pagado): {}", payment.get("valor_pago").unwrap_or(&"NULL".to_string()));
            println!("    - Efectivo: {}", payment.get("efectivo").unwrap_or(&"NULL".to_string()));
            println!("    - Tarjeta Crédito: {}", payment.get("tarjeta_crédito").unwrap_or(&"NULL".to_string()));
            println!("    - Tarjeta Débito: {}", payment.get("tarjeta_débito").unwrap_or(&"NULL".to_string()));
            println!("    - Tarjeta Clave Banistmo: {}", payment.get("tarjeta_clave__banistmo_").unwrap_or(&"NULL".to_string()));
            println!("    - Forma de Pago Otro: {}", payment.get("forma_de_pago_otro").unwrap_or(&"NULL".to_string()));
            println!("    - Vuelto: {}", payment.get("vuelto").unwrap_or(&"NULL".to_string()));
            println!("    - Total Pagado: {}", payment.get("total_pagado").unwrap_or(&"NULL".to_string()));
        }
    }

    println!("\n");
    println!("✅ Test completed!");
}

// ============================================================================
// EXTRACTION FUNCTIONS
// ============================================================================

fn extract_cufe(document: &Html) -> Option<String> {
    let dt_selector = Selector::parse("dt").ok()?;
    
    for dt in document.select(&dt_selector) {
        let dt_text = dt.text().collect::<String>().to_uppercase();
        if dt_text.contains("CÓDIGO ÚNICO") && dt_text.contains("CUFE") {
            let mut current = dt.next_sibling();
            while let Some(node) = current {
                if let Some(element) = ElementRef::wrap(node) {
                    if element.value().name() == "dd" {
                        let cufe = element.text().collect::<String>().trim().to_string();
                        if cufe.starts_with("FE") && cufe.len() > 50 {
                            return Some(cufe);
                        }
                    }
                }
                current = node.next_sibling();
            }
        }
    }
    None
}

fn extract_invoice_info(document: &Html) -> (Option<String>, Option<String>) {
    let h4_selector = Selector::parse("h4").ok();
    let h5_selector = Selector::parse("h5").ok();
    let mut invoice_no = None;
    let mut invoice_date = None;

    // Extract using proximity to "FACTURA" heading
    // Structure: <div class="row"><div><h5>No. 0000181356</h5></div><div><h4>FACTURA</h4></div><div><h5>01/10/2025...</h5></div></div>
    if let Some(h4_sel) = h4_selector {
        for h4 in document.select(&h4_sel) {
            let h4_text = h4.text().collect::<String>().to_uppercase();
            
            // Find h4 containing "FACTURA"
            if h4_text.contains("FACTURA") {
                // Navigate up to find the row container (usually 2-3 levels up)
                let mut row_container = h4.parent();
                for _ in 0..3 {
                    if let Some(parent) = row_container {
                        if let Some(parent_elem) = ElementRef::wrap(parent) {
                            // Check if this element has class="row"
                            let has_row_class = parent_elem.value().attr("class")
                                .map(|c| c.contains("row"))
                                .unwrap_or(false);
                            
                            if has_row_class {
                                // Found the row container
                                let row_elem = parent_elem;
                                // Look for all h5 elements in this row
                                if let Some(h5_sel) = h5_selector.as_ref() {
                                    for h5 in row_elem.select(h5_sel) {
                                        let h5_text = h5.text().collect::<String>().trim().to_string();
                                        
                                        // Extract invoice number: "No. 0000181356" or just "0000181356"
                                        if h5_text.to_uppercase().contains("NO.") {
                                            // Extract the number after "No."
                                            if let Some(no_idx) = h5_text.to_uppercase().find("NO.") {
                                                let after_no = &h5_text[no_idx + 3..].trim();
                                                if after_no.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
                                                    invoice_no = Some(after_no.trim().to_string());
                                                }
                                            }
                                        } else if h5_text.chars().all(|c| c.is_ascii_digit()) && h5_text.len() == 10 {
                                            // Direct number format
                                            invoice_no = Some(h5_text.clone());
                                        }
                                        
                                        // Extract date: DD/MM/YYYY HH:MM:SS
                                        let parts: Vec<&str> = h5_text.split_whitespace().collect();
                                        if parts.len() >= 1 {
                                            let date_part = parts[0];
                                            let date_segments: Vec<&str> = date_part.split('/').collect();
                                            
                                            // Validate DD/MM/YYYY format
                                            if date_segments.len() == 3 
                                                && date_segments[0].len() == 2 
                                                && date_segments[1].len() == 2 
                                                && date_segments[2].len() == 4
                                                && date_segments[0].chars().all(|c| c.is_ascii_digit())
                                                && date_segments[1].chars().all(|c| c.is_ascii_digit())
                                                && date_segments[2].chars().all(|c| c.is_ascii_digit()) {
                                                
                                                // Validate time part if present
                                                if parts.len() == 2 {
                                                    let time_part = parts[1];
                                                    let time_segments: Vec<&str> = time_part.split(':').collect();
                                                    if time_segments.len() == 3
                                                        && time_segments[0].len() == 2
                                                        && time_segments[1].len() == 2
                                                        && time_segments[2].len() == 2
                                                        && time_segments.iter().all(|s| s.chars().all(|c| c.is_ascii_digit())) {
                                                        invoice_date = Some(h5_text.clone());
                                                    }
                                                } else {
                                                    // Date only, no time
                                                    invoice_date = Some(h5_text.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        }
                        row_container = parent.parent();
                    } else {
                        break;
                    }
                }
                if invoice_no.is_some() && invoice_date.is_some() {
                    break;
                }
            }
        }
    }

    (invoice_no, invoice_date)
}

fn extract_panel_data(document: &Html, panel_title: &str) -> HashMap<String, String> {
    let mut data = HashMap::new();
    let panel_heading_selector = Selector::parse("div.panel-heading").unwrap();
    
    for panel_heading in document.select(&panel_heading_selector) {
        let heading_text = panel_heading.text().collect::<String>().trim().to_uppercase();
        
        if heading_text.contains(panel_title) {
            let mut current = panel_heading.next_sibling();
            while let Some(node) = current {
                if let Some(element) = ElementRef::wrap(node) {
                    if element.value().attr("class").unwrap_or("").contains("panel-body") {
                        // Extract all dt/dd pairs
                        let dt_selector = Selector::parse("dt").unwrap();
                        
                        for dt in element.select(&dt_selector) {
                            let key = dt.text().collect::<String>().trim().to_lowercase();
                            
                            // Find next dd sibling
                            let mut dd_search = dt.next_sibling();
                            while let Some(dd_node) = dd_search {
                                if let Some(dd_element) = ElementRef::wrap(dd_node) {
                                    if dd_element.value().name() == "dd" {
                                        let value = dd_element.text().collect::<String>().trim().to_string();
                                        data.insert(key.clone(), value);
                                        break;
                                    }
                                }
                                dd_search = dd_node.next_sibling();
                            }
                        }
                        break;
                    }
                }
                current = node.next_sibling();
            }
        }
    }
    data
}

fn extract_totals(document: &Html) -> HashMap<String, String> {
    let mut data = HashMap::new();
    let td_selector = Selector::parse("td.text-right").unwrap();
    let div_selector = Selector::parse("div").unwrap();

    for td in document.select(&td_selector) {
        let text = td.text().collect::<String>().to_uppercase();

        if let Some(div) = td.select(&div_selector).next() {
            let value = div.text().collect::<String>().trim().to_string();
            
            if text.contains("VALOR TOTAL:") || text.contains("TOTAL:") && !text.contains("ITBMS") {
                data.insert("tot_amount".to_string(), value.clone());
            }
            if text.contains("ITBMS TOTAL:") {
                data.insert("tot_itbms".to_string(), value.clone());
            }
            if text.contains("TOTAL PAGADO:") {
                data.insert("total_pagado".to_string(), value.clone());
            }
        }
    }

    data
}

fn extract_details(document: &Html) -> Vec<HashMap<String, String>> {
    let mut details = Vec::new();
    
    // Look for table with details
    let tbody_selector = Selector::parse("tbody").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    for tbody in document.select(&tbody_selector) {
        for tr in tbody.select(&tr_selector) {
            let mut detail = HashMap::new();
            let cells: Vec<_> = tr.select(&td_selector).collect();
            
            if cells.len() >= 8 {
                // Based on the HTML structure we saw:
                // 0: Linea, 1: Código, 2: Descripción, 3: Info interés, 4: Cantidad,
                // 5: Precio, 6: Descuento, 7: Monto, 8: ITBMS, 9: ISC, 10: Acarreo, 11: Seguro, 12: Total
                
                detail.insert("line".to_string(), cells[0].text().collect::<String>().trim().to_string());
                detail.insert("code".to_string(), cells[1].text().collect::<String>().trim().to_string());
                detail.insert("description".to_string(), cells[2].text().collect::<String>().trim().to_string());
                detail.insert("information_of_interest".to_string(), cells[3].text().collect::<String>().trim().to_string());
                detail.insert("quantity".to_string(), cells[4].text().collect::<String>().trim().to_string());
                detail.insert("unit_price".to_string(), cells[5].text().collect::<String>().trim().to_string());
                detail.insert("unit_discount".to_string(), cells[6].text().collect::<String>().trim().to_string());
                detail.insert("amount".to_string(), cells[7].text().collect::<String>().trim().to_string());
                
                if cells.len() > 8 {
                    detail.insert("itbms".to_string(), cells[8].text().collect::<String>().trim().to_string());
                }
                if cells.len() > 12 {
                    detail.insert("total".to_string(), cells[12].text().collect::<String>().trim().to_string());
                }
                
                details.push(detail);
            }
        }
    }

    details
}

fn extract_payments(document: &Html) -> Vec<HashMap<String, String>> {
    let mut payment = HashMap::new();
    
    // Look for payment information in tfoot using text-relative strategy
    let tfoot_selector = Selector::parse("tfoot").ok();
    let tr_selector = Selector::parse("tr").ok();
    let td_selector = Selector::parse("td").ok();
    let div_selector = Selector::parse("div").ok();

    if let (Some(tfoot_sel), Some(tr_sel), Some(td_sel), Some(div_sel)) = 
        (tfoot_selector, tr_selector, td_selector, div_selector) {
        
        for tfoot in document.select(&tfoot_sel) {
            for tr in tfoot.select(&tr_sel) {
                if let Some(td) = tr.select(&td_sel).next() {
                    let td_text = td.text().collect::<String>();
                    let td_upper = td_text.to_uppercase();
                    
                    // Extract div value
                    let value = if let Some(div) = td.select(&div_sel).next() {
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
                        payment.insert("efectivo".to_string(), value);
                        if !payment.contains_key("forma_de_pago") {
                            payment.insert("forma_de_pago".to_string(), "Efectivo".to_string());
                        }
                    } else if td_upper.contains("TARJETA") && td_upper.contains("CRÉDITO") || td_upper.contains("CREDITO") {
                        payment.insert("tarjeta_crédito".to_string(), value);
                        if !payment.contains_key("forma_de_pago") {
                            payment.insert("forma_de_pago".to_string(), "Tarjeta Crédito".to_string());
                        }
                    } else if td_upper.contains("TARJETA") && td_upper.contains("DÉBITO") || td_upper.contains("DEBITO") {
                        payment.insert("tarjeta_débito".to_string(), value);
                        if !payment.contains_key("forma_de_pago") {
                            payment.insert("forma_de_pago".to_string(), "Tarjeta Débito".to_string());
                        }
                    } else if td_upper.contains("TARJETA CLAVE") && td_upper.contains("BANISTMO") {
                        payment.insert("tarjeta_clave__banistmo_".to_string(), value);
                        if !payment.contains_key("forma_de_pago") {
                            payment.insert("forma_de_pago".to_string(), "Tarjeta Clave Banistmo".to_string());
                        }
                    } else if td_upper.contains("CHEQUE:") {
                        payment.insert("forma_de_pago_otro".to_string(), value);
                        if !payment.contains_key("forma_de_pago") {
                            payment.insert("forma_de_pago".to_string(), "Cheque".to_string());
                        }
                    } else if td_upper.contains("TRANSFERENCIA:") {
                        payment.insert("forma_de_pago_otro".to_string(), value);
                        if !payment.contains_key("forma_de_pago") {
                            payment.insert("forma_de_pago".to_string(), "Transferencia".to_string());
                        }
                    } else if td_upper.contains("ACH:") {
                        payment.insert("forma_de_pago_otro".to_string(), value);
                        if !payment.contains_key("forma_de_pago") {
                            payment.insert("forma_de_pago".to_string(), "ACH".to_string());
                        }
                    } else if td_upper.contains("TOTAL PAGADO:") {
                        payment.insert("total_pagado".to_string(), value.clone());
                        payment.insert("valor_pago".to_string(), value);
                    } else if td_upper.contains("VUELTO:") {
                        payment.insert("vuelto".to_string(), value);
                    }
                }
            }
        }
    }

    // Return as vector with single payment record (all payment info combined)
    if !payment.is_empty() {
        vec![payment]
    } else {
        Vec::new()
    }
}
