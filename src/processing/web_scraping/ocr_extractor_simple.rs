use anyhow::Result;
use sxd_document::{dom::Document, parser};
use sxd_xpath::{Context, Factory, Value};
use std::collections::HashMap;

/// Extracts key-value data from the main invoice info using XPath expressions.
/// All extraction logic migrated from CSS selectors to XPath for robustness.
pub fn extract_main_info(html_content: &str) -> Result<HashMap<String, String>> {
    let package = parser::parse(html_content)
        .map_err(|e| anyhow::anyhow!("Error parsing HTML: {}", e))?;
    let document = package.as_document();
    
    let mut data = HashMap::new();
    
    // 1. Extraer número de factura
    if let Some(invoice_no) = extract_xpath_text(&document, "//h5[contains(text(), 'No.')]") {
        if let Some(cleaned) = invoice_no.strip_prefix("No.") {
            let result = cleaned.trim().to_string();
            if !result.is_empty() {
                data.insert("numero_factura".to_string(), result);
            }
        }
    }
    
    // 2. Extraer fecha usando múltiples estrategias XPath
    if let Some(date) = extract_invoice_date(&document) {
        data.insert("fecha".to_string(), date);
    }
    
    // 3. Extraer CUFE
    if let Some(cufe) = extract_xpath_text(&document, "//dt[contains(text(), 'CÓDIGO ÚNICO DE FACTURA ELECTRÓNICA') and contains(text(), 'CUFE')]/following-sibling::dd") {
        if cufe.starts_with("FE") && cufe.len() > 50 {
            data.insert("cufe".to_string(), cufe);
        }
    }
    
    // 4. Extraer datos del emisor
    let emisor_fields = [
        ("NOMBRE", "emisor_nombre"),
        ("RUC", "emisor_ruc"),
        ("DV", "emisor_dv"),
        ("DIRECCIÓN", "emisor_direccion"),
        ("TELÉFONO", "emisor_telefono"),
    ];
    
    for (dt_text, data_key) in emisor_fields {
        let xpath_expr = format!(
            "//div[contains(@class, 'panel-heading') and text()='EMISOR']/following-sibling::div[contains(@class, 'panel-body')]//dt[text()='{}']/following-sibling::dd",
            dt_text
        );
        
        if let Some(text) = extract_xpath_text(&document, &xpath_expr) {
            data.insert(data_key.to_string(), text);
        }
    }
    
    // 5. Extraer totales
    if let Some(text) = extract_xpath_text(&document, "//td[contains(text(), 'Valor Total:')]/div") {
        data.insert("total_amount".to_string(), text);
    }
    
    if let Some(text) = extract_xpath_text(&document, "//td[contains(text(), 'ITBMS Total:')]/div") {
        data.insert("total_itbms".to_string(), text);
    }
    
    if data.is_empty() {
        return Err(anyhow::anyhow!("No se pudieron extraer datos principales de la factura"));
    }
    
    Ok(data)
}

/// Helper function para extraer texto usando XPath
fn extract_xpath_text(document: &Document, xpath_expr: &str) -> Option<String> {
    let factory = Factory::new();
    let context = Context::new();
    
    let xpath = factory.build(xpath_expr).ok()?;
    let result = xpath.evaluate(&context, document.root()).ok()?;
    
    if let Value::Nodeset(nodes) = result {
        for node in nodes {
            let text = node.string_value().trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// Extrae la fecha usando múltiples estrategias XPath
fn extract_invoice_date(document: &Document) -> Option<String> {
    // Estrategia 1: XPath directo por clase
    if let Some(text) = extract_xpath_text(document, "//div[contains(@class, 'col-sm-4') and contains(@class, 'text-right')]//h5") {
        if is_valid_date_format(&text) {
            return Some(normalize_date_format(&text));
        }
    }
    
    // Estrategia 2: XPath por estructura navegando desde FACTURA
    if let Some(text) = extract_xpath_text(document, "//h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-right')]//h5") {
        if is_valid_date_format(&text) {
            return Some(normalize_date_format(&text));
        }
    }
    
    // Estrategia 3: XPath con validación de formato - buscar h5 que contenga fecha
    if let Some(text) = extract_xpath_text(document, "//h5[contains(text(), '/') and contains(text(), ':')]") {
        if is_valid_date_format(&text) {
            return Some(normalize_date_format(&text));
        }
    }
    
    None
}

/// Valida si el texto tiene formato de fecha DD/MM/YYYY HH:MM:SS o DD/MM/YYYY
fn is_valid_date_format(text: &str) -> bool {
    let slash_count = text.matches('/').count();
    let colon_count = text.matches(':').count();
    
    // Formato completo: DD/MM/YYYY HH:MM:SS
    if slash_count == 2 && colon_count == 2 {
        return true;
    }
    
    // Solo fecha: DD/MM/YYYY
    if slash_count == 2 && colon_count == 0 {
        return true;
    }
    
    false
}

/// Normaliza el formato de fecha agregando hora por defecto si es necesario
fn normalize_date_format(text: &str) -> String {
    if text.matches(':').count() == 0 && text.matches('/').count() == 2 {
        format!("{} 00:00:00", text)
    } else {
        text.to_string()
    }
}

/// Extracts line items from the invoice details table using XPath.
pub fn extract_line_items(html_content: &str) -> Result<Vec<HashMap<String, String>>> {
    let package = parser::parse(html_content)
        .map_err(|e| anyhow::anyhow!("Error parsing HTML: {}", e))?;
    let document = package.as_document();
    
    let factory = Factory::new();
    let context = Context::new();
    
    let mut items = Vec::new();
    
    // Buscar todas las filas que contengan td[@data-title='Cantidad']
    let xpath = factory.build("//tr[td[@data-title='Cantidad']]")
        .map_err(|e| anyhow::anyhow!("Error building XPath: {}", e))?;
    
    let result = xpath.evaluate(&context, document.root())
        .map_err(|e| anyhow::anyhow!("Error evaluating XPath: {}", e))?;
    
    if let Value::Nodeset(row_nodes) = result {
        for row_node in row_nodes {
            let mut item = HashMap::new();
            
            // Extraer todos los campos de esta fila
            let fields = [
                ("Cantidad", "quantity"),
                ("Código", "code"),
                ("Descripción", "description"),
                ("Descuento", "unit_discount"),
                ("Precio", "unit_price"),
                ("Impuesto", "itbms"),
                ("Información de interés", "information_of_interest"),
            ];
            
            for (data_title, data_key) in fields {
                let xpath_expr = format!(".//td[@data-title='{}']", data_title);
                
                if let Ok(field_xpath) = factory.build(&xpath_expr) {
                    if let Ok(field_result) = field_xpath.evaluate(&context, row_node) {
                        if let Value::Nodeset(field_nodes) = field_result {
                            for field_node in field_nodes {
                                let text = field_node.string_value().trim().to_string();
                                item.insert(data_key.to_string(), text);
                                break;
                            }
                        }
                    }
                }
            }
            
            if !item.is_empty() {
                items.push(item);
            }
        }
    }
    
    if items.is_empty() {
        return Err(anyhow::anyhow!("No se encontraron ítems de detalle en la factura"));
    }
    
    Ok(items)
}

/// Extracts payment data from the invoice using XPath.
pub fn extract_payment_data(html_content: &str) -> Result<HashMap<String, String>> {
    let package = parser::parse(html_content)
        .map_err(|e| anyhow::anyhow!("Error parsing HTML: {}", e))?;
    let document = package.as_document();
    
    let mut payment_data = HashMap::new();
    
    // Extraer vuelto
    if let Some(text) = extract_xpath_text(&document, "//td[contains(text(), 'Vuelto:')]/div") {
        payment_data.insert("vuelto".to_string(), text);
    }
    
    // Extraer total pagado
    if let Some(text) = extract_xpath_text(&document, "//td[contains(text(), 'TOTAL PAGADO:')]/div") {
        payment_data.insert("total_pagado".to_string(), text);
    }
    
    Ok(payment_data)
}
