/// Test de validación de extracción de campos usando webscrapy_htmlsample1.html
/// 
/// Este test valida que todos los campos se extraigan correctamente 
/// usando el archivo HTML de muestra real.
use std::fs;
use std::collections::HashMap;

// Import our extraction function
use lum_rust_ws::processing::web_scraping::ocr_extractor::extract_main_info;

fn main() {
    println!("🧪 INICIANDO TEST DE VALIDACIÓN DE EXTRACCIÓN");
    println!("===============================================");
    
    // Leer el archivo HTML de muestra
    let html_content = match fs::read_to_string("webscrapy_htmlsample1.html") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Error leyendo archivo HTML: {}", e);
            return;
        }
    };
    
    println!("✅ Archivo HTML cargado: {} caracteres", html_content.len());
    
    // Ejecutar extracción
    let extracted_data = match extract_main_info(&html_content) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("❌ Error en extracción: {}", e);
            return;
        }
    };
    
    println!("\n📊 RESULTADOS DE EXTRACCIÓN:");
    println!("============================");
    
    // Valores esperados según la documentación
    let expected_values = HashMap::from([
        ("numero", "0031157014"),
        ("fecha", "15/05/2025 09:50:04"),
        ("cufe", "FE01200002679372-1-844914-7300002025051500311570140020317481978892"),
        ("emisor_nombre", "DELIVERY HERO PANAMA (E- COMMERCE) S.A."),
        ("emisor_ruc", "2679372-1-844914"),
        ("emisor_dv", "73"),
        ("emisor_direccion", "Corregimiento de SAN FRANCISCO, Edificio MIDTOWN, Apartamento local PISO 13"),
        ("emisor_telefono", "269-2641"),
        ("total_amount", "2.68"),
        ("total_itbms", "0.18"),
    ]);
    
    let mut total_tests = 0;
    let mut passed_tests = 0;
    
    // Validar cada campo
    for (field_key, expected_value) in &expected_values {
        total_tests += 1;
        
        match extracted_data.get(*field_key) {
            Some(actual_value) => {
                if actual_value == expected_value {
                    println!("✅ {}: '{}' (CORRECTO)", field_key, actual_value);
                    passed_tests += 1;
                } else {
                    println!("❌ {}: esperado '{}', obtenido '{}' (INCORRECTO)", 
                             field_key, expected_value, actual_value);
                }
            },
            None => {
                println!("❌ {}: esperado '{}', NO EXTRAÍDO", field_key, expected_value);
            }
        }
    }
    
    // Mostrar campos adicionales extraídos
    println!("\n📋 CAMPOS ADICIONALES EXTRAÍDOS:");
    println!("=================================");
    for (key, value) in &extracted_data {
        if !expected_values.contains_key(key.as_str()) {
            println!("ℹ️  {}: '{}'", key, value);
        }
    }
    
    // Resumen
    println!("\n📈 RESUMEN DEL TEST:");
    println!("==================");
    println!("Total de campos esperados: {}", total_tests);
    println!("Campos extraídos correctamente: {}", passed_tests);
    println!("Campos fallidos: {}", total_tests - passed_tests);
    println!("Porcentaje de éxito: {:.1}%", (passed_tests as f64 / total_tests as f64) * 100.0);
    
    if passed_tests == total_tests {
        println!("🎉 ¡TODOS LOS TESTS PASARON!");
    } else {
        println!("⚠️  Algunos tests fallaron. Revisar implementación.");
    }
    
    println!("\n🔍 DEBUG: Todos los datos extraídos:");
    for (key, value) in &extracted_data {
        println!("  {}: '{}'", key, value);
    }
}
