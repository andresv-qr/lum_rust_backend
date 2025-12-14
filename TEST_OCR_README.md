# Test OCR - Sistema de Pruebas con Logging Completo

## Descripción
Binario de prueba para OCR que usa modelos de OpenRouter en cascada con logging completo a PostgreSQL para trazabilidad y control de costos.

## Modelos Utilizados (en cascada)
1. **qwen/qwen3-vl-8b-instruct** (Primario - más rápido y económico)
2. **qwen/qwen3-vl-30b-a3b-instruct** (Fallback 1 - mejor precisión)
3. **qwen/qwen2.5-vl-72b-instruct** (Fallback 2 - máxima precisión)

El sistema intenta con el primer modelo. Si falla, automáticamente prueba con el siguiente.

## Uso

```bash
# Compilar
cargo build --release --bin test_ocr

# Ejecutar
./target/release/test_ocr <ruta_a_imagen>

# Ejemplo
./target/release/test_ocr image_invoice.jpg
```

## Información Almacenada en PostgreSQL

### Tabla: `public.ocr_test_logs`

Cada ejecución almacena:

**Información de la Solicitud:**
- `user_id` - ID del usuario (NULL para tests)
- `image_path` - Ruta de la imagen procesada
- `image_size_bytes` - Tamaño de la imagen en bytes

**Información del Modelo:**
- `model_name` - Modelo solicitado (ej: qwen/qwen3-vl-8b-instruct)
- `model_used` - Modelo que realmente procesó (puede diferir del solicitado)
- `provider` - Proveedor del API (openrouter)
- `generation_id` - ID único de la generación

**Resultados de Ejecución:**
- `success` - Si la extracción fue exitosa
- `response_time_ms` - Tiempo de respuesta en milisegundos
- `finish_reason` - Razón de finalización (stop, length, etc)
- `error_message` - Mensaje de error si falló

**Uso de Tokens:**
- `tokens_prompt` - Tokens de entrada/prompt
- `tokens_completion` - Tokens de salida/completación
- `tokens_total` - Tokens totales usados

**Información de Costos:**
- `cost_prompt_usd` - Costo en USD por tokens de entrada
- `cost_completion_usd` - Costo en USD por tokens de salida
- `cost_total_usd` - Costo total en USD

**Datos Extraídos:**
- `extracted_fields` - JSON con todos los campos extraídos de la factura
- `raw_response` - Respuesta completa del API para debugging

**Timestamp:**
- `created_at` - Fecha y hora de creación del registro

## Consultas Útiles

### Ver últimos tests ejecutados
```sql
SELECT 
    id,
    model_name,
    success,
    tokens_total,
    cost_total_usd,
    response_time_ms,
    created_at
FROM public.ocr_test_logs 
ORDER BY created_at DESC 
LIMIT 10;
```

### Resumen diario por modelo
```sql
SELECT * FROM public.ocr_test_logs_summary 
ORDER BY date DESC, model_name;
```

### Datos extraídos del último test
```sql
SELECT 
    extracted_fields->>'issuer_name' as issuer,
    extracted_fields->>'ruc' as ruc,
    extracted_fields->>'invoice_number' as invoice,
    extracted_fields->>'total' as total,
    extracted_fields->'products' as products
FROM public.ocr_test_logs 
ORDER BY created_at DESC 
LIMIT 1;
```

### Análisis de costos por modelo
```sql
SELECT 
    model_name,
    COUNT(*) as total_requests,
    SUM(tokens_total) as total_tokens,
    ROUND(SUM(cost_total_usd)::numeric, 8) as total_cost_usd,
    ROUND(AVG(cost_total_usd)::numeric, 8) as avg_cost_per_request,
    ROUND(AVG(response_time_ms), 2) as avg_response_time_ms
FROM public.ocr_test_logs
WHERE success = true
GROUP BY model_name
ORDER BY total_cost_usd DESC;
```

### Ver errores
```sql
SELECT 
    model_name,
    error_message,
    response_time_ms,
    created_at
FROM public.ocr_test_logs
WHERE success = false
ORDER BY created_at DESC;
```

## Ejemplo de Output

```
🔍 Testing OCR with OpenRouter Models in Cascade
📄 Image: image_invoice.jpg

✅ Image loaded: 3422347 bytes
✅ Connected to database for logging

================================================================================
TEST 1: QWEN3-VL-8B (Primary) - qwen/qwen3-vl-8b-instruct
================================================================================

📤 Calling OpenRouter API with model: qwen/qwen3-vl-8b-instruct
📥 Response status: 200 OK
💾 Raw response saved to /tmp/ocr_qwen_qwen3-vl-8b-instruct_response.json
🎫 Tokens: 12773 total (prompt: 12610, completion: 163)
💰 Cost: $0.00238227 USD (prompt: $0.00226980, completion: $0.00011247)

✅ OCR SUCCESSFUL!

📋 EXTRACTED DATA:
================================================================================

📤 EMISOR:
  - Nombre: INVERZUES CORPORATION PANAMA S.A.
  - RUC: 15575193822024
  - DV: 66
  - Dirección: BRISAS DEL GOLF CALLE 3RA OESTE CASA 86, PANAMA

📄 FACTURA:
  - No. Factura: 10374
  - Fecha: 2025-02-22
  - Total: $1.30

📦 PRODUCTOS (1 items):
  📌 Item #1
    - Nombre: MR BONO VTA
    - Cantidad: 1.00
    - Precio Unit: $1.30
    - Total: $1.30

🔍 VALIDATION:
✅ All required fields extracted successfully!

================================================================================
FINAL RESULT
================================================================================
✅ OCR completed successfully!

📊 Summary:
  - Issuer: INVERZUES CORPORATION PANAMA S.A.
  - Invoice: 10374
  - Total: $1.30
  - Products: 1 items

💾 All attempts have been logged to PostgreSQL table: public.ocr_test_logs
```

## Ventajas del Sistema

1. **Trazabilidad Completa**: Cada intento de OCR queda registrado
2. **Control de Costos**: Tracking detallado de uso de tokens y costos en USD
3. **Performance Monitoring**: Tiempos de respuesta y tasas de éxito
4. **Debugging**: Raw response completo disponible para análisis
5. **Cascada Inteligente**: Fallback automático a modelos más potentes
6. **Analytics**: Vista pre-construida para análisis diarios

## Configuración

El sistema usa la API key de OpenRouter configurada en `.env`:
```
OPENROUTER_API_KEY="sk-or-v1-..."
```

Si no está en `.env`, usa la key hardcodeada en el código como fallback.
