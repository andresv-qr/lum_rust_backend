# API OCR - Documentación del Endpoint

## Información General

- **Versión API:** v4
- **Última actualización:** Septiembre 2025
- **Estado:** Producción
- **Compatibilidad:** Retrocompatible con versiones anteriores

## Endpoint: Upload OCR Invoice

**URL:** `POST /api/v4/invoices/upload-ocr`

**Descripción:** Procesa una imagen de factura mediante OCR (Reconocimiento Óptico de Caracteres) para extraer información estructurada de la factura y almacenarla en la base de datos.

## Autenticación

- **Tipo:** Bearer Token (JWT)
- **Header requerido:** `Authorization: Bearer <token>`
- **Middleware:** `extract_current_user` - El usuario debe estar autenticado
- **Scope:** El endpoint extrae automáticamente el `user_id` del token JWT
- **Validación:** Token debe estar activo y no expirado

## Rate Limiting

- **Límite por usuario:** Según configuración del usuario
- **Ventana de tiempo:** Configurable por administrador
- **Respuesta al exceder límite:** HTTP 429 Too Many Requests
- **Headers de respuesta:**
  - `X-RateLimit-Limit`: Límite máximo
  - `X-RateLimit-Remaining`: Requests restantes
  - `X-RateLimit-Reset`: Timestamp de reset
- **Costo por request:** 15 Lümis (deducidos solo si el procesamiento es exitoso)

## Formato de Request

### Content-Type
```
Content-Type: multipart/form-data
```

### Parámetros

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `image` o `file` | File | Sí | Imagen de la factura a procesar |
| `mode` | String/Integer | No | Modo de procesamiento: `1` = Normal, `2` = Imagen combinada (eliminar duplicados) |

### Restricciones del archivo
- **Formatos soportados:** JPEG, PNG, PDF
- **Tamaño máximo:** 10MB (10,485,760 bytes)
- **Resolución recomendada:** Mínimo 300 DPI para mejor OCR
- **Validación:** Magic bytes para verificar formato real del archivo
- **Codificación:** Multipart form-data con boundary
- **Compresión:** Automática por el cliente HTTP
- **Orientación:** Cualquier orientación (se auto-detecta)

### Calidad de imagen recomendada
- **Nitidez:** Texto claramente legible
- **Contraste:** Alto contraste entre texto y fondo
- **Iluminación:** Uniforme, sin sombras sobre el texto
- **Distorsión:** Mínima perspectiva o curvatura
- **Recorte:** Incluir toda la factura, evitar cortes de texto

### Ejemplo de Request (cURL)
```bash
# Procesamiento normal
curl -X POST "https://api.lumis.com/api/v4/invoices/upload-ocr" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -F "image=@factura.jpg" \
  -F "mode=1"

# Procesamiento de imagen combinada (eliminar duplicados)
curl -X POST "https://api.lumis.com/api/v4/invoices/upload-ocr" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -F "image=@factura_combinada.jpg" \
  -F "mode=2"
```

### Ejemplo de Request (JavaScript/Fetch)
```javascript
const formData = new FormData();
formData.append('image', fileInput.files[0]);
// mode: 1 = Normal, 2 = Imagen combinada
formData.append('mode', '1'); 

const response = await fetch('/api/v4/invoices/upload-ocr', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`
  },
  body: formData
});

const result = await response.json();
```

## Formatos de Respuesta

### Respuesta Exitosa (200 OK)

```json
{
  "success": true,
  "data": {
    "success": true,
    "cufe": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
    "invoice_number": "FACT-2024-001234",
    "issuer_name": "Empresa Ejemplo S.A.S.",
    "total": 125750.50,
    "products_count": 3,
    "cost_lumis": 15,
    "status": "pending_validation",
    "message": "Factura procesada exitosamente"
  },
  "error": null,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-03-15T10:30:00Z",
  "execution_time_ms": null,
  "cached": false
}
```

### Respuesta de Error - Saldo Insuficiente (402 Payment Required)

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "OCR_PROCESSING_FAILED",
    "message": "OCR processing failed",
    "details": {
      "success": false,
      "cost_lumis": 15,
      "message": "Saldo insuficiente de Lümis. Necesitas 15 Lümis.",
      "cufe": null,
      "partial_data": {
        "invoice_number": null,
        "issuer_name": null,
        "total": null,
        "products_count": null
      }
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440001",
  "timestamp": "2024-03-15T10:30:05Z",
  "execution_time_ms": null,
  "cached": false
}
```

### Respuesta de Error - Límite de Rate Limiting (429 Too Many Requests)

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "OCR_PROCESSING_FAILED",
    "message": "OCR processing failed",
    "details": {
      "success": false,
      "cost_lumis": 0,
      "message": "Has alcanzado el límite de procesamiento OCR. Intenta más tarde.",
      "cufe": null,
      "partial_data": {
        "invoice_number": null,
        "issuer_name": null,
        "total": null,
        "products_count": null
      }
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440002",
  "timestamp": "2024-03-15T10:30:10Z",
  "execution_time_ms": null,
  "cached": false
}
```

### Respuesta de Error - Archivo Inválido (400 Bad Request)

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "NO_IMAGE_FILE",
    "message": "No image file provided. Use 'image' or 'file' field name.",
    "details": null
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440003",
  "timestamp": "2024-03-15T10:30:15Z",
  "execution_time_ms": null,
  "cached": false
}
```

### Respuesta de Error - Formato No Soportado (415 Unsupported Media Type)

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "INVALID_FORMAT",
    "message": "Invalid image format. Supported: JPEG, PNG, PDF",
    "details": null
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440004",
  "timestamp": "2024-03-15T10:30:20Z",
  "execution_time_ms": null,
  "cached": false
}
```

### Respuesta de Error - Archivo Muy Grande (413 Payload Too Large)

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "FILE_TOO_LARGE",
    "message": "Image file too large (max 10MB)",
    "details": null
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440005",
  "timestamp": "2024-03-15T10:30:25Z",
  "execution_time_ms": null,
  "cached": false
}
```

## Códigos de Estado HTTP

| Código | Descripción |
|--------|-------------|
| `200` | OCR procesado exitosamente |
| `400` | Request inválido (archivo faltante, datos vacíos) |
| `401` | Token JWT inválido o faltante |
| `402` | Saldo insuficiente de Lümis |
| `413` | Archivo muy grande (>10MB) |
| `415` | Formato de archivo no soportado |
| `422` | Error en procesamiento OCR |
| `429` | Límite de rate limiting alcanzado |
| `500` | Error interno del servidor |

## Campos de Respuesta Detallados

### Respuesta Exitosa - Campo `data`

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `success` | boolean | Siempre `true` en respuesta exitosa |
| `cufe` | string | Código Único de Factura Electrónica generado |
| `invoice_number` | string | Número de factura extraído del documento |
| `issuer_name` | string | Nombre del emisor/empresa |
| `total` | number | Valor total de la factura |
| `products_count` | integer | Cantidad de productos/líneas detectadas |
| `cost_lumis` | integer | Costo en Lümis del procesamiento |
| `status` | string | Estado: `"pending_validation"` |
| `message` | string | Mensaje descriptivo del resultado |

### Respuesta de Error - Campo `error.details`

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `success` | boolean | Siempre `false` en error |
| `cost_lumis` | integer | Lümis deducidos (0 si falló antes del procesamiento) |
| `message` | string | Descripción del error |
| `cufe` | string\|null | CUFE si se generó antes del error |
| `partial_data` | object | Datos parciales extraídos antes del fallo |

## Parámetro Mode - Tipos de Procesamiento

El parámetro `mode` permite especificar el tipo de procesamiento de imagen:

### Modo 1 - Procesamiento Normal
- **Valor**: `1` (por defecto si no se especifica)
- **Uso**: Facturas individuales estándar
- **Comportamiento**: Procesamiento OCR normal sin consideraciones especiales

### Modo 2 - Imagen Combinada
- **Valor**: `2`
- **Uso**: Imágenes que contienen múltiples capturas o están combinadas
- **Comportamiento**: Se agrega instrucción especial a Gemini para:
  - Identificar y eliminar datos duplicados
  - Consolidar información repetida
  - Construir una única factura unificada
- **Prompt adicional**: _"Ten en cuenta que esta imagen es una combinación de varias imágenes, por lo que puede contener datos duplicados. Por favor, elimina los duplicados y construye una única factura consolidada, sin información repetida."_

## Flujo de Procesamiento

1. **Validación de autenticación**: Verificación del token JWT
2. **Extracción del archivo**: Procesamiento del multipart form
3. **Validación del archivo**: Formato, tamaño, magic bytes
4. **Verificación de saldo**: Confirmar Lümis suficientes
5. **Rate limiting**: Verificar límites de uso
6. **Procesamiento OCR**: Envío a Gemini API
7. **Extracción de datos**: Parsing de respuesta JSON
8. **Validación de negocio**: Verificación de datos obligatorios
9. **Persistencia**: Guardado en base de datos
10. **Respuesta**: Retorno de resultado estructurado

## Costo y Límites

- **Costo por procesamiento**: 15 Lümis por factura procesada exitosamente
- **Costo en caso de error**: 0 Lümis (reembolso automático)
- **Rate limiting**: Aplicado por usuario según configuración del administrador
- **Límite de intentos**: Sin límite, pero sujeto a rate limiting
- **Reembolso**: Lümis devueltos automáticamente si el procesamiento falla antes de completarse
- **Validación**: Facturas quedan en estado "pending_validation" para revisión manual en 24-48 horas
- **Timeout de procesamiento**: 30 segundos máximo por request
- **Retención de datos**: Las imágenes se procesan y descartan, no se almacenan permanentemente

## Seguridad y Privacidad

- **Encriptación en tránsito**: TLS 1.2+ obligatorio
- **Validación de entrada**: Sanitización de todos los parámetros
- **Logs de auditoría**: Todas las operaciones se registran para compliance
- **Retención de logs**: 90 días para debugging y auditoría
- **GDPR compliance**: Los datos se procesan según políticas de privacidad
- **Datos sensibles**: Las imágenes no se almacenan después del procesamiento
- **Anonimización**: Los logs no contienen información personal identificable

## Casos de Uso del Parámetro Mode

### Escenario 1: Factura Individual (Mode 1)
- **Situación**: Usuario toma una foto directa de una factura
- **Parámetro**: `mode=1` o sin especificar
- **Resultado**: Procesamiento OCR estándar sin consideraciones especiales

### Escenario 2: Imagen Combinada (Mode 2)
- **Situación**: Usuario combina múltiples capturas de pantalla o fotos de la misma factura
- **Parámetro**: `mode=2`
- **Resultado**: Gemini recibe instrucción especial para:
  - Detectar información duplicada
  - Consolidar datos repetidos
  - Construir una factura única y limpia
- **Casos comunes**:
  - Factura muy larga que requiere múltiples capturas
  - Combinación de header + detalles + footer
  - Screenshots de diferentes secciones de una factura digital

### Recomendaciones de Uso
- **Use Mode 1** para facturas estándar de una sola imagen
- **Use Mode 2** cuando la imagen contenga:
  - Múltiples capturas de la misma factura
  - Datos visiblemente duplicados
  - Secciones combinadas en una sola imagen

## Notas Técnicas

- **Motor OCR**: Google Gemini 1.5 Flash
- **Formatos soportados**: JPEG, PNG, PDF (validación por magic bytes)
- **Timeout**: El procesamiento puede tomar 10-30 segundos
- **Idempotencia**: Múltiples requests con la misma imagen pueden generar CUFEs diferentes
- **Logging**: Todas las operaciones se registran para auditoría
- **Fallback**: Si OCR falla, se intenta reembolso automático de Lümis

## Ejemplos de Integración

### Frontend JavaScript (React/Vue)
```javascript
async function uploadInvoice(file, mode = 1) {
  const formData = new FormData();
  formData.append('image', file);
  formData.append('mode', mode.toString()); // 1 = normal, 2 = combinada
  
  try {
    const response = await fetch('/api/v4/invoices/upload-ocr', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${getAuthToken()}`
      },
      body: formData
    });
    
    const result = await response.json();
    
    if (result.success) {
      console.log('Factura procesada:', result.data);
      return result.data;
    } else {
      console.error('Error:', result.error);
      throw new Error(result.error.message);
    }
  } catch (error) {
    console.error('Error de red:', error);
    throw error;
  }
}

// Uso:
// uploadInvoice(file, 1); // Factura normal
// uploadInvoice(file, 2); // Imagen combinada
```

### Python
```python
import requests

def upload_invoice(file_path, token, mode=1):
    url = "https://api.lumis.com/api/v4/invoices/upload-ocr"
    headers = {"Authorization": f"Bearer {token}"}
    
    with open(file_path, 'rb') as file:
        files = {'image': file}
        data = {'mode': str(mode)}  # 1 = normal, 2 = combinada
        response = requests.post(url, headers=headers, files=files, data=data)
    
    if response.status_code == 200:
        return response.json()
    else:
        raise Exception(f"Error {response.status_code}: {response.text}")

# Uso:
# upload_invoice('factura.jpg', token, 1)  # Normal
# upload_invoice('factura_combinada.jpg', token, 2)  # Combinada
```

### Mobile (Flutter/Dart)
```dart
Future<Map<String, dynamic>> uploadInvoice(File imageFile, String token, {int mode = 1}) async {
  var uri = Uri.parse('https://api.lumis.com/api/v4/invoices/upload-ocr');
  var request = http.MultipartRequest('POST', uri);
  
  request.headers['Authorization'] = 'Bearer $token';
  request.files.add(await http.MultipartFile.fromPath('image', imageFile.path));
  request.fields['mode'] = mode.toString(); // 1 = normal, 2 = combinada
  
  var streamedResponse = await request.send();
  var response = await http.Response.fromStream(streamedResponse);
  
  if (response.statusCode == 200) {
    return json.decode(response.body);
  } else {
    throw Exception('Error ${response.statusCode}: ${response.body}');
  }
}

// Uso:
// await uploadInvoice(file, token, mode: 1); // Normal
// await uploadInvoice(file, token, mode: 2); // Combinada
```