use anyhow::Result;
use sxd_document::parser;
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
    if let Some(invoice_no) = extract_invoice_number(&document) {
        data.insert("numero_factura".to_string(), invoice_no);
    }
    
    // 2. Extraer fecha
    if let Some(date) = extract_invoice_date(&document) {
        data.insert("fecha".to_string(), date);
    }
    
    // 3. Extraer CUFE
    if let Some(cufe) = extract_cufe(&document) {
        data.insert("cufe".to_string(), cufe);
    }
    
    // 4. Extraer datos del emisor
    extract_emisor_data(&document, &mut data);
    
    // 5. Extraer totales
    extract_totals_data(&document, &mut data);
    
    if data.is_empty() {
        return Err(anyhow::anyhow!("No se pudieron extraer datos principales de la factura"));
    }
    
    Ok(data)
}

/// Extrae el número de factura usando XPath
/// XPath: //h5[contains(text(), 'No.')]/text()
/// XPath alternativo: //div[contains(@class, 'col-sm-4') and contains(@class, 'text-left')]//h5[contains(text(), 'No.')]/text()
fn extract_invoice_number(document: &sxd_document::Document) -> Option<String> {
    let factory = Factory::new();
    let context = Context::new();
    
    // Estrategia 1: XPath directo por contenido
    if let Ok(xpath) = factory.build("//h5[contains(text(), 'No.')]/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    if let Some(text) = node.string_value().strip_prefix("No.") {
                        let cleaned = text.trim().to_string();
                        if !cleaned.is_empty() {
                            return Some(cleaned);
                        }
                    }
                }
            }
        }
    }
    
    // Estrategia 2: XPath por estructura específica
    if let Ok(xpath) = factory.build("//div[contains(@class, 'col-sm-4') and contains(@class, 'text-left')]//h5[contains(text(), 'No.')]/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    if let Some(text) = node.string_value().strip_prefix("No.") {
                        let cleaned = text.trim().to_string();
                        if !cleaned.is_empty() {
                            return Some(cleaned);
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Extrae la fecha usando XPath
/// XPath principal: //div[contains(@class, 'col-sm-4') and contains(@class, 'text-right')]//h5/text()
/// XPath por estructura: //h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-right')]//h5/text()
fn extract_invoice_date(document: &sxd_document::Document) -> Option<String> {
    let factory = Factory::new();
    let context = Context::new();
    
    // Estrategia 1: XPath directo por clase
    if let Ok(xpath) = factory.build("//div[contains(@class, 'col-sm-4') and contains(@class, 'text-right')]//h5/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if is_valid_date_format(text) {
                        return Some(normalize_date_format(text));
                    }
                }
            }
        }
    }
    
    // Estrategia 2: XPath por estructura navegando desde FACTURA
    if let Ok(xpath) = factory.build("//h4[contains(text(), 'FACTURA')]/../../div[contains(@class, 'text-right')]//h5/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if is_valid_date_format(text) {
                        return Some(normalize_date_format(text));
                    }
                }
            }
        }
    }
    
    // Estrategia 3: XPath con validación de formato
    if let Ok(xpath) = factory.build("//h5[contains(text(), '/') and contains(text(), ':')]/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if is_valid_date_format(text) {
                        return Some(normalize_date_format(text));
                    }
                }
            }
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

/// Extrae CUFE usando XPath
/// XPath: //dt[contains(text(), 'CÓDIGO ÚNICO DE FACTURA ELECTRÓNICA') and contains(text(), 'CUFE')]/following-sibling::dd/text()
fn extract_cufe(document: &sxd_document::Document) -> Option<String> {
    let factory = Factory::new();
    let context = Context::new();
    
    // Estrategia 1: XPath por etiqueta específica
    if let Ok(xpath) = factory.build("//dt[contains(text(), 'CÓDIGO ÚNICO DE FACTURA ELECTRÓNICA') and contains(text(), 'CUFE')]/following-sibling::dd/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if text.starts_with("FE") && text.len() > 50 {
                        return Some(text.to_string());
                    }
                }
            }
        }
    }
    
    // Estrategia 2: XPath por patrón de CUFE
    if let Ok(xpath) = factory.build("//dd[starts-with(text(), 'FE') and string-length(text()) > 50]/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if text.starts_with("FE") && text.len() > 50 {
                        return Some(text.to_string());
                    }
                }
            }
        }
    }
    
    None
}

/// Extrae datos del emisor usando XPath
/// XPath base: //div[contains(@class, 'panel-heading') and text()='EMISOR']/following-sibling::div[contains(@class, 'panel-body')]//dt[text()='CAMPO']/following-sibling::dd/text()
fn extract_emisor_data(document: &sxd_document::Document, data: &mut HashMap<String, String>) {
    let factory = Factory::new();
    let context = Context::new();
    
    let fields = [
        ("NOMBRE", "emisor_nombre"),
        ("RUC", "emisor_ruc"),
        ("DV", "emisor_dv"),
        ("DIRECCIÓN", "emisor_direccion"),
        ("TELÉFONO", "emisor_telefono"),
    ];
    
    for (dt_text, data_key) in fields {
        let xpath_expr = format!(
            "//div[contains(@class, 'panel-heading') and text()='EMISOR']/following-sibling::div[contains(@class, 'panel-body')]//dt[text()='{}']/following-sibling::dd/text()",
            dt_text
        );
        
        if let Ok(xpath) = factory.build(&xpath_expr) {
            if let Ok(result) = xpath.evaluate(&context, document.root()) {
                if let Value::Nodeset(nodes) = result {
                    for node in nodes {
                        let text = node.string_value().trim();
                        if !text.is_empty() {
                            data.insert(data_key.to_string(), text.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Extrae datos de totales usando XPath
/// XPath para monto total: //td[contains(text(), 'Valor Total:')]/div/text()
/// XPath para ITBMS: //td[contains(text(), 'ITBMS Total:')]/div/text()
fn extract_totals_data(document: &sxd_document::Document, data: &mut HashMap<String, String>) {
    let factory = Factory::new();
    let context = Context::new();
    
    // Extraer monto total
    if let Ok(xpath) = factory.build("//td[contains(text(), 'Valor Total:')]/div/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if !text.is_empty() {
                        data.insert("total_amount".to_string(), text.to_string());
                        break;
                    }
                }
            }
        }
    }
    
    // Extraer ITBMS total
    if let Ok(xpath) = factory.build("//td[contains(text(), 'ITBMS Total:')]/div/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if !text.is_empty() {
                        data.insert("total_itbms".to_string(), text.to_string());
                        break;
                    }
                }
            }
        }
    }
}

/// Extracts line items from the invoice details table using XPath.
/// XPath base: //td[@data-title='Cantidad'] para identificar filas de ítems
pub fn extract_line_items(html_content: &str) -> Result<Vec<HashMap<String, String>>> {
    let package = parser::parse(html_content)
        .map_err(|e| anyhow::anyhow!("Error parsing HTML: {}", e))?;
    let document = package.as_document();
    
    let factory = Factory::new();
    let context = Context::new();
    
    let mut items = Vec::new();
    
    // Buscar todas las filas que contengan td con data-title="Cantidad"
    if let Ok(xpath) = factory.build("//td[@data-title='Cantidad']") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(quantity_nodes) = result {
                for quantity_node in quantity_nodes {
                    let mut item = HashMap::new();
                    
                    // Obtener la fila padre de este td
                    if let Ok(row_xpath) = factory.build("./ancestor::tr[1]") {
                        if let Ok(row_result) = row_xpath.evaluate(&context, quantity_node) {
                            if let Value::Nodeset(row_nodes) = row_result {
                                if let Some(row_node) = row_nodes.into_iter().next() {
                                    // Extraer todos los campos de esta fila
                                    extract_item_fields(&factory, &context, row_node, &mut item);
                                }
                            }
                        }
                    }
                    
                    if !item.is_empty() {
                        items.push(item);
                    }
                }
            }
        }
    }
    
    if items.is_empty() {
        return Err(anyhow::anyhow!("No se encontraron ítems de detalle en la factura"));
    }
    
    Ok(items)
}

/// Extrae todos los campos de un ítem usando XPath
fn extract_item_fields(
    factory: &Factory,
    context: &Context,
    row_node: sxd_xpath::Node,
    item: &mut HashMap<String, String>
) {
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
        
        if let Ok(xpath) = factory.build(&xpath_expr) {
            if let Ok(result) = xpath.evaluate(context, row_node) {
                if let Value::Nodeset(nodes) = result {
                    for node in nodes {
                        let text = node.string_value().trim();
                        item.insert(data_key.to_string(), text.to_string());
                        break;
                    }
                }
            }
        }
    }
}

/// Extracts payment data from the invoice using XPath.
/// XPath para vuelto: //td[contains(text(), 'Vuelto:')]/div/text()
/// XPath para total pagado: //td[contains(text(), 'TOTAL PAGADO:')]/div/text()
pub fn extract_payment_data(html_content: &str) -> Result<HashMap<String, String>> {
    let package = parser::parse(html_content)
        .map_err(|e| anyhow::anyhow!("Error parsing HTML: {}", e))?;
    let document = package.as_document();
    
    let factory = Factory::new();
    let context = Context::new();
    
    let mut payment_data = HashMap::new();
    
    // Extraer vuelto
    if let Ok(xpath) = factory.build("//td[contains(text(), 'Vuelto:')]/div/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if !text.is_empty() {
                        payment_data.insert("vuelto".to_string(), text.to_string());
                        break;
                    }
                }
            }
        }
    }
    
    // Extraer total pagado
    if let Ok(xpath) = factory.build("//td[contains(text(), 'TOTAL PAGADO:')]/div/text()") {
        if let Ok(result) = xpath.evaluate(&context, document.root()) {
            if let Value::Nodeset(nodes) = result {
                for node in nodes {
                    let text = node.string_value().trim();
                    if !text.is_empty() {
                        payment_data.insert("total_pagado".to_string(), text.to_string());
                        break;
                    }
                }
            }
        }
    }
    
    Ok(payment_data)
}
