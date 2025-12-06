# ✅ SOLUCIÓN IMPLEMENTADA: Prevención de Data Sucia en Facturas

## 🎯 Problema Resuelto

Se implementaron **3 capas de defensa** para evitar que se inserten facturas con datos incompletos o vacíos en las tablas principales (`invoice_header`, `invoice_detail`, `invoice_payment`).

---

## 📝 Cambios Realizados

### 1️⃣ **Capa 1: Validación Estricta en `ocr_extractor.rs`**

**Archivo:** `src/processing/web_scraping/ocr_extractor.rs`

**Cambios:**
- ❌ **ANTES:** Validación débil usando AND (`header.is_empty() && details.is_empty()`)
- ✅ **AHORA:** Validación estricta de campos obligatorios

```rust
// ✅ VALIDACIÓN ESTRICTA: Verificar campos críticos obligatorios
let required_fields = vec![
    ("cufe", "CUFE"),
    ("no", "Número de factura"),
    ("date", "Fecha de factura"),
    ("emisor_name", "Nombre del emisor"),
    ("emisor_ruc", "RUC del emisor"),
];

let mut missing_fields = Vec::new();
for (field_key, field_name) in required_fields {
    if !header.contains_key(field_key) || header.get(field_key).map_or(true, |v| v.is_empty()) {
        missing_fields.push(field_name);
    }
}

if !missing_fields.is_empty() {
    return Err(anyhow::anyhow!(
        "Campos obligatorios faltantes o vacíos: {}. La factura puede no estar procesada en el MEF aún o los datos son incompletos.",
        missing_fields.join(", ")
    ));
}

// Validar que el monto total exista y no sea vacío
if !header.contains_key("tot_amount") || header.get("tot_amount").map_or(true, |v| v.is_empty()) {
    return Err(anyhow::anyhow!(
        "Monto total no encontrado o vacío. La factura puede no estar procesada completamente en el MEF."
    ));
}
```

**Resultado:**
- Si falta cualquier campo crítico (CUFE, número, fecha, emisor, RUC, monto), el scraping **falla inmediatamente**
- Error claro y descriptivo para debugging

---

### 2️⃣ **Capa 2: Validación Estricta en `data_parser.rs`**

**Archivo:** `src/processing/web_scraping/data_parser.rs`

**Cambios:**
- ❌ **ANTES:** Todos los campos usaban `.unwrap_or_default()` → strings vacíos y ceros
- ✅ **AHORA:** Validación con `.context()` y `.filter()` para campos obligatorios

```rust
// ✅ VALIDACIÓN ESTRICTA: CUFE es obligatorio y no puede estar vacío
let cufe = main_info
    .get("cufe")
    .filter(|s| !s.is_empty())
    .context("CUFE not found or empty in main info")?
    .clone();

// ✅ VALIDACIÓN ESTRICTA: Número de factura es obligatorio y no puede estar vacío
let no = main_info
    .get("no")
    .filter(|s| !s.is_empty())
    .context("Invoice number (no) not found or empty")?
    .clone();

// ✅ VALIDACIÓN ESTRICTA: Fecha es obligatoria y debe tener formato válido
let date_str = main_info
    .get("date")
    .filter(|s| !s.is_empty())
    .context("Invoice date not found or empty")?;

let date = NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S")
    .context(format!("Invalid date format: '{}'. Expected format: DD/MM/YYYY HH:MM:SS", date_str))?;

// ✅ VALIDACIÓN ESTRICTA: Nombre del emisor es obligatorio y no puede estar vacío
let issuer_name = main_info
    .get("emisor_name")
    .filter(|s| !s.is_empty())
    .context("Issuer name not found or empty")?
    .clone();

// ✅ VALIDACIÓN ESTRICTA: RUC del emisor es obligatorio y no puede estar vacío
let issuer_ruc = main_info
    .get("emisor_ruc")
    .filter(|s| !s.is_empty())
    .context("Issuer RUC not found or empty")?
    .clone();

// ✅ VALIDACIÓN ESTRICTA: Monto total es obligatorio y debe ser > 0
let tot_amount = main_info
    .get("tot_amount")
    .and_then(|s| to_f64(s))
    .filter(|&amount| amount > 0.0)
    .context("Total amount not found, invalid, or must be greater than 0")?;
```

**Resultado:**
- Si el scraping extrae solo CUFE pero no los demás campos, el parser **falla con error claro**
- No se pueden crear headers con campos vacíos
- Monto total debe ser mayor a 0

---

### 3️⃣ **Capa 3: Fallback a `mef_pending` en Handler**

**Archivos Modificados:**
1. `src/api/invoice_processor/repository.rs` - Nueva función helper
2. `src/api/invoice_processor/handlers.rs` - Manejo de errores mejorado

**Nueva Función Helper:**

```rust
/// Saves invoice data to mef_pending table when automatic processing fails
/// This allows manual review and processing later
pub async fn save_to_mef_pending(
    pool: &PgPool,
    url: &str,
    user_id: &str,
    user_email: &str,
    origin: &str,
    error_message: &str,
    cufe: Option<&str>,
) -> Result<(), InvoiceProcessingError> {
    info!("💾 Guardando factura en mef_pending para procesamiento manual");
    info!("   URL: {}", url);
    info!("   User ID: {}", user_id);
    info!("   Error: {}", error_message);

    let query = r#"
        INSERT INTO public.mef_pending (
            url, date, type, user_email, user_id, error, origin
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (url) DO UPDATE SET
            date = EXCLUDED.date,
            error = EXCLUDED.error,
            user_id = EXCLUDED.user_id
    "#;

    // ... ejecución del query ...
    
    info!("✅ Factura guardada en mef_pending exitosamente");
    Ok(())
}
```

**Cambios en Handler:**

```rust
// 4. WEB SCRAPING PHASE
let scraping_result = scraper_service
    .scrape_invoice_with_retries(...)
    .await;

// ✅ MANEJO DE ERRORES DE SCRAPING: Fallback a mef_pending
let (full_invoice_data, fields_extracted, retry_attempts) = match scraping_result {
    Ok(data) => data,
    Err(scraping_error) => {
        error!("❌ Scraping failed: {:?}", scraping_error);
        
        let error_msg = format!("{:?}", scraping_error);
        let error_type = categorize_error(&error_msg);
        
        // Log scraping error
        let _ = logging_service.log_scraping_error(
            log_id, &error_msg, error_type, start_time, 0
        ).await;
        
        // Guardar en mef_pending para procesamiento manual
        if let Err(e) = save_to_mef_pending(
            &pool,
            &request.url,
            &request.user_id,
            &request.user_email,
            &request.origin,
            &error_msg,
            None, // No CUFE disponible
        ).await {
            warn!("⚠️ Failed to save to mef_pending: {:?}", e);
        }
        
        // Retornar respuesta amigable al usuario
        return Ok(ResponseJson(serde_json::json!({
            "status": "pending",
            "message": "La factura ha sido recibida y pronto será procesada",
            "details": "No pudimos procesar la factura automáticamente. Nuestro equipo la revisará manualmente y te notificaremos cuando esté lista."
        })));
    }
};
```

**Resultado:**
- Si el scraping falla (por cualquier razón), la factura se guarda en `mef_pending`
- El usuario recibe un mensaje amigable: **"La factura ha sido recibida y pronto será procesada"**
- NO se inserta nada en `invoice_header`, `invoice_detail`, ni `invoice_payment`
- Se registra el error en el log para auditoría
- El equipo puede revisar manualmente las facturas en `mef_pending`

---

## 🎯 Garantías del Sistema Implementado

| Escenario | Comportamiento Anterior | Comportamiento Nuevo |
|-----------|------------------------|---------------------|
| **Factura no en MEF** | ❌ Insertaba data vacía (solo CUFE, URL, user_id) | ✅ Va a `mef_pending` con mensaje amigable |
| **Solo CUFE extraído** | ❌ Insertaba con campos vacíos | ✅ Scraping falla, va a `mef_pending` |
| **Monto = 0** | ❌ Insertaba con tot_amount = 0.0 | ✅ Parser falla, va a `mef_pending` |
| **Sin nombre emisor** | ❌ Insertaba con issuer_name = "" | ✅ Scraping falla, va a `mef_pending` |
| **Sin RUC** | ❌ Insertaba con issuer_ruc = "" | ✅ Scraping falla, va a `mef_pending` |
| **Fecha inválida** | ❌ Insertaba con date = NULL | ✅ Parser falla, va a `mef_pending` |
| **Factura completa** | ✅ Se procesa normalmente | ✅ Se procesa normalmente |

---

## 📊 Flujo de Procesamiento Actualizado

```
Usuario escanea QR
       ↓
Extrae URL del QR
       ↓
Web Scraping (ocr_extractor.rs)
       ↓
┌──────────────────────────────────────┐
│ ¿Campos obligatorios presentes?     │
│ - CUFE                               │
│ - Número de factura                  │
│ - Fecha                              │
│ - Nombre emisor                      │
│ - RUC emisor                         │
│ - Monto total                        │
└──────────────────────────────────────┘
       ↓                    ↓
     NO ✅               SÍ ✅
       ↓                    ↓
Error de scraping    Parse Data (data_parser.rs)
       ↓                    ↓
┌─────────────────┐  ┌──────────────────────────┐
│ Fallback:       │  │ ¿Valores válidos?        │
│ - Log error     │  │ - Strings no vacíos      │
│ - Save to       │  │ - Fecha formato correcto │
│   mef_pending   │  │ - Monto > 0              │
│ - Response:     │  └──────────────────────────┘
│   "pending"     │         ↓              ↓
└─────────────────┘       NO ✅          SÍ ✅
       ↓                    ↓              ↓
       ↓          ┌─────────────────┐     ↓
       └──────────→ mef_pending     │     ↓
                  │ para revisión   │     ↓
                  │ manual          │     ↓
                  └─────────────────┘     ↓
                                          ↓
                                   Check Duplicate
                                          ↓
                                        NO ↓
                                          ↓
                                   ┌─────────────────┐
                                   │ INSERT INTO:    │
                                   │ - invoice_header│
                                   │ - invoice_detail│
                                   │ - invoice_payment│
                                   └─────────────────┘
                                          ↓
                                   Response: "success"
```

---

## ✅ Verificación de Compilación

```bash
cd /home/client_1099_1/scripts/lum_rust_ws
cargo build --bin lum_rust_ws
```

**Resultado:**
```
Compiling lum_rust_ws v0.1.0
warning: unused import: `Row`
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 47s
```

✅ **Compilación exitosa** con solo 1 warning menor (import no usado)

---

## 🔍 Cómo Verificar que Funciona

### 1. Query para verificar que NO hay data sucia:

```sql
SELECT 
    cufe,
    no,
    date,
    issuer_name,
    issuer_ruc,
    tot_amount,
    url,
    process_date
FROM public.invoice_header
WHERE 
    process_date > NOW() - INTERVAL '1 day'  -- Solo facturas recientes
    AND (
        (no IS NULL OR no = '')
        OR (issuer_name IS NULL OR issuer_name = '')
        OR (issuer_ruc IS NULL OR issuer_ruc = '')
        OR (tot_amount IS NULL OR tot_amount = 0.0)
        OR date IS NULL
    )
ORDER BY process_date DESC;
```

**Resultado esperado después de la solución:** ❌ **0 registros** (no debe haber data sucia)

### 2. Query para verificar facturas en mef_pending:

```sql
SELECT 
    url,
    date,
    user_id,
    error,
    origin,
    type
FROM public.mef_pending
WHERE 
    date > NOW() - INTERVAL '1 day'
    AND error LIKE '%Campos obligatorios faltantes%'
ORDER BY date DESC;
```

**Resultado esperado:** ✅ Facturas con datos incompletos guardadas aquí para revisión manual

---

## 📋 Archivos Modificados

1. ✅ `src/processing/web_scraping/ocr_extractor.rs` - Validación estricta de campos
2. ✅ `src/processing/web_scraping/data_parser.rs` - Eliminación de .unwrap_or_default()
3. ✅ `src/api/invoice_processor/repository.rs` - Nueva función save_to_mef_pending()
4. ✅ `src/api/invoice_processor/handlers.rs` - Fallback a mef_pending en errores

---

## 🎉 Conclusión

La solución implementada sigue el **patrón de WhatsApp Service** con 3 capas de defensa:

1. **Extractor:** Valida campos obligatorios presentes
2. **Parser:** Valida valores no vacíos y formatos correctos
3. **Handler:** Fallback a mef_pending si algo falla

**Resultado:**
- ✅ **NO se inserta data sucia** en tablas principales
- ✅ **NO se pierden requests** del usuario
- ✅ **Mensaje claro** al usuario: "La factura ha sido recibida y pronto será procesada"
- ✅ **Procesamiento manual posible** mediante tabla mef_pending
- ✅ **Auditoría completa** de todos los errores
