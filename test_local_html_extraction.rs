use std::collections::HashMap;
use std::fs;

// Importar el módulo de extracción
mod processing {
    pub mod web_scraping {
        pub mod ocr_extractor;
    }
}

use processing::web_scraping::ocr_extractor::extract_main_info;

fn test_html_file(file_path: &str, expected_values: HashMap<&str, &str>) {
    println!("\n🧪 Testing file: {}", file_path);
    println!("=" .repeat(50));
    
    // Leer el archivo HTML
    let html_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            println!("❌ Error reading file {}: {}", file_path, e);
            return;
        }
    };
    
    println!("✅ HTML file loaded successfully ({} chars)", html_content.len());
    
    // Extraer información
    let extracted_data = match extract_main_info(&html_content) {
        Ok(data) => data,
        Err(e) => {
            println!("❌ Error extracting data: {}", e);
            return;
        }
    };
    
    println!("\n📊 Extracted data:");
    for (key, value) in &extracted_data {
        println!("  {}: {}", key, value);
    }
    
    println!("\n🔍 Validation Results:");
    for (field, expected) in expected_values {
        let actual = extracted_data.get(field).unwrap_or(&"NOT_FOUND".to_string());
        let status = if actual == expected { "✅" } else { "❌" };
        
        println!("  {} {}: Expected '{}', Got '{}'", status, field, expected, actual);
        
        if actual != expected {
            println!("    🔍 MISMATCH in field: {}", field);
        }
    }
}

fn main() {
    println!("🚀 Testing Local HTML Extraction");
    println!("=====================================\n");
    
    // Test webscrapy_htmlsample1.html
    let mut expected_sample1 = HashMap::new();
    expected_sample1.insert("numero_factura", "0031157014");
    expected_sample1.insert("fecha", "15/05/2025 09:50:04");
    // Agregar más campos esperados según la documentación
    
    test_html_file("webscrapy_htmlsample1.html", expected_sample1);
    
    // Test webscrapy_htmlsample2.html
    let mut expected_sample2 = HashMap::new();
    expected_sample2.insert("numero_factura", "0031157014");
    expected_sample2.insert("fecha", "15/05/2025 09:50:04");
    
    test_html_file("webscrapy_htmlsample2.html", expected_sample2);
    
    println!("\n🏁 Test completed!");
}
