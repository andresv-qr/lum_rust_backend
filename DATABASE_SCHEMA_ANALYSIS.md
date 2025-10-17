# Análisis de Schema de Base de Datos - `/invoices/process-from-url`

**Fecha:** 2024-10-01  
**Endpoint:** `POST /api/v4/invoices/process-from-url`

---

## 🚨 PROBLEMA CRÍTICO IDENTIFICADO

### **La implementación actual NO coincide con el schema real de la base de datos**

El código en `src/api/database_persistence.rs` intenta insertar en tablas con nombres y campos que **NO EXISTEN** en la base de datos real.

---

## 📊 COMPARACIÓN: CÓDIGO vs BASE DE DATOS REAL

### Tabla 1: Invoice Headers

#### ❌ CÓDIGO ACTUAL (INCORRECTO)
```rust
// Archivo: src/api/database_persistence.rs (línea 111)
INSERT INTO invoice_headers (
    cufe, numero_factura, fecha_emision, proveedor_nombre, proveedor_ruc,
    cliente_nombre, cliente_ruc, subtotal, impuestos, total, moneda,
    estado, user_id, source_url
)
```

#### ✅ SCHEMA REAL (CORRECTO)
```sql
-- Tabla: public.invoice_header (singular, no plural)
Campos existentes:
- cufe (text)
- no (text) -- NO "numero_factura"
- date (timestamp without time zone) -- NO "fecha_emision"
- issuer_name (text) -- NO "proveedor_nombre"
- issuer_ruc (text) -- NO "proveedor_ruc"
- issuer_dv (text)
- issuer_address (text)
- issuer_phone (text)
- receptor_name (text) -- NO "cliente_nombre"
- receptor_id (text) -- NO "cliente_ruc"
- receptor_dv (text)
- receptor_address (text)
- receptor_phone (text)
- tot_amount (double precision) -- NO "total"
- tot_itbms (double precision) -- NO "impuestos"
- auth_date (text)
- url (character varying)
- type (character varying)
- origin (character varying)
- process_date (timestamp with time zone)
- reception_date (timestamp with time zone)
- time (text)
- user_id (bigint)
- user_email (text)
- user_phone_number (text)
- user_telegram_id (text)
- user_ws (character varying)
```

**Campos que NO EXISTEN en la BD real:**
- ❌ `numero_factura` → debe ser `no`
- ❌ `fecha_emision` → debe ser `date`
- ❌ `proveedor_nombre` → debe ser `issuer_name`
- ❌ `proveedor_ruc` → debe ser `issuer_ruc` (existe pero con otro nombre)
- ❌ `cliente_nombre` → debe ser `receptor_name`
- ❌ `cliente_ruc` → debe ser `receptor_id`
- ❌ `subtotal` → NO existe
- ❌ `impuestos` → debe ser `tot_itbms`
- ❌ `total` → debe ser `tot_amount`
- ❌ `moneda` → NO existe
- ❌ `estado` → NO existe
- ❌ `source_url` → debe ser `url`

**Campos FALTANTES en el código actual:**
- ⚠️ `issuer_dv` - existe en extracción pero no se guarda
- ⚠️ `issuer_address` - existe en extracción pero no se guarda
- ⚠️ `issuer_phone` - existe en extracción pero no se guarda
- ⚠️ `receptor_dv` - existe en extracción pero no se guarda
- ⚠️ `receptor_address` - existe en extracción pero no se guarda
- ⚠️ `receptor_phone` - existe en extracción pero no se guarda
- ⚠️ `auth_date` - NO se extrae ni se guarda
- ⚠️ `type` - NO se guarda (debe venir del usuario)
- ⚠️ `time` - NO se guarda
- ⚠️ `user_phone_number` - NO se guarda
- ⚠️ `user_telegram_id` - NO se guarda
- ⚠️ `user_ws` - NO se guarda

---

### Tabla 2: Invoice Details

#### ❌ CÓDIGO ACTUAL (INCORRECTO)
```rust
// Archivo: src/api/database_persistence.rs (línea 148)
INSERT INTO invoice_details (
    invoice_header_id, cufe, item_numero, descripcion, cantidad,
    precio_unitario, subtotal, impuesto_porcentaje, impuesto_monto, total
)
```

#### ✅ SCHEMA REAL (CORRECTO)
```sql
-- Tabla: public.invoice_detail (singular, no plural)
Campos existentes:
- cufe (text)
- partkey (text)
- date (text)
- quantity (text) -- NO "cantidad"
- code (text)
- description (text) -- NO "descripcion"
- unit_discount (text)
- unit_price (text) -- NO "precio_unitario"
- itbms (text) -- NO "impuesto_monto"
- amount (text) -- NO "subtotal"
- total (text)
- information_of_interest (text)
```

**Campos que NO EXISTEN en la BD real:**
- ❌ `invoice_header_id` → NO existe (relación por CUFE)
- ❌ `item_numero` → NO existe
- ❌ `descripcion` → debe ser `description`
- ❌ `cantidad` → debe ser `quantity`
- ❌ `precio_unitario` → debe ser `unit_price`
- ❌ `subtotal` → debe ser `amount`
- ❌ `impuesto_porcentaje` → NO existe
- ❌ `impuesto_monto` → debe ser `itbms`

**Campos FALTANTES en el código actual:**
- ⚠️ `partkey` - Campo de llave de partición (cufe|linea)
- ⚠️ `date` - Fecha de emisión
- ⚠️ `code` - Código del producto
- ⚠️ `unit_discount` - Descuento unitario
- ⚠️ `information_of_interest` - Información adicional

**IMPORTANTE:** ⚠️ Todos los campos son tipo `text` en la BD real, NO hay tipos numéricos

---

### Tabla 3: Invoice Payments

#### ❌ CÓDIGO ACTUAL (INCORRECTO)
```rust
// Archivo: src/api/database_persistence.rs (línea 190)
INSERT INTO invoice_payments (
    invoice_header_id, cufe, metodo_pago, monto, referencia
)
```

#### ✅ SCHEMA REAL (CORRECTO)
```sql
-- Tabla: public.invoice_payment (singular, no plural)
Campos existentes:
- cufe (text)
- forma_de_pago (text) -- NO "metodo_pago"
- forma_de_pago_otro (text)
- valor_pago (text) -- NO "monto"
- efectivo (text)
- tarjeta_débito (text)
- tarjeta_crédito (text)
- tarjeta_clave__banistmo_ (text)
- vuelto (text)
- total_pagado (text)
- descuentos (text)
- merged (json)
```

**Campos que NO EXISTEN en la BD real:**
- ❌ `invoice_header_id` → NO existe (relación por CUFE)
- ❌ `metodo_pago` → debe ser `forma_de_pago`
- ❌ `monto` → debe ser `valor_pago`
- ❌ `referencia` → NO existe como campo separado

**Campos FALTANTES en el código actual:**
- ⚠️ `forma_de_pago_otro` - Otra forma de pago
- ⚠️ `efectivo` - Monto en efectivo
- ⚠️ `tarjeta_débito` - Monto en tarjeta débito
- ⚠️ `tarjeta_crédito` - Monto en tarjeta crédito
- ⚠️ `tarjeta_clave__banistmo_` - Tarjeta clave Banistmo
- ⚠️ `vuelto` - Vuelto dado
- ⚠️ `total_pagado` - Total pagado
- ⚠️ `descuentos` - Descuentos aplicados
- ⚠️ `merged` - Datos JSON adicionales

**IMPORTANTE:** ⚠️ Todos los campos son tipo `text`, NO hay tipos numéricos

---

## 🔍 ANÁLISIS DE CAMPOS POR ORIGEN

### Campos Extraídos del HTML (Web Scraping)

Estos campos se extraen del HTML de la factura DGI:

| Campo | Implementado | Se Guarda | Tabla Real |
|-------|--------------|-----------|------------|
| `no` (número factura) | ✅ | ❌ | invoice_header.no |
| `date` (fecha emisión) | ✅ | ❌ | invoice_header.date |
| `cufe` | ✅ | ✅ | invoice_header.cufe |
| `issuer_name` | ✅ | ❌ | invoice_header.issuer_name |
| `issuer_ruc` | ✅ | ❌ | invoice_header.issuer_ruc |
| `issuer_dv` | ✅ | ❌ | invoice_header.issuer_dv |
| `issuer_address` | ✅ | ❌ | invoice_header.issuer_address |
| `issuer_phone` | ✅ | ❌ | invoice_header.issuer_phone |
| `receptor_name` | ✅ | ❌ | invoice_header.receptor_name |
| `receptor_id` | ✅ | ❌ | invoice_header.receptor_id |
| `receptor_dv` | ✅ | ❌ | invoice_header.receptor_dv |
| `receptor_address` | ✅ | ❌ | invoice_header.receptor_address |
| `receptor_phone` | ✅ | ❌ | invoice_header.receptor_phone |
| `tot_amount` | ✅ | ❌ | invoice_header.tot_amount |
| `tot_itbms` | ✅ | ❌ | invoice_header.tot_itbms |
| `auth_date` | ❌ | ❌ | invoice_header.auth_date |

**Resumen:** Se extraen 14 de 16 campos, pero NINGUNO se guarda correctamente en la BD.

---

### Campos Proporcionados por el Usuario (API Input)

Estos campos deben venir en el request o del contexto del usuario:

| Campo | Origen | Implementado | Se Guarda |
|-------|--------|--------------|-----------|
| `url` | Request body | ✅ | ❌ (se guarda como "source_url") |
| `type` | Request body | ❌ | ❌ |
| `origin` | Request body / headers | ✅ | ❌ (hardcoded "app") |
| `user_id` | JWT / Auth | ✅ | ✅ |
| `user_email` | JWT / Auth | ❌ | ❌ |
| `user_phone_number` | JWT / Auth | ❌ | ❌ |
| `user_telegram_id` | JWT / Auth | ❌ | ❌ |
| `user_ws` | JWT / Auth | ❌ | ❌ |
| `process_date` | Sistema (now) | ✅ | ❌ |
| `reception_date` | Sistema (now) | ✅ | ❌ |
| `time` | Sistema (now) | ❌ | ❌ |

**Resumen:** Faltan 8 de 11 campos de usuario en el request/respuesta.

---

## 📝 LISTADO COMPLETO: QUÉ FALTA IMPLEMENTAR

### 1. Actualizar Nombres de Tablas
- ❌ `invoice_headers` → ✅ `invoice_header` (singular)
- ❌ `invoice_details` → ✅ `invoice_detail` (singular)
- ❌ `invoice_payments` → ✅ `invoice_payment` (singular)

### 2. Actualizar Campos de `invoice_header`

**Campos a corregir:**
```rust
// ANTES (incorrecto):
numero_factura → CAMBIAR A: no
fecha_emision → CAMBIAR A: date  
proveedor_nombre → CAMBIAR A: issuer_name
proveedor_ruc → CAMBIAR A: issuer_ruc (ya correcto)
cliente_nombre → CAMBIAR A: receptor_name
cliente_ruc → CAMBIAR A: receptor_id
impuestos → CAMBIAR A: tot_itbms
total → CAMBIAR A: tot_amount
source_url → CAMBIAR A: url

// ELIMINAR (no existen):
subtotal
moneda
estado
```

**Campos a AGREGAR:**
```rust
issuer_dv: Option<String>,
issuer_address: Option<String>,
issuer_phone: Option<String>,
receptor_dv: Option<String>,
receptor_address: Option<String>,
receptor_phone: Option<String>,
auth_date: Option<String>,
type_field: Option<String>, // "QR" o "CUFE"
time: Option<String>,
user_phone_number: Option<String>,
user_telegram_id: Option<String>,
user_ws: Option<String>,
```

**Cambios de tipo:**
```rust
// ANTES:
date: Option<String> → parse_date_string() → NaiveDate

// AHORA:
date: timestamp without time zone (puede recibir String y la BD lo convierte)
tot_amount: Option<rust_decimal::Decimal> → CAMBIAR A: Option<f64> (double precision)
tot_itbms: Option<rust_decimal::Decimal> → CAMBIAR A: Option<f64> (double precision)
```

### 3. Actualizar Campos de `invoice_detail`

**Campos a corregir:**
```rust
// ELIMINAR:
invoice_header_id: Option<i32>, // NO existe FK explícito
item_numero: Option<i32>, // NO existe

// CAMBIAR nombres:
descripcion → description
cantidad → quantity
precio_unitario → unit_price
subtotal → amount
impuesto_monto → itbms
// total ya es correcto

// ELIMINAR:
impuesto_porcentaje // NO existe
```

**Campos a AGREGAR:**
```rust
partkey: Option<String>, // cufe|linea
date: Option<String>, // fecha de emisión
code: Option<String>, // código producto
unit_discount: Option<String>,
information_of_interest: Option<String>,
```

**Cambios de tipo:**
```rust
// TODOS los campos deben ser String, NO Decimal
// ANTES:
cantidad: Option<rust_decimal::Decimal>
precio_unitario: Option<rust_decimal::Decimal>
subtotal: Option<rust_decimal::Decimal>
total: Option<rust_decimal::Decimal>

// AHORA:
quantity: Option<String>
unit_price: Option<String>
amount: Option<String>
total: Option<String>
itbms: Option<String>
```

### 4. Actualizar Campos de `invoice_payment`

**Campos a corregir:**
```rust
// ELIMINAR:
invoice_header_id: Option<i32>, // NO existe FK explícito
referencia: Option<String>, // NO existe como campo separado

// CAMBIAR nombres:
metodo_pago → forma_de_pago
monto → valor_pago
```

**Campos a AGREGAR:**
```rust
forma_de_pago_otro: Option<String>,
efectivo: Option<String>,
tarjeta_debito: Option<String>,
tarjeta_credito: Option<String>,
tarjeta_clave_banistmo: Option<String>,
vuelto: Option<String>,
total_pagado: Option<String>,
descuentos: Option<String>,
merged: Option<serde_json::Value>, // JSON
```

**Cambios de tipo:**
```rust
// ANTES:
monto: Option<rust_decimal::Decimal>

// AHORA:
valor_pago: Option<String>
efectivo: Option<String>
// TODOS string
```

### 5. Agregar Campos al Request

**Actualizar `UrlRequest` (línea 17):**
```rust
#[derive(serde::Deserialize)]
pub struct UrlRequest {
    pub url: String,
    
    // Campos opcionales del usuario
    pub type_field: Option<String>, // "QR" o "CUFE"
    pub origin: Option<String>, // "app", "whatsapp", "telegram"
    pub user_email: Option<String>,
    pub user_phone_number: Option<String>,
    pub user_telegram_id: Option<String>,
    pub user_ws: Option<String>,
}
```

### 6. Actualizar Extracción de Datos

El módulo `webscraping` ya extrae muchos campos correctamente, pero hay que asegurar que se mapeen bien:

```rust
// src/api/webscraping/mod.rs
// Los campos ya existen en InvoiceHeader pero no se guardan
// Verificar que se extraen:
- issuer_dv ✅
- issuer_address ✅
- issuer_phone ✅
- receptor_dv ✅
- receptor_address ✅
- receptor_phone ✅
- auth_date ❌ (agregar extracción)
```

### 7. Cambiar Tipos de Retorno

**Eliminar parsing de fecha:**
```rust
// ANTES (database_persistence.rs línea 127):
parse_date_string(&header.date) // → NaiveDate

// AHORA:
header.date // → String directo, la BD lo convierte
```

---

## 🎯 PLAN DE ACCIÓN RECOMENDADO

### Fase 1: Corrección Crítica (URGENTE)
1. ✅ Cambiar nombres de tablas (headers → header, etc.)
2. ✅ Corregir nombres de campos en queries SQL
3. ✅ Cambiar tipos Decimal → String en details y payments
4. ✅ Cambiar Decimal → f64 en header (tot_amount, tot_itbms)
5. ✅ Eliminar campos que no existen (subtotal, moneda, estado)
6. ✅ Corregir source_url → url

### Fase 2: Campos Faltantes (MEDIO)
1. ⚠️ Agregar campos de usuario al request
2. ⚠️ Agregar campos faltantes a invoice_header
3. ⚠️ Agregar campos faltantes a invoice_detail
4. ⚠️ Agregar campos faltantes a invoice_payment
5. ⚠️ Implementar extracción de auth_date del HTML

### Fase 3: Optimización (BAJO)
1. 🔵 Implementar extracción real de details (actualmente mock)
2. 🔵 Implementar extracción real de payments (actualmente mock)
3. 🔵 Validar formato de campos según documentación
4. 🔵 Agregar logs de auditoría

---

## ⚠️ RIESGOS ACTUALES

### 🔴 CRÍTICO: La API actual NO FUNCIONA
- Las queries SQL fallan porque las tablas/campos no existen
- Todos los inserts retornan error de PostgreSQL
- No se está guardando NADA en la base de datos real
- El error está siendo capturado silenciosamente

### 🟡 MEDIO: Pérdida de Datos
- Se extraen campos del HTML que NO se guardan
- Se ignoran campos de usuario importantes
- No hay validación de datos antes de guardar

### 🟢 BAJO: Inconsistencias
- Tipos de datos incorrectos (Decimal vs String)
- Fechas parseadas innecesariamente
- Hardcoding de valores (moneda, estado)

---

## 📚 REFERENCIAS

- **Schema Real:** Proporcionado por el usuario (tabla con columnas)
- **Documentación de Extracción:** `/home/client_1099_1/scripts/lum_rust_ws/INVOICE_EXTRACTION_DOCUMENTATION.md`
- **Código Actual:** 
  - `src/api/url_processing_v4.rs`
  - `src/api/webscraping/mod.rs`
  - `src/api/database_persistence.rs`
- **Análisis Previo:** `PROCESS_FROM_URL_ANALYSIS.md`

---

**Generado:** 2024-10-01  
**Estado:** ⚠️ REQUIERE CORRECCIÓN INMEDIATA
