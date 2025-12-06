use anyhow::Result;
use scraper::{Html, Selector, ElementRef, Element};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ExtractedData {
    pub header: HashMap<String, String>,
    pub details: Vec<HashMap<String, String>>,
}

/// Checks for MEF error messages in the HTML document.
/// Returns Some(error_message) if an error is found, None otherwise.
fn check_for_mef_errors(document: &Html) -> Option<String> {
    // Common selectors for error messages
    let error_selectors = vec![
        "div.alert-danger",
        "div.alert-warning",
        "div.alert-error",
        ".alert.alert-danger",
        ".alert.alert-warning", 
        ".alert.alert-error",
        "#validacionMensajeCriterioResultado",
        "#cuerpoVentanaMensajes",
        ".error-message",
        ".validation-summary-errors",
        ".field-validation-error"
    ];
    
    // Check for alert/error divs first
    for selector_str in error_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    
    // Check for common error text patterns in the entire document
    let all_text = document.root_element().text().collect::<String>().to_lowercase();
    
    // More specific error patterns that are more likely to be actual errors
    let specific_error_patterns = vec![
        "factura no encontrada",
        "cufe no encontrado", 
        "documento no existe",
        "no se pudo procesar",
        "access denied",
        "acceso denegado",
        "página no encontrada",
        "error interno",
        "internal server error",
        "service unavailable",
        "servicio no disponible",
        "connection timeout",
        "request timeout",
        "session expired",
        "sesión expirada",
        "error de conexión",
        "servidor no disponible"
    ];
    
    for pattern in specific_error_patterns {
        if all_text.contains(pattern) {
            return Some(format!("Detected error pattern: {}", pattern));
        }
    }
    
    // Check for generic error words only if they appear in error contexts
    let generic_error_patterns = vec![
        ("error", vec!["script", "var ", "function", "javascript", "console.error"]), // Skip if in JS context
        ("not found", vec!["script", "var ", "function"]),
        ("no encontrado", vec!["script", "var ", "function"]),
        ("invalid", vec!["script", "var ", "function", "validation"]),
        ("inválido", vec!["script", "var ", "function"]),
        // Skip timeout if it's in JavaScript contexts - be more lenient
        ("timeout", vec!["var ", "settimeout", "script", "timeout =", "timeout=", ".timeout"]), 
        ("expired", vec!["script", "var ", "function"]),
        ("expirado", vec!["script", "var ", "function"])
    ];
    
    for (pattern, skip_contexts) in generic_error_patterns {
        if all_text.contains(pattern) {
            // Check if this error word appears in a context we should skip
            // Use case-insensitive matching for better detection
            let should_skip = skip_contexts.iter().any(|context| {
                // Check both with space and without space, case-insensitive
                let context_lower = context.to_lowercase();
                all_text.contains(&format!("{} {}", context_lower, pattern)) ||
                all_text.contains(&format!("{}{}", context_lower, pattern)) ||
                // Also check if the pattern appears near the context (within 10 chars)
                {
                    if let Some(pos) = all_text.find(pattern) {
                        let start = pos.saturating_sub(20);
                        let context_slice = &all_text[start..pos];
                        context_slice.contains(&context_lower)
                    } else {
                        false
                    }
                }
            });
            
            if !should_skip {
                return Some(format!("Detected error pattern: {}", pattern));
            }
        }
    }
    
    // Check if the document is suspiciously short (might be an error page)
    if all_text.len() < 500 && !all_text.contains("factura") && !all_text.contains("invoice") {
        return Some("Document too short or missing expected content".to_string());
    }
    
    None
}

/// Extracts key-value data from the main invoice info using updated selectors.
pub fn extract_main_info(html_content: &str) -> Result<ExtractedData> {
    let document = Html::parse_document(html_content);
    
    // Check for MEF error messages first
    if let Some(error_msg) = check_for_mef_errors(&document) {
        return Err(anyhow::anyhow!("Error de MEF: {}", error_msg));
    }
    
    let mut header = HashMap::new();

    if let Some(no) = extract_invoice_number(&document) {
        header.insert("no".to_string(), no);
    }
    if let Some(date) = extract_invoice_date(&document) {
        header.insert("date".to_string(), date);
    }
    if let Some(cufe) = extract_cufe(&document) {
        header.insert("cufe".to_string(), cufe);
    }

    let emisor_data = extract_panel_data(&document, "EMISOR");
    header.extend(emisor_data);

    let receptor_data = extract_panel_data(&document, "RECEPTOR");
    header.extend(receptor_data);

    let totals_data = extract_totals_data(&document);
    header.extend(totals_data);

    let details = extract_line_items(&document);

    // ✅ VALIDACIÓN ESTRICTA: Verificar campos críticos obligatorios
    let required_fields = vec![
        ("cufe", "CUFE"),
        ("no", "Número de factura"),
        ("date", "Fecha de factura"),
        ("emisor_name", "Nombre del emisor"),
        ("emisor_ruc", "RUC del emisor"),
    ];
    
    let mut missing_fields = Vec::new();
    for (field_key, field_name) in required_fields {
        if !header.contains_key(field_key) || header.get(field_key).map_or(true, |v| v.is_empty()) {
            missing_fields.push(field_name);
        }
    }
    
    if !missing_fields.is_empty() {
        return Err(anyhow::anyhow!(
            "Campos obligatorios faltantes o vacíos: {}. La factura puede no estar procesada en el MEF aún o los datos son incompletos.",
            missing_fields.join(", ")
        ));
    }
    
    // Validar que el monto total exista y no sea vacío
    if !header.contains_key("tot_amount") || header.get("tot_amount").map_or(true, |v| v.is_empty()) {
        return Err(anyhow::anyhow!(
            "Monto total no encontrado o vacío. La factura puede no estar procesada completamente en el MEF."
        ));
    }

    Ok(ExtractedData { header, details })
}

/// Extracts the invoice number using a structure-based approach, as documented.
/// Implements the strategy: Find h4 with "FACTURA" and navigate to h5 sibling with "No."
/// XPath equivalent: //h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-left')]//h5
fn extract_invoice_number(document: &Html) -> Option<String> {
    let h4_selector = Selector::parse("h4").ok()?;
    let h5_selector = Selector::parse("h5").ok()?;
    
    // Find h4 containing "FACTURA" and navigate to row container
    for h4 in document.select(&h4_selector) {
        let h4_text = h4.text().collect::<String>().to_uppercase();
        if h4_text.contains("FACTURA") {
            // Navigate up to find the row container
            let mut row_container = h4.parent();
            for _ in 0..3 {
                if let Some(parent) = row_container {
                    if let Some(parent_elem) = ElementRef::wrap(parent) {
                        let has_row_class = parent_elem.value().attr("class")
                            .map(|c| c.contains("row"))
                            .unwrap_or(false);
                        
                        if has_row_class {
                            // Look for h5 with invoice number in this row
                            for h5 in parent_elem.select(&h5_selector) {
                                let h5_text = h5.text().collect::<String>().trim().to_string();
                                
                                // Extract invoice number: "No. 0000181356" or just "0000181356"
                                if h5_text.to_uppercase().contains("NO.") {
                                    if let Some(no_idx) = h5_text.to_uppercase().find("NO.") {
                                        let after_no = &h5_text[no_idx + 3..].trim();
                                        if after_no.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
                                            return Some(after_no.trim().to_string());
                                        }
                                    }
                                } else if h5_text.chars().all(|c| c.is_ascii_digit()) && h5_text.len() == 10 {
                                    return Some(h5_text);
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
            break;
        }
    }
    None
}

/// Extracts the invoice date using a structure-based approach, as documented.
/// Implements the strategy: Find h4 with "FACTURA" and navigate to h5 sibling in div.text-right
/// XPath equivalent: //h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-right')]//h5/text()
fn extract_invoice_date(document: &Html) -> Option<String> {
    let h4_selector = Selector::parse("h4").ok()?;
    let h5_selector = Selector::parse("h5").ok()?;
    
    // Find h4 containing "FACTURA" and navigate to row container
    for h4 in document.select(&h4_selector) {
        let h4_text = h4.text().collect::<String>().to_uppercase();
        if h4_text.contains("FACTURA") {
            // Navigate up to find the row container
            let mut row_container = h4.parent();
            for _ in 0..3 {
                if let Some(parent) = row_container {
                    if let Some(parent_elem) = ElementRef::wrap(parent) {
                        let has_row_class = parent_elem.value().attr("class")
                            .map(|c| c.contains("row"))
                            .unwrap_or(false);
                        
                        if has_row_class {
                            // Look for h5 with date pattern in this row
                            for h5 in parent_elem.select(&h5_selector) {
                                let h5_text = h5.text().collect::<String>().trim().to_string();
                                
                                // Match pattern: DD/MM/YYYY or DD/MM/YYYY HH:MM:SS
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
                                                return Some(h5_text);
                                            }
                                        } else {
                                            // Date only, add default time
                                            return Some(format!("{} 00:00:00", h5_text));
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
            break;
        }
    }
    None
}

/// Extracts CUFE using a structure-based approach, as documented.
/// Implements the strategy: Find dt with "CÓDIGO ÚNICO DE FACTURA ELECTRÓNICA [CUFE]" and extract dd sibling
/// XPath equivalent: //dt[contains(text(), 'CÓDIGO ÚNICO DE FACTURA ELECTRÓNICA') and contains(text(), 'CUFE')]/following-sibling::dd/text()
fn extract_cufe(document: &Html) -> Option<String> {
    let dt_selector = Selector::parse("dt").ok()?;
    
    for dt in document.select(&dt_selector) {
        let dt_text = dt.text().collect::<String>().to_uppercase();
        if dt_text.contains("CÓDIGO ÚNICO") && dt_text.contains("CUFE") {
            // Buscar dd hermano siguiente
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

/// Extracts data from EMISOR and RECEPTOR panels using structure-based approach.
/// Implements the strategy: Find panel-heading with specified title, navigate to panel-body, extract dt/dd pairs
/// XPath equivalent: //div[contains(@class, 'panel-heading') and text()='PANEL_TITLE']/following-sibling::div[contains(@class, 'panel-body')]//dt[text()='FIELD']/following-sibling::dd/text()
fn extract_panel_data(document: &Html, panel_title: &str) -> HashMap<String, String> {
    let mut data = HashMap::new();
    let panel_heading_selector = Selector::parse("div.panel-heading").unwrap();
    
    for panel_heading in document.select(&panel_heading_selector) {
        let heading_text = panel_heading.text().collect::<String>().trim().to_uppercase();
        // Use .contains() for flexibility with surrounding text/whitespace
        if heading_text.contains(panel_title) {
            // Search for the next sibling that is a panel-body
            let mut current = panel_heading.next_sibling();
            while let Some(node) = current {
                if let Some(element) = ElementRef::wrap(node) {
                    if element.value().attr("class").unwrap_or("").contains("panel-body") {
                        extract_dt_dd_pairs(&element, &mut data, panel_title);
                        break; // Found and processed, exit loop
                    }
                }
                current = node.next_sibling();
            }
        }
    }
    data
}

/// Helper function to extract dt/dd pairs from a panel-body element
fn extract_dt_dd_pairs(panel_body: &ElementRef, data: &mut HashMap<String, String>, panel_title: &str) {
    let dt_selector = Selector::parse("dt").unwrap();
    
    for dt in panel_body.select(&dt_selector) {
        let key = dt.text().collect::<String>().trim().to_lowercase();
        if let Some(dd) = dt.next_sibling_element() {
            if dd.value().name() == "dd" {
                let value = dd.text().collect::<String>().trim().to_string();
                let mapped_key = match key.as_str() {
                    "nombre" => format!("{}_name", panel_title.to_lowercase()),
                    "ruc" | "cédula de identidad" => format!("{}_ruc", panel_title.to_lowercase()),
                    "dv" => format!("{}_dv", panel_title.to_lowercase()),
                    "dirección" => format!("{}_address", panel_title.to_lowercase()),
                    "teléfono" => format!("{}_phone", panel_title.to_lowercase()),
                    _ => key,
                };
                data.insert(mapped_key, value);
            }
        }
    }
}

/// Extracts total amounts from the summary table using structure-based approach.
/// Implements the strategy: Find td elements that contain specific text patterns and extract div child values
/// XPath equivalent: //td[contains(text(), 'VALOR TOTAL:')]/div/text()
fn extract_totals_data(document: &Html) -> HashMap<String, String> {
    let mut data = HashMap::new();
    // The `colspan` attribute is not always present or consistent, removing it makes the selector more robust.
    let td_selector = Selector::parse("td.text-right").unwrap();

    for td in document.select(&td_selector) {
        let text = td.text().collect::<String>().to_uppercase();
        let div_selector = Selector::parse("div").unwrap();

        if let Some(div) = td.select(&div_selector).next() {
            let value = div.text().collect::<String>().trim().to_string();
            if text.contains("VALOR TOTAL:") {
                data.insert("tot_amount".to_string(), value);
            } else if text.contains("ITBMS TOTAL:") {
                data.insert("tot_itbms".to_string(), value);
            } else if text.contains("VUELTO:") {
                data.insert("vuelto".to_string(), value);
            } else if text.contains("TOTAL PAGADO:") {
                data.insert("total_pagado".to_string(), value);
            }
        }
    }
    data
}

/// Extracts line items from the invoice details table using structure-based approach.
/// Implements the strategy: Find tbody tr elements in detalle section, extract td with data-title attributes
/// XPath equivalent: //td[@data-title='FIELD_NAME']/text()
fn extract_line_items(document: &Html) -> Vec<HashMap<String, String>> {
    let mut items = Vec::new();
    let tr_selector = Selector::parse("div.panel-body.collapse.in tbody tr").unwrap();
    let td_selector = Selector::parse("td[data-title]").unwrap();

    for row in document.select(&tr_selector) {
        let mut item = HashMap::new();
        for td in row.select(&td_selector) {
            if let Some(data_title) = td.value().attr("data-title") {
                let value = td.text().collect::<String>().trim().to_string();
                let mapped_key = match data_title {
                    "Cantidad" => "quantity",
                    "Código" => "code",
                    "Descripción" => "description",
                    "Descuento" => "unit_discount",
                    "Precio" => "unit_price",
                    "Impuesto" => "itbms",
                    "Información de interés" => "information_of_interest",
                    "Monto" => "amount",
                    "Total" => "total",
                    "Linea" => "linea",
                    _ => data_title,
                };
                item.insert(mapped_key.to_string(), value);
            }
        }
        if !item.is_empty() {
            items.push(item);
        }
    }
    items
}

// Tests removed as requested by user to simplify the codebase
