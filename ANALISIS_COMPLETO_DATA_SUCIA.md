# 🔍 ANÁLISIS EXHAUSTIVO: Por qué se están insertando facturas con data en blanco

## 🚨 PROBLEMA CONFIRMADO

Efectivamente, el sistema **SÍ está insertando facturas con data sucia** (solo URL, user_id, fecha) cuando el MEF no tiene la factura aún. Aquí está el análisis completo del flujo defectuoso.

---

## 📍 Punto de Falla #1: Validación Débil en `ocr_extractor.rs`

**Archivo:** `src/processing/web_scraping/ocr_extractor.rs` (líneas 119-154)

```rust
pub fn extract_main_info(html_content: &str) -> Result<ExtractedData> {
    let document = Html::parse_document(html_content);
    
    // ✅ BIEN: Detecta errores del MEF
    if let Some(error_msg) = check_for_mef_errors(&document) {
        return Err(anyhow::anyhow!("Error de MEF: {}", error_msg));
    }
    
    let mut header = HashMap::new();

    // ⚠️ PROBLEMA: Estos insert solo se ejecutan SI encuentra los datos
    if let Some(no) = extract_invoice_number(&document) {
        header.insert("no".to_string(), no);
    }
    if let Some(date) = extract_invoice_date(&document) {
        header.insert("date".to_string(), date);
    }
    if let Some(cufe) = extract_cufe(&document) {
        header.insert("cufe".to_string(), cufe);
    }

    let emisor_data = extract_panel_data(&document, "EMISOR");
    header.extend(emisor_data);

    let receptor_data = extract_panel_data(&document, "RECEPTOR");
    header.extend(receptor_data);

    let totals_data = extract_totals_data(&document);
    header.extend(totals_data);

    let details = extract_line_items(&document);

    // ❌ VALIDACIÓN DÉBIL: Solo falla si AMBOS están vacíos
    // Si encuentra CUFE pero nada más, PASA LA VALIDACIÓN
    if header.is_empty() && details.is_empty() {
        return Err(anyhow::anyhow!("No se pudieron extraer datos de la factura"));
    }

    Ok(ExtractedData { header, details })  // ⚠️ Retorna éxito aunque header tenga solo CUFE
}
```

### 🔴 Problema: La validación usa AND (&&) en lugar de OR (||)

**Escenarios que pasan la validación incorrectamente:**

| Escenario | header | details | Validación | ¿Debería pasar? |
|-----------|--------|---------|------------|-----------------|
| HTML vacío | `{}` (vacío) | `[]` (vacío) | ❌ Falla | ✅ Correcto |
| Solo CUFE extraído | `{"cufe": "ABC123"}` | `[]` (vacío) | ✅ **PASA** | ❌ **INCORRECTO** |
| Solo emisor extraído | `{"emisor_name": "X"}` | `[]` (vacío) | ✅ **PASA** | ❌ **INCORRECTO** |
| CUFE + fecha, sin emisor | `{"cufe": "ABC", "date": "..."}` | `[]` (vacío) | ✅ **PASA** | ❌ **INCORRECTO** |

---

## 📍 Punto de Falla #2: `data_parser.rs` usa `.unwrap_or_default()` en TODOS los campos

**Archivo:** `src/processing/web_scraping/data_parser.rs` (líneas 9-42)

```rust
pub fn parse_invoice_data(
    extracted_data: &ExtractedData,
    url: &str,
) -> Result<(InvoiceHeader, Vec<InvoiceDetail>, Vec<InvoicePayment>)> {
    let main_info = &extracted_data.header;
    let line_items = &extracted_data.details;

    // ✅ BIEN: CUFE es obligatorio (usa .context que retorna error)
    let cufe = main_info
        .get("cufe")
        .context("CUFE not found in main info")?  // ⬅️ FALLA si no hay CUFE
        .clone();

    // ❌ PROBLEMA: El resto de campos usan .unwrap_or_default()
    let header = InvoiceHeader {
        no: main_info.get("no").cloned().unwrap_or_default(),  // ⬅️ "" si no existe
        date: main_info.get("date").and_then(...).ok(),        // ⬅️ None si no existe
        cufe: main_info.get("cufe").cloned().unwrap_or_default(),  // ⬅️ Ya validado arriba
        issuer_name: main_info.get("emisor_name").cloned().unwrap_or_default(),  // ⬅️ ""
        issuer_ruc: main_info.get("emisor_ruc").cloned().unwrap_or_default(),    // ⬅️ ""
        issuer_dv: main_info.get("emisor_dv").cloned().unwrap_or_default(),      // ⬅️ ""
        issuer_address: main_info.get("emisor_address").cloned().unwrap_or_default(), // ⬅️ ""
        issuer_phone: main_info.get("emisor_phone").cloned().unwrap_or_default(),     // ⬅️ ""
        tot_amount: main_info.get("tot_amount").and_then(|s| to_f64(s)).unwrap_or(0.0), // ⬅️ 0.0
        tot_itbms: main_info.get("tot_itbms").and_then(|s| to_f64(s)).unwrap_or(0.0),   // ⬅️ 0.0
        url: url.to_string(),           // ⬅️ Siempre se setea
        r#type: "".to_string(),         // ⬅️ Siempre se setea
        process_date: chrono::Utc::now(),   // ⬅️ Siempre se setea
        reception_date: chrono::Utc::now(), // ⬅️ Siempre se setea
        user_id: 0,                     // ⬅️ Se llena después
        origin: "WHATSAPP".to_string(), // ⬅️ Siempre se setea
        user_email: "".to_string(),     // ⬅️ Se llena después
    };

    Ok((header, details, payments))  // ⚠️ ÉXITO aunque todos los campos estén en blanco
}
```

### 🔴 Problema: `.unwrap_or_default()` oculta la ausencia de datos

**Resultado:** Si el scraper solo encuentra CUFE (línea 16-19), el resto de campos se llenan con valores vacíos/default:
- `no` = `""`
- `issuer_name` = `""`
- `issuer_ruc` = `""`
- `tot_amount` = `0.0`
- etc.

Y el parser retorna **éxito** con esta data incompleta.

---

## 📍 Punto de Falla #3: Ninguna validación en `repository.rs` antes de INSERT

**Archivo:** `src/api/invoice_processor/repository.rs` (líneas 68-120)

```rust
pub async fn save_invoice_data(
    pool: &PgPool,
    invoice_data: &FullInvoiceData,
) -> Result<(), InvoiceProcessingError> {
    let mut tx = pool.begin().await?;

    // ⚠️ NO HAY VALIDACIÓN de campos obligatorios aquí
    // Solo logs informativos
    info!("🗃️ About to insert invoice header with values:");
    info!("   📄 no: '{:?}'", invoice_data.header.no);
    info!("   📅 date: '{:?}'", parsed_date);
    // ... más logs

    // ❌ INSERT directo sin validar que los campos no estén vacíos
    let header_query = r#"
        INSERT INTO public.invoice_header (
            no, date, cufe, issuer_name, issuer_ruc, issuer_dv, 
            issuer_address, issuer_phone, tot_amount, tot_itbms,
            url, type, process_date, reception_date, user_id, origin, user_email
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
    "#;
    
    sqlx::query(header_query)
        .bind(&invoice_data.header.no)          // ⬅️ Puede ser ""
        .bind(parsed_date)                      // ⬅️ Puede ser None
        .bind(&invoice_data.header.cufe)        // ⬅️ Existe (validado antes)
        .bind(&invoice_data.header.issuer_name) // ⬅️ Puede ser ""
        .bind(&invoice_data.header.issuer_ruc)  // ⬅️ Puede ser ""
        .bind(&invoice_data.header.tot_amount)  // ⬅️ Puede ser 0.0
        // ... etc
        .execute(&mut *tx)
        .await?;

    // ✅ Commit exitoso aunque todos los campos estén vacíos
    tx.commit().await?;
    Ok(())
}
```

---

## 🎯 FLUJO COMPLETO DEL BUG

### Escenario: Usuario escanea QR de factura que aún no está en MEF

```
1. Usuario escanea QR
   ↓
2. URL se extrae del QR
   ↓
3. fetch_url_content() obtiene HTML del MEF
   ↓
4. ocr_extractor::extract_main_info(html) analiza el HTML
   
   MEF responde con HTML que contiene:
   - ✅ CUFE visible (extraído de la URL o de algún elemento)
   - ❌ Sin datos de emisor
   - ❌ Sin datos de totales
   - ❌ Sin detalles de productos
   
   ↓
5. check_for_mef_errors() ⚠️ NO detecta error
   
   Porque el HTML NO contiene mensajes explícitos como:
   - "factura no encontrada"
   - "CUFE no encontrado"
   - Alertas div.alert-danger
   
   El MEF simplemente muestra una página "vacía" o "en proceso"
   
   ↓
6. extract_cufe() encuentra el CUFE (extraído de la URL o del HTML)
   header = {"cufe": "FE012000..."}
   
   ↓
7. extract_invoice_number() → None
   extract_invoice_date() → None
   extract_panel_data("EMISOR") → {}
   extract_totals_data() → {}
   extract_line_items() → []
   
   ↓
8. Validación: if header.is_empty() && details.is_empty()
   
   header = {"cufe": "FE012000..."}  ← NO está vacío ✅
   details = []                      ← Está vacío
   
   header.is_empty() = false
   details.is_empty() = true
   
   false && true = false  ⬅️ NO retorna error, CONTINÚA
   
   ↓
9. data_parser::parse_invoice_data() construye InvoiceHeader:
   
   InvoiceHeader {
       no: "",                        ← unwrap_or_default()
       date: None,                    ← and_then().ok()
       cufe: "FE012000...",          ← ✅ Existe
       issuer_name: "",              ← unwrap_or_default()
       issuer_ruc: "",               ← unwrap_or_default()
       issuer_dv: "",                ← unwrap_or_default()
       issuer_address: "",           ← unwrap_or_default()
       issuer_phone: "",             ← unwrap_or_default()
       tot_amount: 0.0,              ← unwrap_or(0.0)
       tot_itbms: 0.0,               ← unwrap_or(0.0)
       url: "https://...",           ← ✅ Siempre se setea
       type: "QR",                   ← ✅ Siempre se setea
       process_date: 2025-11-22...,  ← ✅ Siempre se setea
       reception_date: 2025-11-22..., ← ✅ Siempre se setea
       user_id: 12345,               ← ✅ Siempre se setea
       origin: "WHATSAPP",           ← ✅ Siempre se setea
       user_email: "user@example.com" ← ✅ Siempre se setea
   }
   
   details: []  ← Vacío
   payments: []  ← Vacío
   
   ↓
10. scraper_service retorna OK(full_invoice_data, 1, 0)
    fields_extracted: 1 (solo CUFE)
    retry_attempts: 0
    
    ↓
11. handler NO detecta problema porque scraping retornó Ok()
    
    ↓
12. check_duplicate_invoice() → No es duplicado
    
    ↓
13. save_invoice_data() ⚠️ NO valida campos obligatorios
    
    INSERT INTO public.invoice_header (...) VALUES (
        '',              -- no
        NULL,            -- date
        'FE012000...',   -- cufe
        '',              -- issuer_name
        '',              -- issuer_ruc
        '',              -- issuer_dv
        '',              -- issuer_address
        '',              -- issuer_phone
        0.0,             -- tot_amount
        0.0,             -- tot_itbms
        'https://...',   -- url
        'QR',            -- type
        '2025-11-22...', -- process_date
        '2025-11-22...', -- reception_date
        12345,           -- user_id
        'WHATSAPP',      -- origin
        'user@...'       -- user_email
    )
    
    ✅ INSERT exitoso ⬅️ DATA SUCIA EN LA BASE DE DATOS
    
    ↓
14. Response al usuario: "✅ Factura procesada exitosamente"
    
    Pero en realidad tiene:
    - ❌ No tiene número de factura
    - ❌ No tiene fecha
    - ❌ No tiene nombre de emisor
    - ❌ No tiene RUC
    - ❌ No tiene monto total
    - ❌ No tiene productos (details vacío)
    - ❌ No tiene pagos (payments vacío)
```

---

## 🔍 Por qué `check_for_mef_errors()` NO detecta el problema

**Archivo:** `src/processing/web_scraping/ocr_extractor.rs` (líneas 13-115)

El detector de errores busca:
1. Divs con clases `.alert-danger`, `.alert-warning`, etc.
2. Textos específicos como "factura no encontrada", "CUFE no encontrado"
3. Patrones genéricos de error

**Problema:** Cuando una factura aún no está procesada en el MEF, el HTML puede:
- Mostrar solo el CUFE (en la URL o en algún elemento)
- Tener una estructura HTML válida pero SIN datos
- NO mostrar mensajes de error explícitos
- Simplemente estar "vacío" o mostrar "En proceso..."

Esto hace que `check_for_mef_errors()` retorne `None` (no hay error) y el scraping continúe.

---

## 📊 Evidencia del Problema

### Query para verificar facturas con data sucia:

```sql
SELECT 
    cufe,
    no,
    date,
    issuer_name,
    issuer_ruc,
    tot_amount,
    url,
    process_date,
    user_id
FROM public.invoice_header
WHERE 
    (no IS NULL OR no = '')
    OR (issuer_name IS NULL OR issuer_name = '')
    OR (issuer_ruc IS NULL OR issuer_ruc = '')
    OR (tot_amount IS NULL OR tot_amount = 0.0)
    OR date IS NULL
ORDER BY process_date DESC
LIMIT 50;
```

**Resultado esperado:** Encontrarás facturas con:
- `no` = `''` o `NULL`
- `issuer_name` = `''`
- `issuer_ruc` = `''`
- `tot_amount` = `0.0`
- Pero **SÍ tienen:** `cufe`, `url`, `process_date`, `user_id`, `origin`

---

## 🎯 SOLUCIÓN CORRECTA

### Opción 1: Validar campos críticos en `data_parser.rs` (Recomendado)

```rust
pub fn parse_invoice_data(
    extracted_data: &ExtractedData,
    url: &str,
) -> Result<(InvoiceHeader, Vec<InvoiceDetail>, Vec<InvoicePayment>)> {
    let main_info = &extracted_data.header;
    
    // ✅ Validar TODOS los campos críticos
    let cufe = main_info.get("cufe")
        .context("CUFE not found in main info")?
        .clone();
    
    let no = main_info.get("no")
        .filter(|s| !s.is_empty())
        .context("Invoice number (no) not found or empty")?
        .clone();
    
    let issuer_name = main_info.get("emisor_name")
        .filter(|s| !s.is_empty())
        .context("Issuer name not found or empty")?
        .clone();
    
    let issuer_ruc = main_info.get("emisor_ruc")
        .filter(|s| !s.is_empty())
        .context("Issuer RUC not found or empty")?
        .clone();
    
    let date_str = main_info.get("date")
        .filter(|s| !s.is_empty())
        .context("Invoice date not found or empty")?;
    
    let date = NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S")
        .context("Invalid date format")?;
    
    let tot_amount = main_info.get("tot_amount")
        .and_then(|s| to_f64(s))
        .filter(|&amount| amount > 0.0)
        .context("Total amount not found or invalid (must be > 0)")?;
    
    // Construir header con valores validados (sin unwrap_or_default)
    let header = InvoiceHeader {
        no,
        date: Some(date),
        cufe,
        issuer_name,
        issuer_ruc,
        issuer_dv: main_info.get("emisor_dv").cloned().unwrap_or_default(),
        issuer_address: main_info.get("emisor_address").cloned().unwrap_or_default(),
        issuer_phone: main_info.get("emisor_phone").cloned().unwrap_or_default(),
        tot_amount,
        tot_itbms: main_info.get("tot_itbms").and_then(|s| to_f64(s)).unwrap_or(0.0),
        url: url.to_string(),
        r#type: "".to_string(),
        process_date: chrono::Utc::now(),
        reception_date: chrono::Utc::now(),
        user_id: 0,
        origin: "WHATSAPP".to_string(),
        user_email: "".to_string(),
    };
    
    Ok((header, details, payments))
}
```

### Opción 2: Fortalecer validación en `ocr_extractor.rs`

```rust
pub fn extract_main_info(html_content: &str) -> Result<ExtractedData> {
    // ... código existente ...
    
    // ✅ Validación más estricta
    let required_fields = ["cufe", "no", "date", "emisor_name", "emisor_ruc"];
    let missing_fields: Vec<_> = required_fields
        .iter()
        .filter(|&field| !header.contains_key(*field) || header[*field].is_empty())
        .collect();
    
    if !missing_fields.is_empty() {
        return Err(anyhow::anyhow!(
            "Campos obligatorios faltantes o vacíos: {:?}. La factura puede no estar procesada en el MEF aún.",
            missing_fields
        ));
    }
    
    // Validar totales
    if !header.contains_key("tot_amount") {
        return Err(anyhow::anyhow!("Monto total no encontrado"));
    }
    
    Ok(ExtractedData { header, details })
}
```

### Opción 3: Guardar en `mef_pending` cuando faltan datos críticos (Patrón WhatsApp)

```rust
// En handler después del scraping
match scraper_service.scrape_invoice_with_retries(...).await {
    Ok((full_invoice_data, fields_extracted, retry_attempts)) => {
        // ✅ Validar que tenga datos mínimos
        if full_invoice_data.header.issuer_name.is_empty() 
            || full_invoice_data.header.issuer_ruc.is_empty()
            || full_invoice_data.header.no.is_empty()
            || full_invoice_data.header.tot_amount <= 0.0 {
            
            warn!("Datos incompletos extraídos, guardando en mef_pending");
            
            let pending_entry = MefPending {
                url: Some(request.url.clone()),
                cufe: Some(full_invoice_data.header.cufe.clone()),
                error_message: Some("Datos incompletos - Factura aún no procesada en MEF".to_string()),
                // ... otros campos
            };
            
            save_to_mef_pending(&pool, &pending_entry).await?;
            
            return Ok(ResponseJson(json!({
                "status": "pending",
                "message": "La factura ha sido recibida y pronto será procesada",
                "cufe": full_invoice_data.header.cufe
            })));
        }
        
        // Continuar con flujo normal solo si datos están completos
        // ...
    }
    Err(e) => {
        // ... manejo de error de scraping
    }
}
```

---

## ✅ RECOMENDACIÓN FINAL

**Implementar las 3 opciones en capas:**

1. **Capa 1 - Extractor:** Validación estricta en `ocr_extractor.rs` (campos obligatorios)
2. **Capa 2 - Parser:** Validación en `data_parser.rs` (valores no vacíos, formatos correctos)
3. **Capa 3 - Handler:** Fallback a `mef_pending` si scraping exitoso pero datos incompletos

Esto garantiza:
- ✅ NO se inserta data sucia en `invoice_header`
- ✅ NO se pierden requests del usuario
- ✅ Se permite procesamiento manual posterior
- ✅ Mensaje claro al usuario: "La factura ha sido recibida y pronto será procesada"
