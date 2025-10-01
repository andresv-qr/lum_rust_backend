#!/usr/bin/env rust-script
//! Test específico para verificar la extracción de fecha
//! 
//! ```cargo
//! [dependencies]
//! reqwest = { version = "0.11", features = ["json"] }
//! scraper = "0.20"
//! tokio = { version = "1", features = ["full"] }
//! ```

use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashMap;

// Función simplificada de extracción de fecha
pub fn extract_invoice_date(html: &str) -> Option<String> {
    println!("🔍 Buscando fecha en HTML...");
    
    let document = Html::parse_document(html);
    
    // Buscar el h4 con "FACTURA"
    if let Ok(factura_selector) = Selector::parse("h4") {
        for h4_element in document.select(&factura_selector) {
            let h4_text = h4_element.text().collect::<String>();
            println!("📝 H4 encontrado: '{}'", h4_text.trim());
            
            if h4_text.contains("FACTURA") {
                println!("✅ Encontrado H4 con 'FACTURA'");
                
                // Navegar al div padre del h4, luego al div hermano con clase text-right
                if let Some(h4_parent) = h4_element.parent() {
                    if let Some(grandparent) = h4_parent.parent() {
                        println!("🔗 Navegando a grandparent...");
                        
                        // Buscar div con text-right dentro del grandparent
                        if let Ok(text_right_selector) = Selector::parse("div.text-right") {
                            for text_right_div in grandparent.select(&text_right_selector) {
                                println!("📍 Encontrado div.text-right");
                                
                                // Buscar h5 dentro de este div
                                if let Ok(h5_selector) = Selector::parse("h5") {
                                    for h5_element in text_right_div.select(&h5_selector) {
                                        let h5_text = h5_element.text().collect::<String>().trim().to_string();
                                        println!("🎯 H5 encontrado: '{}'", h5_text);
                                        
                                        // Verificar si parece una fecha
                                        if h5_text.contains("/") && (h5_text.contains("2025") || h5_text.contains("2024")) {
                                            println!("✅ FECHA ENCONTRADA: '{}'", h5_text);
                                            return Some(h5_text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("❌ No se encontró fecha");
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 TEST: Extracción de fecha de factura DGI");
    println!("==========================================");
    
    let url = "https://dgi-fep.mef.gob.pa/FacturasPorQR/ConsultasFactura/Consultar?FacturaConsultar=FE01200002679372-1-844914-7300002025051500311570140020317481978892";
    
    println!("📄 URL: {}", url);
    
    // Fetch HTML
    let client = Client::new();
    println!("🌐 Descargando HTML...");
    
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .send()
        .await?;
    
    let html = response.text().await?;
    println!("📦 HTML descargado: {} caracteres", html.len());
    
    // Buscar texto "FACTURA" en el HTML
    if html.contains("FACTURA") {
        println!("✅ HTML contiene 'FACTURA'");
    } else {
        println!("❌ HTML NO contiene 'FACTURA'");
    }
    
    // Buscar patrones de fecha
    if html.contains("2025") {
        println!("✅ HTML contiene '2025'");
    } else {
        println!("❌ HTML NO contiene '2025'");
    }
    
    // Extraer fecha
    if let Some(fecha) = extract_invoice_date(&html) {
        println!("🎉 ÉXITO: Fecha extraída = '{}'", fecha);
    } else {
        println!("💥 FALLÓ: No se pudo extraer la fecha");
        
        // Debug: mostrar fragmento del HTML
        println!("\n🔍 FRAGMENTO DE HTML (primeros 2000 caracteres):");
        println!("{}", &html[..std::cmp::min(2000, html.len())]);
    }
    
    Ok(())
}
