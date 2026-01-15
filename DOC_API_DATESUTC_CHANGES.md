# 📅 API Date Parameters - Cambios Requeridos para UTC

> **Versión:** 1.0  
> **Fecha:** 14 de Enero, 2026  
> **Estado:** Análisis de Endpoints

---

## 📋 Resumen Ejecutivo

Este documento detalla los endpoints que reciben parámetros de fecha y los cambios necesarios para garantizar consistencia con el manejo UTC del sistema.

### Clasificación de Endpoints

| Categoría | Estado | Cantidad |
|-----------|--------|----------|
| ✅ Ya compatible con UTC | Correcto | 6 |
| ⚠️ Requiere documentación | Bajo riesgo | 4 |
| 🔴 Requiere cambios en código | Alto riesgo | 4 |

---

## ✅ Endpoints YA Compatibles con UTC

Estos endpoints ya reciben y procesan fechas correctamente en UTC.

### 1. `GET /api/v4/sync/user-invoice-headers`
- **Archivo:** `src/api/user_invoice_headers_v4.rs`
- **Parámetro:** `update_date_from: Option<String>`
- **Formato esperado:** ISO 8601 UTC (`2025-01-14T00:00:00Z`)
- **Estado:** ✅ Correcto - usa `parse_date_to_utc()`

### 2. `GET /api/v4/sync/user-invoice-details`
- **Archivo:** `src/api/user_invoice_details_v4.rs`
- **Parámetro:** `update_date_from: Option<String>`
- **Formato esperado:** ISO 8601 UTC (`2025-01-14T00:00:00Z`)
- **Estado:** ✅ Correcto - usa `parse_date_to_utc()`

### 3. `GET /api/v4/invoices/issuers`
- **Archivo:** `src/api/user_issuers_v4.rs`
- **Parámetro:** `update_date_from: Option<String>`
- **Formato esperado:** ISO 8601 UTC
- **Estado:** ✅ Correcto - parsea a `DateTime<Utc>`

### 4. `GET /api/v4/invoices/products`
- **Archivo:** `src/api/user_products_v4.rs`
- **Parámetro:** `update_date_from: Option<String>`
- **Formato esperado:** ISO 8601 UTC
- **Estado:** ✅ Correcto - parsea a `DateTime<Utc>`

### 5. `GET /api/v4/notifications`
- **Archivo:** `src/api/notifications_v4.rs`
- **Parámetro:** `since: Option<DateTime<Utc>>`
- **Formato esperado:** ISO 8601 UTC
- **Estado:** ✅ Correcto - ya usa `DateTime<Utc>` en el tipo

### 6. `POST /api/v4/invoices/process-from-url` y `process-from-cufe`
- **Archivo:** `src/api/url_processing_v4.rs`, `src/api/database_persistence.rs`
- **Parámetro de entrada:** Ninguno (no recibe fechas del cliente)
- **Estado:** ✅ Correcto - fechas DGI se convierten de Panamá a UTC

---

## 📘 Detalle: Endpoints de Sincronización de Facturas

> **IMPORTANTE PARA FRONTEND:** Esta sección documenta el formato exacto de fechas para sincronización incremental.

### Endpoints de Sincronización

| Endpoint | Parámetro | Descripción |
|----------|-----------|-------------|
| `GET /api/v4/sync/user-invoice-headers` | `update_date_from` | Sincronizar headers de facturas |
| `GET /api/v4/sync/user-invoice-details` | `update_date_from` | Sincronizar detalles de facturas |
| `GET /api/v4/invoices/issuers` | `update_date_from` | Sincronizar emisores |
| `GET /api/v4/invoices/products` | `update_date_from` | Sincronizar productos |

### Formato del Parámetro `update_date_from`

```
Formato: ISO 8601 con zona horaria UTC
Patrón:  YYYY-MM-DDTHH:MM:SSZ
         YYYY-MM-DDTHH:MM:SS.ffffffZ (con microsegundos)
```

### ✅ Ejemplos CORRECTOS

```bash
# Fecha y hora completa en UTC
?update_date_from=2025-01-14T00:00:00Z

# Con microsegundos
?update_date_from=2025-01-14T10:30:45.123456Z

# Medianoche UTC del 1 de enero
?update_date_from=2025-01-01T00:00:00Z
```

### ❌ Ejemplos INCORRECTOS

```bash
# Sin sufijo Z (ambiguo)
?update_date_from=2025-01-14T00:00:00

# Solo fecha sin hora (no soportado)
?update_date_from=2025-01-14

# Formato de fecha incorrecto
?update_date_from=14/01/2025

# Con offset en lugar de Z
?update_date_from=2025-01-14T00:00:00+00:00  # ⚠️ Funciona pero preferir Z
```

### Flujo de Sincronización Incremental

```
┌─────────────────────────────────────────────────────────────────┐
│  PRIMERA SINCRONIZACIÓN (sin parámetro)                         │
│                                                                 │
│  GET /api/v4/sync/user-invoice-headers?limit=100               │
│                                                                 │
│  Response:                                                      │
│  {                                                              │
│    "data": [...],                                               │
│    "sync_metadata": {                                           │
│      "max_update_date": "2025-01-14T15:30:00Z"  ← GUARDAR      │
│    }                                                            │
│  }                                                              │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  SINCRONIZACIONES SIGUIENTES (con parámetro)                    │
│                                                                 │
│  GET /api/v4/sync/user-invoice-headers                          │
│      ?update_date_from=2025-01-14T15:30:00Z                    │
│      &limit=100                                                 │
│                                                                 │
│  → Solo retorna registros modificados después de esa fecha     │
│  → Actualizar max_update_date con el nuevo valor               │
└─────────────────────────────────────────────────────────────────┘
```

### Almacenamiento en Frontend (Recomendado)

```dart
// Flutter/Dart - Guardar última sincronización
class SyncStorage {
  static const _keyHeaders = 'sync_invoice_headers_last';
  static const _keyDetails = 'sync_invoice_details_last';
  static const _keyIssuers = 'sync_issuers_last';
  static const _keyProducts = 'sync_products_last';
  
  // Guardar timestamp de última sincronización
  Future<void> saveLastSync(String key, String maxUpdateDate) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(key, maxUpdateDate);
  }
  
  // Obtener timestamp para siguiente request
  Future<String?> getLastSync(String key) async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(key);
  }
}

// Uso en sincronización
Future<void> syncInvoiceHeaders() async {
  final lastSync = await SyncStorage().getLastSync(_keyHeaders);
  
  final url = lastSync != null
    ? '/api/v4/sync/user-invoice-headers?update_date_from=$lastSync'
    : '/api/v4/sync/user-invoice-headers';
  
  final response = await api.get(url);
  
  // Guardar nuevo max_update_date para siguiente sync
  final newMaxDate = response['sync_metadata']['max_update_date'];
  if (newMaxDate != null) {
    await SyncStorage().saveLastSync(_keyHeaders, newMaxDate);
  }
}
```

### Validación en Backend

El backend valida el formato usando `parse_date_to_utc()`:

```rust
// src/api/common/sync_helpers.rs
pub fn parse_date_to_utc(date_str: &str) -> Result<DateTime<Utc>, String> {
    // RFC3339 con Z
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.with_timezone(&Utc));
    }
    
    // Formatos alternativos...
    Err(format!("Could not parse date: {}", date_str))
}
```

### Respuesta de Error (Formato Inválido)

Si el formato es inválido, el endpoint retorna:

```json
{
  "success": false,
  "error": {
    "code": "INVALID_DATE_FORMAT",
    "message": "Invalid update_date_from format. Expected ISO 8601 UTC (e.g., 2025-01-14T00:00:00Z)"
  }
}
```

---

## ⚠️ Endpoints que Requieren Actualización de Documentación

Estos endpoints funcionan pero necesitan documentación clara sobre el formato esperado.

### 1. `GET /api/v4/rewards/history`
- **Archivo:** `src/api/rewards_history_v4.rs`
- **Parámetros:** 
  ```rust
  pub date_from: Option<String>,
  pub date_to: Option<String>,
  ```
- **Formato actual:** No documentado explícitamente
- **Problema:** No hay validación de formato, se pasa directamente a SQL
- **Riesgo:** Bajo (la BD filtra por TIMESTAMPTZ)

**Documentación requerida:**
```
date_from: ISO 8601 UTC (2025-01-14T00:00:00Z)
date_to: ISO 8601 UTC (2025-01-14T23:59:59Z)
```

### 2. `GET /api/v1/rewards/reports`
- **Archivo:** `src/api/rewards/reports.rs`
- **Parámetros:**
  ```rust
  pub start_date: Option<String>,  // "YYYY-MM-DD"
  pub end_date: Option<String>,    // "YYYY-MM-DD"
  ```
- **Formato actual:** Solo fecha (`YYYY-MM-DD`)
- **Problema:** Asume medianoche, no especifica zona horaria
- **Riesgo:** Bajo (interno para admin)

**Documentación requerida:**
```
start_date: YYYY-MM-DD (se asume inicio del día en UTC)
end_date: YYYY-MM-DD (se asume fin del día en UTC)
```

### 3. `POST /api/v1/rewards/export`
- **Archivo:** `src/api/rewards/reports.rs`
- **Parámetros:** Igual que reports
- **Estado:** Igual que reports

### 4. `GET /api/v1/admin/redemptions/reports`
- **Archivo:** `src/api/rewards/reports.rs`
- **Estado:** Igual que reports (solo admin)

---

## 🔴 Endpoints que Requieren Cambios en Código

Estos endpoints tienen problemas de diseño que deben corregirse.

### 1. `GET /api/v1/merchant/dashboard`

**Archivo:** `src/api/merchant/dashboard.rs`

**Parámetros actuales:**
```rust
#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub period: Option<String>,     // today, week, month, year, all
    pub start_date: Option<String>, // ISO 8601 (sin validación)
    pub end_date: Option<String>,   // ISO 8601 (sin validación)
}
```

**Problema identificado:**
```rust
fn calculate_date_range(period: &str, params: &DashboardQuery) -> Result<(String, String), ApiError> {
    // Si hay fechas personalizadas, usarlas SIN VALIDACIÓN
    if let (Some(start), Some(end)) = (&params.start_date, &params.end_date) {
        return Ok((start.clone(), end.clone())); // ⚠️ Sin parseo ni validación
    }
    
    let now = Utc::now();
    let end = now.format("%Y-%m-%d 23:59:59").to_string(); // ⚠️ Sin timezone suffix
    // ...
}
```

**Cambios requeridos:**
```rust
fn calculate_date_range(period: &str, params: &DashboardQuery) -> Result<(DateTime<Utc>, DateTime<Utc>), ApiError> {
    use crate::api::common::sync_helpers::parse_date_to_utc;
    
    // Validar y parsear fechas personalizadas
    if let (Some(start_str), Some(end_str)) = (&params.start_date, &params.end_date) {
        let start = parse_date_to_utc(start_str)
            .map_err(|_| ApiError::validation("Invalid start_date format. Use ISO 8601 UTC"))?;
        let end = parse_date_to_utc(end_str)
            .map_err(|_| ApiError::validation("Invalid end_date format. Use ISO 8601 UTC"))?;
        return Ok((start, end));
    }
    
    let now = Utc::now();
    // ... resto de la lógica con DateTime<Utc>
}
```

**Queries afectadas:**
- `get_overview()` línea 322-323
- `get_top_offers()` línea 507-508

---

### 2. `GET /api/v1/merchant/analytics`

**Archivo:** `src/api/merchant/analytics.rs`

**Parámetros actuales:**
```rust
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub range: Option<String>,      // "today", "week", "month", "custom"
    pub start_date: Option<String>, // ISO 8601 (sin validación)
    pub end_date: Option<String>,   // ISO 8601 (sin validación)
}
```

**Problema:** Mismo patrón que dashboard - fechas se usan sin validar

**Cambios requeridos:** Aplicar mismo patrón de validación con `parse_date_to_utc()`

---

### 3. `GET /api/v4/rewards/history` (Upgrade a validación)

**Archivo:** `src/api/rewards_history_v4.rs`

**Código actual (líneas 92-101):**
```rust
// Filtro por fecha desde
if params.date_from.is_some() {
    param_count += 1;
    where_conditions.push(format!("date >= ${}", param_count));
}

// Filtro por fecha hasta
if params.date_to.is_some() {
    param_count += 1;
    where_conditions.push(format!("date <= ${}", param_count));
}
```

**Problema:** Las fechas se pasan sin validar al bind de SQLx

**Cambio recomendado:**
```rust
// Validar y parsear fecha desde
let date_from_utc = if let Some(date_str) = &params.date_from {
    Some(parse_date_to_utc(date_str).map_err(|_| {
        error!("Invalid date_from format: {}", date_str);
        StatusCode::BAD_REQUEST
    })?)
} else {
    None
};

// Similar para date_to
```

---

### 4. `GET /api/v1/rewards/reports` y relacionados

**Archivo:** `src/api/rewards/reports.rs`

**Código actual (líneas 255-269):**
```rust
fn get_date_range(start: &Option<String>, end: &Option<String>) -> (String, String) {
    let now = Utc::now();
    
    let start_date = start
        .clone()
        .unwrap_or_else(|| (now - Duration::days(30)).format("%Y-%m-%d").to_string());
    
    let end_date = end
        .clone()
        .unwrap_or_else(|| now.format("%Y-%m-%d").to_string());
    
    (start_date, end_date)
}
```

**Problemas:**
1. No valida formato de entrada
2. Solo usa fecha sin hora
3. No especifica timezone en output

**Cambio recomendado:**
```rust
fn get_date_range_utc(
    start: &Option<String>, 
    end: &Option<String>
) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let now = Utc::now();
    
    let start_date = match start {
        Some(s) => parse_date_to_utc(s)?,
        None => now - Duration::days(30),
    };
    
    let end_date = match end {
        Some(e) => parse_date_to_utc(e)?,
        None => now,
    };
    
    Ok((start_date, end_date))
}
```

---

## 📊 Resumen de Cambios por Prioridad

### Alta Prioridad (Afectan a usuarios/merchants)

| Endpoint | Archivo | Cambio |
|----------|---------|--------|
| `GET /api/v1/merchant/dashboard` | `merchant/dashboard.rs` | Validar `start_date`, `end_date` |
| `GET /api/v1/merchant/analytics` | `merchant/analytics.rs` | Validar `start_date`, `end_date` |

### Media Prioridad (Mejoran robustez)

| Endpoint | Archivo | Cambio |
|----------|---------|--------|
| `GET /api/v4/rewards/history` | `rewards_history_v4.rs` | Validar `date_from`, `date_to` |

### Baja Prioridad (Solo admin/interno)

| Endpoint | Archivo | Cambio |
|----------|---------|--------|
| `GET /api/v1/rewards/reports` | `rewards/reports.rs` | Validar fechas y usar UTC |
| `POST /api/v1/rewards/export` | `rewards/reports.rs` | Igual que reports |

---

## 🔧 Función Utilitaria Recomendada

Para todos los endpoints, usar la función existente en `sync_helpers.rs`:

```rust
// src/api/common/sync_helpers.rs
pub fn parse_date_to_utc(date_str: &str) -> Result<chrono::DateTime<chrono::Utc>, String>
```

**Formatos soportados:**
- RFC3339 con timezone: `2025-01-14T10:00:00Z`
- DateTime sin timezone: `2025-01-14T10:00:00` (asume UTC)
- Solo fecha: `2025-01-14` (asume 00:00:00 UTC)

---

## 📝 Checklist de Implementación

### Merchant Dashboard (`merchant/dashboard.rs`)
- [ ] Cambiar tipo de retorno de `calculate_date_range` a `(DateTime<Utc>, DateTime<Utc>)`
- [ ] Agregar validación con `parse_date_to_utc()`
- [ ] Actualizar `get_overview()` para usar `DateTime<Utc>`
- [ ] Actualizar `get_top_offers()` para usar `DateTime<Utc>`
- [ ] Actualizar documentación de API

### Merchant Analytics (`merchant/analytics.rs`)
- [ ] Aplicar mismo patrón que dashboard
- [ ] Validar fechas de entrada
- [ ] Actualizar documentación

### Rewards History (`rewards_history_v4.rs`)
- [ ] Agregar validación de `date_from` y `date_to`
- [ ] Retornar error 400 si formato inválido
- [ ] Actualizar documentación

### Reports (`rewards/reports.rs`)
- [ ] Refactorizar `get_date_range()` para retornar `DateTime<Utc>`
- [ ] Agregar manejo de errores
- [ ] Actualizar endpoint de export

---

## 🔗 Referencias

- [docs/UTC_TIMEZONE_HANDLING.md](docs/UTC_TIMEZONE_HANDLING.md) - Documento principal de UTC
- [API_ENDPOINTS.md](API_ENDPOINTS.md) - Documentación general de API
- [src/api/common/sync_helpers.rs](src/api/common/sync_helpers.rs) - Funciones utilitarias de fecha
