# 📸 API de OCR para Facturas Sin Código QR

## Descripción General

Este sistema permite procesar facturas que **no tienen código QR** mediante OCR (Reconocimiento Óptico de Caracteres) usando modelos de visión por computadora de última generación.

### Características Principales

- ✅ **Cascade de 3 modelos LLM** para máxima precisión
- ✅ **Logging completo** de cada llamada API (tokens, costos, tiempos)
- ✅ **Sistema de retry inteligente** con contexto de datos previos
- ✅ **Prompts optimizados** para facturas de Panamá

---

## 🔄 Flujo Completo de Procesamiento

```
┌─────────────────────────────────────────────────────────────────────┐
│                    FLUJO OCR SIN QR                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. Usuario toma foto de factura                                     │
│              │                                                       │
│              ▼                                                       │
│  ┌─────────────────────────────────────┐                            │
│  │  POST /api/v4/invoices/upload-ocr   │                            │
│  │  (Primera imagen)                   │                            │
│  └─────────────────────────────────────┘                            │
│              │                                                       │
│              ▼                                                       │
│     ┌────────────────┐                                              │
│     │ ¿Todos los     │──── SÍ ───▶ ✅ Factura completa             │
│     │ campos OK?     │              (guardar/procesar)              │
│     └────────────────┘                                              │
│              │                                                       │
│             NO                                                       │
│              │                                                       │
│              ▼                                                       │
│     Respuesta con:                                                   │
│     - success: false                                                 │
│     - extracted_data: {datos parciales}                             │
│     - missing_fields: ["ruc", "dv", ...]                            │
│              │                                                       │
│              ▼                                                       │
│  Usuario toma OTRA foto                                              │
│  (enfocada en campos faltantes)                                      │
│              │                                                       │
│              ▼                                                       │
│  ┌─────────────────────────────────────┐                            │
│  │ POST /api/v4/invoices/upload-ocr-retry │                         │
│  │ + missing_fields                    │                            │
│  │ + previous_data                     │                            │
│  └─────────────────────────────────────┘                            │
│              │                                                       │
│              ▼                                                       │
│     ┌────────────────┐                                              │
│     │ ¿Datos         │──── SÍ ───▶ ✅ Factura completa             │
│     │ completos?     │                                              │
│     └────────────────┘                                              │
│              │                                                       │
│             NO ──────▶ 🔄 Repetir retry o ❌ Rechazar               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 📡 Endpoint 1: Upload OCR (Primera Imagen)

### `POST /api/v4/invoices/upload-ocr`

Procesa la primera imagen de una factura y extrae todos los campos posibles.

### Headers Requeridos

```http
Authorization: Bearer <JWT_TOKEN>
Content-Type: multipart/form-data
```

### Parámetros (multipart/form-data)

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `image` o `file` | File | ✅ Sí | Imagen de la factura (JPEG, PNG, PDF) |
| `mode` | String | ❌ No | `1` = Normal (default), `2` = Combinada |

### Ejemplo de Request (cURL)

```bash
curl -X POST "https://webh.lumapp.org/api/v4/invoices/upload-ocr" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGci..." \
  -F "image=@factura.jpg" \
  -F "mode=1"
```

### Ejemplo de Request (Flutter/Dart)

```dart
final formData = FormData.fromMap({
  'image': await MultipartFile.fromFile(
    imagePath,
    filename: 'invoice.jpg',
    contentType: MediaType('image', 'jpeg'),
  ),
  'mode': '1',
});

final response = await dio.post(
  '/api/v4/invoices/upload-ocr',
  data: formData,
  options: Options(
    headers: {'Authorization': 'Bearer $token'},
  ),
);
```

### Respuesta Exitosa (200 OK)

```json
{
  "success": true,
  "data": {
    "success": true,
    "cufe": null,
    "invoice_number": "001-002-123456",
    "issuer_name": "RESTAURANTE EJEMPLO S.A.",
    "issuer_ruc": "1234567-1-654321",
    "issuer_dv": "89",
    "issuer_address": "Calle 50, Local 123",
    "date": "2025-12-01",
    "total": 125.50,
    "tot_itbms": 8.75,
    "products": [
      {
        "name": "Hamburguesa Premium",
        "quantity": 2,
        "unit_price": 15.00,
        "total_price": 30.00
      },
      {
        "name": "Bebida Grande",
        "quantity": 2,
        "unit_price": 3.50,
        "total_price": 7.00
      }
    ],
    "products_count": 2,
    "cost_lumis": 0,
    "status": "pending_validation",
    "message": "Factura procesada exitosamente",
    "missing_fields": null
  },
  "error": null,
  "request_id": "abc123-def456-...",
  "timestamp": "2025-12-12T17:00:00Z"
}
```

### Respuesta con Campos Faltantes (200 OK, success: false)

```json
{
  "success": true,
  "data": {
    "success": false,
    "cufe": null,
    "invoice_number": null,
    "issuer_name": "PURA VIDA BEACH CORP",
    "issuer_ruc": null,
    "issuer_dv": null,
    "issuer_address": "MAREAS MALL",
    "date": "2025-11-01",
    "total": 26.75,
    "tot_itbms": null,
    "products": [
      {"name": "HAMBURGER", "quantity": 1, "unit_price": 9.0, "total_price": 9.0},
      {"name": "Pizza Raptor", "quantity": 1, "unit_price": 16.0, "total_price": 16.0}
    ],
    "products_count": 2,
    "cost_lumis": 0,
    "status": "missing_fields",
    "message": "No se pudieron detectar todos los campos obligatorios. Campos faltantes: RUC del comercio, Dígito Verificador (DV), Número de Factura.",
    "missing_fields": [
      {"field_key": "ruc", "field_name": "RUC del comercio", "description": "Número de RUC del emisor"},
      {"field_key": "dv", "field_name": "Dígito Verificador (DV)", "description": "DV del RUC"},
      {"field_key": "invoice_number", "field_name": "Número de Factura", "description": "Número único de factura"}
    ],
    "extracted_data": {
      "issuer_name": "PURA VIDA BEACH CORP",
      "issuer_address": "MAREAS MALL",
      "date": "2025-11-01",
      "total": 26.75,
      "products": [...]
    }
  },
  "error": null,
  "request_id": "xyz789-...",
  "timestamp": "2025-12-12T17:05:00Z"
}
```

---

## 📡 Endpoint 2: Upload OCR Retry (Imagen Adicional)

### `POST /api/v4/invoices/upload-ocr-retry`

Procesa una imagen adicional para completar campos faltantes, usando el contexto de los datos ya extraídos.

### Headers Requeridos

```http
Authorization: Bearer <JWT_TOKEN>
Content-Type: multipart/form-data
```

### Parámetros (multipart/form-data)

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `image` o `file` | File | ✅ Sí | Nueva imagen enfocada en campos faltantes |
| `missing_fields` | JSON String | ✅ Sí | Array de field_keys a buscar |
| `previous_data` | JSON String | ⚠️ Recomendado | Datos extraídos del primer OCR |

### Valores válidos para `missing_fields`

```json
["ruc", "dv", "invoice_number", "total", "products"]
```

### Ejemplo de Request (cURL)

```bash
curl -X POST "https://webh.lumapp.org/api/v4/invoices/upload-ocr-retry" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGci..." \
  -F "image=@factura_detalle.jpg" \
  -F 'missing_fields=["ruc", "dv", "invoice_number"]' \
  -F 'previous_data={"issuer_name":"PURA VIDA BEACH CORP","issuer_address":"MAREAS MALL","date":"2025-11-01","total":26.75,"products":[{"name":"HAMBURGER","quantity":1,"unit_price":9.0,"total_price":9.0}]}'
```

### Ejemplo de Request (Flutter/Dart)

```dart
// Después de recibir respuesta con missing_fields del primer OCR
final previousOcrResponse = firstOcrResponse.data;

final formData = FormData.fromMap({
  'image': await MultipartFile.fromFile(
    newImagePath,
    filename: 'invoice_detail.jpg',
  ),
  'missing_fields': jsonEncode(['ruc', 'dv', 'invoice_number']),
  'previous_data': jsonEncode(previousOcrResponse['extracted_data']),
});

final response = await dio.post(
  '/api/v4/invoices/upload-ocr-retry',
  data: formData,
  options: Options(
    headers: {'Authorization': 'Bearer $token'},
  ),
);
```

### Respuesta Exitosa - Datos Completos (200 OK)

```json
{
  "success": true,
  "data": {
    "success": true,
    "cufe": null,
    "invoice_number": "001-002-789012",
    "issuer_name": "PURA VIDA BEACH CORP",
    "issuer_ruc": "8-765-4321",
    "issuer_dv": "56",
    "issuer_address": "MAREAS MALL",
    "date": "2025-11-01",
    "total": 26.75,
    "tot_itbms": null,
    "products": [
      {"name": "HAMBURGER", "quantity": 1, "unit_price": 9.0, "total_price": 9.0},
      {"name": "Pizza Raptor", "quantity": 1, "unit_price": 16.0, "total_price": 16.0}
    ],
    "products_count": 2,
    "cost_lumis": 5,
    "message": "¡Factura completa! Todos los campos obligatorios fueron extraídos.",
    "missing_fields": null,
    "extracted_data": {
      "ruc": "8-765-4321",
      "dv": "56",
      "invoice_number": "001-002-789012",
      "total": 26.75,
      "products": [...],
      "issuer_name": "PURA VIDA BEACH CORP",
      "issuer_address": "MAREAS MALL",
      "date": "2025-11-01"
    }
  },
  "error": null,
  "request_id": "retry123-...",
  "timestamp": "2025-12-12T17:10:00Z"
}
```

### Respuesta - Aún Faltan Campos (200 OK)

```json
{
  "success": true,
  "data": {
    "success": false,
    "invoice_number": "001-002-789012",
    "issuer_name": "PURA VIDA BEACH CORP",
    "issuer_ruc": null,
    "issuer_dv": null,
    "date": "2025-11-01",
    "total": 26.75,
    "cost_lumis": 5,
    "message": "Aún no se pudieron detectar todos los campos requeridos. Faltan: RUC del comercio, Dígito Verificador (DV)",
    "missing_fields": [
      {"field_key": "ruc", "field_name": "RUC del comercio"},
      {"field_key": "dv", "field_name": "Dígito Verificador (DV)"}
    ],
    "extracted_data": {...}
  }
}
```

---

## 🧠 Sistema de Modelos LLM (Cascade)

Ambos endpoints usan un sistema de **cascade de 3 modelos** para máxima precisión:

| Orden | Modelo | Descripción | Costo Aproximado |
|-------|--------|-------------|------------------|
| 1️⃣ | `qwen/qwen3-vl-8b-instruct` | Rápido y económico | ~$0.0016/imagen |
| 2️⃣ | `qwen/qwen3-vl-30b-a3b-instruct` | Balance precisión/costo | ~$0.003/imagen |
| 3️⃣ | `qwen/qwen2.5-vl-72b-instruct` | Máxima precisión | ~$0.008/imagen |

### Comportamiento del Cascade

1. Se intenta primero con el modelo más rápido
2. Si falla (error de API o parsing), se intenta con el siguiente
3. Solo se usa el modelo más caro si los anteriores fallan
4. Cada intento se registra en la base de datos

---

## 📊 Logging y Trazabilidad

Cada llamada a la API de OCR se registra en la tabla `public.ocr_test_logs`:

### Campos Registrados

| Campo | Descripción |
|-------|-------------|
| `user_id` | ID del usuario que realizó la solicitud |
| `model_name` | Modelo LLM utilizado |
| `success` | Si la extracción fue exitosa |
| `response_time_ms` | Tiempo de respuesta en milisegundos |
| `tokens_prompt` | Tokens de entrada (imagen + prompt) |
| `tokens_completion` | Tokens de salida (JSON) |
| `tokens_total` | Total de tokens consumidos |
| `cost_total_usd` | Costo total de la llamada en USD |
| `extracted_fields` | Datos extraídos (JSONB) |
| `raw_response` | Respuesta completa de la API (JSONB) |
| `error_message` | Mensaje de error si falló |

### Consultas Útiles

```sql
-- Últimas 10 llamadas OCR
SELECT 
    created_at,
    user_id,
    model_name,
    success,
    response_time_ms,
    tokens_total,
    cost_total_usd
FROM public.ocr_test_logs
ORDER BY created_at DESC
LIMIT 10;

-- Costo total por día
SELECT 
    DATE(created_at) as fecha,
    COUNT(*) as llamadas,
    SUM(cost_total_usd) as costo_total_usd,
    AVG(response_time_ms) as tiempo_promedio_ms
FROM public.ocr_test_logs
GROUP BY DATE(created_at)
ORDER BY fecha DESC;

-- Tasa de éxito por modelo
SELECT 
    model_name,
    COUNT(*) as total,
    SUM(CASE WHEN success THEN 1 ELSE 0 END) as exitosos,
    ROUND(100.0 * SUM(CASE WHEN success THEN 1 ELSE 0 END) / COUNT(*), 2) as tasa_exito
FROM public.ocr_test_logs
GROUP BY model_name;
```

---

## 💡 Sugerencias y Mejores Prácticas

### Para el Frontend (Flutter)

#### 1. Guiar al usuario para mejores fotos

```dart
void showPhotoTips(BuildContext context) {
  showDialog(
    context: context,
    builder: (ctx) => AlertDialog(
      title: Text('📸 Tips para mejor lectura'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('✅ Buena iluminación, sin sombras'),
          Text('✅ Imagen nítida, sin movimiento'),
          Text('✅ Factura completa en el encuadre'),
          Text('✅ Evitar reflejos si es papel brillante'),
          Text('✅ Para retry: enfocar en datos faltantes'),
        ],
      ),
    ),
  );
}
```

#### 2. Manejo inteligente de respuestas

```dart
Future<void> handleOcrResponse(Map<String, dynamic> response) async {
  final data = response['data'];
  
  if (data['success'] == true) {
    // ✅ Factura completa - proceder a guardar
    await saveInvoice(data);
    showSuccess('¡Factura registrada exitosamente!');
  } else if (data['status'] == 'missing_fields') {
    // ⚠️ Faltan campos - mostrar datos parciales y pedir retry
    final missingFields = data['missing_fields'] as List;
    final extractedData = data['extracted_data'];
    
    showMissingFieldsDialog(
      extractedData: extractedData,
      missingFields: missingFields,
      onRetry: () => promptForRetryImage(missingFields, extractedData),
    );
  } else {
    // ❌ Error general
    showError(data['message']);
  }
}
```

#### 3. UI para mostrar campos faltantes

```dart
Widget buildMissingFieldsCard(List<dynamic> missingFields, Map<String, dynamic> extractedData) {
  return Card(
    child: Padding(
      padding: EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('📋 Datos detectados:', style: TextStyle(fontWeight: FontWeight.bold)),
          if (extractedData['issuer_name'] != null)
            Text('• Comercio: ${extractedData['issuer_name']}'),
          if (extractedData['total'] != null)
            Text('• Total: \$${extractedData['total']}'),
          if (extractedData['date'] != null)
            Text('• Fecha: ${extractedData['date']}'),
          
          SizedBox(height: 16),
          Text('❌ Campos faltantes:', style: TextStyle(fontWeight: FontWeight.bold, color: Colors.red)),
          ...missingFields.map((f) => Text('• ${f['field_name']}')),
          
          SizedBox(height: 16),
          Text('💡 Toma otra foto enfocando en:', style: TextStyle(fontStyle: FontStyle.italic)),
          if (missingFields.any((f) => f['field_key'] == 'ruc'))
            Text('   - La parte superior donde aparece el RUC'),
          if (missingFields.any((f) => f['field_key'] == 'invoice_number'))
            Text('   - El número de factura (usualmente arriba)'),
        ],
      ),
    ),
  );
}
```

#### 4. Límite de reintentos

```dart
const int MAX_RETRIES = 3;
int retryCount = 0;

Future<void> handleRetry(List<String> missingFields, Map<String, dynamic> previousData) async {
  if (retryCount >= MAX_RETRIES) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('⚠️ Máximo de intentos alcanzado'),
        content: Text(
          'No pudimos leer todos los datos de esta factura. '
          '¿Deseas ingresarlos manualmente?'
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: Text('Cancelar'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(ctx);
              openManualEntryForm(previousData);
            },
            child: Text('Ingresar manualmente'),
          ),
        ],
      ),
    );
    return;
  }
  
  retryCount++;
  // Continuar con retry...
}
```

### Para el Backend

#### 1. Monitoreo de costos

```sql
-- Alerta si el costo diario supera $10
SELECT 
    CASE 
        WHEN SUM(cost_total_usd) > 10 
        THEN 'ALERTA: Costo diario alto'
        ELSE 'OK'
    END as status,
    SUM(cost_total_usd) as costo_hoy
FROM public.ocr_test_logs
WHERE created_at >= CURRENT_DATE;
```

#### 2. Identificar imágenes problemáticas

```sql
-- Facturas que requirieron múltiples intentos
SELECT 
    user_id,
    DATE(created_at) as fecha,
    COUNT(*) as intentos,
    SUM(CASE WHEN success THEN 1 ELSE 0 END) as exitosos,
    SUM(cost_total_usd) as costo_total
FROM public.ocr_test_logs
GROUP BY user_id, DATE(created_at)
HAVING COUNT(*) > 3
ORDER BY fecha DESC;
```

---

## 🔧 Configuración Requerida

### Variables de Entorno

```bash
# OpenRouter API Key (requerido)
OPENROUTER_API_KEY="sk-or-v1-..."

# Base de datos (requerido)
DATABASE_URL="postgres://user:pass@host:5432/db"

# Opcional: Gemini como fallback (deprecated)
GEMINI_API_KEY="AIza..."
```

### Tabla de Logs (SQL)

```sql
CREATE TABLE IF NOT EXISTS public.ocr_test_logs (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES public.dim_users(id),
    image_path TEXT,
    image_size_bytes BIGINT,
    model_name VARCHAR(100) NOT NULL,
    provider VARCHAR(50) DEFAULT 'openrouter',
    success BOOLEAN NOT NULL,
    response_time_ms BIGINT,
    error_message TEXT,
    tokens_prompt INTEGER,
    tokens_completion INTEGER,
    tokens_total INTEGER,
    cost_prompt_usd NUMERIC(12,8),
    cost_completion_usd NUMERIC(12,8),
    cost_total_usd NUMERIC(12,8),
    generation_id VARCHAR(100),
    model_used VARCHAR(100),
    finish_reason VARCHAR(50),
    extracted_fields JSONB,
    raw_response JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_ocr_test_logs_user_id ON public.ocr_test_logs(user_id);
CREATE INDEX idx_ocr_test_logs_created_at ON public.ocr_test_logs(created_at DESC);
CREATE INDEX idx_ocr_test_logs_success ON public.ocr_test_logs(success);
```

---

## 📝 Códigos de Error

| Código | HTTP Status | Descripción |
|--------|-------------|-------------|
| `NO_IMAGE_FILE` | 400 | No se envió imagen |
| `NO_IMAGE_DATA` | 400 | Imagen vacía |
| `FILE_TOO_LARGE` | 413 | Imagen > 10MB |
| `INVALID_FORMAT` | 415 | Formato no soportado |
| `MISSING_FIELDS` | 200 | Campos obligatorios faltantes |
| `DUPLICATE_INVOICE` | 409 | Factura ya registrada |
| `RATE_LIMIT_EXCEEDED` | 429 | Límite de uso excedido |
| `OCR_PROCESSING_FAILED` | 500 | Error interno de OCR |
| `EMPTY_MISSING_FIELDS` | 400 | Array missing_fields vacío |
| `INVALID_FIELD_KEY` | 400 | Campo no válido en missing_fields |

---

## 🚀 Ejemplo Completo de Integración

```dart
class OcrService {
  final Dio _dio;
  
  OcrService(this._dio);
  
  /// Paso 1: Primera imagen
  Future<OcrResult> uploadInvoice(File image) async {
    final formData = FormData.fromMap({
      'image': await MultipartFile.fromFile(image.path),
      'mode': '1',
    });
    
    final response = await _dio.post('/api/v4/invoices/upload-ocr', data: formData);
    return OcrResult.fromJson(response.data['data']);
  }
  
  /// Paso 2: Retry con imagen adicional
  Future<OcrResult> retryWithNewImage(
    File newImage,
    List<String> missingFieldKeys,
    Map<String, dynamic> previousData,
  ) async {
    final formData = FormData.fromMap({
      'image': await MultipartFile.fromFile(newImage.path),
      'missing_fields': jsonEncode(missingFieldKeys),
      'previous_data': jsonEncode(previousData),
    });
    
    final response = await _dio.post('/api/v4/invoices/upload-ocr-retry', data: formData);
    return OcrResult.fromJson(response.data['data']);
  }
}

// Uso
void processInvoice() async {
  final ocrService = OcrService(dio);
  
  // Paso 1
  final result1 = await ocrService.uploadInvoice(invoiceImage);
  
  if (result1.success) {
    // ✅ Listo!
    saveInvoice(result1);
  } else if (result1.missingFields != null) {
    // ⚠️ Necesita retry
    final newImage = await takePhotoForMissingFields(result1.missingFields!);
    
    final result2 = await ocrService.retryWithNewImage(
      newImage,
      result1.missingFields!.map((f) => f.fieldKey).toList(),
      result1.extractedData!,
    );
    
    if (result2.success) {
      saveInvoice(result2);
    } else {
      // Mostrar opción de ingreso manual
    }
  }
}
```

---

## 📞 Soporte

Para problemas o sugerencias:
- Revisar logs en `public.ocr_test_logs`
- Verificar configuración de `OPENROUTER_API_KEY`
- Consultar métricas de costos y tiempos de respuesta

**Última actualización:** 12 de Diciembre de 2025
