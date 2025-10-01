# 🚀 ROBUST INVOICE PROCESSING API

## 📋 Resumen

Se ha implementado exitosamente el API robusto para procesamiento de facturas DGI Panamá según las especificaciones de la documentación. Este API reemplaza el flujo básico de WhatsApp con un sistema completo de validación, scraping, persistencia y logging.

## 🏗️ Arquitectura Implementada

### Módulos Creados

```
src/api/invoice_processor/
├── mod.rs                    # Módulo principal
├── handlers.rs               # HTTP handlers
├── models.rs                 # Estructuras (Request/Response)
├── validation.rs             # Validaciones de entrada
├── scraper_service.rs        # Integración con web scraping
├── repository.rs             # Operaciones de base de datos
├── logging_service.rs        # Gestión de logs en bot_rust_scrapy
└── error_handling.rs         # Manejo centralizado de errores
```

### Endpoint Principal

```
POST /api/invoices/process
```

**Request Body:**
```json
{
  "url": "https://dgi-fep.mef.gob.pa/...",
  "user_id": "user123",
  "user_email": "user@example.com",
  "origin": "whatsapp"
}
```

### Endpoints Adicionales

- `GET /api/invoices/health` - Health check del servicio
- `GET /api/invoices/stats/user/:user_id` - Estadísticas por usuario
- `GET /api/invoices/stats/system` - Estadísticas del sistema

## 🔧 Configuración de Base de Datos

### 1. Ejecutar Script SQL

```bash
psql -U tu_usuario -d tu_base_de_datos -f invoice_processing_schema.sql
```

### 2. Verificar Tablas

El script creará:
- `logs.bot_rust_scrapy` - Nueva tabla de logging
- Índices optimizados para consultas frecuentes
- Función de limpieza automática

### 3. Tablas Existentes Requeridas

Asegúrate de que existan estas tablas:
- `public.invoice_header`
- `public.invoice_detail` 
- `public.invoice_payment`

## ⚡ Características Implementadas

### ✅ Validación Robusta
- Validación de URL DGI
- Validación de email
- Validación de origen (whatsapp, aplicacion, telegram)

### ✅ Web Scraping con Reintentos
- Timeout de 30 segundos
- Máximo 2 reintentos con backoff exponencial
- Integración con módulo `webscraping` existente

### ✅ Idempotencia
- Verificación de duplicados por CUFE
- Respuesta 409 para facturas existentes

### ✅ Transacciones Atómicas
- TODO o NADA en persistencia
- Rollback automático en caso de error

### ✅ Logging Completo
- Tabla `logs.bot_rust_scrapy` con métricas detalladas
- Trazabilidad completa de operaciones
- Categorización de errores

### ✅ Manejo de Errores
- Responses HTTP estructuradas
- Logging automático de errores
- Categorización granular

## 📊 Respuestas del API

### ✅ 200 - Éxito
```json
{
  "status": "success",
  "message": "Su factura de DELIVERY HERO PANAMA S.A. por valor de $2.68 fue procesada exitosamente",
  "data": {
    "cufe": "FE012000...",
    "invoice_number": "0031157014",
    "issuer_name": "DELIVERY HERO PANAMA S.A.",
    "tot_amount": "2.68",
    "items_count": 2
  }
}
```

### ⚠️ 409 - Duplicado
```json
{
  "status": "duplicate",
  "message": "Esta factura ya fue procesada anteriormente",
  "data": {
    "cufe": "FE012000...",
    "processed_date": "2025-09-07T10:30:00-05:00",
    "original_user": "user456"
  }
}
```

### ❌ 400 - Error de Validación
```json
{
  "status": "validation_error",
  "message": "Datos de entrada inválidos",
  "errors": [
    "URL no corresponde a DGI Panamá",
    "Email inválido"
  ]
}
```

### ❌ 500 - Error de Procesamiento
```json
{
  "status": "processing_error",
  "message": "Su factura no pudo ser procesada",
  "error": {
    "type": "CUFE_NOT_FOUND",
    "details": "No se pudo extraer el campo CUFE del HTML",
    "retry_attempts": 2
  }
}
```

## 🔄 Flujo de Procesamiento

1. **Validación de Entrada** → 400 si falla
2. **Logging Inicial** → Crea registro en `logs.bot_rust_scrapy`
3. **Web Scraping** → Con reintentos y timeout
4. **Verificación de Duplicados** → 409 si existe
5. **Transacción Atómica** → Guarda en todas las tablas
6. **Logging Final** → Actualiza con métricas y resultado

## 🏃‍♂️ Ejecución

### Desarrollo
```bash
cargo run
```

### Testing
```bash
# Health check
curl http://localhost:3000/api/invoices/health

# Procesar factura
curl -X POST http://localhost:3000/api/invoices/process \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://dgi-fep.mef.gob.pa/FacturasPorQR?chFE=...",
    "user_id": "test_user",
    "user_email": "test@example.com",
    "origin": "whatsapp"
  }'

# Estadísticas de usuario
curl http://localhost:3000/api/invoices/stats/user/test_user

# Estadísticas del sistema
curl http://localhost:3000/api/invoices/stats/system
```

## 📈 Monitoreo y Métricas

### Consultas Útiles

**Rendimiento del sistema (últimas 24h):**
```sql
SELECT 
    COUNT(*) as total_requests,
    COUNT(CASE WHEN status = 'SUCCESS' THEN 1 END) as successful_requests,
    AVG(execution_time_ms) as avg_execution_time_ms
FROM logs.bot_rust_scrapy 
WHERE request_timestamp >= NOW() - INTERVAL '24 hours';
```

**Top usuarios (último mes):**
```sql
SELECT 
    user_id,
    COUNT(*) as total_requests,
    COUNT(CASE WHEN status = 'SUCCESS' THEN 1 END) as successful_requests
FROM logs.bot_rust_scrapy 
WHERE request_timestamp >= NOW() - INTERVAL '30 days'
GROUP BY user_id
ORDER BY total_requests DESC
LIMIT 10;
```

**Errores frecuentes:**
```sql
SELECT 
    error_type,
    COUNT(*) as count,
    AVG(retry_attempts) as avg_retries
FROM logs.bot_rust_scrapy 
WHERE status NOT IN ('SUCCESS', 'DUPLICATE')
AND request_timestamp >= NOW() - INTERVAL '7 days'
GROUP BY error_type
ORDER BY count DESC;
```

## 🔒 Seguridad y Mantenimiento

### Limpieza Automática
La función `cleanup_old_bot_logs()` puede programarse para ejecutarse semanalmente:

```sql
-- Con pg_cron (si está disponible)
SELECT cron.schedule('cleanup-bot-logs', '0 2 * * 0', 'SELECT cleanup_old_bot_logs();');
```

### Backups
```sql
-- Backup de logs importantes
pg_dump -t logs.bot_rust_scrapy tu_base_de_datos > backup_logs.sql
```

## 📚 Próximos Pasos

1. **Integración con WhatsApp**: Modificar el bot para usar este endpoint
2. **Rate Limiting**: Implementar límites por usuario/IP
3. **Caching**: Cache de resultados para URLs frecuentes
4. **Alertas**: Monitoreo automático de errores
5. **Analytics**: Dashboard de métricas en tiempo real

## 🤝 Contribución

El código está modularizado y documentado para facilitar mantenimiento y extensión. Cada módulo tiene responsabilidades claras y está bien probado.

---

**✨ El API está listo para producción y sigue todas las mejores prácticas de la documentación.**
