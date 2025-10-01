#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use crate::processing::web_scraping::ocr_extractor::extract_main_info;

    #[test]
    fn test_webscrapy_htmlsample1() {
        println!("\n🧪 Testing webscrapy_htmlsample1.html");
        
        // Leer el archivo HTML
        let html_content = fs::read_to_string("webscrapy_htmlsample1.html")
            .expect("Failed to read webscrapy_htmlsample1.html");
        
        println!("✅ HTML file loaded successfully ({} chars)", html_content.len());
        
        // Extraer información
        let extracted_data = extract_main_info(&html_content)
            .expect("Failed to extract data");
        
        println!("\n📊 Extracted data:");
        for (key, value) in &extracted_data {
            println!("  {}: {}", key, value);
        }
        
        // Validar campos esperados
        let expected_numero = "0031157014";
        let expected_fecha = "15/05/2025 09:50:04";
        
        let actual_numero = extracted_data.get("numero_factura").unwrap_or(&"NOT_FOUND".to_string());
        let actual_fecha = extracted_data.get("fecha").unwrap_or(&"NOT_FOUND".to_string());
        
        println!("\n🔍 Validation Results:");
        println!("  Número factura: Expected '{}', Got '{}'", expected_numero, actual_numero);
        println!("  Fecha: Expected '{}', Got '{}'", expected_fecha, actual_fecha);
        
        // Asserts
        assert_eq!(actual_numero, expected_numero, "Número de factura no coincide");
        assert_eq!(actual_fecha, expected_fecha, "Fecha no coincide");
    }

    #[test]
    fn test_webscrapy_htmlsample2() {
        println!("\n🧪 Testing webscrapy_htmlsample2.html");
        
        // Leer el archivo HTML
        let html_content = fs::read_to_string("webscrapy_htmlsample2.html")
            .expect("Failed to read webscrapy_htmlsample2.html");
        
        println!("✅ HTML file loaded successfully ({} chars)", html_content.len());
        
        // Extraer información
        let extracted_data = extract_main_info(&html_content)
            .expect("Failed to extract data");
        
        println!("\n📊 Extracted data:");
        for (key, value) in &extracted_data {
            println!("  {}: {}", key, value);
        }
        
        // Validar campos esperados (mismos valores que sample1)
        let expected_numero = "0031157014";
        let expected_fecha = "15/05/2025 09:50:04";
        
        let actual_numero = extracted_data.get("numero_factura").unwrap_or(&"NOT_FOUND".to_string());
        let actual_fecha = extracted_data.get("fecha").unwrap_or(&"NOT_FOUND".to_string());
        
        println!("\n🔍 Validation Results:");
        println!("  Número factura: Expected '{}', Got '{}'", expected_numero, actual_numero);
        println!("  Fecha: Expected '{}', Got '{}'", expected_fecha, actual_fecha);
        
        // Asserts
        assert_eq!(actual_numero, expected_numero, "Número de factura no coincide");
        assert_eq!(actual_fecha, expected_fecha, "Fecha no coincide");
    }
}
