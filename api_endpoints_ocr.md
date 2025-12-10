# API OCR - Documentación Completa# API OCR - Documentación del Endpoint



## Información General## Información General



- **Versión API:** v4- **Versión API:** v4

- **Última actualización:** Diciembre 2025- **Última actualización:** Diciembre 2025

- **Estado:** Producción- **Estado:** Producción

- **Motor OCR Primary:** Google Gemini 2.0 Flash- **Motor OCR Primary:** Google Gemini 2.0 Flash

- **Motor OCR Fallback:** OpenRouter (Qwen3-VL-30B)- **Motor OCR Fallback:** OpenRouter (Qwen3-VL-30B)

- **Compatibilidad:** Retrocompatible con versiones anteriores

---

## Endpoint: Upload OCR Invoice

## Índice

**URL:** `POST /api/v4/invoices/upload-ocr`

1. [Endpoint Upload OCR](#endpoint-1-upload-ocr-invoice)

2. [Endpoint Upload OCR Retry](#endpoint-2-upload-ocr-retry)**Descripción:** Procesa una imagen de factura mediante OCR (Reconocimiento Óptico de Caracteres) para extraer información estructurada de la factura y almacenarla en la base de datos.

3. [Campos Obligatorios](#campos-obligatorios-y-validación)

4. [Flujo Completo de Uso](#flujo-completo-de-uso)## Autenticación

5. [Ejemplos de Integración](#ejemplos-de-integración)

6. [Notas Técnicas](#notas-técnicas)- **Tipo:** Bearer Token (JWT)

- **Header requerido:** `Authorization: Bearer <token>`

---- **Middleware:** `extract_current_user` - El usuario debe estar autenticado

- **Scope:** El endpoint extrae automáticamente el `user_id` del token JWT

# Endpoint 1: Upload OCR Invoice- **Validación:** Token debe estar activo y no expirado



**URL:** `POST /api/v4/invoices/upload-ocr`## Rate Limiting



**Descripción:** Procesa una imagen de factura mediante OCR para extraer información estructurada. Valida que todos los campos obligatorios estén presentes.- **Límite por usuario:** Según configuración del usuario

- **Ventana de tiempo:** Configurable por administrador

## Autenticación- **Respuesta al exceder límite:** HTTP 429 Too Many Requests

- **Headers de respuesta:**

| Parámetro | Valor |  - `X-RateLimit-Limit`: Límite máximo

|-----------|-------|  - `X-RateLimit-Remaining`: Requests restantes

| **Tipo** | Bearer Token (JWT) |  - `X-RateLimit-Reset`: Timestamp de reset

| **Header** | `Authorization: Bearer <token>` |- **Costo por request:** 15 Lümis (deducidos solo si el procesamiento es exitoso)

| **Costo exitoso** | 15 Lümis |

| **Costo si falla** | 0 Lümis (si error antes de procesar) |## Formato de Request



## Request### Content-Type

```

### Content-TypeContent-Type: multipart/form-data

``````

Content-Type: multipart/form-data

```### Parámetros



### Parámetros| Campo | Tipo | Requerido | Descripción |

|-------|------|-----------|-------------|

| Campo | Tipo | Requerido | Descripción || `image` o `file` | File | Sí | Imagen de la factura a procesar |

|-------|------|-----------|-------------|| `mode` | String/Integer | No | Modo de procesamiento: `1` = Normal, `2` = Imagen combinada (eliminar duplicados) |

| `image` o `file` | File | ✅ Sí | Imagen de la factura (JPEG, PNG, PDF) |

| `mode` | String/Integer | ❌ No | `1` = Normal (default), `2` = Imagen combinada |### Restricciones del archivo

- **Formatos soportados:** JPEG, PNG, PDF

### Restricciones del Archivo- **Tamaño máximo:** 10MB (10,485,760 bytes)

- **Formatos:** JPEG, PNG, PDF- **Resolución recomendada:** Mínimo 300 DPI para mejor OCR

- **Tamaño máximo:** 10MB- **Validación:** Magic bytes para verificar formato real del archivo

- **Resolución recomendada:** Mínimo 300 DPI- **Codificación:** Multipart form-data con boundary

- **Compresión:** Automática por el cliente HTTP

---- **Orientación:** Cualquier orientación (se auto-detecta)



## Ejemplos de Request### Calidad de imagen recomendada

- **Nitidez:** Texto claramente legible

### cURL- **Contraste:** Alto contraste entre texto y fondo

```bash- **Iluminación:** Uniforme, sin sombras sobre el texto

curl -X POST "https://api.lumis.com/api/v4/invoices/upload-ocr" \- **Distorsión:** Mínima perspectiva o curvatura

  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \- **Recorte:** Incluir toda la factura, evitar cortes de texto

  -F "image=@factura.jpg" \

  -F "mode=1"### Ejemplo de Request (cURL)

``````bash

# Procesamiento normal

### JavaScript/Fetchcurl -X POST "https://api.lumis.com/api/v4/invoices/upload-ocr" \

```javascript  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \

const formData = new FormData();  -F "image=@factura.jpg" \

formData.append('image', fileInput.files[0]);  -F "mode=1"

formData.append('mode', '1');

# Procesamiento de imagen combinada (eliminar duplicados)

const response = await fetch('/api/v4/invoices/upload-ocr', {curl -X POST "https://api.lumis.com/api/v4/invoices/upload-ocr" \

  method: 'POST',  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \

  headers: {  -F "image=@factura_combinada.jpg" \

    'Authorization': `Bearer ${token}`  -F "mode=2"

  },```

  body: formData

});### Ejemplo de Request (JavaScript/Fetch)

```javascript

const result = await response.json();const formData = new FormData();

```formData.append('image', fileInput.files[0]);

// mode: 1 = Normal, 2 = Imagen combinada

### Flutter/DartformData.append('mode', '1'); 

```dart

final formData = FormData.fromMap({const response = await fetch('/api/v4/invoices/upload-ocr', {

  'image': await MultipartFile.fromFile(imageFile.path),  method: 'POST',

  'mode': '1',  headers: {

});    'Authorization': `Bearer ${token}`

  },

final response = await dio.post(  body: formData

  '/api/v4/invoices/upload-ocr',});

  data: formData,

  options: Options(headers: {'Authorization': 'Bearer $token'}),const result = await response.json();

);```

```

## Formatos de Respuesta

---

### Respuesta Exitosa (200 OK)

## Ejemplos de Response

```json

### ✅ Éxito Completo (200 OK){

  "success": true,

Todos los campos obligatorios fueron detectados:  "data": {

    "success": true,

```json    "cufe": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",

{    "invoice_number": "FACT-2024-001234",

  "success": true,    "issuer_name": "Empresa Ejemplo S.A.S.",

  "data": {    "total": 125750.50,

    "success": true,    "products_count": 3,

    "cufe": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",    "cost_lumis": 15,

    "invoice_number": "001-002-123456",    "status": "pending_validation",

    "issuer_name": "Super Xtra Centro",    "message": "Factura procesada exitosamente"

    "issuer_ruc": "155751938-2-2024",  },

    "issuer_dv": "66",  "error": null,

    "date": "2024-12-15",  "request_id": "550e8400-e29b-41d4-a716-446655440000",

    "total": 125.50,  "timestamp": "2024-03-15T10:30:00Z",

    "products": [  "execution_time_ms": null,

      {  "cached": false

        "name": "Coca Cola 500ml",}

        "quantity": 2,```

        "unit_price": 1.50,

        "total_price": 3.00### Respuesta de Error - Saldo Insuficiente (402 Payment Required)

      },

      {```json

        "name": "Pan Integral",{

        "quantity": 1,  "success": false,

        "unit_price": 2.25,  "data": null,

        "total_price": 2.25  "error": {

      }    "code": "OCR_PROCESSING_FAILED",

    ],    "message": "OCR processing failed",

    "products_count": 2,    "details": {

    "cost_lumis": 15,      "success": false,

    "status": "pending_validation",      "cost_lumis": 15,

    "message": "Factura procesada exitosamente",      "message": "Saldo insuficiente de Lümis. Necesitas 15 Lümis.",

    "missing_fields": null,      "cufe": null,

    "extracted_data": {      "partial_data": {

      "ruc": "155751938-2-2024",        "invoice_number": null,

      "dv": "66",        "issuer_name": null,

      "invoice_number": "001-002-123456",        "total": null,

      "total": 125.50,        "products_count": null

      "products": [      }

        {"name": "Coca Cola 500ml", "quantity": 2, "unit_price": 1.50, "total_price": 3.00},    }

        {"name": "Pan Integral", "quantity": 1, "unit_price": 2.25, "total_price": 2.25}  },

      ],  "request_id": "550e8400-e29b-41d4-a716-446655440001",

      "issuer_name": "Super Xtra Centro",  "timestamp": "2024-03-15T10:30:05Z",

      "issuer_address": "Plaza Central, Local 5",  "execution_time_ms": null,

      "date": "2024-12-15",  "cached": false

      "tot_itbms": 8.93}

    }```

  },

  "request_id": "550e8400-e29b-41d4-a716-446655440000"### Respuesta de Error - Límite de Rate Limiting (429 Too Many Requests)

}

``````json

{

---  "success": false,

  "data": null,

### ❌ Campos Faltantes (422 Unprocessable Entity)  "error": {

    "code": "OCR_PROCESSING_FAILED",

No todos los campos obligatorios fueron detectados. **Incluye `missing_fields` y `extracted_data`**:    "message": "OCR processing failed",

    "details": {

```json      "success": false,

{      "cost_lumis": 0,

  "success": false,      "message": "Has alcanzado el límite de procesamiento OCR. Intenta más tarde.",

  "error": {      "cufe": null,

    "code": "VALIDATION_FAILED",      "partial_data": {

    "message": "No se pudieron detectar todos los campos obligatorios. Campos faltantes: Dígito Verificador (DV), Detalle de Productos. Por favor, sube una nueva imagen donde estos campos sean claramente visibles, o usa el endpoint /api/v4/invoices/upload-ocr-retry para reintentar con una imagen adicional.",        "invoice_number": null,

    "details": {        "issuer_name": null,

      "success": false,        "total": null,

      "cost_lumis": 15,        "products_count": null

      "invoice_number": "001-002-123456",      }

      "issuer_name": "Super Xtra Centro",    }

      "issuer_ruc": "155751938-2-2024",  },

      "issuer_dv": null,  "request_id": "550e8400-e29b-41d4-a716-446655440002",

      "date": "2024-12-15",  "timestamp": "2024-03-15T10:30:10Z",

      "total": 125.50,  "execution_time_ms": null,

      "products": [],  "cached": false

      "products_count": 0,}

      "missing_fields": [```

        {

          "field_name": "Dígito Verificador (DV)",### Respuesta de Error - Archivo Inválido (400 Bad Request)

          "field_key": "dv",

          "description": "Dígito verificador que acompaña al RUC"```json

        },{

        {  "success": false,

          "field_name": "Detalle de Productos",  "data": null,

          "field_key": "products",  "error": {

          "description": "Al menos un producto con descripción y monto (ej: 'Coca Cola 500ml - $1.50')"    "code": "NO_IMAGE_FILE",

        }    "message": "No image file provided. Use 'image' or 'file' field name.",

      ],    "details": null

      "extracted_data": {  },

        "ruc": "155751938-2-2024",  "request_id": "550e8400-e29b-41d4-a716-446655440003",

        "dv": null,  "timestamp": "2024-03-15T10:30:15Z",

        "invoice_number": "001-002-123456",  "execution_time_ms": null,

        "total": 125.50,  "cached": false

        "products": [],}

        "issuer_name": "Super Xtra Centro",```

        "issuer_address": "Plaza Central, Local 5",

        "date": "2024-12-15",### Respuesta de Error - Formato No Soportado (415 Unsupported Media Type)

        "tot_itbms": null

      }```json

    }{

  },  "success": false,

  "request_id": "550e8400-e29b-41d4-a716-446655440001"  "data": null,

}  "error": {

```    "code": "INVALID_FORMAT",

    "message": "Invalid image format. Supported: JPEG, PNG, PDF",

> **⚠️ Importante:** El `extracted_data` contiene TODOS los campos que SÍ se detectaron. Este objeto debe guardarse para enviarlo al endpoint de retry.    "details": null

  },

---  "request_id": "550e8400-e29b-41d4-a716-446655440004",

  "timestamp": "2024-03-15T10:30:20Z",

### ❌ Saldo Insuficiente (402 Payment Required)  "execution_time_ms": null,

  "cached": false

```json}

{```

  "success": false,

  "error": {### Respuesta de Error - Archivo Muy Grande (413 Payload Too Large)

    "code": "OCR_PROCESSING_FAILED",

    "message": "OCR processing failed",```json

    "details": {{

      "success": false,  "success": false,

      "cost_lumis": 15,  "data": null,

      "message": "Saldo insuficiente de Lümis. Necesitas 15 Lümis.",  "error": {

      "extracted_data": null    "code": "FILE_TOO_LARGE",

    }    "message": "Image file too large (max 10MB)",

  },    "details": null

  "request_id": "550e8400-e29b-41d4-a716-446655440002"  },

}  "request_id": "550e8400-e29b-41d4-a716-446655440005",

```  "timestamp": "2024-03-15T10:30:25Z",

  "execution_time_ms": null,

---  "cached": false

}

### ❌ Otros Errores```



| Código | Error | Descripción |## Códigos de Estado HTTP

|--------|-------|-------------|

| 400 | NO_IMAGE_FILE | No se envió imagen || Código | Descripción |

| 413 | FILE_TOO_LARGE | Archivo > 10MB ||--------|-------------|

| 415 | INVALID_FORMAT | Formato no soportado || `200` | OCR procesado exitosamente |

| 429 | RATE_LIMITED | Límite de requests alcanzado || `400` | Request inválido (archivo faltante, datos vacíos) |

| `401` | Token JWT inválido o faltante |

---| `402` | Saldo insuficiente de Lümis |

| `413` | Archivo muy grande (>10MB) |

# Endpoint 2: Upload OCR Retry| `415` | Formato de archivo no soportado |

| `422` | Error en procesamiento OCR |

**URL:** `POST /api/v4/invoices/upload-ocr-retry`| `429` | Límite de rate limiting alcanzado |

| `500` | Error interno del servidor |

**Descripción:** Endpoint especializado para reintentar la extracción de campos específicos que no se detectaron en el primer OCR. **Combina automáticamente** los datos previos con los nuevos para determinar si la factura está completa.

---

## Autenticación y Costo

## Endpoint: Upload OCR Retry (Campos Faltantes)

| Parámetro | Valor |

|-----------|-------|**URL:** `POST /api/v4/invoices/upload-ocr-retry`

| **Tipo** | Bearer Token (JWT) |

| **Header** | `Authorization: Bearer <token>` |**Descripción:** Endpoint especializado para reintentar la extracción de campos específicos que no se detectaron en el primer procesamiento OCR. Combina los datos extraídos previamente con los nuevos para determinar si la factura está completa.

| **Costo** | **5 Lümis** (reducido vs 15 del OCR completo) |

### Autenticación

## Request- **Tipo:** Bearer Token (JWT)

- **Header requerido:** `Authorization: Bearer <token>`

### Content-Type

```### Costo

Content-Type: multipart/form-data- **5 Lümis** por intento (reducido comparado con el OCR completo de 15 Lümis)

```

### Content-Type

### Parámetros```

Content-Type: multipart/form-data

| Campo | Tipo | Requerido | Descripción |```

|-------|------|-----------|-------------|

| `image` o `file` | File | ✅ Sí | Nueva imagen enfocada en los campos faltantes |### Parámetros

| `missing_fields` | JSON Array | ✅ Sí | Array con los `field_key` de campos a buscar |

| `previous_data` | JSON Object | ⭐ Recomendado | `extracted_data` del primer OCR para merge || Campo | Tipo | Requerido | Descripción |

|-------|------|-----------|-------------|

### Campos Válidos para `missing_fields`| `image` o `file` | File | Sí | Nueva imagen de la factura enfocada en los campos faltantes |

| `missing_fields` | JSON Array | Sí | Array JSON con los field_keys de los campos a buscar |

| field_key | Descripción | Ejemplo || `previous_data` | JSON Object | **Recomendado** | Datos extraídos previamente (del `extracted_data` del primer OCR) |

|-----------|-------------|---------|

| `ruc` | RUC del comercio emisor | "155751938-2-2024" |### Campos Válidos para `missing_fields`

| `dv` | Dígito Verificador | "66" |

| `invoice_number` | Número de factura | "001-002-123456" || field_key | Descripción | Ejemplo en factura |

| `total` | Monto total | 125.50 ||-----------|-------------|-------------------|

| `products` | Detalle de productos | Lista con nombre + precio || `ruc` | RUC del comercio emisor | "155751938-2-2024" |

| `dv` | Dígito Verificador | "66", "89" |

### Estructura de `previous_data`| `invoice_number` | Número de factura | "001-002-123456", "FACT-2024-001" |

| `total` | Monto total de la factura | 125.50, 1250.00 |

```json| `products` | Detalle de productos (nombre + precio) | Lista de ítems con descripción y monto |

{

  "ruc": "155751938-2-2024",### Estructura de `previous_data`

  "dv": null,

  "invoice_number": "001-002-123456",```json

  "total": 125.50,{

  "products": [],  "ruc": "155751938-2-2024",

  "issuer_name": "Super Xtra Centro",  "dv": null,

  "issuer_address": "Plaza Central",  "invoice_number": "FACT-001",

  "date": "2024-12-15",  "total": 125.50,

  "tot_itbms": null  "products": [

}    {"name": "Producto 1", "quantity": 1, "unit_price": 10.00, "total_price": 10.00}

```  ],

  "issuer_name": "Super Xtra",

> **💡 Tip:** Este objeto es exactamente el `extracted_data` que viene en la respuesta del primer endpoint cuando hay campos faltantes.  "issuer_address": "Plaza Central",

  "date": "2024-12-15",

---  "tot_itbms": null

}

## Ejemplos de Request```



### cURL### Ejemplo de Request (cURL)

```bash```bash

curl -X POST "https://api.lumis.com/api/v4/invoices/upload-ocr-retry" \# Retry con datos previos (recomendado)

  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \curl -X POST "https://api.lumis.com/api/v4/invoices/upload-ocr-retry" \

  -F "image=@factura_detalle.jpg" \  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \

  -F 'missing_fields=["dv", "products"]' \  -F "image=@factura_ruc_closeup.jpg" \

  -F 'previous_data={"ruc":"155751938-2-2024","dv":null,"invoice_number":"001-002-123456","total":125.50,"products":[],"issuer_name":"Super Xtra Centro","date":"2024-12-15"}'  -F 'missing_fields=["dv", "products"]' \

```  -F 'previous_data={"ruc":"155751938-2-2024","invoice_number":"FACT-001","total":125.50,"products":[],"issuer_name":"Super Xtra"}'

```

### JavaScript/Fetch

```javascript### Ejemplo de Request (JavaScript/Fetch)

// El extracted_data viene de la respuesta del primer OCR```javascript

const previousData = firstOcrResponse.error.details.extracted_data;// El extracted_data viene de la respuesta del primer OCR

const missingFieldKeys = firstOcrResponse.error.details.missing_fields.map(f => f.field_key);const previousData = firstOcrResponse.extracted_data;

const missingFields = firstOcrResponse.missing_fields.map(f => f.field_key);

const formData = new FormData();

formData.append('image', newImageFile);const formData = new FormData();

formData.append('missing_fields', JSON.stringify(missingFieldKeys));formData.append('image', fileInput.files[0]);

formData.append('previous_data', JSON.stringify(previousData));formData.append('missing_fields', JSON.stringify(missingFields));

formData.append('previous_data', JSON.stringify(previousData));

const response = await fetch('/api/v4/invoices/upload-ocr-retry', {

  method: 'POST',const response = await fetch('/api/v4/invoices/upload-ocr-retry', {

  headers: {  method: 'POST',

    'Authorization': `Bearer ${token}`  headers: {

  },    'Authorization': `Bearer ${token}`

  body: formData  },

});  body: formData

});

const result = await response.json();

```const result = await response.json();

```

### Flutter/Dart

```dart### Respuesta Exitosa - Factura Completa (200 OK)

// Datos del primer OCR```json

final previousData = firstOcrResponse['error']['details']['extracted_data'];{

final missingFields = (firstOcrResponse['error']['details']['missing_fields'] as List)  "success": true,

    .map((f) => f['field_key'])  "data": {

    .toList();    "success": true,

    "retry_mode": true,

final formData = FormData.fromMap({    "searched_fields": ["dv", "products"],

  'image': await MultipartFile.fromFile(newImageFile.path),    "cufe": null,

  'missing_fields': jsonEncode(missingFields),    "invoice_number": "FACT-001",

  'previous_data': jsonEncode(previousData),    "issuer_name": "Super Xtra",

});    "issuer_ruc": "155751938-2-2024",

    "issuer_dv": "66",

final response = await dio.post(    "date": "2024-12-15",

  '/api/v4/invoices/upload-ocr-retry',    "total": 125.50,

  data: formData,    "products": [

  options: Options(headers: {'Authorization': 'Bearer $token'}),      {"name": "Coca Cola 500ml", "quantity": 2, "unit_price": 1.50, "total_price": 3.00}

);    ],

```    "products_count": 1,

    "cost_lumis": 5,

---    "message": "¡Factura completa! Todos los campos obligatorios fueron extraídos.",

    "missing_fields": null,

## Ejemplos de Response    "extracted_data": {

      "ruc": "155751938-2-2024",

### ✅ Factura Completa (200 OK)      "dv": "66",

      "invoice_number": "FACT-001",

Todos los campos ahora están presentes (combinando previos + nuevos):      "total": 125.50,

      "products": [...],

```json      "issuer_name": "Super Xtra"

{    }

  "success": true,  },

  "data": {  "request_id": "550e8400-e29b-41d4-a716-446655440006"

    "success": true,}

    "retry_mode": true,```

    "searched_fields": ["dv", "products"],

    "cufe": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",### Respuesta con Campos Aún Faltantes (422 Unprocessable Entity)

    "invoice_number": "001-002-123456",```json

    "issuer_name": "Super Xtra Centro",{

    "issuer_ruc": "155751938-2-2024",  "success": false,

    "issuer_dv": "66",  "error": {

    "date": "2024-12-15",    "code": "RETRY_EXTRACTION_INCOMPLETE",

    "total": 125.50,    "message": "Aún no se pudieron detectar todos los campos requeridos. Faltan: Dígito Verificador (DV)",

    "products": [    "details": {

      {      "success": false,

        "name": "Coca Cola 500ml",      "retry_mode": true,

        "quantity": 2,      "searched_fields": ["ruc", "dv"],

        "unit_price": 1.50,      "cost_lumis": 5,

        "total_price": 3.00      "issuer_ruc": "155751938-2-2024",

      },      "issuer_dv": null,

      {      "missing_fields": [

        "name": "Pan Integral",        {

        "quantity": 1,          "field_name": "Dígito Verificador (DV)",

        "unit_price": 2.25,          "field_key": "dv",

        "total_price": 2.25          "description": "Dígito verificador que acompaña al RUC"

      }        }

    ],      ]

    "products_count": 2,    }

    "cost_lumis": 5,  },

    "message": "¡Factura completa! Todos los campos obligatorios fueron extraídos.",  "request_id": "550e8400-e29b-41d4-a716-446655440007"

    "missing_fields": null,}

    "extracted_data": {```

      "ruc": "155751938-2-2024",

      "dv": "66",### Errores Comunes

      "invoice_number": "001-002-123456",

      "total": 125.50,| Código | Error | Causa |

      "products": [|--------|-------|-------|

        {"name": "Coca Cola 500ml", "quantity": 2, "unit_price": 1.50, "total_price": 3.00},| 400 | MISSING_FIELDS_REQUIRED | No se envió el parámetro `missing_fields` |

        {"name": "Pan Integral", "quantity": 1, "unit_price": 2.25, "total_price": 2.25}| 400 | EMPTY_MISSING_FIELDS | El array `missing_fields` está vacío |

      ],| 400 | INVALID_FIELD_KEY | Se envió un field_key no válido |

      "issuer_name": "Super Xtra Centro",| 400 | INVALID_MISSING_FIELDS_FORMAT | El parámetro no es un JSON array válido |

      "issuer_address": "Plaza Central",| 422 | RETRY_EXTRACTION_INCOMPLETE | No se pudieron extraer todos los campos solicitados |

      "date": "2024-12-15",

      "tot_itbms": 8.93---

    }

  },## Campos Obligatorios y Validación

  "request_id": "550e8400-e29b-41d4-a716-446655440003"

}El sistema requiere los siguientes campos para procesar una factura exitosamente:

```

### Campos Requeridos

---

| Campo | Descripción | Validación |

### ❌ Aún Faltan Campos (422 Unprocessable Entity)|-------|-------------|------------|

| **RUC** | Número de RUC del comercio | No vacío |

La nueva extracción no encontró todos los campos solicitados:| **DV** | Dígito Verificador | No vacío |

| **Número de Factura** | Identificador de la factura | No vacío |

```json| **Total** | Monto total | Mayor a 0.0 |

{| **Productos** | Al menos 1 producto | Descripción no vacía + precio > 0 |

  "success": false,

  "error": {### Flujo de Validación

    "code": "RETRY_EXTRACTION_INCOMPLETE",

    "message": "Aún no se pudieron detectar todos los campos requeridos. Faltan: Detalle de Productos",```

    "details": {┌────────────────────────────────────────────────────────────────────┐

      "success": false,│                      POST /upload-ocr                              │

      "retry_mode": true,│                    (Primera imagen)                                │

      "searched_fields": ["dv", "products"],└────────────────────────┬───────────────────────────────────────────┘

      "cost_lumis": 5,                         │

      "invoice_number": "001-002-123456",                         ▼

      "issuer_name": "Super Xtra Centro",              ┌──────────────────────┐

      "issuer_ruc": "155751938-2-2024",              │  ¿Todos los campos   │

      "issuer_dv": "66",              │   obligatorios OK?   │

      "date": "2024-12-15",              └──────────┬───────────┘

      "total": 125.50,                   │     │

      "products": [],           ┌──────┘     └──────┐

      "products_count": 0,           │ SÍ              NO │

      "missing_fields": [           ▼                    ▼

        {   ┌─────────────────┐   ┌─────────────────────────────────────┐

          "field_name": "Detalle de Productos",   │ ✅ success:true │   │ ❌ success:false                    │

          "field_key": "products",   │ Factura guardada│   │ "missing_fields": [                 │

          "description": "Al menos un producto con descripción y monto"   │                 │   │   {"field_key": "ruc", ...},        │

        }   │                 │   │   {"field_key": "products", ...}    │

      ],   │                 │   │ ]                                   │

      "extracted_data": {   └─────────────────┘   └──────────────────┬──────────────────┘

        "ruc": "155751938-2-2024",                                            │

        "dv": "66",                                            ▼

        "invoice_number": "001-002-123456",                         ┌──────────────────────────────────────────┐

        "total": 125.50,                         │    App muestra campos faltantes al       │

        "products": [],                         │    usuario y solicita nueva foto         │

        "issuer_name": "Super Xtra Centro",                         └──────────────────┬───────────────────────┘

        "date": "2024-12-15"                                            │

      }                                            ▼

    }                         ┌──────────────────────────────────────────┐

  },                         │      POST /upload-ocr-retry              │

  "request_id": "550e8400-e29b-41d4-a716-446655440004"                         │  missing_fields=["ruc", "products"]      │

}                         │     (Nueva imagen enfocada)              │

```                         └──────────────────┬───────────────────────┘

                                            │

> **💡 Nota:** El `extracted_data` ahora incluye el DV que sí se encontró en este retry. Usa este nuevo `extracted_data` para el siguiente retry si es necesario.                                            ▼

                              ┌──────────────────────┐

---                              │  ¿Campos solicitados │

                              │    extraídos OK?     │

### ❌ Errores de Validación                              └──────────┬───────────┘

                                   │     │

| Código | Error | Descripción |                           ┌──────┘     └──────┐

|--------|-------|-------------|                           │ SÍ              NO │

| 400 | MISSING_FIELDS_REQUIRED | No se envió `missing_fields` |                           ▼                    ▼

| 400 | EMPTY_MISSING_FIELDS | Array `missing_fields` vacío |                  ┌─────────────────┐   ┌─────────────────────────┐

| 400 | INVALID_FIELD_KEY | Se envió un field_key inválido |                  │ ✅ Campos listos│   │ ❌ Aún faltan campos    │

| 400 | INVALID_MISSING_FIELDS_FORMAT | `missing_fields` no es JSON válido |                  │ App combina con │   │ Solicitar otra imagen   │

| 400 | INVALID_PREVIOUS_DATA_FORMAT | `previous_data` no es JSON válido |                  │ datos previos   │   │ o entrada manual        │

                  └─────────────────┘   └─────────────────────────┘

---```



# Campos Obligatorios y Validación### Ejemplo de Respuesta con Campos Faltantes



El sistema requiere los siguientes campos para procesar una factura exitosamente:Cuando el OCR no detecta todos los campos obligatorios:



| Campo | Descripción | Validación |```json

|-------|-------------|------------|{

| **RUC** | Número de RUC del comercio | No vacío |  "success": false,

| **DV** | Dígito Verificador | No vacío |  "error": {

| **Número de Factura** | Identificador único | No vacío |    "code": "VALIDATION_FAILED",

| **Total** | Monto total de la factura | Mayor a 0.0 |    "message": "No se pudieron detectar todos los campos obligatorios. Campos faltantes: RUC del comercio, Detalle de Productos. Por favor, sube una nueva imagen donde estos campos sean claramente visibles, o usa el endpoint /api/v4/invoices/upload-ocr-retry para reintentar con una imagen adicional.",

| **Productos** | Lista de ítems comprados | Mínimo 1 con nombre + precio > 0 |    "details": {

      "success": false,

---      "cost_lumis": 15,

      "invoice_number": "FACT-2024-001234",

# Flujo Completo de Uso      "issuer_name": "Super Xtra",

      "issuer_ruc": null,

## Diagrama de Flujo      "issuer_dv": "66",

      "date": "2024-12-15",

```      "total": 125.50,

┌─────────────────────────────────────────────────────────────────────┐      "products": [],

│                   PASO 1: POST /upload-ocr                          │      "products_count": 0,

│                      (Primera imagen)                               │      "missing_fields": [

└─────────────────────────────┬───────────────────────────────────────┘        {

                              │          "field_name": "RUC del comercio",

                              ▼          "field_key": "ruc",

                   ┌────────────────────┐          "description": "Número de RUC del comercio emisor (ej: 155751938-2-2024)"

                   │  ¿Todos los campos │        },

                   │   obligatorios?    │        {

                   └─────────┬──────────┘          "field_name": "Detalle de Productos",

                        │    │          "field_key": "products",

              ┌─────────┘    └─────────┐          "description": "Al menos un producto con descripción y monto (ej: 'Coca Cola 500ml - $1.50')"

              │ SÍ                   NO │        }

              ▼                        ▼      ]

    ┌─────────────────┐     ┌─────────────────────────────────────┐    }

    │ ✅ success:true │     │ ❌ success:false                    │  }

    │ Factura guardada│     │ Respuesta incluye:                  │}

    │ ¡Proceso        │     │  • missing_fields (campos faltantes)│```

    │  completo!      │     │  • extracted_data (campos SÍ        │

    │                 │     │    encontrados) ← GUARDAR ESTO      │### Manejo en Flutter/Dart

    └─────────────────┘     └──────────────────┬──────────────────┘

                                               │```dart

                                               ▼// Procesar respuesta de upload-ocr

                           ┌──────────────────────────────────────────┐if (!response.success && response.error?.details?['missing_fields'] != null) {

                           │    App muestra al usuario:               │  final missingFields = response.error.details['missing_fields'] as List;

                           │    "Faltan: DV, Productos"               │  

                           │    "Toma una foto enfocada en estos      │  // Mostrar al usuario qué campos faltan

                           │     datos de la factura"                 │  final fieldNames = missingFields.map((f) => f['field_name']).join(', ');

                           └──────────────────┬───────────────────────┘  showDialog(

                                              │    context: context,

                                              ▼    builder: (_) => AlertDialog(

┌─────────────────────────────────────────────────────────────────────┐      title: Text('Campos faltantes'),

│                 PASO 2: POST /upload-ocr-retry                      │      content: Text('No se pudieron detectar: $fieldNames\n\nToma una nueva foto enfocada en estos datos.'),

│                                                                     │      actions: [

│   Parámetros:                                                       │        TextButton(

│   • image: Nueva foto enfocada en campos faltantes                  │          onPressed: () {

│   • missing_fields: ["dv", "products"]                              │            // Preparar retry con campos faltantes

│   • previous_data: { extracted_data del paso 1 }                    │            final fieldKeys = missingFields.map((f) => f['field_key']).toList();

└─────────────────────────────┬───────────────────────────────────────┘            Navigator.pop(context);

                              │            _captureRetryImage(fieldKeys);

                              ▼          },

            ┌─────────────────────────────────────┐          child: Text('Tomar nueva foto'),

            │   Backend hace MERGE inteligente:   │        ),

            │   • Extrae nuevos datos de imagen   │      ],

            │   • Combina con previous_data       │    ),

            │   • Valida completitud del merge    │  );

            └─────────────────┬───────────────────┘}

                              │

                              ▼// Enviar retry

                   ┌────────────────────┐Future<void> _submitRetry(File image, List<String> missingFields) async {

                   │  ¿Datos mergeados  │  final formData = FormData.fromMap({

                   │   están completos? │    'image': await MultipartFile.fromFile(image.path),

                   └─────────┬──────────┘    'missing_fields': jsonEncode(missingFields),

                        │    │  });

              ┌─────────┘    └─────────┐  

              │ SÍ                   NO │  final response = await dio.post(

              ▼                        ▼    '/api/v4/invoices/upload-ocr-retry',

    ┌─────────────────┐     ┌─────────────────────────────────────┐    data: formData,

    │ ✅ success:true │     │ ❌ Aún faltan campos                │    options: Options(headers: {'Authorization': 'Bearer $token'}),

    │ "¡Factura       │     │ Respuesta incluye nuevo             │  );

    │  completa!"     │     │ extracted_data con merge parcial    │  

    │ Costo: 5 Lümis  │     │                                     │  // Combinar datos del retry con la respuesta original

    └─────────────────┘     │ → Repetir PASO 2 con nueva imagen   │  if (response.data['success']) {

                            │   usando el nuevo extracted_data    │    _mergeOcrData(originalData, response.data['data']);

                            └─────────────────────────────────────┘  }

```}

```

## Lógica de Merge

---

Cuando envías `previous_data` al retry, el sistema combina los datos así:

### Respuesta Exitosa - Campo `data`

```

Para cada campo:| Campo | Tipo | Descripción |

  IF campo fue buscado en este retry AND se encontró valor:|-------|------|-------------|

    usar valor nuevo| `success` | boolean | Siempre `true` en respuesta exitosa |

  ELSE:| `cufe` | string | Código Único de Factura Electrónica generado |

    usar valor de previous_data (si existe)| `invoice_number` | string | Número de factura extraído del documento |

```| `issuer_name` | string | Nombre del emisor/empresa |

| `total` | number | Valor total de la factura |

**Ejemplo:**| `products_count` | integer | Cantidad de productos/líneas detectadas |

| `cost_lumis` | integer | Costo en Lümis del procesamiento |

| Campo | previous_data | Nuevo OCR (buscando dv, products) | Resultado Merge || `status` | string | Estado: `"pending_validation"` |

|-------|---------------|-----------------------------------|-----------------|| `message` | string | Mensaje descriptivo del resultado |

| ruc | "155751938-2-2024" | - (no buscado) | "155751938-2-2024" |

| dv | null | "66" (encontrado) | "66" ✅ |### Respuesta de Error - Campo `error.details`

| invoice_number | "001-002-123456" | - (no buscado) | "001-002-123456" |

| total | 125.50 | - (no buscado) | 125.50 || Campo | Tipo | Descripción |

| products | [] | [2 productos] (encontrado) | [2 productos] ✅ ||-------|------|-------------|

| `success` | boolean | Siempre `false` en error |

---| `cost_lumis` | integer | Lümis deducidos (0 si falló antes del procesamiento) |

| `message` | string | Descripción del error |

# Ejemplos de Integración| `cufe` | string\|null | CUFE si se generó antes del error |

| `partial_data` | object | Datos parciales extraídos antes del fallo |

## Flujo Completo en JavaScript

## Parámetro Mode - Tipos de Procesamiento

```javascript

class OcrService {El parámetro `mode` permite especificar el tipo de procesamiento de imagen:

  constructor(apiUrl, token) {

    this.apiUrl = apiUrl;### Modo 1 - Procesamiento Normal

    this.token = token;- **Valor**: `1` (por defecto si no se especifica)

    this.extractedData = null; // Guardar datos entre intentos- **Uso**: Facturas individuales estándar

  }- **Comportamiento**: Procesamiento OCR normal sin consideraciones especiales



  // Paso 1: Primer OCR### Modo 2 - Imagen Combinada

  async uploadInvoice(imageFile) {- **Valor**: `2`

    const formData = new FormData();- **Uso**: Imágenes que contienen múltiples capturas o están combinadas

    formData.append('image', imageFile);- **Comportamiento**: Se agrega instrucción especial a Gemini para:

  - Identificar y eliminar datos duplicados

    const response = await fetch(`${this.apiUrl}/api/v4/invoices/upload-ocr`, {  - Consolidar información repetida

      method: 'POST',  - Construir una única factura unificada

      headers: { 'Authorization': `Bearer ${this.token}` },- **Prompt adicional**: _"Ten en cuenta que esta imagen es una combinación de varias imágenes, por lo que puede contener datos duplicados. Por favor, elimina los duplicados y construye una única factura consolidada, sin información repetida."_

      body: formData

    });## Flujo de Procesamiento



    const result = await response.json();1. **Validación de autenticación**: Verificación del token JWT

2. **Extracción del archivo**: Procesamiento del multipart form

    if (result.success) {3. **Validación del archivo**: Formato, tamaño, magic bytes

      // ✅ Factura completa4. **Verificación de saldo**: Confirmar Lümis suficientes

      return { success: true, data: result.data };5. **Rate limiting**: Verificar límites de uso

    }6. **Procesamiento OCR**: Envío a Gemini API

7. **Extracción de datos**: Parsing de respuesta JSON

    // ❌ Faltan campos - guardar extracted_data para retry8. **Validación de negocio**: Verificación de datos obligatorios

    const details = result.error?.details;9. **Persistencia**: Guardado en base de datos

    if (details?.missing_fields && details?.extracted_data) {10. **Respuesta**: Retorno de resultado estructurado

      this.extractedData = details.extracted_data;

      return {## Costo y Límites

        success: false,

        missingFields: details.missing_fields,- **Costo por procesamiento**: 15 Lümis por factura procesada exitosamente

        extractedData: details.extracted_data- **Costo en caso de error**: 0 Lümis (reembolso automático)

      };- **Rate limiting**: Aplicado por usuario según configuración del administrador

    }- **Límite de intentos**: Sin límite, pero sujeto a rate limiting

- **Reembolso**: Lümis devueltos automáticamente si el procesamiento falla antes de completarse

    throw new Error(result.error?.message || 'Error desconocido');- **Validación**: Facturas quedan en estado "pending_validation" para revisión manual en 24-48 horas

  }- **Timeout de procesamiento**: 30 segundos máximo por request

- **Retención de datos**: Las imágenes se procesan y descartan, no se almacenan permanentemente

  // Paso 2: Retry con campos específicos

  async retryMissingFields(newImageFile, missingFieldKeys) {## Seguridad y Privacidad

    if (!this.extractedData) {

      throw new Error('No hay datos previos. Usa uploadInvoice primero.');- **Encriptación en tránsito**: TLS 1.2+ obligatorio

    }- **Validación de entrada**: Sanitización de todos los parámetros

- **Logs de auditoría**: Todas las operaciones se registran para compliance

    const formData = new FormData();- **Retención de logs**: 90 días para debugging y auditoría

    formData.append('image', newImageFile);- **GDPR compliance**: Los datos se procesan según políticas de privacidad

    formData.append('missing_fields', JSON.stringify(missingFieldKeys));- **Datos sensibles**: Las imágenes no se almacenan después del procesamiento

    formData.append('previous_data', JSON.stringify(this.extractedData));- **Anonimización**: Los logs no contienen información personal identificable



    const response = await fetch(`${this.apiUrl}/api/v4/invoices/upload-ocr-retry`, {## Casos de Uso del Parámetro Mode

      method: 'POST',

      headers: { 'Authorization': `Bearer ${this.token}` },### Escenario 1: Factura Individual (Mode 1)

      body: formData- **Situación**: Usuario toma una foto directa de una factura

    });- **Parámetro**: `mode=1` o sin especificar

- **Resultado**: Procesamiento OCR estándar sin consideraciones especiales

    const result = await response.json();

### Escenario 2: Imagen Combinada (Mode 2)

    if (result.success) {- **Situación**: Usuario combina múltiples capturas de pantalla o fotos de la misma factura

      // ✅ Factura ahora está completa- **Parámetro**: `mode=2`

      this.extractedData = null; // Limpiar- **Resultado**: Gemini recibe instrucción especial para:

      return { success: true, data: result.data };  - Detectar información duplicada

    }  - Consolidar datos repetidos

  - Construir una factura única y limpia

    // ❌ Aún faltan campos - actualizar extracted_data para próximo retry- **Casos comunes**:

    const details = result.error?.details;  - Factura muy larga que requiere múltiples capturas

    if (details?.extracted_data) {  - Combinación de header + detalles + footer

      this.extractedData = details.extracted_data; // Actualizar con merge parcial  - Screenshots de diferentes secciones de una factura digital

    }

### Recomendaciones de Uso

    return {- **Use Mode 1** para facturas estándar de una sola imagen

      success: false,- **Use Mode 2** cuando la imagen contenga:

      missingFields: details?.missing_fields || [],  - Múltiples capturas de la misma factura

      extractedData: details?.extracted_data  - Datos visiblemente duplicados

    };  - Secciones combinadas en una sola imagen

  }

}## Notas Técnicas



// ============================================================### Proveedores de IA (con Fallback)

// EJEMPLO DE USO COMPLETO

// ============================================================El sistema utiliza un mecanismo de fallback automático para garantizar disponibilidad:



const ocr = new OcrService('https://api.lumis.com', userToken);| Prioridad | Proveedor | Modelo | API |

|-----------|-----------|--------|-----|

// Primer intento| **1 (Primary)** | Google Gemini | `gemini-2.0-flash` | `generativelanguage.googleapis.com/v1beta` |

const firstResult = await ocr.uploadInvoice(photoFile);| **2 (Fallback)** | OpenRouter | `qwen/qwen3-vl-30b-a3b-instruct` | `openrouter.ai/api/v1` |



if (!firstResult.success) {### Flujo de Fallback

  console.log('Faltan campos:', firstResult.missingFields.map(f => f.field_name));

  // Usuario toma nueva foto...```

  ┌─────────────────────────────────────────┐

  const retryResult = await ocr.retryMissingFields(│  POST /api/v4/invoices/upload-ocr      │

    newPhotoFile, └─────────────────────────────────────────┘

    firstResult.missingFields.map(f => f.field_key)                    │

  );                    ▼

  ┌─────────────────────────────────────────┐

  if (retryResult.success) {│  1. Intenta Gemini 2.0 Flash           │

    console.log('¡Factura procesada!', retryResult.data);│     - temperature: 0.1                  │

  } else {│     - maxOutputTokens: 2048            │

    // Puede reintentar de nuevo con otra imagen└─────────────────────────────────────────┘

    console.log('Aún faltan:', retryResult.missingFields.map(f => f.field_name));                    │

  }           ┌───────┴───────┐

}           │               │

```        ✅ OK          ❌ Error

           │               │

---           ▼               ▼

┌──────────────┐  ┌─────────────────────────────┐

## Flujo Completo en Flutter/Dart│  Retorna     │  │  2. FALLBACK: OpenRouter    │

│  resultado   │  │     Qwen3-VL-30B            │

```dart└──────────────┘  │     - temperature: 0.1      │

import 'dart:convert';                  │     - max_tokens: 2048      │

import 'dart:io';                  └─────────────────────────────┘

import 'package:dio/dio.dart';                              │

                     ┌───────┴───────┐

class OcrService {                     │               │

  final Dio _dio;                  ✅ OK          ❌ Error

  final String _baseUrl;                     │               │

  Map<String, dynamic>? _extractedData;                     ▼               ▼

              ┌──────────────┐  ┌──────────────┐

  OcrService(this._baseUrl, String token) : _dio = Dio() {              │  Retorna     │  │  Error 500   │

    _dio.options.headers['Authorization'] = 'Bearer $token';              │  resultado   │  │  Ambos       │

  }              └──────────────┘  │  fallaron    │

                               └──────────────┘

  /// Paso 1: Primer OCR```

  Future<OcrResult> uploadInvoice(File imageFile) async {

    final formData = FormData.fromMap({### Configuración de Modelos

      'image': await MultipartFile.fromFile(imageFile.path),

    });**Gemini 2.0 Flash (Primary):**

```json

    try {{

      final response = await _dio.post(  "temperature": 0.1,

        '$_baseUrl/api/v4/invoices/upload-ocr',  "maxOutputTokens": 2048

        data: formData,}

      );```



      final data = response.data;**Qwen3-VL-30B (Fallback):**

```json

      if (data['success'] == true) {{

        return OcrResult.success(data['data']);  "temperature": 0.1,

      }  "max_tokens": 2048

}

      // Faltan campos```

      final details = data['error']?['details'];

      if (details != null) {### Variables de Entorno

        _extractedData = details['extracted_data'];

        return OcrResult.incomplete(| Variable | Descripción | Requerido |

          missingFields: List<Map<String, dynamic>>.from(details['missing_fields'] ?? []),|----------|-------------|-----------|

          extractedData: details['extracted_data'],| `GEMINI_API_KEY` | API key para Google Gemini | ✅ Sí |

        );| `OPENROUTER_API_KEY` | API key para OpenRouter (fallback) | ❌ Tiene default |

      }

- **Formatos soportados**: JPEG, PNG, PDF (validación por magic bytes)

      throw Exception(data['error']?['message'] ?? 'Error desconocido');- **Timeout**: El procesamiento puede tomar 10-30 segundos

    } on DioException catch (e) {- **Idempotencia**: Múltiples requests con la misma imagen pueden generar CUFEs diferentes

      // Manejar errores HTTP específicos- **Logging**: Todas las operaciones se registran para auditoría

      if (e.response?.statusCode == 422) {- **Fallback**: Si Gemini falla, se intenta automáticamente con OpenRouter

        final details = e.response?.data['error']?['details'];

        if (details != null) {## Integración con Lumimatch (Segmentación)

          _extractedData = details['extracted_data'];

          return OcrResult.incomplete(El procesamiento de facturas via OCR genera **tags automáticos** que se usan para segmentación en el módulo Lumimatch:

            missingFields: List<Map<String, dynamic>>.from(details['missing_fields'] ?? []),

            extractedData: details['extracted_data'],### Tags generados automáticamente

          );

        }| Tipo | Formato del Tag | Ejemplo |

      }|------|-----------------|---------|

      rethrow;| Código de producto | `product_code:{valor}` | `product_code:ABC123` |

    }| Categoría L1 | `product_l1:{valor}` | `product_l1:alimentos` |

  }| Categoría L2 | `product_l2:{valor}` | `product_l2:lacteos` |

| Marca de producto | `product_brand:{valor}` | `product_brand:cocacola` |

  /// Paso 2: Retry con campos específicos| RUC del emisor | `issuer_ruc:{valor}` | `issuer_ruc:12345678` |

  Future<OcrResult> retryMissingFields(File imageFile, List<String> fieldKeys) async {| Marca del comercio | `issuer_brand_name:{valor}` | `issuer_brand_name:mcdonalds` |

    if (_extractedData == null) {| Tipo de comercio | `issuer_l1:{valor}` | `issuer_l1:restaurantes` |

      throw Exception('No hay datos previos. Usa uploadInvoice primero.');

    }Estos tags se almacenan en `lumimatch.user_tags` y permiten mostrar preguntas segmentadas basadas en el historial de compras del usuario.



    final formData = FormData.fromMap({Ver: [API_DOC_LUMIMATCH.md](./API_DOC_LUMIMATCH.md) para documentación completa del motor de preguntas.

      'image': await MultipartFile.fromFile(imageFile.path),

      'missing_fields': jsonEncode(fieldKeys),## Ejemplos de Integración

      'previous_data': jsonEncode(_extractedData),

    });### Frontend JavaScript (React/Vue)

```javascript

    try {async function uploadInvoice(file, mode = 1) {

      final response = await _dio.post(  const formData = new FormData();

        '$_baseUrl/api/v4/invoices/upload-ocr-retry',  formData.append('image', file);

        data: formData,  formData.append('mode', mode.toString()); // 1 = normal, 2 = combinada

      );  

  try {

      final data = response.data;    const response = await fetch('/api/v4/invoices/upload-ocr', {

      method: 'POST',

      if (data['success'] == true) {      headers: {

        _extractedData = null; // Limpiar        'Authorization': `Bearer ${getAuthToken()}`

        return OcrResult.success(data['data']);      },

      }      body: formData

    });

      // Actualizar extracted_data con el merge parcial    

      final details = data['error']?['details'];    const result = await response.json();

      if (details?['extracted_data'] != null) {    

        _extractedData = details['extracted_data'];    if (result.success) {

      }      console.log('Factura procesada:', result.data);

      return result.data;

      return OcrResult.incomplete(    } else {

        missingFields: List<Map<String, dynamic>>.from(details?['missing_fields'] ?? []),      console.error('Error:', result.error);

        extractedData: details?['extracted_data'],      throw new Error(result.error.message);

      );    }

    } on DioException catch (e) {  } catch (error) {

      if (e.response?.statusCode == 422) {    console.error('Error de red:', error);

        final details = e.response?.data['error']?['details'];    throw error;

        if (details?['extracted_data'] != null) {  }

          _extractedData = details['extracted_data'];}

        }

        return OcrResult.incomplete(// Uso:

          missingFields: List<Map<String, dynamic>>.from(details?['missing_fields'] ?? []),// uploadInvoice(file, 1); // Factura normal

          extractedData: details?['extracted_data'],// uploadInvoice(file, 2); // Imagen combinada

        );```

      }

      rethrow;### Python

    }```python

  }import requests



  /// Limpiar datos guardadosdef upload_invoice(file_path, token, mode=1):

  void reset() {    url = "https://api.lumis.com/api/v4/invoices/upload-ocr"

    _extractedData = null;    headers = {"Authorization": f"Bearer {token}"}

  }    

}    with open(file_path, 'rb') as file:

        files = {'image': file}

class OcrResult {        data = {'mode': str(mode)}  # 1 = normal, 2 = combinada

  final bool success;        response = requests.post(url, headers=headers, files=files, data=data)

  final Map<String, dynamic>? data;    

  final List<Map<String, dynamic>>? missingFields;    if response.status_code == 200:

  final Map<String, dynamic>? extractedData;        return response.json()

    else:

  OcrResult.success(this.data)        raise Exception(f"Error {response.status_code}: {response.text}")

      : success = true,

        missingFields = null,# Uso:

        extractedData = null;# upload_invoice('factura.jpg', token, 1)  # Normal

# upload_invoice('factura_combinada.jpg', token, 2)  # Combinada

  OcrResult.incomplete({this.missingFields, this.extractedData})```

      : success = false,

        data = null;### Mobile (Flutter/Dart)

}```dart

Future<Map<String, dynamic>> uploadInvoice(File imageFile, String token, {int mode = 1}) async {

// ============================================================  var uri = Uri.parse('https://api.lumis.com/api/v4/invoices/upload-ocr');

// EJEMPLO DE USO EN FLUTTER UI  var request = http.MultipartRequest('POST', uri);

// ============================================================  

  request.headers['Authorization'] = 'Bearer $token';

class InvoiceUploadPage extends StatefulWidget {  request.files.add(await http.MultipartFile.fromPath('image', imageFile.path));

  @override  request.fields['mode'] = mode.toString(); // 1 = normal, 2 = combinada

  _InvoiceUploadPageState createState() => _InvoiceUploadPageState();  

}  var streamedResponse = await request.send();

  var response = await http.Response.fromStream(streamedResponse);

class _InvoiceUploadPageState extends State<InvoiceUploadPage> {  

  late OcrService _ocrService;  if (response.statusCode == 200) {

  bool _isLoading = false;    return json.decode(response.body);

  List<Map<String, dynamic>>? _missingFields;  } else {

    throw Exception('Error ${response.statusCode}: ${response.body}');

  @override  }

  void initState() {}

    super.initState();

    _ocrService = OcrService('https://api.lumis.com', userToken);// Uso:

  }// await uploadInvoice(file, token, mode: 1); // Normal

// await uploadInvoice(file, token, mode: 2); // Combinada

  Future<void> _processInvoice(File imageFile) async {```
    setState(() => _isLoading = true);
    
    try {
      final result = await _ocrService.uploadInvoice(imageFile);
      
      if (result.success) {
        _showSuccess('¡Factura procesada correctamente!');
        Navigator.pop(context, result.data);
      } else {
        setState(() => _missingFields = result.missingFields);
        _showMissingFieldsDialog(result.missingFields!);
      }
    } catch (e) {
      _showError('Error: $e');
    } finally {
      setState(() => _isLoading = false);
    }
  }

  void _showMissingFieldsDialog(List<Map<String, dynamic>> fields) {
    final fieldNames = fields.map((f) => f['field_name']).join(', ');
    
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        title: Text('Campos faltantes'),
        content: Text(
          'No se pudieron detectar: $fieldNames\n\n'
          'Toma una nueva foto enfocada en estos datos de la factura.'
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text('Cancelar'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(context);
              _captureRetryImage();
            },
            child: Text('Tomar nueva foto'),
          ),
        ],
      ),
    );
  }

  Future<void> _captureRetryImage() async {
    // Capturar nueva imagen con cámara
    final newImage = await ImagePicker().pickImage(source: ImageSource.camera);
    if (newImage == null) return;
    
    setState(() => _isLoading = true);
    
    try {
      final fieldKeys = _missingFields!
          .map((f) => f['field_key'] as String)
          .toList();
      
      final result = await _ocrService.retryMissingFields(
        File(newImage.path), 
        fieldKeys
      );
      
      if (result.success) {
        _showSuccess('¡Factura procesada correctamente!');
        Navigator.pop(context, result.data);
      } else {
        setState(() => _missingFields = result.missingFields);
        _showMissingFieldsDialog(result.missingFields!);
      }
    } catch (e) {
      _showError('Error: $e');
    } finally {
      setState(() => _isLoading = false);
    }
  }

  void _showSuccess(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), backgroundColor: Colors.green)
    );
  }

  void _showError(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), backgroundColor: Colors.red)
    );
  }

  @override
  Widget build(BuildContext context) {
    // UI implementation...
  }
}
```

---

## Python

```python
import requests
import json
from typing import Optional, List, Dict, Any

class OcrService:
    def __init__(self, base_url: str, token: str):
        self.base_url = base_url
        self.headers = {"Authorization": f"Bearer {token}"}
        self.extracted_data: Optional[Dict[str, Any]] = None
    
    def upload_invoice(self, image_path: str, mode: int = 1) -> Dict[str, Any]:
        """
        Paso 1: Primer OCR
        
        Args:
            image_path: Ruta a la imagen de la factura
            mode: 1 = Normal, 2 = Imagen combinada
            
        Returns:
            dict con success, data/missing_fields/extracted_data
        """
        with open(image_path, 'rb') as f:
            files = {'image': f}
            data = {'mode': str(mode)}
            response = requests.post(
                f"{self.base_url}/api/v4/invoices/upload-ocr",
                headers=self.headers,
                files=files,
                data=data
            )
        
        result = response.json()
        
        if result.get('success'):
            return {"success": True, "data": result['data']}
        
        # Extraer details del error
        details = result.get('error', {}).get('details', {})
        
        if details.get('missing_fields') and details.get('extracted_data'):
            self.extracted_data = details['extracted_data']
            return {
                "success": False,
                "missing_fields": details['missing_fields'],
                "extracted_data": details['extracted_data']
            }
        
        raise Exception(result.get('error', {}).get('message', 'Error desconocido'))
    
    def retry_missing_fields(self, image_path: str, field_keys: List[str]) -> Dict[str, Any]:
        """
        Paso 2: Retry con campos específicos
        
        Args:
            image_path: Ruta a la nueva imagen enfocada en campos faltantes
            field_keys: Lista de field_keys a buscar (ej: ["dv", "products"])
            
        Returns:
            dict con success, data/missing_fields/extracted_data
        """
        if not self.extracted_data:
            raise Exception("No hay datos previos. Usa upload_invoice primero.")
        
        with open(image_path, 'rb') as f:
            files = {'image': f}
            form_data = {
                'missing_fields': json.dumps(field_keys),
                'previous_data': json.dumps(self.extracted_data)
            }
            response = requests.post(
                f"{self.base_url}/api/v4/invoices/upload-ocr-retry",
                headers=self.headers,
                files=files,
                data=form_data
            )
        
        result = response.json()
        
        if result.get('success'):
            self.extracted_data = None  # Limpiar
            return {"success": True, "data": result['data']}
        
        # Actualizar extracted_data con merge parcial
        details = result.get('error', {}).get('details', {})
        if details.get('extracted_data'):
            self.extracted_data = details['extracted_data']
        
        return {
            "success": False,
            "missing_fields": details.get('missing_fields', []),
            "extracted_data": details.get('extracted_data')
        }
    
    def reset(self):
        """Limpiar datos guardados"""
        self.extracted_data = None


# ============================================================
# EJEMPLO DE USO COMPLETO
# ============================================================

if __name__ == "__main__":
    # Inicializar servicio
    ocr = OcrService("https://api.lumis.com", "tu_token_jwt")
    
    # Primer intento
    print("📷 Procesando primera imagen...")
    result = ocr.upload_invoice("factura.jpg")
    
    if result['success']:
        print("✅ ¡Factura procesada exitosamente!")
        print(f"   CUFE: {result['data']['cufe']}")
        print(f"   Total: ${result['data']['total']}")
    else:
        # Mostrar campos faltantes
        print("❌ Faltan campos obligatorios:")
        for field in result['missing_fields']:
            print(f"   - {field['field_name']}: {field['description']}")
        
        # Obtener field_keys para retry
        field_keys = [f['field_key'] for f in result['missing_fields']]
        
        # Retry con nueva imagen
        print("\n📷 Procesando segunda imagen (retry)...")
        retry_result = ocr.retry_missing_fields("factura_detalle.jpg", field_keys)
        
        if retry_result['success']:
            print("✅ ¡Factura completada exitosamente!")
            print(f"   CUFE: {retry_result['data']['cufe']}")
            print(f"   Total: ${retry_result['data']['total']}")
            print(f"   Productos: {retry_result['data']['products_count']}")
        else:
            print("❌ Aún faltan campos:")
            for field in retry_result['missing_fields']:
                print(f"   - {field['field_name']}")
            # Puede reintentar de nuevo...
```

---

# Notas Técnicas

## Proveedores de IA (con Fallback)

| Prioridad | Proveedor | Modelo | Config |
|-----------|-----------|--------|--------|
| **1 (Primary)** | Google Gemini | `gemini-2.0-flash` | temperature: 0.1, maxTokens: 2048 |
| **2 (Fallback)** | OpenRouter | `qwen/qwen3-vl-30b` | temperature: 0.1, maxTokens: 2048 |

## Flujo de Fallback

```
┌─────────────────────────────────────────┐
│  1. Intenta Gemini 2.0 Flash           │
└─────────────────────────────────────────┘
                    │
           ┌───────┴───────┐
           │               │
        ✅ OK          ❌ Error
           │               │
           ▼               ▼
┌──────────────┐  ┌─────────────────────────────┐
│  Retorna     │  │  2. FALLBACK: OpenRouter    │
│  resultado   │  │     Qwen3-VL-30B            │
└──────────────┘  └─────────────────────────────┘
                              │
                     ┌───────┴───────┐
                     │               │
                  ✅ OK          ❌ Error
                     │               │
                     ▼               ▼
              ┌──────────────┐  ┌──────────────┐
              │  Retorna     │  │  Error 500   │
              │  resultado   │  │  Ambos       │
              └──────────────┘  │  fallaron    │
                               └──────────────┘
```

## Variables de Entorno

| Variable | Descripción | Requerido |
|----------|-------------|-----------|
| `GEMINI_API_KEY` | API key para Google Gemini | ✅ Sí |
| `OPENROUTER_API_KEY` | API key para OpenRouter | ❌ Default |

## Códigos de Estado HTTP

| Código | Descripción |
|--------|-------------|
| `200` | OCR procesado exitosamente, factura completa |
| `400` | Request inválido (archivo faltante, datos inválidos) |
| `401` | Token JWT inválido o faltante |
| `402` | Saldo insuficiente de Lümis |
| `413` | Archivo muy grande (>10MB) |
| `415` | Formato de archivo no soportado |
| `422` | Campos obligatorios faltantes |
| `429` | Límite de rate limiting alcanzado |
| `500` | Error interno del servidor |

## Costos en Lümis

| Operación | Costo | Condición |
|-----------|-------|-----------|
| Upload OCR exitoso | 15 Lümis | Factura completa |
| Upload OCR con campos faltantes | 15 Lümis | Ya procesó imagen |
| Upload OCR error | 0 Lümis | Error antes de procesar |
| Retry exitoso | 5 Lümis | Campos encontrados |
| Retry incompleto | 5 Lümis | Aún faltan campos |
| Retry error | 0 Lümis | Error antes de procesar |

## Parámetro Mode (solo upload-ocr)

| Mode | Descripción | Uso |
|------|-------------|-----|
| `1` | Normal (default) | Factura individual estándar |
| `2` | Imagen combinada | Múltiples fotos combinadas de la misma factura |

---

## Integración con Lumimatch

El OCR genera tags automáticos para segmentación:

| Tag | Formato | Ejemplo |
|-----|---------|---------|
| Código producto | `product_code:{valor}` | `product_code:ABC123` |
| Categoría L1 | `product_l1:{valor}` | `product_l1:alimentos` |
| Categoría L2 | `product_l2:{valor}` | `product_l2:lacteos` |
| Marca producto | `product_brand:{valor}` | `product_brand:cocacola` |
| RUC emisor | `issuer_ruc:{valor}` | `issuer_ruc:12345678` |
| Marca comercio | `issuer_brand_name:{valor}` | `issuer_brand_name:mcdonalds` |

Ver: [API_DOC_LUMIMATCH.md](./API_DOC_LUMIMATCH.md)

---

## Resumen de Endpoints

| Endpoint | Método | Costo | Descripción |
|----------|--------|-------|-------------|
| `/api/v4/invoices/upload-ocr` | POST | 15 Lümis | OCR completo de factura |
| `/api/v4/invoices/upload-ocr-retry` | POST | 5 Lümis | Retry enfocado en campos específicos |
