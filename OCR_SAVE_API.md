# API Guardado de Facturas OCR - Datos Completos

## Descripción General
API para guardar facturas procesadas con OCR una vez que todos los campos requeridos han sido detectados y validados.

## Endpoint Principal

### POST `/api/v4/invoices/save-ocr`

Guarda los datos de factura en la base de datos siguiendo la misma lógica de `factura_sin_qr`.

## Request

### Headers
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

### Body
```json
{
  "session_id": "ocr_sess_123",
  "invoice_data": {
    "issuer_name": "Supermercado La Pradera",
    "invoice_number": "F001-000123", 
    "date": "2025-09-05",
    "total": 45.67,
    "products": [
      {
        "name": "Arroz Diana",
        "quantity": 2,
        "unit_price": 12.50,
        "total_price": 25.00
      },
      {
        "name": "Aceite Mazeite",
        "quantity": 1,
        "unit_price": 20.67,
        "total_price": 20.67
      }
    ]
  },
  "consolidated_image": "base64_image_string",
  "validation_status": "complete" | "manual_review"
}
```

## Response

### Éxito
```json
{
  "success": true,
  "invoice_id": 12345,
  "cufe": "OCR-1234-F001000123-a1b2c3d4",
  "status": "pending_validation",
  "message": "Factura guardada exitosamente",
  "rewards": {
    "lumis_earned": 0,
    "xp_earned": 0,
    "note": "Los Lümis se otorgarán después de la validación"
  },
  "next_steps": [
    "La factura será validada por nuestro equipo en 24-48 horas",
    "Recibirás una notificación cuando esté certificada",
    "Los Lümis se acreditarán automáticamente"
  ]
}
```

### Error
```json
{
  "success": false,
  "error_code": "DUPLICATE_INVOICE",
  "message": "Esta factura ya fue registrada previamente",
  "details": {
    "existing_cufe": "OCR-1234-F001000123-xyz789",
    "registration_date": "2025-09-01T10:30:00Z"
  }
}
```

## Validaciones Previas al Guardado

### 1. Validación de Sesión
- Verificar que `session_id` existe y está activo
- Confirmar que pertenece al usuario autenticado
- Validar que no haya expirado (TTL: 30 minutos)

### 2. Validación de Campos Requeridos
Usando la misma lógica de `validate_required_fields()`:
```rust
fn validate_invoice_data(invoice_data: &InvoiceData) -> Result<()> {
    if invoice_data.issuer_name.trim().is_empty() {
        return Err(anyhow!("Nombre del comercio requerido"));
    }
    if invoice_data.invoice_number.trim().is_empty() {
        return Err(anyhow!("Número de factura requerido"));
    }
    if invoice_data.date.trim().is_empty() {
        return Err(anyhow!("Fecha requerida"));
    }
    if invoice_data.total <= 0.0 {
        return Err(anyhow!("Total debe ser mayor a 0"));
    }
    if invoice_data.products.is_empty() {
        return Err(anyhow!("Debe incluir al menos un producto"));
    }
    Ok(())
}
```

### 3. Validación de Duplicados
```sql
SELECT COUNT(*) FROM public.invoice_header 
WHERE invoice_number = $1 
AND issuer_name ILIKE $2 
AND total = $3
AND user_id = $4;
```

### 4. Validación de Formato
- **Fecha**: Formato válido YYYY-MM-DD, no futura
- **Total**: Número positivo, máximo 2 decimales
- **Productos**: Suma de totales = total de factura
- **Números**: Solo caracteres alfanuméricos en invoice_number

## Proceso de Guardado

### 1. Generar CUFE Temporal
```rust
async fn generate_ocr_cufe(invoice_data: &InvoiceData, user_id: i64) -> Result<String> {
    let uuid = Uuid::new_v4();
    let cufe = format!(
        "OCR-{}-{}-{}",
        user_id,
        invoice_data.invoice_number.replace([' ', '-'], ""),
        &uuid.to_string()[..8]
    );
    Ok(cufe)
}
```

### 2. Guardar en Base de Datos
```sql
-- Insertar en invoice_header
INSERT INTO public.invoice_header (
    user_id, cufe, issuer_name, invoice_number, 
    invoice_date, total_amount, status, 
    processing_method, created_at
) VALUES (
    $1, $2, $3, $4, $5, $6, 
    'pending_validation', 'ocr', NOW()
);

-- Insertar productos en invoice_details
INSERT INTO public.invoice_details (
    invoice_header_id, product_name, quantity, 
    unit_price, total_price, line_number
) VALUES ($1, $2, $3, $4, $5, $6);
```

### 3. Guardar Imagen Consolidada
```sql
INSERT INTO public.invoice_images (
    invoice_header_id, image_data, image_type, 
    is_consolidated, created_at
) VALUES ($1, $2, 'ocr_consolidated', true, NOW());
```

### 4. Registrar Proceso OCR
```sql
INSERT INTO analytics.ocr_processing_log (
    user_id, session_id, invoice_header_id,
    total_attempts, total_tokens, processing_time,
    final_status, created_at
) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW());
```

## Estados de Factura

| Estado | Descripción | Siguiente Paso |
|--------|-------------|----------------|
| `pending_validation` | Esperando validación humana | Revisión en 24-48h |
| `manual_review` | Requiere revisión manual detallada | Contacto del equipo |
| `validated` | Aprobada por el equipo | Lümis otorgados |
| `rejected` | Rechazada por irregularidades | Notificación al usuario |

## Notificaciones

### WhatsApp (Inmediata)
```
✅ **Factura guardada exitosamente**

📋 **Resumen:**
🏪 Comercio: {issuer_name}
📄 Número: {invoice_number}
📅 Fecha: {date}
💰 Total: ${total}
📦 Productos: {product_count} artículos

⏳ **Estado:** Pendiente de validación
👥 **Proceso:** Nuestro equipo revisará en 24-48 horas
📱 **Notificación:** Te avisaremos cuando esté certificada

🔗 **CUFE:** `{cufe}`
```

### Email (24-48h después)
Notificación del resultado de validación.

## Funciones Abstraídas

### Core OCR Logic Abstraction
```rust
// Función abstraída de save_invoice_to_database
pub async fn save_processed_invoice(
    pool: &sqlx::PgPool,
    user_id: i64,
    invoice_data: InvoiceData,
    cufe: String,
    image_data: Option<&[u8]>,
    processing_method: ProcessingMethod
) -> Result<i64> {
    // Lógica compartida entre factura_sin_qr y nueva API
}

pub enum ProcessingMethod {
    QrCode,
    OcrSingle,
    OcrIterative,
    ManualEntry
}
```

### Data Transformation
```rust
pub fn transform_ocr_to_db_format(
    ocr_data: OcrResponse,
    user_id: i64
) -> Result<DatabaseInvoice> {
    // Transformar datos de OCR a formato de BD
}
```

## Códigos de Error

| Código | Error | Descripción |
|--------|-------|-------------|
| 400 | INVALID_DATA | Datos de factura inválidos |
| 401 | UNAUTHORIZED | Token JWT inválido |
| 404 | SESSION_NOT_FOUND | ID de sesión no existe |
| 409 | DUPLICATE_INVOICE | Factura ya registrada |
| 422 | VALIDATION_ERROR | Error en validación de campos |
| 500 | DATABASE_ERROR | Error interno de base de datos |

## Rate Limiting

- Máximo 10 guardados por hora por usuario
- Cooldown de 30 segundos entre guardados

## Logging y Auditoría

### Analytics Tracking
```sql
-- Métricas de éxito por método
INSERT INTO analytics.invoice_processing_metrics (
    user_id, processing_method, success, 
    processing_time, total_amount, created_at
) VALUES ($1, 'ocr_iterative', $2, $3, $4, NOW());
```

### Audit Log
```sql
INSERT INTO audit.invoice_operations (
    user_id, operation_type, invoice_id, 
    details, ip_address, created_at
) VALUES ($1, 'create_ocr', $2, $3, $4, NOW());
```

## Integración con Gamificación

Una vez validada la factura:
```rust
// Trigger automático después de validación
pub async fn process_validated_ocr_invoice(invoice_id: i64) -> Result<()> {
    // 1. Otorgar Lümis según el total
    // 2. Actualizar XP del usuario  
    // 3. Verificar logros/badges
    // 4. Actualizar streaks
    // 5. Enviar notificación de recompensas
}
```

## Ejemplos de Uso

### Cliente JavaScript
```javascript
const saveInvoice = async (sessionId, invoiceData, consolidatedImage) => {
  const response = await fetch('/api/v4/invoices/save-ocr', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      session_id: sessionId,
      invoice_data: invoiceData,
      consolidated_image: consolidatedImage,
      validation_status: 'complete'
    })
  });
  
  const result = await response.json();
  if (result.success) {
    showSuccessMessage(result.cufe);
  }
};
```

### Cliente Flutter
```dart
Future<SaveResponse> saveOcrInvoice(
  String sessionId,
  InvoiceData invoiceData,
  String consolidatedImage
) async {
  final response = await http.post(
    Uri.parse('$baseUrl/api/v4/invoices/save-ocr'),
    headers: {
      'Authorization': 'Bearer $token',
      'Content-Type': 'application/json',
    },
    body: jsonEncode({
      'session_id': sessionId,
      'invoice_data': invoiceData.toJson(),
      'consolidated_image': consolidatedImage,
      'validation_status': 'complete',
    }),
  );
  
  return SaveResponse.fromJson(jsonDecode(response.body));
}
```
