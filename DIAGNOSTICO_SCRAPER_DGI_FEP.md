# 🔍 DIAGNÓSTICO: Problema con Extracción de Productos en DGI-FEP

**Fecha**: Octubre 22, 2025  
**URL Problema**: https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR  
**Síntoma**: La sección de detalle de productos NO se está extrayendo correctamente

---

## 📊 ANÁLISIS DEL CÓDIGO ACTUAL

### **Estado del Scraper**:

1. **`scraper_service_clean.rs`** (L140-146):
   ```rust
   async fn perform_scraping(...) -> Result<...> {
       // TODO: Temporarily disabled to allow compilation for other tests.
       Err(InvoiceProcessingError::ScrapingError {
           message: "Scraping logic is temporarily disabled.".to_string(),
           ...
       })
   }
   ```
   ⚠️ **PROBLEMA PRINCIPAL**: El scraper está **DESHABILITADO COMPLETAMENTE**
   
2. **`ocr_extractor_xpath_v2.rs`** (L205-245):
   - Función `extract_line_items()` implementada correctamente ✅
   - Usa XPath para buscar productos: `//td[@data-title='Cantidad']`
   - Extrae campos: Cantidad, Código, Descripción, Descuento, Precio, Impuesto

---

## 🧐 POSIBLES CAUSAS DEL PROBLEMA

### **1. Scraper Deshabilitado (CONFIRMADO)**
```rust
// En scraper_service_clean.rs línea 133
async fn perform_scraping(...) {
    // TODO: Temporarily disabled
    Err(...)  // ❌ Siempre retorna error
}
```

**Solución**: Reconectar la lógica de extracción.

---

### **2. XPath Selector Incorrecto o Estructura HTML Cambió**

**XPath actual** (L216):
```rust
"//td[@data-title='Cantidad']"
```

**Problema potencial**:
- La página de DGI-FEP puede haber cambiado su estructura HTML
- Los atributos `data-title` pueden ser diferentes
- Los productos pueden estar en un `<table>` con diferente clase/id

**Necesito verificar**: ¿La página usa `data-title="Cantidad"` en los `<td>`?

---

### **3. Contenido Dinámico (JavaScript)**

La página DGI-FEP probablemente carga productos vía JavaScript:

```html
<!-- HTML inicial (lo que ve el scraper): -->
<div id="productos-container">
    <div class="loading">Cargando productos...</div>
</div>

<!-- HTML después de JavaScript (lo que ve el usuario): -->
<table class="productos">
    <tr>
        <td data-title="Cantidad">2</td>
        <td data-title="Descripción">Producto X</td>
        ...
    </tr>
</table>
```

**El scraper Rust usa `reqwest`**, que solo obtiene HTML estático (no ejecuta JavaScript).

**Solución potencial**: Usar un navegador headless como:
- Chromiumoxide (Rust)
- Headless Chrome via API
- Selenium/Puppeteer (Python/Node)

---

### **4. Estructura de Tabla Anidada**

El HTML real puede ser más complejo:

```html
<div class="factura-detalle">
    <table class="resumen">...</table>  <!-- Primera tabla -->
    
    <table class="productos">           <!-- Segunda tabla con productos -->
        <tbody>
            <tr>
                <td data-title="Cantidad">2</td>
                <td data-title="Descripción">Pan</td>
                ...
            </tr>
        </tbody>
    </table>
</div>
```

**XPath actual** busca CUALQUIER `<td data-title="Cantidad">` en TODA la página.

**Puede estar capturando**:
- Totales en vez de productos
- Elementos de resumen
- Tablas incorrectas

**XPath más específico**:
```rust
// Opción A: Solo dentro de tbody
"//tbody/tr/td[@data-title='Cantidad']"

// Opción B: Solo en tabla con clase específica
"//table[contains(@class, 'productos')]//td[@data-title='Cantidad']"

// Opción C: Excluir tabla de resumen
"//table[not(contains(@class, 'resumen'))]//td[@data-title='Cantidad']"
```

---

## 🔧 SOLUCIONES PROPUESTAS

### **SOLUCIÓN 1: Reconectar el Scraper (URGENTE)**

**Archivo**: `src/api/invoice_processor/scraper_service_clean.rs`

```rust
// CAMBIAR ESTO (línea 133-146):
async fn perform_scraping(...) -> Result<...> {
    Err(InvoiceProcessingError::ScrapingError {
        message: "Scraping logic is temporarily disabled.".to_string(),
        ...
    })
}

// POR ESTO:
async fn perform_scraping(
    &self,
    url: &str,
    user_id: &str,
    user_email: &str,
    origin: &str,
    invoice_type: &str,
    reception_date: DateTime<Utc>,
    process_date: DateTime<Utc>,
) -> Result<(FullInvoiceData, u32), InvoiceProcessingError> {
    use crate::processing::web_scraping::ocr_extractor_xpath_v2;
    use crate::processing::web_scraping::http_client;
    
    // 1. Fetch HTML
    let client = reqwest::Client::new();
    let html_content = http_client::fetch_invoice_html(&client, url)
        .await
        .map_err(|e| InvoiceProcessingError::ScrapingError {
            message: format!("Failed to fetch HTML: {}", e),
            error_type: ErrorType::NetworkError,
            retry_attempts: 0,
        })?;
    
    // 2. Extract main info
    let main_info = ocr_extractor_xpath_v2::extract_main_info(&html_content)
        .map_err(|e| InvoiceProcessingError::ScrapingError {
            message: format!("Failed to extract main info: {}", e),
            error_type: ErrorType::ExtractionError,
            retry_attempts: 0,
        })?;
    
    // 3. Extract line items (PRODUCTOS)
    let line_items = ocr_extractor_xpath_v2::extract_line_items(&html_content)
        .map_err(|e| InvoiceProcessingError::ScrapingError {
            message: format!("Failed to extract line items: {}", e),
            error_type: ErrorType::ExtractionError,
            retry_attempts: 0,
        })?;
    
    // 4. Extract payment data
    let payment_data = ocr_extractor_xpath_v2::extract_payment_data(&html_content)
        .unwrap_or_default();
    
    // 5. Build FullInvoiceData
    let invoice_data = FullInvoiceData {
        user_id: user_id.to_string(),
        user_email: user_email.to_string(),
        origin: origin.to_string(),
        invoice_type: invoice_type.to_string(),
        reception_date,
        process_date,
        invoice_url: Some(url.to_string()),
        main_info,
        line_items,
        payment_data,
        // ... otros campos
    };
    
    let fields_count = invoice_data.line_items.len() as u32;
    
    Ok((invoice_data, fields_count))
}
```

---

### **SOLUCIÓN 2: Mejorar el XPath de Productos**

**Archivo**: `src/processing/web_scraping/ocr_extractor_xpath_v2.rs`

Cambiar línea 216:

```rust
// ACTUAL (puede capturar elementos incorrectos):
if let Ok(xpath) = factory.build("//td[@data-title='Cantidad']") {

// MEJORADO (más específico):
if let Ok(xpath) = factory.build("//tbody/tr/td[@data-title='Cantidad']") {
    // O incluso mejor:
    // "//table[contains(@class, 'detalle')]//tbody/tr/td[@data-title='Cantidad']"
```

**Agregar logging para debugging**:

```rust
pub fn extract_line_items(html_content: &str) -> Result<Vec<HashMap<String, String>>> {
    println!("🔍 HTML length: {} bytes", html_content.len());
    
    let package = parser::parse(html_content)
        .map_err(|e| anyhow::anyhow!("Error parsing HTML: {}", e))?;
    let document = package.as_document();
    
    let factory = Factory::new();
    let context = Context::new();
    
    let mut items = Vec::new();
    
    // Debug: Probar diferentes XPath
    let xpaths_to_try = vec![
        "//td[@data-title='Cantidad']",
        "//tbody/tr/td[@data-title='Cantidad']",
        "//table[@class='table-detalle']//td[@data-title='Cantidad']",
        "//div[@id='detalle']//td[@data-title='Cantidad']",
    ];
    
    for xpath_expr in xpaths_to_try {
        println!("🔍 Trying XPath: {}", xpath_expr);
        
        if let Ok(xpath) = factory.build(xpath_expr) {
            if let Ok(result) = xpath.evaluate(&context, document.root()) {
                if let Value::Nodeset(quantity_nodes) = result {
                    println!("✅ Found {} nodes with XPath: {}", quantity_nodes.size(), xpath_expr);
                    
                    if quantity_nodes.size() > 0 {
                        // Procesar con este XPath que funciona
                        for quantity_node in quantity_nodes {
                            // ... extraer item
                        }
                        break;
                    }
                }
            }
        }
    }
    
    // ... resto del código
}
```

---

### **SOLUCIÓN 3: Implementar Navegador Headless (Si JavaScript es necesario)**

Si la página requiere JavaScript, necesitas usar un navegador:

**Opción A: Chromiumoxide (Rust nativo)**

```rust
// Agregar a Cargo.toml:
chromiumoxide = "0.5"

// En scraper_service_clean.rs:
use chromiumoxide::browser::{Browser, BrowserConfig};

async fn fetch_with_browser(url: &str) -> Result<String> {
    let (browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .no_sandbox()
            .build()
            .map_err(|e| anyhow!("Failed to start browser: {}", e))?
    ).await?;
    
    let handle = tokio::spawn(async move {
        loop {
            let _ = handler.next().await;
        }
    });
    
    let page = browser.new_page("about:blank").await?;
    page.goto(url).await?;
    page.wait_for_navigation().await?;
    
    // Esperar a que carguen los productos
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    let html = page.content().await?;
    
    browser.close().await?;
    handle.abort();
    
    Ok(html)
}
```

**Opción B: Llamar a Python con Selenium** (más fácil de debuggear):

```python
# scraper_dgi_fep.py
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
import sys

url = sys.argv[1]

options = webdriver.ChromeOptions()
options.add_argument('--headless')
driver = webdriver.Chrome(options=options)

driver.get(url)

# Esperar a que cargue la tabla de productos
WebDriverWait(driver, 10).until(
    EC.presence_of_element_located((By.XPATH, "//td[@data-title='Cantidad']"))
)

html = driver.page_source
print(html)

driver.quit()
```

```rust
// Llamar desde Rust:
use tokio::process::Command;

async fn fetch_with_python_selenium(url: &str) -> Result<String> {
    let output = Command::new("python3")
        .arg("scraper_dgi_fep.py")
        .arg(url)
        .output()
        .await?;
    
    let html = String::from_utf8(output.stdout)?;
    Ok(html)
}
```

---

### **SOLUCIÓN 4: Guardar HTML para Análisis**

**Debug script** para ver qué HTML se está obteniendo:

```rust
// En ocr_extractor_xpath_v2.rs
pub fn extract_line_items(html_content: &str) -> Result<Vec<HashMap<String, String>>> {
    // GUARDAR HTML PARA DEBUGGING
    use std::fs;
    let debug_path = format!("/tmp/dgi_fep_debug_{}.html", chrono::Utc::now().timestamp());
    fs::write(&debug_path, html_content).ok();
    println!("📄 HTML saved to: {}", debug_path);
    
    // ... resto del código
}
```

Luego puedes:
1. Ejecutar el scraper
2. Abrir `/tmp/dgi_fep_debug_XXXX.html` en un editor
3. Buscar manualmente cómo están estructurados los productos
4. Ajustar el XPath basado en la estructura real

---

## 🎯 PLAN DE ACCIÓN RECOMENDADO

### **Paso 1: Reconectar el Scraper** (5 min)
- Implementar SOLUCIÓN 1
- Reconectar `perform_scraping()` con la lógica de extracción

### **Paso 2: Agregar Debug Logging** (3 min)
- Agregar `println!` para ver qué se está extrayendo
- Guardar HTML en archivo temporal

### **Paso 3: Probar con URL Real** (2 min)
```bash
# Test directo
curl -X POST http://localhost:8000/api/v4/process_from_url \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=...",
    "user_id": "test",
    "user_email": "test@test.com"
  }'
```

### **Paso 4: Analizar HTML Real** (10 min)
- Abrir `/tmp/dgi_fep_debug_XXXX.html`
- Buscar estructura de productos
- Ajustar XPath si es necesario

### **Paso 5: Si no funciona, implementar navegador headless** (30 min)
- Usar Chromiumoxide o Python+Selenium
- Ejecutar JavaScript antes de extraer

---

## 📊 COMPARACIÓN DE SOLUCIONES

| Solución | Dificultad | Tiempo | Efectividad | Recomendado |
|----------|-----------|--------|-------------|-------------|
| 1. Reconectar scraper | ⭐ Fácil | 5 min | Alta (si HTML es estático) | ✅ **PRIMERO** |
| 2. Mejorar XPath | ⭐⭐ Media | 10 min | Alta (si selector está mal) | ✅ **SEGUNDO** |
| 3. Debug logging | ⭐ Fácil | 3 min | Alta (para diagnosticar) | ✅ **SIEMPRE** |
| 4. Navegador headless | ⭐⭐⭐ Difícil | 30-60 min | Muy Alta (JavaScript) | ⚠️ **Si nada funciona** |

---

## ❓ PREGUNTAS PARA TI

Para ayudarte mejor, necesito saber:

1. **¿El scraper está corriendo actualmente?**
   ```bash
   curl -X POST http://localhost:8000/api/v4/process_from_url \
     -H "Content-Type: application/json" \
     -d '{"url": "https://dgi-fep.mef.gob.pa/...", ...}'
   ```
   ¿Qué error retorna?

2. **¿La página DGI-FEP usa JavaScript para cargar productos?**
   - Abre la URL en un navegador
   - Deshabilita JavaScript (DevTools → Settings → Debugger → Disable JavaScript)
   - ¿Sigues viendo los productos? 
     - **Sí** → HTML estático, SOLUCIÓN 1-2 funcionará
     - **No** → Requiere JavaScript, necesitas SOLUCIÓN 4

3. **¿Qué versión del extractor estás usando?**
   - `ocr_extractor.rs` (CSS selectors)
   - `ocr_extractor_xpath.rs` (XPath v1)
   - `ocr_extractor_xpath_v2.rs` (XPath v2) ← **Recomendado**

---

## 🚀 ¿QUIERES QUE IMPLEMENTE LA SOLUCIÓN?

Dime:
- ✅ **Sí, reconecta el scraper** (SOLUCIÓN 1)
- ✅ **Sí, agrega debug logging** (SOLUCIÓN 3)
- ✅ **Sí, mejora el XPath** (SOLUCIÓN 2)
- ⏳ **Necesito navegador headless** (SOLUCIÓN 4)

O si prefieres, puedo:
1. Implementar todo de una vez
2. Solo reconectar el scraper y ver qué pasa
3. Crear un script de testing para diagnosticar

¿Qué prefieres? 🤓
