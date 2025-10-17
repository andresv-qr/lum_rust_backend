# Plan de Corrección: `/invoices/process-from-url`

**Fecha:** 2024-10-01  
**Prioridad:** 🔴 CRÍTICA  
**Estado:** Pendiente de implementación

---

## 🎯 OBJETIVO

Corregir la implementación del endpoint `/invoices/process-from-url` para que coincida con el schema real de la base de datos y guarde correctamente todos los campos extraídos.

---

## 📋 CHECKLIST DE CORRECCIONES

### ✅ Fase 1: Correcciones Críticas (BLOQUEANTE)

#### 1.1 Corregir Nombres de Tablas
**Archivo:** `src/api/database_persistence.rs`

- [ ] Línea 123: `invoice_headers` → `invoice_header`
- [ ] Línea 158: `invoice_details` → `invoice_detail`
- [ ] Línea 195: `invoice_payments` → `invoice_payment`

#### 1.2 Corregir Campos de `invoice_header`
**Archivo:** `src/api/database_persistence.rs` (línea 111-135)

Query actual:
```sql
INSERT INTO invoice_headers (
    cufe, numero_factura, fecha_emision, proveedor_nombre, proveedor_ruc,
    cliente_nombre, cliente_ruc, subtotal, impuestos, total, moneda,
    estado, user_id, source_url
)
```

Query corregida:
```sql
INSERT INTO invoice_header (
    cufe, 
    no, 
    date, 
    issuer_name, 
    issuer_ruc,
    issuer_dv,
    issuer_address,
    issuer_phone,
    receptor_name, 
    receptor_id,
    receptor_dv,
    receptor_address,
    receptor_phone,
    tot_amount, 
    tot_itbms,
    auth_date,
    url,
    type,
    origin,
    process_date,
    reception_date,
    time,
    user_id,
    user_email,
    user_phone_number,
    user_telegram_id,
    user_ws
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)
```

**Cambios en el código:**
```rust
// ANTES:
header.no, // numero_factura
parse_date_string(&header.date), // fecha_emision
header.issuer_name, // proveedor_nombre
header.issuer_ruc, // proveedor_ruc
header.receptor_name, // cliente_nombre
header.receptor_id, // cliente_ruc
None::<rust_decimal::Decimal>, // subtotal
header.tot_itbms, // impuestos
header.tot_amount, // total
Some("PAB".to_string()), // moneda
Some("ACTIVO".to_string()), // estado
header.user_id,
Some(header.url.clone()), // source_url

// DESPUÉS:
header.no, // no (directo)
header.date, // date (String directo, no parsear)
header.issuer_name, // issuer_name
header.issuer_ruc, // issuer_ruc
header.issuer_dv, // issuer_dv (NUEVO)
header.issuer_address, // issuer_address (NUEVO)
header.issuer_phone, // issuer_phone (NUEVO)
header.receptor_name, // receptor_name
header.receptor_id, // receptor_id
header.receptor_dv, // receptor_dv (NUEVO)
header.receptor_address, // receptor_address (NUEVO)
header.receptor_phone, // receptor_phone (NUEVO)
header.tot_amount, // tot_amount (f64, no Decimal)
header.tot_itbms, // tot_itbms (f64, no Decimal)
header.auth_date, // auth_date (NUEVO)
header.url, // url (no "source_url")
header.type_field, // type (NUEVO)
header.origin, // origin (dinámico, no hardcoded)
header.process_date, // process_date
header.reception_date, // reception_date
header.time, // time (NUEVO)
header.user_id,
header.user_email, // user_email (NUEVO)
header.user_phone_number, // user_phone_number (NUEVO)
header.user_telegram_id, // user_telegram_id (NUEVO)
header.user_ws, // user_ws (NUEVO)
```

#### 1.3 Corregir Campos de `invoice_detail`
**Archivo:** `src/api/database_persistence.rs` (línea 148-175)

Query actual:
```sql
INSERT INTO invoice_details (
    invoice_header_id, cufe, item_numero, descripcion, cantidad,
    precio_unitario, subtotal, impuesto_porcentaje, impuesto_monto, total
)
```

Query corregida:
```sql
INSERT INTO invoice_detail (
    cufe,
    partkey,
    date,
    quantity,
    code,
    description,
    unit_discount,
    unit_price,
    itbms,
    amount,
    total,
    information_of_interest
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
```

**Cambios en el código:**
```rust
// ELIMINAR de struct InvoiceDetail:
invoice_header_id: Option<i32>,
item_numero: Option<i32>,

// CAMBIAR tipos de Decimal a String:
cantidad: Option<rust_decimal::Decimal> → quantity: Option<String>
precio_unitario: Option<rust_decimal::Decimal> → unit_price: Option<String>
subtotal: Option<rust_decimal::Decimal> → amount: Option<String>
impuesto_monto: Option<rust_decimal::Decimal> → itbms: Option<String>
total: Option<rust_decimal::Decimal> → total: Option<String>

// ELIMINAR:
impuesto_porcentaje: Option<rust_decimal::Decimal>

// AGREGAR:
partkey: Option<String>,
date: Option<String>,
code: Option<String>,
unit_discount: Option<String>,
information_of_interest: Option<String>,
```

#### 1.4 Corregir Campos de `invoice_payment`
**Archivo:** `src/api/database_persistence.rs` (línea 190-208)

Query actual:
```sql
INSERT INTO invoice_payments (
    invoice_header_id, cufe, metodo_pago, monto, referencia
)
```

Query corregida:
```sql
INSERT INTO invoice_payment (
    cufe,
    forma_de_pago,
    forma_de_pago_otro,
    valor_pago,
    efectivo,
    tarjeta_débito,
    tarjeta_crédito,
    tarjeta_clave__banistmo_,
    vuelto,
    total_pagado,
    descuentos,
    merged
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
```

**Cambios en el código:**
```rust
// ELIMINAR:
invoice_header_id: Option<i32>,
referencia: Option<String>,

// CAMBIAR:
metodo_pago → forma_de_pago
monto: Option<rust_decimal::Decimal> → valor_pago: Option<String>

// AGREGAR:
forma_de_pago_otro: Option<String>,
efectivo: Option<String>,
tarjeta_debito: Option<String>,
tarjeta_credito: Option<String>,
tarjeta_clave_banistmo: Option<String>,
vuelto: Option<String>,
total_pagado: Option<String>,
descuentos: Option<String>,
merged: Option<serde_json::Value>,
```

#### 1.5 Actualizar Struct InvoiceHeader
**Archivo:** `src/api/webscraping/mod.rs` (línea 11-47)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceHeader {
    // Core invoice fields
    pub cufe: String,
    pub no: Option<String>,
    pub date: Option<String>, // String directo, no timestamp
    pub auth_date: Option<String>,
    pub tot_amount: Option<f64>, // CAMBIAR de Decimal a f64
    pub tot_itbms: Option<f64>, // CAMBIAR de Decimal a f64
    
    // Issuer fields  
    pub issuer_name: Option<String>,
    pub issuer_ruc: Option<String>,
    pub issuer_dv: Option<String>,
    pub issuer_address: Option<String>,
    pub issuer_phone: Option<String>,
    
    // Receptor fields
    pub receptor_name: Option<String>,
    pub receptor_id: Option<String>,
    pub receptor_dv: Option<String>,
    pub receptor_address: Option<String>,
    pub receptor_phone: Option<String>,
    
    // User fields
    pub user_id: i64, // CAMBIAR de i32 a i64 (BIGINT)
    pub user_email: Option<String>,
    pub user_phone_number: Option<String>,
    pub user_telegram_id: Option<String>,
    pub user_ws: Option<String>,
    
    // Processing metadata
    pub origin: String,
    pub type_field: String, // "QR" o "CUFE"
    pub url: String,
    pub process_date: chrono::DateTime<chrono::Utc>,
    pub reception_date: chrono::DateTime<chrono::Utc>,
    pub time: Option<String>,
}
```

#### 1.6 Actualizar Request Body
**Archivo:** `src/api/url_processing_v4.rs` (línea 17-20)

```rust
#[derive(serde::Deserialize)]
pub struct UrlRequest {
    pub url: String,
    
    // Campos del usuario
    pub type_field: Option<String>, // "QR" o "CUFE"
    pub origin: Option<String>, // "app", "whatsapp", "telegram"
    pub user_email: Option<String>,
    pub user_phone_number: Option<String>,
    pub user_telegram_id: Option<String>,
    pub user_ws: Option<String>,
}
```

#### 1.7 Eliminar Parsing de Fecha
**Archivo:** `src/api/database_persistence.rs` (línea 11-29)

```rust
// ELIMINAR toda la función parse_date_string()
// La BD acepta String directamente en el formato DD/MM/YYYY HH:MM:SS
```

---

### ⚠️ Fase 2: Mejoras de Extracción

#### 2.1 Extraer `auth_date` del HTML
**Archivo:** `src/api/webscraping/mod.rs`

Agregar extracción de fecha de autorización:
```rust
// Buscar en HTML: "CÓDIGO DE AUTORIZACIÓN" o "PROTOCOLO"
// XPath: //dt[contains(text(), 'CÓDIGO DE AUTORIZACIÓN')]/following-sibling::dd/text()
```

#### 2.2 Implementar Extracción Real de Details
**Archivo:** `src/api/webscraping/mod.rs` (línea 297-328)

Actualmente retorna datos mock. Implementar extracción real de tabla de items:
- Buscar tabla con items de factura
- Extraer: code, description, quantity, unit_price, unit_discount, itbms, amount, total
- Generar partkey: `{cufe}|{line_number}`

#### 2.3 Implementar Extracción Real de Payments
**Archivo:** `src/api/webscraping/mod.rs` (línea 329-355)

Actualmente retorna datos mock. Implementar extracción real de información de pago:
- Buscar sección de pagos
- Extraer: forma_de_pago, efectivo, tarjetas, vuelto, total_pagado, descuentos

---

### 🔵 Fase 3: Validaciones y Testing

#### 3.1 Agregar Validaciones
- [ ] Validar formato CUFE (debe empezar con "FE")
- [ ] Validar formato de fecha (DD/MM/YYYY HH:MM:SS)
- [ ] Validar formato RUC (XXX-X-XXX)
- [ ] Validar que tot_amount > 0
- [ ] Validar que user_id exista

#### 3.2 Tests Unitarios
- [ ] Test de extracción de campos del HTML
- [ ] Test de conversión de tipos (Decimal → String)
- [ ] Test de queries SQL con schema correcto
- [ ] Test de validación de duplicados

#### 3.3 Tests de Integración
- [ ] Test end-to-end con URL real
- [ ] Test de manejo de errores
- [ ] Test de transacciones (rollback en caso de error)

---

## 🛠️ ARCHIVOS A MODIFICAR

### Archivos Críticos (Fase 1)
1. ✅ `src/api/webscraping/mod.rs` - Structs y extracción
2. ✅ `src/api/database_persistence.rs` - Queries SQL
3. ✅ `src/api/url_processing_v4.rs` - Request handling
4. ⚠️ `src/api/templates/url_processing_templates.rs` - Response templates

### Archivos Opcionales (Fases 2-3)
5. 🔵 `src/processing/web_scraping/ocr_extractor.rs` - Mejoras de extracción
6. 🔵 `tests/integration/url_processing_tests.rs` - Tests

---

## 📝 EJEMPLO DE CORRECCIÓN COMPLETA

### Antes (Incorrecto):
```rust
// database_persistence.rs
let rec = sqlx::query!(
    r#"
    INSERT INTO invoice_headers (
        cufe, numero_factura, fecha_emision, proveedor_nombre, 
        proveedor_ruc, cliente_nombre, cliente_ruc, subtotal, 
        impuestos, total, moneda, estado, user_id, source_url
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
    RETURNING id
    "#,
    header.cufe,
    header.no,
    parse_date_string(&header.date),
    header.issuer_name,
    header.issuer_ruc,
    header.receptor_name,
    header.receptor_id,
    None::<rust_decimal::Decimal>,
    header.tot_itbms,
    header.tot_amount,
    Some("PAB".to_string()),
    Some("ACTIVO".to_string()),
    header.user_id,
    Some(header.url.clone()),
)
.fetch_one(&mut **tx)
.await?;
```

### Después (Correcto):
```rust
// database_persistence.rs
let rec = sqlx::query!(
    r#"
    INSERT INTO invoice_header (
        cufe, no, date, issuer_name, issuer_ruc, issuer_dv, 
        issuer_address, issuer_phone, receptor_name, receptor_id, 
        receptor_dv, receptor_address, receptor_phone, tot_amount, 
        tot_itbms, auth_date, url, type, origin, process_date, 
        reception_date, time, user_id, user_email, user_phone_number, 
        user_telegram_id, user_ws
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 
            $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, 
            $25, $26, $27)
    RETURNING cufe
    "#,
    header.cufe,
    header.no,
    header.date, // String directo, no parsear
    header.issuer_name,
    header.issuer_ruc,
    header.issuer_dv,
    header.issuer_address,
    header.issuer_phone,
    header.receptor_name,
    header.receptor_id,
    header.receptor_dv,
    header.receptor_address,
    header.receptor_phone,
    header.tot_amount, // f64, no Decimal
    header.tot_itbms, // f64, no Decimal
    header.auth_date,
    header.url,
    header.type_field,
    header.origin,
    header.process_date,
    header.reception_date,
    header.time,
    header.user_id as i64, // BIGINT
    header.user_email,
    header.user_phone_number,
    header.user_telegram_id,
    header.user_ws,
)
.fetch_one(&mut **tx)
.await?;
```

---

## 🎯 CRITERIOS DE ÉXITO

### ✅ Funcionalidad Básica
- [ ] El endpoint guarda datos en la BD sin errores SQL
- [ ] Se guardan todos los campos del header correctamente
- [ ] Se guardan los detalles (aunque sean mock por ahora)
- [ ] Se guardan los pagos (aunque sean mock por ahora)
- [ ] La respuesta incluye el CUFE y confirma guardado exitoso

### ✅ Validación de Datos
- [ ] Los campos extraídos del HTML son correctos
- [ ] Los tipos de datos coinciden con el schema (TEXT, f64, etc.)
- [ ] Los timestamps tienen la zona horaria correcta
- [ ] No se insertan valores hardcoded innecesarios

### ✅ Manejo de Errores
- [ ] Los duplicados se detectan y reportan correctamente
- [ ] Los errores SQL se loggean y se manejan
- [ ] Las transacciones se revierten en caso de error
- [ ] La respuesta indica claramente el tipo de error

---

## 📊 ESTIMACIÓN DE ESFUERZO

| Fase | Tareas | Complejidad | Tiempo Est. |
|------|--------|-------------|-------------|
| Fase 1.1-1.4 | Corrección de queries SQL | Baja | 2 horas |
| Fase 1.5-1.7 | Actualización de structs | Media | 3 horas |
| Fase 2.1 | Extracción auth_date | Baja | 1 hora |
| Fase 2.2 | Extracción details real | Alta | 8 horas |
| Fase 2.3 | Extracción payments real | Alta | 8 horas |
| Fase 3 | Testing y validaciones | Media | 4 horas |
| **TOTAL** | | | **26 horas** |

### Prioridades:
- 🔴 **URGENTE:** Fase 1 (5 horas) - Sin esto, el endpoint NO funciona
- 🟡 **IMPORTANTE:** Fase 2.1 (1 hora) - Campo faltante simple
- 🔵 **DESEABLE:** Fases 2.2-2.3 (16 horas) - Mejora de extracción
- 🟢 **OPCIONAL:** Fase 3 (4 horas) - Calidad y robustez

---

## 📚 REFERENCIAS

- **Schema Real:** Proporcionado por el usuario
- **Análisis Completo:** `DATABASE_SCHEMA_ANALYSIS.md`
- **Análisis Paso a Paso:** `PROCESS_FROM_URL_ANALYSIS.md`
- **Extracción HTML:** `INVOICE_EXTRACTION_DOCUMENTATION.md`

---

**Generado:** 2024-10-01  
**Última actualización:** 2024-10-01  
**Estado:** 📋 Pendiente de implementación
