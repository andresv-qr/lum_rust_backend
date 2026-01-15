# 📄 API Doc: `POST /api/v4/invoices/process-from-cufe`

## Descripción general
Este endpoint procesa una factura electrónica de Panamá **a partir de un CUFE** (código de factura), consultando directamente el servicio de DGI/MEF que retorna un `FacturaHTML`. Luego:

1. Extrae header/detalles/pagos desde el HTML.
2. Persiste la factura en BD (mismas reglas de validación/duplicados que `process-from-url`).
3. (Si aplica) acredita Lümis por gamificación.

> **Compatibilidad de output (misma estructura/reglas que QR)**
>
> La **estructura de respuesta** es **idéntica** a la usada por el flujo de procesamiento por QR/URL (en este código: `ProcessUrlResponse` envuelto por `ApiResponse`).

---

## ⏰ Manejo de Timestamps y Zonas Horarias (UTC)

> **IMPORTANTE:** Todos los timestamps se almacenan y retornan en **UTC (Coordinated Universal Time)**.

### Campos de fecha en `invoice_header`:

| Campo | Origen | Descripción |
|-------|--------|-------------|
| `date` | **DGI/MEF** | Fecha de **emisión de la factura** según DGI. Se extrae del HTML en formato Panamá (`DD/MM/YYYY HH:MM:SS`) y se **convierte a UTC** antes de almacenar. |
| `process_date` | **Servidor UTC** | Timestamp del servidor cuando **terminó** el procesamiento. |
| `reception_date` | **Servidor UTC** | Timestamp del servidor cuando **recibió** la petición. |

### Conversión de zona horaria:

```
Fecha DGI (Panamá UTC-5):  "25/06/2025 14:30:00"
                                    ↓
Conversión a UTC:          +5 horas
                                    ↓
Almacenado en BD (UTC):    "2025-06-25T19:30:00Z"
```

### Formato de respuesta:
- Todos los timestamps en respuestas usan formato **RFC3339/ISO8601** con sufijo `Z` (UTC).
- Ejemplo: `"2025-01-14T15:30:00Z"`

---

## Endpoint
- **Method:** `POST`
- **Path:** `/api/v4/invoices/process-from-cufe`
- **Auth:** **Requerida** (JWT Bearer)
- **Content-Type:** `application/json`

### Headers
| Header | Requerido | Descripción |
|---|---:|---|
| `Authorization` | ✅ | `Bearer <JWT_TOKEN>` |
| `Content-Type` | ✅ | `application/json` |
| `x-request-id` | ❌ | Si se envía, se reutiliza en `request_id` de la respuesta. |

---

## Request body
### Modelo (`CufeRequest`)

```json
{
  "cufe": "FE...",
  "origin": "app",
  "user_email": "user@example.com",
  "user_phone_number": "+50760000000",
  "user_telegram_id": "12345678",
  "user_ws": "whatsapp_chat_id"
}
```

| Campo | Tipo | Requerido | Descripción |
|---|---|---:|---|
| `cufe` | string | ✅ | CUFE. Se valida: `FE` al inicio, 60–75 chars, alfanumérico y `-`. |
| `origin` | string | ❌ | Origen lógico (p.ej. `app`, `whatsapp`, `telegram`, `API`). |
| `user_email` | string | ❌ | Copiado al `invoice_header.user_email` al persistir. |
| `user_phone_number` | string | ❌ | Copiado al `invoice_header.user_phone_number`. |
| `user_telegram_id` | string | ❌ | Copiado al `invoice_header.user_telegram_id`. |
| `user_ws` | string | ❌ | Copiado al `invoice_header.user_ws` / referencia de chat. |

---

## Flujo funcional (alto nivel)
1. **Autenticación JWT** (middleware): extrae `CurrentUser.user_id` desde el claim `sub`.
2. **Validación de CUFE** (formato): si falla, retorna `400` con `ApiResponse` de error (`VALIDATION_ERROR`).
3. **Lectura de credenciales DGI en runtime** (desde `AppState`):
   - `DGI_CAPTCHA_TOKEN` (token dinámico)
   - `DGI_SESSION_ID` (cookie `ASP.NET_SessionId`)
4. **Llamada a DGI** (POST form-urlencoded) a `ConsultarFacturasPorCUFE`.
5. **Parseo de respuesta JSON** (`FacturaHTML`, `Error`, `Mensaje`).
6. **Extracción HTML (CUFE-specific)**:
   - Emisor/Receptor desde pares `dt`/`dd`.
   - Totales desde `tfoot`.
   - Número/fecha desde `div.panel-heading h5`.
   - Detalles/pagos desde tablas HTML.
7. **Persistencia** con las mismas reglas del flujo QR/URL:
   - Rechaza facturas incompletas (monto/emisor/no/fecha requeridos).
   - Rechaza duplicados por `cufe`.
   - Si hay fallo de persistencia (no-duplicado), registra en `mef_pending`.
8. **Gamificación**: intenta acreditar Lümis (no bloquea si falla).

---

## Response

### Envelope estándar (`ApiResponse<T>`)
**Tipo:** `ApiResponse<ProcessUrlResponse>`

```json
{
  "success": true,
  "data": { "...": "..." },
  "error": null,
  "request_id": "uuid",
  "timestamp": "2025-12-26T00:00:00Z",
  "execution_time_ms": 1234,
  "cached": false
}
```

| Campo | Tipo | Descripción |
|---|---|---|
| `success` | boolean | Resultado del request a nivel API. En errores “de negocio” suele ser `false` pero con HTTP 200. |
| `data` | object\|null | Payload del endpoint (ver `ProcessUrlResponse`). |
| `error` | object\|null | Se usa cuando el handler retorna `ApiError` (p.ej. validación global). |
| `request_id` | string | UUID (o el valor de `x-request-id`). |
| `timestamp` | string (RFC3339) | Fecha de servidor. |
| `execution_time_ms` | number\|null | Tiempo total medido en el handler (ms). |
| `cached` | boolean | Siempre `false` en este endpoint. |

### Payload (`ProcessUrlResponse`)

```json
{
  "success": true,
  "message": "...",
  "process_type": "API",
  "invoice_id": null,
  "cufe": "FE...",
  "processing_time_ms": 0,
  "issuer_name": "...",
  "tot_amount": 12.34,
  "lumis_earned": 5,
  "lumis_balance": 120
}
```

| Campo | Tipo | Descripción |
|---|---|---|
| `success` | boolean | Resultado de la operación de negocio (extracción + persistencia). |
| `message` | string | Mensaje user-facing (éxito/pendiente/error/duplicado). |
| `process_type` | string\|null | **Actualmente** el persist devuelve `"API"` en el success path. En error suele ser `null`. |
| `invoice_id` | number\|null | Actualmente `null` (la persistencia retorna CUFE, no ID). |
| `cufe` | string\|null | CUFE persistido (en error suele ser `null`). |
| `processing_time_ms` | number\|null | **Actualmente** suele ser `0`/`null` (el tiempo real va en `execution_time_ms`). |
| `issuer_name` | string\|null | Nombre del emisor si fue extraído. |
| `tot_amount` | number\|null | Monto total (float). |
| `lumis_earned` | number\|null | Lümis acreditados en este procesamiento. |
| `lumis_balance` | number\|null | Balance resultante de Lümis. |

---

## Reglas de HTTP status y shape de errores

### 1) Errores de autenticación (401) — **NO usa `ApiResponse`**
Si falta/está mal el header `Authorization`, el middleware responde con `ErrorResponse`:

```json
{
  "error": "Missing Authorization header",
  "message": "Authentication required. Please provide a valid Bearer token.",
  "details": null
}
```

### 2) Errores de validación del CUFE (400) — usa `ApiResponse` + `error`
Ejemplo: CUFE no inicia con `FE` / longitud inválida.

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "CUFE debe comenzar con 'FE'",
    "details": null
  },
  "request_id": "uuid",
  "timestamp": "...",
  "execution_time_ms": null,
  "cached": false
}
```

### 3) Errores operativos DGI / factura no disponible / parsing — HTTP 200 con `success=false` y `data`
En estos casos, el handler **no lanza `ApiError`**; responde con envelope `ApiResponse` y `data` en formato `ProcessUrlResponse`:

```json
{
  "success": false,
  "data": {
    "success": false,
    "message": "Tu factura ha sido recibida. Aún no está disponible para ser procesada. Te notificaremos cuando esté lista.",
    "process_type": null,
    "invoice_id": null,
    "cufe": null,
    "processing_time_ms": null,
    "issuer_name": null,
    "tot_amount": null,
    "lumis_earned": null,
    "lumis_balance": null
  },
  "error": null,
  "request_id": "uuid",
  "timestamp": "...",
  "execution_time_ms": 1234,
  "cached": false
}
```

### 4) Duplicados — HTTP 200 con `success=false` y `data.message="Factura duplicada detectada"`
La duplicidad se detecta por CUFE en BD.

---

## Ejemplos de uso (cURL)

### Request mínimo
```bash
curl -X POST "https://<host>/api/v4/invoices/process-from-cufe" \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{
    "cufe": "FE0120000047028-19-305805-1800002025091902233506140010110199616775"
  }'
```

### Request con metadatos de usuario
```bash
curl -X POST "https://<host>/api/v4/invoices/process-from-cufe" \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -H "Content-Type: application/json" \
  -H "x-request-id: 550e8400-e29b-41d4-a716-446655440000" \
  -d '{
    "cufe": "FE...",
    "origin": "app",
    "user_email": "user@example.com",
    "user_phone_number": "+50760000000",
    "user_ws": "<whatsapp_chat_id>"
  }'
```

---

## Dependencias/configuración requerida

Este endpoint requiere que el servidor tenga configurado (y/o actualizado en runtime):
- `DGI_CAPTCHA_TOKEN`
- `DGI_SESSION_ID`

Estos valores se cargan en `AppState` al iniciar y pueden actualizarse en runtime (vía endpoints admin) sin reiniciar.

---

## Referencias de implementación
- Handler principal: [process_cufe_handler](src/api/url_processing_v4.rs#L719)
- Router protegido: [rutas v4 invoices](src/api/mod.rs#L149)
- Envelope de respuesta: [ApiResponse](src/api/common/mod.rs#L29)
- Payload: [ProcessUrlResponse](src/api/templates/url_processing_templates.rs#L13)
- Persistencia/duplicados/validaciones: [persist_scraped_data](src/api/database_persistence.rs#L34)
