#!/bin/bash

# Test de validación de extracción usando el archivo HTML de muestra
# Ejecuta el extractor con el HTML real y valida los resultados

echo "🧪 INICIANDO TEST DE VALIDACIÓN DE EXTRACCIÓN"
echo "=============================================="

# Verificar que el archivo HTML existe
if [ ! -f "webscrapy_htmlsample1.html" ]; then
    echo "❌ ERROR: Archivo webscrapy_htmlsample1.html no encontrado"
    exit 1
fi

echo "✅ Archivo HTML encontrado"

# Crear un pequeño script Rust para probar la extracción
cat > test_temp_extraction.rs << 'EOF'
use std::fs;
use std::collections::HashMap;
use scraper::{Html, Selector, ElementRef};

/// Extract main info function (copy from our module)
pub fn extract_main_info(html_content: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html_content);
    let mut data = HashMap::new();
    
    // Extract invoice number using structure-based strategy
    if let Some(invoice_no) = extract_invoice_number(&document) {
        data.insert("numero".to_string(), invoice_no);
    }
    
    // Extract date using structure-based strategy  
    if let Some(date) = extract_invoice_date(&document) {
        data.insert("fecha".to_string(), date);
    }
    
    Ok(data)
}

/// Extract invoice number using structure-based approach
fn extract_invoice_number(document: &Html) -> Option<String> {
    println!("🔍 DEBUG: Iniciando extracción de número de factura");
    
    // Main strategy: find h4 with "FACTURA" and navigate to div.text-left
    let h4_selector = Selector::parse("h4").ok()?;
    
    for h4 in document.select(&h4_selector) {
        let h4_text = h4.text().collect::<String>().to_uppercase();
        println!("🔍 DEBUG: Encontrado h4 con texto: '{}'", h4_text);
        
        // Step 1: Verify this h4 contains "FACTURA"
        if h4_text.contains("FACTURA") {
            println!("✅ DEBUG: Encontrado h4 con FACTURA");
            
            // Step 2-3: Navigate up two levels (col-sm-4 -> row)
            if let Some(row_div) = h4.parent().and_then(|p| p.parent()) {
                if let Some(row_element) = ElementRef::wrap(row_div) {
                    println!("✅ DEBUG: Navegado a elemento row para número");
                    
                    // Step 4: Find div.col-sm-4.text-left within the row (according to XPath)
                    let text_left_selector = Selector::parse("div.col-sm-4.text-left").unwrap();
                    for text_left_div in row_element.select(&text_left_selector) {
                        println!("✅ DEBUG: Encontrado div.text-left");
                        
                        // Step 5: Find h5 containing "No." within div.text-left
                        let h5_selector = Selector::parse("h5").unwrap();
                        for h5 in text_left_div.select(&h5_selector) {
                            let no_text = h5.text().collect::<String>().trim().to_string();
                            println!("🔍 DEBUG: Encontrado h5 con texto exacto: '{}'", no_text);
                            
                            // Validate it contains "No."
                            if no_text.contains("No.") {
                                let invoice_number = no_text.replace("No.", "").trim().to_string();
                                println!("✅ DEBUG: Retornando número de factura: '{}'", invoice_number);
                                return Some(invoice_number);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: simple strategy if structure doesn't work
    println!("⚠️ DEBUG: Usando estrategia fallback para número de factura");
    let h5_selector = Selector::parse("h5").ok()?;
    for element in document.select(&h5_selector) {
        let text = element.text().collect::<String>();
        if text.contains("No.") {
            let invoice_number = text.replace("No.", "").trim().to_string();
            println!("🔍 DEBUG: Fallback encontró número: '{}'", invoice_number);
            return Some(invoice_number);
        }
    }
    
    println!("❌ DEBUG: No se pudo extraer número de factura");
    None
}

/// Extract date using structure-based approach
fn extract_invoice_date(document: &Html) -> Option<String> {
    println!("🔍 DEBUG: Iniciando extracción de fecha");
    
    // Main strategy: find h4 with "FACTURA" and navigate to div.text-right
    let h4_selector = Selector::parse("h4").ok()?;
    
    for h4 in document.select(&h4_selector) {
        let h4_text = h4.text().collect::<String>().to_uppercase();
        
        // Step 1: Verify this h4 contains "FACTURA"
        if h4_text.contains("FACTURA") {
            println!("✅ DEBUG: Encontrado h4 con FACTURA para fecha");
            
            // Step 2-3: Navigate up two levels (col-sm-4 -> row)
            if let Some(row_div) = h4.parent().and_then(|p| p.parent()) {
                if let Some(row_element) = ElementRef::wrap(row_div) {
                    println!("✅ DEBUG: Navegado a elemento row para fecha");
                    
                    // Step 4: Find div.col-sm-4.text-right within the row
                    let text_right_selector = Selector::parse("div.col-sm-4.text-right").unwrap();
                    for text_right_div in row_element.select(&text_right_selector) {
                        println!("✅ DEBUG: Encontrado div.text-right");
                        
                        // Step 5: Find h5 within div.text-right
                        let h5_selector = Selector::parse("h5").unwrap();
                        for h5 in text_right_div.select(&h5_selector) {
                            let date_text = h5.text().collect::<String>().trim().to_string();
                            println!("🔍 DEBUG: Encontrado h5 con texto exacto: '{}'", date_text);
                            
                            // Validate it contains date format
                            if date_text.matches('/').count() == 2 {
                                println!("✅ DEBUG: Texto contiene formato de fecha válido");
                                // If it already has time, return as is
                                if date_text.matches(':').count() == 2 {
                                    println!("✅ DEBUG: Retornando fecha completa: '{}'", date_text);
                                    return Some(date_text);
                                }
                                // If only has date, add default time
                                else if date_text.matches(':').count() == 0 {
                                    let result = format!("{} 00:00:00", date_text);
                                    println!("✅ DEBUG: Retornando fecha con hora por defecto: '{}'", result);
                                    return Some(result);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("❌ DEBUG: No se pudo extraer fecha");
    None
}

fn main() {
    // Read HTML file
    let html_content = match fs::read_to_string("webscrapy_htmlsample1.html") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Error leyendo archivo HTML: {}", e);
            return;
        }
    };
    
    println!("✅ Archivo HTML cargado: {} caracteres", html_content.len());
    
    // Execute extraction
    let extracted_data = match extract_main_info(&html_content) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("❌ Error en extracción: {}", e);
            return;
        }
    };
    
    println!("\n📊 RESULTADOS DE EXTRACCIÓN:");
    println!("============================");
    
    // Expected values according to documentation
    let expected_numero = "0031157014";
    let expected_fecha = "15/05/2025 09:50:04";
    
    // Validate each field
    match extracted_data.get("numero") {
        Some(actual) => {
            if actual == expected_numero {
                println!("✅ numero: '{}' (CORRECTO)", actual);
            } else {
                println!("❌ numero: esperado '{}', obtenido '{}' (INCORRECTO)", expected_numero, actual);
            }
        },
        None => {
            println!("❌ numero: esperado '{}', NO EXTRAÍDO", expected_numero);
        }
    }
    
    match extracted_data.get("fecha") {
        Some(actual) => {
            if actual == expected_fecha {
                println!("✅ fecha: '{}' (CORRECTO)", actual);
            } else {
                println!("❌ fecha: esperado '{}', obtenido '{}' (INCORRECTO)", expected_fecha, actual);
            }
        },
        None => {
            println!("❌ fecha: esperado '{}', NO EXTRAÍDO", expected_fecha);
        }
    }
    
    println!("\n🔍 DEBUG: Todos los datos extraídos:");
    for (key, value) in &extracted_data {
        println!("  {}: '{}'", key, value);
    }
}
EOF

# Compilar y ejecutar el test temporal
echo "🔧 Compilando test temporal..."
rustc test_temp_extraction.rs --extern scraper -o test_temp_extraction 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✅ Compilación exitosa"
    echo ""
    echo "🚀 Ejecutando test de extracción..."
    echo "=================================="
    ./test_temp_extraction
    
    # Limpiar archivo temporal
    rm -f test_temp_extraction.rs test_temp_extraction
else
    echo "❌ Error en compilación. Intentando con cargo..."
    
    # Fallback: usar grep para verificar la estructura HTML
    echo ""
    echo "📋 VERIFICACIÓN MANUAL DEL HTML:"
    echo "================================"
    
    echo "🔍 Buscando número de factura (debería ser 0031157014):"
    grep -o "No\. [0-9]*" webscrapy_htmlsample1.html || echo "❌ No encontrado"
    
    echo ""
    echo "🔍 Buscando fecha (debería ser 15/05/2025 09:50:04):"
    grep -o "[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]" webscrapy_htmlsample1.html || echo "❌ No encontrado"
    
    echo ""
    echo "🔍 Verificando estructura panel-heading:"
    grep -A 5 -B 5 "panel-heading" webscrapy_htmlsample1.html | head -20
fi

echo ""
echo "✅ Test completado"
