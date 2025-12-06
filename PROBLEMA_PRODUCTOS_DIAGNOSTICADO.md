# 🐛 PROBLEMA ENCONTRADO: Extracción de Productos

**Fecha**: Octubre 22, 2025  
**Estado**: ✅ DIAGNOSTICADO

---

## 📊 RESUMEN DEL PROBLEMA

### **SÍNTOMA**:
La API `/api/v4/process_from_url` NO extrae los productos correctamente.

### **CAUSA RAÍZ** ✅:
El archivo `src/api/webscraping/mod.rs` (línea 340-370) tiene una función `extract_invoice_details()` que retorna **datos MOCK** en lugar de extraer productos reales del HTML.

```rust
// ARCHIVO: src/api/webscraping/mod.rs
// LÍNEA: 340-370

fn extract_invoice_details(document: &Html, cufe: &str, _user_id: i64) -> Vec<InvoiceDetail> {
    // TODO: Implement real extraction from HTML table
    //       ^^^^ ⚠️ FUNCIÓN INCOMPLETA!
    
    // Retorna datos MOCK:
    return vec![InvoiceDetail {
        code: Some("PROD-001".to_string()),  // ❌ FALSO
        description: Some("Extracted item".to_string()),  // ❌ FALSO
        quantity: Some("1.00".to_string()),  // ❌ FALSO
        // ... todos MOCK
    }];
}
```

---

## ✅ PRUEBA DE CONCEPTO

El binario `test_webscrappy` **SÍ FUNCIONA CORRECTAMENTE**:

```bash
$ cargo run --bin test_webscrappy "https://dgi-fep.mef.gob.pa/..."

✅ HTML Downloaded: 28,863 bytes

📦 TABLA 2: INVOICE DETAILS
================================================================================
✓ Total items: 1

  📌 Item #1
    - Código: 1001002           ✅ REAL
    - Descripción: Whopper CM   ✅ REAL
    - Cantidad: 1               ✅ REAL
    - Precio Unitario: 7.8000   ✅ REAL
    - Total: 7.8000             ✅ REAL
```

**Función que funciona** (en `test_webscrappy.rs`, línea 335-370):

```rust
fn extract_details(document: &Html) -> Vec<HashMap<String, String>> {
    let mut details = Vec::new();
    
    let tbody_selector = Selector::parse("tbody").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    for tbody in document.select(&tbody_selector) {
        for tr in tbody.select(&tr_selector) {
            let mut detail = HashMap::new();
            let cells: Vec<_> = tr.select(&td_selector).collect();
            
            if cells.len() >= 8 {
                // ✅ EXTRAE DATOS REALES del HTML:
                detail.insert("code".to_string(), cells[1].text().collect::<String>().trim().to_string());
                detail.insert("description".to_string(), cells[2].text().collect::<String>().trim().to_string());
                detail.insert("quantity".to_string(), cells[4].text().collect::<String>().trim().to_string());
                // ... etc
                
                details.push(detail);
            }
        }
    }

    details
}
```

---

## 🔧 SOLUCIÓN PROPUESTA

### **Opción 1: Implementar Lógica Real en el API** (RECOMENDADO)

Reemplazar la función `extract_invoice_details()` en `src/api/webscraping/mod.rs` con la lógica de `test_webscrappy.rs`:

```rust
// ARCHIVO: src/api/webscraping/mod.rs
// FUNCIÓN: extract_invoice_details (línea 340)

fn extract_invoice_details(document: &Html, cufe: &str, _user_id: i64) -> Vec<InvoiceDetail> {
    info!("Extracting invoice details from document");
    
    let mut details = Vec::new();
    let mut line_number = 0;
    
    // Parse tbody rows
    let tbody_selector = match Selector::parse("tbody") {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to parse tbody selector: {}", e);
            return Vec::new();
        }
    };
    
    let tr_selector = match Selector::parse("tr") {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to parse tr selector: {}", e);
            return Vec::new();
        }
    };
    
    let td_selector = match Selector::parse("td") {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to parse td selector: {}", e);
            return Vec::new();
        }
    };

    for tbody in document.select(&tbody_selector) {
        for tr in tbody.select(&tr_selector) {
            let cells: Vec<_> = tr.select(&td_selector).collect();
            
            // DGI-FEP table structure:
            // 0: Linea, 1: Código, 2: Descripción, 3: Info interés, 4: Cantidad,
            // 5: Precio, 6: Descuento, 7: Monto, 8: ITBMS, 9: ISC, 10: Acarreo, 11: Seguro, 12: Total
            
            if cells.len() >= 8 {
                line_number += 1;
                
                // Extract text from each cell
                let line = cells[0].text().collect::<String>().trim().to_string();
                let code = cells[1].text().collect::<String>().trim().to_string();
                let description = cells[2].text().collect::<String>().trim().to_string();
                let information_of_interest = cells[3].text().collect::<String>().trim().to_string();
                let quantity = cells[4].text().collect::<String>().trim().to_string();
                let unit_price = cells[5].text().collect::<String>().trim().to_string();
                let unit_discount = cells[6].text().collect::<String>().trim().to_string();
                let amount = cells[7].text().collect::<String>().trim().to_string();
                let itbms = if cells.len() > 8 {
                    cells[8].text().collect::<String>().trim().to_string()
                } else {
                    "0.00".to_string()
                };
                let total = if cells.len() > 12 {
                    cells[12].text().collect::<String>().trim().to_string()
                } else {
                    amount.clone() // Fallback to amount if total not present
                };
                
                // Skip if this row doesn't have actual data (header rows, etc.)
                if code.is_empty() && description.is_empty() {
                    continue;
                }
                
                info!("✅ Extracted detail #{}: code={}, desc={}, qty={}", 
                      line_number, code, description, quantity);
                
                let detail = InvoiceDetail {
                    cufe: cufe.to_string(),
                    partkey: Some(format!("{}|{}", cufe, line)),
                    date: Some(chrono::Utc::now().format("%d/%m/%Y").to_string()),
                    quantity: Some(quantity),
                    code: Some(code),
                    description: Some(description),
                    unit_discount: Some(unit_discount),
                    unit_price: Some(unit_price),
                    itbms: Some(itbms),
                    amount: Some(amount),
                    total: Some(total),
                    information_of_interest: if information_of_interest.is_empty() {
                        None
                    } else {
                        Some(information_of_interest)
                    },
                };
                
                details.push(detail);
            }
        }
    }

    if details.is_empty() {
        warn!("❌ Could not extract invoice details - no valid rows found");
    } else {
        info!("✅ Successfully extracted {} invoice details", details.len());
    }

    details
}
```

---

### **Opción 2: Usar el Extractor Unificado** (ALTERNATIVA)

Usar `crate::processing::web_scraping::ocr_extractor::extract_line_items()`:

```rust
fn extract_invoice_details(document: &Html, cufe: &str, _user_id: i64) -> Vec<InvoiceDetail> {
    use crate::processing::web_scraping::ocr_extractor;
    
    let html_str = document.html();
    
    match ocr_extractor::extract_line_items(&html_str) {
        Ok(items) => {
            items.into_iter().enumerate().map(|(i, mut item)| {
                InvoiceDetail {
                    cufe: cufe.to_string(),
                    partkey: Some(format!("{}|{}", cufe, i + 1)),
                    date: Some(chrono::Utc::now().format("%d/%m/%Y").to_string()),
                    quantity: item.remove("quantity"),
                    code: item.remove("code"),
                    description: item.remove("description"),
                    unit_discount: item.remove("unit_discount"),
                    unit_price: item.remove("unit_price"),
                    itbms: item.remove("itbms"),
                    amount: item.remove("amount"),
                    total: item.remove("total"),
                    information_of_interest: item.remove("information_of_interest"),
                }
            }).collect()
        },
        Err(e) => {
            error!("Failed to extract line items: {}", e);
            Vec::new()
        }
    }
}
```

---

## 📝 PASOS PARA IMPLEMENTAR

### **1. Aplicar el Fix** (5 min):

```bash
cd /home/client_1099_1/scripts/lum_rust_ws
# Editar src/api/webscraping/mod.rs línea 340-370
# Reemplazar función extract_invoice_details con la nueva lógica
```

### **2. Recompilar** (2 min):

```bash
cargo build --release
```

### **3. Probar con API** (3 min):

```bash
# Iniciar servidor
nohup cargo run &

# Test API endpoint
curl -X POST http://localhost:8000/api/v4/process_from_url \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {JWT_TOKEN}" \
  -d '{
    "url": "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=...",
    "origin": "test"
  }'
```

### **4. Verificar Respuesta**:

Deberías ver:

```json
{
  "success": true,
  "data": {
    "header": { ... },
    "details": [
      {
        "code": "1001002",           // ✅ REAL
        "description": "Whopper CM",  // ✅ REAL
        "quantity": "1",              // ✅ REAL
        "unit_price": "7.8000",       // ✅ REAL
        "total": "7.8000"             // ✅ REAL
      }
    ]
  }
}
```

---

## 🎯 RESULTADO ESPERADO

**ANTES** (con datos MOCK):
```json
{
  "details": [
    {
      "code": "PROD-001",           // ❌ MOCK
      "description": "Extracted item",  // ❌ MOCK
      "quantity": "1.00"            // ❌ MOCK
    }
  ]
}
```

**DESPUÉS** (con datos reales):
```json
{
  "details": [
    {
      "code": "1001002",           // ✅ REAL
      "description": "Whopper CM",  // ✅ REAL
      "quantity": "1",              // ✅ REAL
      "unit_price": "7.8000",       // ✅ REAL
      "total": "7.8000"             // ✅ REAL
    }
  ]
}
```

---

## ✅ CONFIRMACIÓN

- [x] Problema diagnosticado: Función con datos MOCK
- [x] Solución identificada: Reemplazar con lógica de test_webscrappy.rs
- [x] Test de concepto validado: test_webscrappy extrae correctamente
- [ ] Fix aplicado (PENDIENTE)
- [ ] API testeada (PENDIENTE)
- [ ] Deployment (PENDIENTE)

---

**Estado**: ✅ **LISTO PARA IMPLEMENTAR**  
**Dificultad**: ⭐ Fácil (copiar lógica funcional)  
**Tiempo estimado**: 10 minutos
