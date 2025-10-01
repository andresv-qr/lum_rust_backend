# API de OCR Iterativo - Documentación Técnica

## Resumen Ejecutivo

API REST para procesamiento iterativo de facturas sin QR que permite hasta 3 intentos de captura, con validación de campos obligatorios y preview consolidado tipo WhatsApp.

## Endpoints Disponibles

### 1. POST `/api/v4/invoices/ocr-process`
**Descripción**: Procesa imágenes de facturas de manera iterativa hasta obtener todos los campos requeridos.

### 2. POST `/api/v4/invoices/save-ocr`
**Descripción**: Guarda la factura procesada en la base de datos después de la confirmación del usuario.

---

## 1. OCR Process Endpoint

### **POST** `/api/v4/invoices/ocr-process`

#### **Headers Requeridos**
```http
Content-Type: multipart/form-data
Authorization: Bearer {jwt_token}
```

#### **Parámetros**

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `image` | File | ✅ | Imagen de la factura (JPG, PNG, PDF) |
| `session_id` | String | ❌ | ID de sesión existente (para reintentos) |
| `action` | String | ✅ | Tipo de acción: `"initial"` o `"retry"` |
| `missing_fields` | JSON Array | ❌ | Lista de campos faltantes para enfocar |

#### **Ejemplo de Request - Primer Intento**

```bash
curl -X POST "https://api.example.com/api/v4/invoices/ocr-process" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -F "image=@factura.jpg" \
  -F "action=initial"
```

#### **Ejemplo de Request - Reintento**

```bash
curl -X POST "https://api.example.com/api/v4/invoices/ocr-process" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -F "image=@factura_detalle.jpg" \
  -F "action=retry" \
  -F "session_id=ocr_sess_a1b2c3d4" \
  -F 'missing_fields=["total", "products"]'
```

#### **Response Format**

```json
{
  "status": "success",
  "session_id": "ocr_sess_a1b2c3d4",
  "attempt_count": 2,
  "processing_result": "needs_more_data",
  "detected_fields": {
    "issuer_name": "Supermercado La Pradera S.A.",
    "issuer_ruc": "1234567890123",
    "invoice_number": "F001-000123456",
    "date": "2025-09-05T14:30:00Z",
    "total": 45.75,
    "tax_amount": 6.89,
    "products": [
      {
        "name": "Arroz Blanco 5lb",
        "code": "ARR001",
        "quantity": 2,
        "unit_price": 12.50,
        "discount": 0.00,
        "total": 25.00
      },
      {
        "name": "Aceite Vegetal 1L",
        "code": "ACE002", 
        "quantity": 1,
        "unit_price": 13.86,
        "discount": 0.00,
        "total": 13.86
      }
    ]
  },
  "missing_fields": [],
  "validation_errors": [],
  "consolidated_image": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQ...",
  "next_action": "ready_for_preview",
  "cost_info": {
    "tokens_used": 1250,
    "estimated_cost_usd": 0.0025
  }
}
```

#### **Estados de Response**

| Status | processing_result | next_action | Descripción |
|--------|-------------------|-------------|-------------|
| `success` | `needs_more_data` | `take_another_photo` | Faltan campos, necesita más imágenes |
| `success` | `ready_for_preview` | `ready_for_preview` | Todos los campos detectados |
| `success` | `max_attempts_reached` | `manual_review` | 3 intentos alcanzados |
| `error` | `processing_failed` | `retry_or_cancel` | Error en procesamiento |

#### **Ejemplo - Campos Faltantes**

```json
{
  "status": "success",
  "session_id": "ocr_sess_a1b2c3d4",
  "attempt_count": 1,
  "processing_result": "needs_more_data",
  "detected_fields": {
    "issuer_name": "Supermercado La Pradera S.A.",
    "date": "2025-09-05T14:30:00Z"
  },
  "missing_fields": [
    "invoice_number",
    "total", 
    "products"
  ],
  "validation_errors": [
    {
      "field": "invoice_number",
      "message": "Número de factura no detectado",
      "suggestion": "Enfoca la parte superior donde aparece el número"
    },
    {
      "field": "total",
      "message": "Total a pagar no detectado", 
      "suggestion": "Enfoca la parte inferior derecha de la factura"
    }
  ],
  "next_action": "take_another_photo",
  "guidance": {
    "focus_areas": [
      {
        "field": "invoice_number",
        "description": "Busca un número como F001-123456 en la parte superior",
        "position": "top_center"
      },
      {
        "field": "total",
        "description": "Busca el monto total en la parte inferior",
        "position": "bottom_right"
      }
    ]
  }
}
```

#### **Ejemplo - Límite Alcanzado**

```json
{
  "status": "success",
  "session_id": "ocr_sess_a1b2c3d4", 
  "attempt_count": 3,
  "processing_result": "max_attempts_reached",
  "detected_fields": {
    "issuer_name": "Supermercado La Pradera S.A.",
    "date": "2025-09-05T14:30:00Z",
    "total": 45.75
  },
  "missing_fields": [
    "invoice_number",
    "products"
  ],
  "next_action": "manual_review",
  "message": "Se alcanzó el límite de 3 intentos. La factura será revisada manualmente.",
  "review_options": {
    "submit_for_manual_review": true,
    "estimated_review_time": "24-48 horas"
  }
}
```

---

## 2. Save OCR Invoice Endpoint

### **POST** `/api/v4/invoices/save-ocr`

#### **Headers Requeridos**
```http
Content-Type: application/json
Authorization: Bearer {jwt_token}
```

#### **Parámetros**

```json
{
  "session_id": "ocr_sess_a1b2c3d4",
  "invoice_data": {
    "issuer_name": "Supermercado La Pradera S.A.",
    "issuer_ruc": "1234567890123", 
    "invoice_number": "F001-000123456",
    "date": "2025-09-05T14:30:00Z",
    "total": 45.75,
    "tax_amount": 6.89,
    "products": [
      {
        "name": "Arroz Blanco 5lb",
        "code": "ARR001",
        "quantity": 2,
        "unit_price": 12.50,
        "discount": 0.00,
        "total": 25.00
      }
    ]
  },
  "consolidated_image": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQ...",
  "user_confirmed": true
}
```

#### **Ejemplo de Request**

```bash
curl -X POST "https://api.example.com/api/v4/invoices/save-ocr" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "ocr_sess_a1b2c3d4",
    "invoice_data": {
      "issuer_name": "Supermercado La Pradera S.A.",
      "issuer_ruc": "1234567890123",
      "invoice_number": "F001-000123456", 
      "date": "2025-09-05T14:30:00Z",
      "total": 45.75,
      "tax_amount": 6.89,
      "products": [
        {
          "name": "Arroz Blanco 5lb",
          "code": "ARR001",
          "quantity": 2,
          "unit_price": 12.50,
          "discount": 0.00,
          "total": 25.00
        }
      ]
    },
    "consolidated_image": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQ...",
    "user_confirmed": true
  }'
```

#### **Response Format - Éxito**

```json
{
  "status": "success",
  "message": "Factura guardada exitosamente",
  "invoice_id": 12345,
  "cufe": "1234567890abcdef1234567890abcdef12345678",
  "processing_summary": {
    "session_id": "ocr_sess_a1b2c3d4",
    "total_attempts": 2,
    "processing_time_seconds": 45.3,
    "total_tokens_used": 2100,
    "total_cost_usd": 0.0042
  },
  "next_steps": {
    "validation_time": "24-48 horas",
    "notification_method": "whatsapp",
    "tracking_url": "https://app.example.com/invoices/12345"
  }
}
```

#### **Response Format - Error**

```json
{
  "status": "error",
  "error_code": "DUPLICATE_INVOICE",
  "message": "Esta factura ya fue registrada anteriormente",
  "details": {
    "existing_invoice_id": 11234,
    "existing_cufe": "abcdef1234567890abcdef1234567890abcdef12",
    "registered_date": "2025-09-03T10:15:00Z"
  },
  "suggested_action": "verify_existing_invoice"
}
```

---

## Códigos de Error

### **OCR Process Errors**

| Código | Descripción | Acción Sugerida |
|--------|-------------|-----------------|
| `INVALID_IMAGE` | Imagen corrupta o formato no soportado | Subir nueva imagen |
| `IMAGE_TOO_LARGE` | Imagen excede 10MB | Comprimir imagen |
| `SESSION_NOT_FOUND` | Sesión no encontrada o expirada | Iniciar nueva sesión |
| `MAX_ATTEMPTS_REACHED` | Se alcanzaron 3 intentos | Revisión manual |
| `OCR_PROCESSING_FAILED` | Error en servicio OCR | Reintentar más tarde |
| `INVALID_INVOICE_FORMAT` | No se detectó formato de factura | Verificar imagen |

### **Save Invoice Errors**

| Código | Descripción | Acción Sugerida |
|--------|-------------|-----------------|
| `DUPLICATE_INVOICE` | Factura ya registrada | Verificar factura existente |
| `INVALID_DATA` | Datos de factura inválidos | Corregir datos |
| `SESSION_EXPIRED` | Sesión expiró | Reiniciar proceso |
| `VALIDATION_FAILED` | Falló validación de campos | Revisar campos obligatorios |
| `DATABASE_ERROR` | Error en base de datos | Contactar soporte |

---

## Flujo Completo de Uso

### **Scenario 1: Éxito en 1 Intento**

```mermaid
sequenceDiagram
    participant App as Flutter App
    participant API as OCR API
    participant DB as Database

    App->>API: POST /ocr-process (action=initial, image)
    API->>API: Procesar OCR
    API->>App: 200 OK (all fields detected, ready_for_preview)
    App->>App: Mostrar preview consolidado
    App->>API: POST /save-ocr (confirmed data)
    API->>DB: Guardar factura
    API->>App: 200 OK (CUFE, invoice_id)
```

### **Scenario 2: Requiere 2 Intentos**

```mermaid
sequenceDiagram
    participant App as Flutter App  
    participant API as OCR API

    App->>API: POST /ocr-process (action=initial, image)
    API->>App: 200 OK (missing fields: total, products)
    App->>App: Mostrar campos faltantes + guidance
    App->>API: POST /ocr-process (action=retry, session_id, image)
    API->>App: 200 OK (all fields detected, ready_for_preview)
    App->>API: POST /save-ocr (confirmed data)
    API->>App: 200 OK (CUFE, invoice_id)
```

### **Scenario 3: Límite Alcanzado**

```mermaid
sequenceDiagram
    participant App as Flutter App
    participant API as OCR API

    App->>API: POST /ocr-process (action=initial, image)
    API->>App: 200 OK (missing fields)
    App->>API: POST /ocr-process (action=retry, session_id, image)  
    API->>App: 200 OK (still missing fields)
    App->>API: POST /ocr-process (action=retry, session_id, image)
    API->>App: 200 OK (max_attempts_reached, manual_review)
    App->>App: Mostrar opción de revisión manual
```

---

## Campos Obligatorios de Factura

### **Campos Requeridos**

| Campo | Nombre Display | Tipo | Ejemplo | Descripción |
|-------|----------------|------|---------|-------------|
| `issuer_name` | Nombre del Comercio | String | "Supermercado La Pradera S.A." | Razón social del emisor |
| `issuer_ruc` | RUC del Comercio | String | "1234567890123" | Identificación fiscal |
| `invoice_number` | Número de Factura | String | "F001-000123456" | Número único de factura |
| `date` | Fecha | ISO DateTime | "2025-09-05T14:30:00Z" | Fecha de emisión |
| `total` | Total a Pagar | Number | 45.75 | Monto total incluyendo impuestos |
| `tax_amount` | Total ITBMS | Number | 6.89 | Impuesto sobre transferencia |
| `products` | Productos | Array | Ver estructura abajo | Lista de artículos |

### **Estructura de Productos**

```json
{
  "name": "Arroz Blanco 5lb",           // Nombre del producto
  "code": "ARR001",                     // Código interno (opcional)
  "quantity": 2,                        // Cantidad  
  "unit_price": 12.50,                  // Precio unitario
  "discount": 0.00,                     // Descuento aplicado
  "total": 25.00                        // Total línea (qty * unit_price - discount)
}
```

---

## Tipos de Archivo Soportados

| Formato | Extensión | Tamaño Máximo | Notas |
|---------|-----------|---------------|-------|
| JPEG | `.jpg`, `.jpeg` | 10MB | Recomendado para fotos |
| PNG | `.png` | 10MB | Buena calidad |
| PDF | `.pdf` | 10MB | Solo primera página |

---

## Rate Limiting

- **OCR Process**: 5 requests por minuto por usuario
- **Save Invoice**: 10 requests por minuto por usuario
- **Headers de Rate Limit**:
  ```http
  X-RateLimit-Limit: 5
  X-RateLimit-Remaining: 3
  X-RateLimit-Reset: 1693920000
  ```

---

## Autenticación

Todas las APIs requieren autenticación JWT:

```http
Authorization: Bearer {jwt_token}
```

El token debe incluir:
- `user_id`: ID del usuario
- `exp`: Timestamp de expiración
- `iat`: Timestamp de emisión

---

## Monitoreo y Logs

### **Headers de Response**

```http
X-Request-ID: req_a1b2c3d4e5f6
X-Processing-Time: 1.234
X-Tokens-Used: 1250
X-Cost-USD: 0.0025
```

### **Logs de Actividad**

La API registra automáticamente:
- Intentos de OCR por usuario
- Tokens consumidos por sesión
- Costos por procesamiento
- Errores y reintentos
- Tiempo de procesamiento

---

## Ejemplos de Integración

### **Flutter/Dart**

```dart
class OcrApiService {
  static const String baseUrl = 'https://api.example.com';
  
  Future<OcrProcessResponse> processImage({
    required File image,
    String? sessionId,
    required String action,
    List<String>? missingFields,
  }) async {
    var request = http.MultipartRequest(
      'POST',
      Uri.parse('$baseUrl/api/v4/invoices/ocr-process'),
    );
    
    request.headers['Authorization'] = 'Bearer $jwt_token';
    request.files.add(await http.MultipartFile.fromPath('image', image.path));
    request.fields['action'] = action;
    
    if (sessionId != null) {
      request.fields['session_id'] = sessionId;
    }
    
    if (missingFields != null) {
      request.fields['missing_fields'] = jsonEncode(missingFields);
    }
    
    final response = await request.send();
    final responseBody = await response.stream.bytesToString();
    
    if (response.statusCode == 200) {
      return OcrProcessResponse.fromJson(jsonDecode(responseBody));
    } else {
      throw ApiException('OCR processing failed: $responseBody');
    }
  }
  
  Future<SaveOcrResponse> saveInvoice({
    required String sessionId,
    required InvoiceData invoiceData,
    required String consolidatedImage,
  }) async {
    final response = await http.post(
      Uri.parse('$baseUrl/api/v4/invoices/save-ocr'),
      headers: {
        'Authorization': 'Bearer $jwt_token',
        'Content-Type': 'application/json',
      },
      body: jsonEncode({
        'session_id': sessionId,
        'invoice_data': invoiceData.toJson(),
        'consolidated_image': consolidatedImage,
        'user_confirmed': true,
      }),
    );
    
    if (response.statusCode == 200) {
      return SaveOcrResponse.fromJson(jsonDecode(response.body));
    } else {
      throw ApiException('Save failed: ${response.body}');
    }
  }
}
```

### **JavaScript/React Native**

```javascript
class OcrApiClient {
  constructor(baseUrl, jwtToken) {
    this.baseUrl = baseUrl;
    this.jwtToken = jwtToken;
  }
  
  async processImage({ image, sessionId, action, missingFields }) {
    const formData = new FormData();
    formData.append('image', image);
    formData.append('action', action);
    
    if (sessionId) {
      formData.append('session_id', sessionId);
    }
    
    if (missingFields) {
      formData.append('missing_fields', JSON.stringify(missingFields));
    }
    
    const response = await fetch(`${this.baseUrl}/api/v4/invoices/ocr-process`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.jwtToken}`,
      },
      body: formData,
    });
    
    return await response.json();
  }
  
  async saveInvoice({ sessionId, invoiceData, consolidatedImage }) {
    const response = await fetch(`${this.baseUrl}/api/v4/invoices/save-ocr`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.jwtToken}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        session_id: sessionId,
        invoice_data: invoiceData,
        consolidated_image: consolidatedImage,
        user_confirmed: true,
      }),
    });
    
    return await response.json();
  }
}
```

---

Esta documentación proporciona toda la información necesaria para integrar las APIs de OCR iterativo en aplicaciones cliente, con ejemplos completos y manejo de errores.
