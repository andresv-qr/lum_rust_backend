# ⏰ Manejo de Timestamps y Zonas Horarias (UTC)

> **Versión:** 1.0  
> **Fecha:** 14 de Enero, 2026  
> **Aplica a:** Toda la API v4 de Lüm

---

## 📋 Resumen

Toda la plataforma Lüm opera en **UTC (Coordinated Universal Time)**. Este documento describe cómo se manejan los timestamps en cada capa del sistema.

---

## 1. Base de Datos (PostgreSQL)

### Tipo de columna
Todas las columnas de timestamp usan `TIMESTAMP WITH TIME ZONE` (TIMESTAMPTZ):

```sql
-- Ejemplo de definición
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
update_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
process_date TIMESTAMPTZ NOT NULL,
```

### Ventajas de TIMESTAMPTZ
- PostgreSQL almacena internamente en UTC
- Conversiones automáticas al leer/escribir
- Soporte nativo para operaciones de zona horaria

---

## 2. Backend (Rust)

### Tipo de dato
Se usa `chrono::DateTime<chrono::Utc>` para todos los timestamps:

```rust
use chrono::{DateTime, Utc};

pub struct InvoiceHeader {
    pub date: Option<DateTime<Utc>>,          // Fecha de emisión (convertida de Panamá)
    pub process_date: DateTime<Utc>,          // Timestamp del servidor
    pub reception_date: DateTime<Utc>,        // Timestamp del servidor
}
```

### Generación de timestamps del servidor
```rust
let now_utc = chrono::Utc::now();
```

### Conversión de fechas de Panamá (DGI/MEF)
Las facturas de DGI vienen en hora local de Panamá. Se convierten a UTC:

```rust
use chrono_tz::America::Panama;

fn convert_panama_date_to_utc(date_str: &str) -> Option<DateTime<Utc>> {
    // "25/06/2025 14:30:00" (Panamá UTC-5)
    //          ↓
    // "2025-06-25T19:30:00Z" (UTC)
    
    let naive_dt = NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S")?;
    let panama_dt = Panama.from_local_datetime(&naive_dt).single()?;
    Some(panama_dt.with_timezone(&Utc))
}
```

---

## 3. API Responses

### Formato de salida
Todos los timestamps en respuestas JSON usan formato **ISO 8601 / RFC3339**:

```json
{
  "timestamp": "2025-01-14T15:30:00Z",
  "created_at": "2025-01-14T10:00:00Z",
  "expires_at": "2025-01-21T23:59:59Z"
}
```

### Características del formato
- Sufijo `Z` indica UTC
- Sin offset numérico (no `+00:00`)
- Precisión hasta segundos (sin milisegundos por defecto)

---

## 4. Campos de Fecha por Entidad

### invoice_header

| Campo | Origen | Descripción |
|-------|--------|-------------|
| `date` | DGI/MEF | Fecha de emisión de la factura. **Convertida de hora Panamá a UTC**. |
| `process_date` | Servidor | Momento UTC cuando terminó el procesamiento |
| `reception_date` | Servidor | Momento UTC cuando se recibió la petición |
| `update_date` | Trigger | Última modificación del registro (auto-actualizado) |

### dim_users

| Campo | Origen | Descripción |
|-------|--------|-------------|
| `created_at` | Servidor | Momento UTC de creación de cuenta |
| `last_login_at` | Servidor | Último inicio de sesión (UTC) |
| `email_verified_at` | Servidor | Momento de verificación de email (UTC) |

### notifications

| Campo | Origen | Descripción |
|-------|--------|-------------|
| `created_at` | Servidor | Momento UTC de creación |
| `read_at` | Servidor | Momento UTC cuando se leyó |
| `expires_at` | Configurado | Fecha de expiración (UTC) |

### redemptions

| Campo | Origen | Descripción |
|-------|--------|-------------|
| `created_at` | Servidor | Momento UTC de creación del código |
| `expires_at` | Calculado | 60 segundos después de created_at |
| `validated_at` | Servidor | Momento UTC de validación por comercio |
| `used_at` | Servidor | Momento UTC de confirmación de uso |

---

## 5. Conversión a Hora Local (Frontend)

### Panamá (UTC-5)
Panamá **no tiene horario de verano**, por lo que siempre es UTC-5:

```dart
// Flutter/Dart
DateTime utcTime = DateTime.parse(apiResponse['timestamp']);
DateTime panamaTime = utcTime.subtract(Duration(hours: 5));

// Formato para mostrar
String formatted = DateFormat('dd/MM/yyyy HH:mm').format(panamaTime);
// "14/01/2025 10:30" (si UTC era 15:30)
```

### Ejemplo de conversión

| UTC (API) | Panamá (Display) |
|-----------|------------------|
| `2025-01-14T15:30:00Z` | `14/01/2025 10:30 AM` |
| `2025-01-14T05:00:00Z` | `14/01/2025 12:00 AM` |
| `2025-01-14T00:00:00Z` | `13/01/2025 07:00 PM` |

---

## 6. Filtros de Fecha en Queries

### Parámetro `since`
Los endpoints de sincronización aceptan `since` en formato UTC:

```
GET /api/v4/sync/user-invoice-headers?since=2025-01-14T00:00:00Z
```

### Parámetro `update_date_from`
Similar para otros endpoints:

```
GET /api/v4/invoices/issuers?update_date_from=2025-01-01T00:00:00Z
```

### Recomendación
Siempre enviar fechas con el sufijo `Z` o con offset explícito:
- ✅ `2025-01-14T00:00:00Z`
- ✅ `2025-01-14T00:00:00+00:00`
- ❌ `2025-01-14T00:00:00` (ambiguo)

---

## 7. Triggers de Auto-Actualización

### update_date automático
Todas las tablas con campo `update_date` tienen un trigger que lo actualiza automáticamente:

```sql
CREATE OR REPLACE FUNCTION auto_update_update_date()
RETURNS TRIGGER AS $$
BEGIN
    NEW.update_date = NOW();  -- NOW() en TIMESTAMPTZ = UTC
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

Tablas con este trigger:
- `invoice_header`
- `invoice_detail`
- `dim_issuer`
- `dim_product`
- `dim_accumulations`
- `redemption_offers`
- `dim_issuer_stores`

---

## 8. Debugging

### Ver configuración de timezone en PostgreSQL
```sql
SHOW timezone;  -- Debe ser 'UTC'
SELECT NOW();   -- Timestamp actual en UTC
```

### Ver timestamp de un registro
```sql
SELECT 
    cufe,
    date,
    date AT TIME ZONE 'America/Panama' AS date_panama,
    process_date
FROM invoice_header 
WHERE cufe = 'FE...'
LIMIT 1;
```

---

## 9. Checklist de Implementación

### Backend (Rust)
- [x] Usar `DateTime<Utc>` en lugar de `NaiveDateTime`
- [x] Convertir fechas de Panamá con `chrono_tz::America::Panama`
- [x] Generar timestamps con `chrono::Utc::now()`
- [x] Serializar en formato RFC3339

### Base de Datos
- [x] Todas las columnas de fecha como `TIMESTAMPTZ`
- [x] Triggers para auto-update de `update_date`
- [x] Configuración `timezone = 'UTC'`

### Frontend
- [ ] Parsear timestamps como UTC
- [ ] Convertir a hora local para display (UTC-5 para Panamá)
- [ ] Enviar timestamps con sufijo `Z`

---

## 10. Referencias

- [RFC 3339 - Date and Time on the Internet](https://www.ietf.org/rfc/rfc3339.txt)
- [PostgreSQL TIMESTAMPTZ](https://www.postgresql.org/docs/current/datatype-datetime.html)
- [chrono-tz crate](https://docs.rs/chrono-tz/latest/chrono_tz/)
