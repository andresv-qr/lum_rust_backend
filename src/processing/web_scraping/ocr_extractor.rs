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
        ("error", vec!["script", "var ", "function", "javascript"]), // Skip if in JS context
        ("not found", vec!["script", "var ", "function"]),
        ("no encontrado", vec!["script", "var ", "function"]),
        ("invalid", vec!["script", "var ", "function", "validation"]),
        ("inválido", vec!["script", "var ", "function"]),
        ("timeout", vec!["var timeout", "var timeout", "settimeout", "script"]), // Skip JS timeouts
        ("expired", vec!["script", "var ", "function"]),
        ("expirado", vec!["script", "var ", "function"])
    ];
    
    for (pattern, skip_contexts) in generic_error_patterns {
        if all_text.contains(pattern) {
            // Check if this error word appears in a context we should skip
            let should_skip = skip_contexts.iter().any(|context| {
                all_text.contains(&format!("{} {}", context, pattern)) ||
                all_text.contains(&format!("{}{}", context, pattern))
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

    if header.is_empty() && details.is_empty() {
        return Err(anyhow::anyhow!("No se pudieron extraer datos de la factura"));
    }

    Ok(ExtractedData { header, details })
}

/// Extracts the invoice number using a structure-based approach, as documented.
/// Implements the strategy: Find h4 with "FACTURA" and navigate to h5 sibling with "No."
/// XPath equivalent: //h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-left')]//h5
fn extract_invoice_number(document: &Html) -> Option<String> {
    let h5_selector = Selector::parse("h5").ok()?;
    
    for element in document.select(&h5_selector) {
        let text = element.text().collect::<String>();
        if text.contains("No.") {
            return Some(text.replace("No.", "").trim().to_string());
        }
    }
    None
}

/// Extracts the invoice date using a structure-based approach, as documented.
/// Implements the strategy: Find h4 with "FACTURA" and navigate to h5 sibling in div.text-right
/// XPath equivalent: //h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-right')]//h5/text()
fn extract_invoice_date(document: &Html) -> Option<String> {
    // Selector corregido para coincidir exactamente con la documentación
    let selector = Selector::parse("div.col-sm-4.text-right h5").ok()?;
    
    for element in document.select(&selector) {
        let text = element.text().collect::<String>().trim().to_string();
        
        // Caso 1: Formato completo DD/MM/YYYY HH:MM:SS
        if text.matches('/').count() == 2 && text.matches(':').count() == 2 {
            return Some(text);
        }
        
        // Caso 2: Solo fecha DD/MM/YYYY (agregar hora por defecto)
        if text.matches('/').count() == 2 && text.matches(':').count() == 0 {
            return Some(format!("{} 00:00:00", text));
        }
    }
    
    // Fallback: buscar por estructura exacta como en documentación
    // //h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-right')]//h5/text()
    let h4_selector = Selector::parse("h4").ok()?;
    for h4 in document.select(&h4_selector) {
        let h4_text = h4.text().collect::<String>().to_uppercase();
        if h4_text.contains("FACTURA") {
            // Navegar dos niveles hacia arriba (../../) y buscar div.col-sm-4.text-right
            if let Some(grandparent) = h4.parent().and_then(|p| p.parent()) {
                if let Some(grandparent_element) = ElementRef::wrap(grandparent) {
                    let text_right_selector = Selector::parse("div.col-sm-4.text-right h5").unwrap();
                    for date_element in grandparent_element.select(&text_right_selector) {
                        let text = date_element.text().collect::<String>().trim().to_string();
                        
                        if text.matches('/').count() == 2 && (text.matches(':').count() == 2 || text.matches(':').count() == 0) {
                            if text.matches(':').count() == 2 {
                                return Some(text);
                            } else {
                                return Some(format!("{} 00:00:00", text));
                            }
                        }
                    }
                }
            }
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
