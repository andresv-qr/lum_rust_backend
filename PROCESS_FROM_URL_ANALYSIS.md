# Análisis Completo: `/invoices/process-from-url`

## 📋 Descripción General
Este endpoint procesa facturas electrónicas de Panamá mediante web scraping de URLs de la DGI (Dirección General de Ingresos).

---

## 🔄 FLUJO PASO A PASO

### **PASO 1: Recepción del Request**
**Archivo:** `src/api/url_processing_v4.rs` (línea 23-28)

**Handler:** `process_url_handler`

**Input esperado:**
```json
{
  "url": "https://dgi-fep.mef.gob.pa/consultas/facturasporcufe?chFE=ABC123..."
}
```

**Validación inicial:**
- Verifica que la URL no esté vacía
- Si está vacía → retorna error `"URL is required"`

---

### **PASO 2: Web Scraping**
**Archivo:** `src/api/webscraping/mod.rs` (línea 88)

**Función:** `scrape_invoice(client, url)`

#### 2.1 Fetch HTML
**Función:** `fetch_html_with_final_url()` (línea 135)

- Hace request HTTP GET con headers especiales:
  - User-Agent: Mozilla/5.0
  - Accept: text/html
  - Accept-Language: es-ES
- **Sigue redirecciones automáticamente**
- Captura la URL final (después de redirecciones)
- Descarga el contenido HTML

#### 2.2 Extracción del CUFE
**Función:** `extract_cufe_from_url()` (línea 175)

- Busca el parámetro `chFE=` en la URL final
- Ejemplo: `...?chFE=ABC123XYZ&...` → extrae `"ABC123XYZ"`
- Si no encuentra CUFE → usa `"UNKNOWN"`

#### 2.3 Parsing HTML
**Función:** `extract_invoice_header()` (línea 200)

Utiliza el extractor unificado:
- **Parser:** `scraper::Html` (biblioteca Rust)
- **Extractor:** `extract_main_info()` del módulo `ocr_extractor`
- Parsea el HTML completo buscando patrones específicos

**Datos extraídos:**
```rust
// Del header HTML:
- date (fecha emisión)
- no (número de factura)
- emisor_name (nombre del proveedor)
- emisor_ruc (RUC del proveedor)
- emisor_dv (dígito verificador)
- emisor_address (dirección)
- emisor_phone (teléfono)
- receptor_name (nombre del cliente)
- receptor_ruc (RUC del cliente)
- receptor_dv (dígito verificador)
- receptor_address
- receptor_phone
- tot_amount (monto total)
- tot_itbms (impuestos)
```

**Proceso de conversión:**
- Montos: texto → `rust_decimal::Decimal`
  - Remueve: "B/.", "$", ",", espacios
  - Ejemplo: "B/. 1,234.56" → Decimal(123456, escala 2)

#### 2.4 Extracción de Detalles
**Función:** `extract_invoice_details()` (línea 297)

- Busca tablas con selectores CSS: `"tr"`, `".detail-row"`, `".item-row"`, `"tbody tr"`
- **Actualmente:** implementación básica (retorna datos de ejemplo)
- Estructura esperada:
  ```rust
  InvoiceDetail {
    item_numero,
    descripcion,
    cantidad,
    precio_unitario,
    subtotal,
    impuesto_porcentaje,
    impuesto_monto,
    total
  }
  ```

#### 2.5 Extracción de Pagos
**Función:** `extract_invoice_payments()` (línea 329)

- Busca elementos con clases: `.payment`, `.pago`, `#payment-info`
- **Actualmente:** implementación básica
- Estructura:
  ```rust
  InvoicePayment {
    metodo_pago,
    monto,
    referencia
  }
  ```

#### 2.6 Resultado del Scraping
**Retorna:** `ScrapingResult`
```rust
ScrapingResult {
  success: true,
  header: Option<InvoiceHeader>,
  details: Vec<InvoiceDetail>,
  payments: Vec<InvoicePayment>,
  error_message: None
}
```

**Nota importante:** Siempre retorna `success: true` porque aunque no extraiga todos los datos, siempre crea un header mínimo con el CUFE.

---

### **PASO 3: Persistencia en Base de Datos**
**Archivo:** `src/api/database_persistence.rs` (línea 32)

**Función:** `persist_scraped_data(db_pool, scraping_result, source_url)`

#### 3.1 Verificación de Duplicados
```sql
SELECT id, cufe FROM invoice_headers WHERE cufe = $1
```
- Si existe → retorna error: `"Duplicate invoice detected"`
- Si no existe → continúa

#### 3.2 Inicio de Transacción
```rust
let mut tx = db_pool.begin().await
```

#### 3.3 Guardar Invoice Header
**Función:** `save_invoice_header()` (línea 111)

**Query SQL:**
```sql
INSERT INTO invoice_headers (
    cufe, 
    numero_factura, 
    fecha_emision, 
    proveedor_nombre, 
    proveedor_ruc,
    cliente_nombre, 
    cliente_ruc, 
    subtotal, 
    impuestos, 
    total, 
    moneda,
    estado, 
    user_id, 
    source_url
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
RETURNING id
```

**Mapeo de campos:**
| Campo DB | Origen | Transformación |
|----------|--------|----------------|
| `cufe` | `header.cufe` | Directo (String) |
| `numero_factura` | `header.no` | Directo (Option<String>) |
| `fecha_emision` | `header.date` | String → NaiveDate (parseado) |
| `proveedor_nombre` | `header.issuer_name` | Directo |
| `proveedor_ruc` | `header.issuer_ruc` | Directo |
| `cliente_nombre` | `header.receptor_name` | Directo |
| `cliente_ruc` | `header.receptor_id` | Directo |
| `subtotal` | - | NULL (calculado después) |
| `impuestos` | `header.tot_itbms` | Decimal |
| `total` | `header.tot_amount` | Decimal |
| `moneda` | - | "PAB" (hardcoded) |
| `estado` | - | "ACTIVO" (hardcoded) |
| `user_id` | `header.user_id` | i32 |
| `source_url` | `header.url` | String (URL final) |

**Parsing de fecha:**
- Formato esperado: `"DD/MM/YYYY HH:MM:SS"` o `"DD/MM/YYYY"`
- Ejemplo: `"25/10/2024 14:30:00"` → NaiveDate(2024-10-25)

**Retorna:** `invoice_id` (i32) - ID autogenerado de la tabla

#### 3.4 Guardar Invoice Details
**Función:** `save_invoice_details()` (línea 148)

**Query SQL (por cada detalle):**
```sql
INSERT INTO invoice_details (
    invoice_header_id, 
    cufe, 
    item_numero, 
    descripcion, 
    cantidad,
    precio_unitario, 
    subtotal, 
    impuesto_porcentaje, 
    impuesto_monto, 
    total
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
```

**Mapeo:**
| Campo DB | Origen |
|----------|--------|
| `invoice_header_id` | ID retornado del header |
| `cufe` | `detail.cufe` |
| `item_numero` | `detail.item_numero` |
| `descripcion` | `detail.descripcion` |
| `cantidad` | `detail.cantidad` (Decimal) |
| `precio_unitario` | `detail.precio_unitario` (Decimal) |
| `subtotal` | `detail.subtotal` (Decimal) |
| `impuesto_porcentaje` | `detail.impuesto_porcentaje` (Decimal) |
| `impuesto_monto` | `detail.impuesto_monto` (Decimal) |
| `total` | `detail.total` (Decimal) |

**Nota:** Se insertan **todos los detalles** en un loop.

#### 3.5 Guardar Invoice Payments
**Función:** `save_invoice_payments()` (línea 172)

**Query SQL (por cada pago):**
```sql
INSERT INTO invoice_payments (
    invoice_header_id, 
    cufe, 
    metodo_pago, 
    monto, 
    referencia
)
VALUES ($1, $2, $3, $4, $5)
```

**Mapeo:**
| Campo DB | Origen |
|----------|--------|
| `invoice_header_id` | ID retornado del header |
| `cufe` | `payment.cufe` |
| `metodo_pago` | `payment.metodo_pago` |
| `monto` | `payment.monto` (Decimal) |
| `referencia` | `payment.referencia` |

#### 3.6 Commit de Transacción
```rust
tx.commit().await
```

Si cualquier paso falla:
- Se hace ROLLBACK automático
- No se guarda NADA en la base de datos
- Se retorna error

---

### **PASO 4: Construcción del Response**
**Archivo:** `src/api/url_processing_v4.rs` (línea 49-71)

#### Caso ÉXITO:
```rust
ProcessUrlResponse {
  success: true,
  message: "URL processed successfully (API)",
  process_type: Some("API"),
  invoice_id: Some(123),  // ID generado
  cufe: Some("ABC123XYZ"),
  processing_time_ms: Some(1234)
}
```

#### Caso ERROR (duplicado):
```rust
ProcessUrlResponse {
  success: false,
  message: "Duplicate invoice detected",
  process_type: None,
  invoice_id: None,
  cufe: None,
  processing_time_ms: Some(456)
}
```

#### Caso ERROR (scraping falló):
```rust
ApiError {
  code: "SCRAPING_ERROR",
  message: "Failed to scrape invoice data"
}
```

---

## 📤 RESPONSE FINAL (JSON)

### Éxito:
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "URL processed successfully (API)",
    "process_type": "API",
    "invoice_id": 123,
    "cufe": "ABC123XYZ",
    "processing_time_ms": 1234
  },
  "error": null,
  "request_id": "uuid-1234-5678",
  "timestamp": "2024-10-25T14:30:00Z",
  "execution_time_ms": 1234,
  "cached": false
}
```

### Error (duplicado):
```json
{
  "success": false,
  "data": {
    "success": false,
    "message": "Duplicate invoice detected",
    "process_type": null,
    "invoice_id": null,
    "cufe": null,
    "processing_time_ms": 456
  },
  "error": null,
  "request_id": "uuid-1234-5678",
  "timestamp": "2024-10-25T14:30:00Z",
  "execution_time_ms": 456,
  "cached": false
}
```

### Error (scraping falló):
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "SCRAPING_ERROR",
    "message": "Failed to scrape invoice data"
  },
  "request_id": "uuid-1234-5678",
  "timestamp": "2024-10-25T14:30:00Z",
  "execution_time_ms": 789,
  "cached": false
}
```

---

## 🗄️ CAMPOS GUARDADOS EN LA BASE DE DATOS

## ⚠️ ADVERTENCIA: SCHEMA INCORRECTO EN CÓDIGO ACTUAL

**El código actual intenta guardar en tablas y campos que NO EXISTEN.**  
**Ver `DATABASE_SCHEMA_ANALYSIS.md` para el análisis completo.**

### Tabla Real: `invoice_header` (singular, no plural)
| Campo | Tipo Real | Ejemplo | Origen | Estado |
|-------|-----------|---------|--------|--------|
| `cufe` | TEXT | "ABC123XYZ..." | Parámetro URL `chFE` | ✅ Guardado |
| `no` | TEXT | "F001-0001234" | HTML scrapeado | ❌ NO guardado (código usa "numero_factura") |
| `date` | TIMESTAMP | 2024-10-25 14:30:00 | HTML scrapeado | ❌ NO guardado (código usa "fecha_emision") |
| `issuer_name` | TEXT | "EMPRESA S.A." | HTML scrapeado | ❌ NO guardado (código usa "proveedor_nombre") |
| `issuer_ruc` | TEXT | "123456-1-123456" | HTML scrapeado | ❌ NO guardado |
| `issuer_dv` | TEXT | "73" | HTML scrapeado | ❌ NO guardado |
| `issuer_address` | TEXT | "Calle 123" | HTML scrapeado | ❌ NO guardado |
| `issuer_phone` | TEXT | "555-1234" | HTML scrapeado | ❌ NO guardado |
| `receptor_name` | TEXT | "CLIENTE X" | HTML scrapeado | ❌ NO guardado (código usa "cliente_nombre") |
| `receptor_id` | TEXT | "987654-1-654321" | HTML scrapeado | ❌ NO guardado (código usa "cliente_ruc") |
| `receptor_dv` | TEXT | "45" | HTML scrapeado | ❌ NO guardado |
| `receptor_address` | TEXT | "Avenida 456" | HTML scrapeado | ❌ NO guardado |
| `receptor_phone` | TEXT | "555-5678" | HTML scrapeado | ❌ NO guardado |
| `tot_amount` | DOUBLE PRECISION | 107.50 | HTML scrapeado | ❌ NO guardado (código usa "total") |
| `tot_itbms` | DOUBLE PRECISION | 7.50 | HTML scrapeado | ❌ NO guardado (código usa "impuestos") |
| `auth_date` | TEXT | "25/10/2024" | HTML scrapeado | ❌ NO extraído ni guardado |
| `url` | VARCHAR | "https://..." | URL final (post-redirect) | ❌ NO guardado (código usa "source_url") |
| `type` | VARCHAR | "QR"/"CUFE" | Request del usuario | ❌ NO recibido ni guardado |
| `origin` | VARCHAR | "app" | Request del usuario | ❌ Hardcoded, no dinámico |
| `process_date` | TIMESTAMP TZ | 2024-10-25 14:30:00-05 | Sistema (now) | ❌ NO guardado |
| `reception_date` | TIMESTAMP TZ | 2024-10-25 14:30:00-05 | Sistema (now) | ❌ NO guardado |
| `time` | TEXT | "14:30:00" | Sistema (now) | ❌ NO guardado |
| `user_id` | BIGINT | 1 | JWT / Auth | ⚠️ Hardcoded como 1 |
| `user_email` | TEXT | "user@example.com" | JWT / Auth | ❌ NO recibido ni guardado |
| `user_phone_number` | TEXT | "+507 6000-0000" | JWT / Auth | ❌ NO recibido ni guardado |
| `user_telegram_id` | TEXT | "@usuario" | JWT / Auth | ❌ NO recibido ni guardado |
| `user_ws` | VARCHAR | "workspace1" | JWT / Auth | ❌ NO recibido ni guardado |

**Campos que el código intenta guardar pero NO EXISTEN:**
- ❌ `numero_factura` - debe ser `no`
- ❌ `fecha_emision` - debe ser `date`
- ❌ `proveedor_nombre` - debe ser `issuer_name`
- ❌ `cliente_nombre` - debe ser `receptor_name`
- ❌ `cliente_ruc` - debe ser `receptor_id`
- ❌ `subtotal` - NO existe en schema
- ❌ `impuestos` - debe ser `tot_itbms`
- ❌ `total` - debe ser `tot_amount`
- ❌ `moneda` - NO existe en schema
- ❌ `estado` - NO existe en schema
- ❌ `source_url` - debe ser `url`

### Tabla Real: `invoice_detail` (singular, no plural)
| Campo | Tipo Real | Ejemplo | Origen | Estado |
|-------|-----------|---------|--------|--------|
| `cufe` | TEXT | "ABC123XYZ..." | Mismo que header | ✅ Guardado |
| `partkey` | TEXT | "ABC...¦1" | Generado (cufe¦linea) | ❌ NO guardado |
| `date` | TEXT | "25/10/2024" | HTML scrapeado | ❌ NO guardado |
| `quantity` | TEXT | "2.00" | HTML scrapeado | ❌ NO guardado (código usa "cantidad" Decimal) |
| `code` | TEXT | "PROD-001" | HTML scrapeado | ❌ NO extraído ni guardado |
| `description` | TEXT | "Producto X" | HTML scrapeado | ❌ NO guardado (código usa "descripcion") |
| `unit_discount` | TEXT | "5.00" | HTML scrapeado | ❌ NO extraído ni guardado |
| `unit_price` | TEXT | "50.00" | HTML scrapeado | ❌ NO guardado (código usa "precio_unitario" Decimal) |
| `itbms` | TEXT | "7.00" | HTML scrapeado | ❌ NO guardado (código usa "impuesto_monto") |
| `amount` | TEXT | "100.00" | HTML scrapeado | ❌ NO guardado (código usa "subtotal" Decimal) |
| `total` | TEXT | "107.00" | HTML scrapeado | ⚠️ Guardado pero tipo incorrecto (Decimal vs TEXT) |
| `information_of_interest` | TEXT | "Info adicional" | HTML scrapeado | ❌ NO extraído ni guardado |

**Campos que el código intenta guardar pero NO EXISTEN:**
- ❌ `invoice_header_id` - NO existe (relación por CUFE)
- ❌ `item_numero` - NO existe
- ❌ `descripcion` - debe ser `description`
- ❌ `cantidad` - debe ser `quantity` (y TEXT, no Decimal)
- ❌ `precio_unitario` - debe ser `unit_price` (y TEXT, no Decimal)
- ❌ `subtotal` - debe ser `amount` (y TEXT, no Decimal)
- ❌ `impuesto_porcentaje` - NO existe
- ❌ `impuesto_monto` - debe ser `itbms` (y TEXT, no Decimal)

**IMPORTANTE:** ⚠️ Todos los campos son TEXT, NO hay tipos numéricos

### Tabla Real: `invoice_payment` (singular, no plural)
| Campo | Tipo Real | Ejemplo | Origen | Estado |
|-------|-----------|---------|--------|--------|
| `cufe` | TEXT | "ABC123XYZ..." | Mismo que header | ✅ Guardado |
| `forma_de_pago` | TEXT | "EFECTIVO" | HTML scrapeado | ❌ NO guardado (código usa "metodo_pago") |
| `forma_de_pago_otro` | TEXT | "Otro método" | HTML scrapeado | ❌ NO extraído ni guardado |
| `valor_pago` | TEXT | "107.50" | HTML scrapeado | ❌ NO guardado (código usa "monto" Decimal) |
| `efectivo` | TEXT | "100.00" | HTML scrapeado | ❌ NO extraído ni guardado |
| `tarjeta_débito` | TEXT | "50.00" | HTML scrapeado | ❌ NO extraído ni guardado |
| `tarjeta_crédito` | TEXT | "50.00" | HTML scrapeado | ❌ NO extraído ni guardado |
| `tarjeta_clave__banistmo_` | TEXT | "20.00" | HTML scrapeado | ❌ NO extraído ni guardado |
| `vuelto` | TEXT | "2.50" | HTML scrapeado | ❌ NO extraído ni guardado |
| `total_pagado` | TEXT | "110.00" | HTML scrapeado | ❌ NO extraído ni guardado |
| `descuentos` | TEXT | "5.00" | HTML scrapeado | ❌ NO extraído ni guardado |
| `merged` | JSON | {...} | HTML scrapeado | ❌ NO extraído ni guardado |

**Campos que el código intenta guardar pero NO EXISTEN:**
- ❌ `invoice_header_id` - NO existe (relación por CUFE)
- ❌ `metodo_pago` - debe ser `forma_de_pago`
- ❌ `monto` - debe ser `valor_pago` (y TEXT, no Decimal)
- ❌ `referencia` - NO existe como campo separado

**IMPORTANTE:** ⚠️ Todos los campos son TEXT (excepto merged que es JSON)

---

## 🔒 MIDDLEWARE APLICADO

### 1. **Idempotency** (`src/middleware/idempotency.rs`)
- **Aplica a:** `/invoices/process-from-url`
- Requiere header `Idempotency-Key`
- Previene procesamiento duplicado de misma URL

### 2. **Rate Limiting** (`src/middleware/rate_limit.rs`)
- **Scope:** `"invoice_proc"`
- Límites **dinámicos** basados en Trust Score del usuario
- Valida requests por hora/minuto

### 3. **Authentication**
- Requiere JWT válido
- Extrae `user_id` del token

---

## ⚠️ CASOS ESPECIALES

### CUFE No Encontrado
- Se usa `"UNKNOWN"` como CUFE
- Se crea header mínimo con fecha actual
- **Sigue procesando** (no falla)

### HTML Sin Datos
- Retorna header con solo CUFE y timestamp
- `details` y `payments` vacíos
- **No falla** el proceso

### URL con Redirección
- Captura URL final
- **Guarda URL final en DB** (no la original)
- Log indica redirección

### Factura Duplicada
- Verifica por CUFE antes de insertar
- Retorna respuesta "exitosa" pero con mensaje de duplicado
- **No falla con error 500**

---

## 📊 LOGGING

```
INFO: Processing URL request: https://...
INFO: Starting to scrape invoice from URL: https://...
INFO: Fetching HTML with final URL tracking from: https://...
INFO: 🔄 URL redirection in scraping: original → final
INFO: Successfully fetched HTML content (12345 bytes) from final URL: https://...
INFO: Extracting invoice header from document using ocr_extractor
INFO: Extracted data - RUC: Some("123456-1-123456"), Nombre: Some("EMPRESA S.A."), Total: Some(107.50), ITBMS: Some(7.50)
INFO: Saving invoice header with CUFE: ABC123XYZ
INFO: Saving 3 invoice details for invoice_id: 123
INFO: Saving 1 invoice payments for invoice_id: 123
```

---

## 🎯 RESUMEN EJECUTIVO

### Input:
- URL de factura DGI Panamá

### Proceso:
1. Fetch HTML (sigue redirects)
2. Extrae CUFE de URL
3. Parsea HTML para extraer datos
4. Valida duplicados por CUFE
5. Inserta en 3 tablas (headers, details, payments) en transacción
6. Retorna ID generado + CUFE

### Output:
- `invoice_id`: ID generado
- `cufe`: CUFE extraído
- `success`: boolean
- `processing_time_ms`: tiempo de ejecución

### Tablas Afectadas:
1. ✅ `invoice_headers` (1 registro)
2. ✅ `invoice_details` (N registros)
3. ✅ `invoice_payments` (M registros)

### Garantías:
- ✅ Transaccional (todo o nada)
- ✅ Previene duplicados
- ✅ Maneja redirects
- ✅ Resiliente a datos faltantes
- ✅ Rate limited
- ✅ Idempotente

---

## 📝 NOTAS IMPORTANTES

1. **Extracción de detalles:** Implementación básica, puede mejorarse para extraer items reales del HTML
2. **Campos no guardados del header:** `issuer_dv`, `issuer_address`, `issuer_phone`, `receptor_dv`, `receptor_address`, `receptor_phone` se extraen pero no se guardan (no están en el schema de DB)
3. **Subtotal:** Se deja NULL en DB, debería calcularse sumando detalles
4. **User_id:** Se asume que viene del contexto (JWT), pero actualmente se hardcodea como 1 en algunas partes
5. **Moneda:** Siempre "PAB" (Balboa Panameño)
6. **Estado:** Siempre "ACTIVO"

---

## 🚨 ESTADO ACTUAL: REQUIERE CORRECCIÓN

### ❌ PROBLEMA CRÍTICO IDENTIFICADO

**La implementación actual NO coincide con el schema real de la base de datos.**

El código en `src/api/database_persistence.rs` intenta insertar en:
- Tablas con nombres incorrectos (`invoice_headers` vs `invoice_header`)
- Campos que NO existen (`numero_factura`, `fecha_emision`, `proveedor_nombre`, etc.)
- Tipos de datos incorrectos (Decimal en lugar de TEXT)

**Resultado:** ⚠️ Todos los inserts fallan silenciosamente. NO se guarda NADA en la base de datos real.

### 📄 Documentación Relacionada

Para el análisis completo de lo que falta corregir, ver:
- **`DATABASE_SCHEMA_ANALYSIS.md`** - Análisis detallado de schema real vs implementación
- **`INVOICE_EXTRACTION_DOCUMENTATION.md`** - Documentación de extracción de campos del HTML

### ✅ Próximos Pasos

1. Corregir nombres de tablas y campos en `database_persistence.rs`
2. Cambiar tipos de datos (Decimal → String en details/payments)
3. Agregar campos faltantes al request (user_email, type, etc.)
4. Agregar extracción de campos faltantes (auth_date, code, etc.)
5. Implementar extracción real de details y payments (actualmente mock)

---

**Generado:** 2024-10-01
**Versión API:** v4
**Endpoint:** `POST /api/v4/invoices/process-from-url`
**Estado:** ⚠️ REQUIERE CORRECCIÓN INMEDIATA
