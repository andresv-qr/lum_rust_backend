# API OCR Iterativo - Procesamiento Multi-Imagen

## Descripción General
API para procesar facturas sin QR mediante OCR iterativo, permitiendo hasta 5 intentos para obtener todos los campos requeridos.

## Endpoint Principal

### POST `/api/v4/invoices/ocr-process`

Procesa imágenes de facturas de manera iterativa hasta obtener todos los campos requeridos.

## Request

### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: multipart/form-data
```

### Body (multipart/form-data)
```
image: file (requerido) - Imagen de la factura
session_id: string (opcional) - ID de sesión para continuidad
action: string (requerido) - Tipo de acción: "initial" | "retry" | "consolidate"
missing_fields: array<string> (opcional) - Campos específicos a enfocar
```

## Response

### Estructura Base
```json
{
  "success": boolean,
  "session_id": string,
  "attempt_count": number,
  "max_attempts": 5,
  "status": "processing" | "complete" | "needs_retry" | "manual_review" | "failed",
  "detected_fields": {
    "issuer_name": string | null,
    "invoice_number": string | null,
    "date": string | null,
    "total": number | null,
    "products": array | null
  },
  "missing_fields": array<string>,
  "consolidated_image": string | null, // Base64 cuando status = "complete"
  "message": string,
  "cost": {
    "lumis_used": number,
    "tokens_used": number
  }
}
```

## Estados y Flujos

### 1. Primer Intento (action: "initial")
```json
// Request
{
  "image": file,
  "action": "initial"
}

// Response - Datos Completos
{
  "success": true,
  "session_id": "ocr_sess_123",
  "attempt_count": 1,
  "max_attempts": 5,
  "status": "complete",
  "detected_fields": {
    "issuer_name": "Supermercado La Pradera",
    "invoice_number": "F001-000123",
    "date": "2025-09-05",
    "total": 45.67,
    "products": [...]
  },
  "missing_fields": [],
  "consolidated_image": "base64_consolidated_image",
  "message": "Todos los campos fueron detectados correctamente",
  "cost": {
    "lumis_used": 0,
    "tokens_used": 1250
  }
}

// Response - Datos Incompletos
{
  "success": true,
  "session_id": "ocr_sess_123",
  "attempt_count": 1,
  "max_attempts": 5,
  "status": "needs_retry",
  "detected_fields": {
    "issuer_name": "Supermercado La Pradera",
    "invoice_number": null,
    "date": "2025-09-05",
    "total": null,
    "products": []
  },
  "missing_fields": ["invoice_number", "total", "products"],
  "consolidated_image": null,
  "message": "Faltan campos: número de factura, total y productos. Sube una foto enfocando estas áreas",
  "cost": {
    "lumis_used": 0,
    "tokens_used": 1250
  }
}
```

### 2. Reintento (action: "retry")
```json
// Request
{
  "image": file,
  "action": "retry",
  "session_id": "ocr_sess_123",
  "missing_fields": ["invoice_number", "total"]
}

// Response - Progreso
{
  "success": true,
  "session_id": "ocr_sess_123",
  "attempt_count": 2,
  "max_attempts": 5,
  "status": "needs_retry",
  "detected_fields": {
    "issuer_name": "Supermercado La Pradera",
    "invoice_number": "F001-000123", // ✅ Detectado
    "date": "2025-09-05",
    "total": null, // ❌ Aún falta
    "products": []
  },
  "missing_fields": ["total", "products"],
  "consolidated_image": null,
  "message": "¡Progreso! Se detectó el número de factura. Sube una foto clara del área del total y productos",
  "cost": {
    "lumis_used": 0,
    "tokens_used": 800
  }
}
```

### 3. Límite Alcanzado (5 intentos)
```json
{
  "success": false,
  "session_id": "ocr_sess_123",
  "attempt_count": 5,
  "max_attempts": 5,
  "status": "manual_review",
  "detected_fields": {
    "issuer_name": "Supermercado La Pradera",
    "invoice_number": "F001-000123",
    "date": "2025-09-05",
    "total": null,
    "products": []
  },
  "missing_fields": ["total", "products"],
  "consolidated_image": null,
  "message": "Se alcanzó el límite de intentos. Esta factura será revisada por nuestro equipo",
  "cost": {
    "lumis_used": 0,
    "tokens_used": 4200
  }
}
```

## Lógica de Prompts Específicos

### Prompt Base
```
Analiza esta imagen de factura y extrae la siguiente información en formato JSON:
- issuer_name: Nombre del comercio/empresa
- invoice_number: Número de factura
- date: Fecha (formato YYYY-MM-DD)
- total: Monto total (número)
- products: Array de productos con name, quantity, unit_price, total_price
```

### Prompt Enfocado (campos faltantes)
```
Esta es una imagen adicional de una factura. ENFÓCATE ESPECÍFICAMENTE en detectar:
- {missing_fields}

Información ya detectada en intentos anteriores:
{detected_fields}

Solo actualiza o agrega los campos faltantes. No cambies los datos ya detectados.
```

## Campos Requeridos Mínimos

Basado en `validate_required_fields()`:
- `issuer_name`: Nombre del comercio (no vacío)
- `invoice_number`: Número de factura (no vacío)
- `date`: Fecha válida (no vacía)
- `total`: Monto mayor a 0
- `products`: Al menos 1 producto

## Consolidación de Imágenes

### POST `/api/v4/invoices/ocr-consolidate`

Genera imagen única optimizada de todas las imágenes subidas.

```json
// Request
{
  "session_id": "ocr_sess_123"
}

// Response
{
  "success": true,
  "consolidated_image": "base64_optimized_image",
  "original_images_count": 3,
  "optimization": {
    "quality": 85,
    "format": "JPEG",
    "size_reduction": "45%"
  }
}
```

## Códigos de Error

| Código | Descripción |
|--------|-------------|
| 400 | Imagen inválida o datos faltantes |
| 401 | Token JWT inválido |
| 429 | Límite de rate limiting excedido |
| 500 | Error interno del servidor |
| 503 | Servicio OCR temporalmente no disponible |

## Rate Limiting

- **Usuarios básicos**: 3/hora, 10/día
- **Usuarios premium**: 10/hora, 50/día
- Basado en trust_score del usuario

## Logging y Analytics

Cada llamada registra en `analytics.ocr_token_usage`:
```sql
INSERT INTO analytics.ocr_token_usage (
  user_id, session_id, attempt_number, 
  tokens_used, lumis_cost, success, 
  missing_fields, processing_time
) VALUES (...)
```

## Ejemplos de Uso

### Cliente JavaScript
```javascript
// Primer intento
const formData = new FormData();
formData.append('image', imageFile);
formData.append('action', 'initial');

const response = await fetch('/api/v4/invoices/ocr-process', {
  method: 'POST',
  headers: { 'Authorization': `Bearer ${token}` },
  body: formData
});

const result = await response.json();
if (result.status === 'needs_retry') {
  // Mostrar campos faltantes al usuario
  showMissingFields(result.missing_fields);
}
```

### Cliente Flutter
```dart
Future<OcrResponse> processInvoice(File image, {String? sessionId, List<String>? missingFields}) async {
  final request = http.MultipartRequest('POST', Uri.parse('$baseUrl/api/v4/invoices/ocr-process'));
  request.headers['Authorization'] = 'Bearer $token';
  request.files.add(await http.MultipartFile.fromPath('image', image.path));
  request.fields['action'] = sessionId != null ? 'retry' : 'initial';
  
  if (sessionId != null) request.fields['session_id'] = sessionId;
  if (missingFields != null) request.fields['missing_fields'] = jsonEncode(missingFields);
  
  final response = await request.send();
  final responseData = await response.stream.bytesToString();
  return OcrResponse.fromJson(jsonDecode(responseData));
}
```
