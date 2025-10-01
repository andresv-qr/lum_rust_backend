# User Issuers API v4 - Documentación

## 🎯 Descripción General

Esta API permite obtener una lista paginada de todos los **emisores** (companies/merchants) que tienen facturas asociadas con un usuario específico. Es útil para mostrar al usuario todas las empresas con las que ha realizado transacciones.

## 📊 Consulta SQL Implementada

La API implementa la siguiente consulta SQL optimizada:

```sql
SELECT DISTINCT 
    a.issuer_ruc,
    a.issuer_name,
    a.issuer_best_name,
    a.issuer_l1,
    a.issuer_l2,
    a.issuer_l3,
    a.issuer_l4,
    a.update_date
FROM public.dim_issuer a 
WHERE EXISTS (
    SELECT 1 
    FROM public.invoice_header ih 
    WHERE ih.user_id = $1 
    AND a.issuer_ruc = ih.issuer_ruc 
    AND a.issuer_name = ih.issuer_name
)
ORDER BY a.issuer_name ASC
LIMIT $2 OFFSET $3
```

## 🔒 Autenticación

- **Tipo:** JWT Bearer Token
- **Header:** `Authorization: Bearer <jwt_token>`
- **Extracción del user_id:** Automática desde el token JWT

## 📡 Endpoint

```http
GET /api/v4/invoices/issuers
```

## 📥 Query Parameters

| Parameter | Type | Required | Default | Max | Description |
|-----------|------|----------|---------|-----|-------------|
| `limit` | integer | No | 20 | 100 | Número máximo de resultados por página |
| `offset` | integer | No | 0 | - | Número de resultados a omitir (para paginación) |
| `update_date_from` | string | No | - | - | Filtrar emisores con update_date >= esta fecha (formato ISO 8601) |

## 📤 Estructura de Respuesta

### Success Response (200)

```json
{
  "success": true,
  "data": {
    "issuers": [
      {
        "issuer_ruc": "155112341-2-DV",
        "issuer_name": "Super 99",
        "issuer_best_name": "Super99 Panamá",
        "issuer_l1": "Retail",
        "issuer_l2": "Supermercados",
        "issuer_l3": "Alimentación",
        "issuer_l4": "General",
        "update_date": "2024-08-10T14:30:00Z"
      }
    ],
    "pagination": {
      "total": 15,
      "limit": 20,
      "offset": 0,
      "has_next": false,
      "has_previous": false,
      "total_pages": 1,
      "current_page": 1
    }
  },
  "error": null,
  "request_id": "user-issuers-12345",
  "timestamp": "2025-09-13T15:30:00Z",
  "execution_time_ms": 45,
  "cached": false
}
```

### Error Responses

#### 400 Bad Request (Invalid Date Format)
```json
{
  "error": "BAD_REQUEST",
  "message": "Invalid date format"
}
```

#### 401 Unauthorized
```json
{
  "error": "AUTH_REQUIRED",
  "message": "Authentication required"
}
```

#### 500 Internal Server Error
```json
{
  "error": "INTERNAL_SERVER_ERROR",
  "message": "Database query failed"
}
```

## 🏗️ Arquitectura y Buenas Prácticas v4

### ✅ Implementadas

- **JWT Authentication:** Extracción automática del user_id desde el token
- **Paginación Estándar:** Límites de seguridad (max 100 per página)
- **Structured Logging:** Request ID único para tracing
- **Performance Monitoring:** Headers con tiempo de ejecución
- **ApiResponse Standard:** Formato consistente v4
- **Error Handling:** Códigos de error estandarizados (400 para fechas inválidas)
- **SQL Optimization:** DISTINCT + EXISTS para eficiencia
- **Security:** Validación automática de permisos por JWT
- **Date Filtering:** Filtro opcional por update_date con validación ISO 8601

### 🔄 Próximas Mejoras

- **Caching:** Redis cache con TTL de 10 minutos
- **Filtros:** Búsqueda por nombre de emisor
- **Sorting:** Múltiples opciones de ordenamiento
- **Rate Limiting:** Control granular por endpoint

## 🧪 Testing

### Script de Prueba

```bash
# Hacer ejecutable
chmod +x test_user_issuers_api.sh

# Ejecutar tests
JWT_TOKEN='your_jwt_token_here' ./test_user_issuers_api.sh
```

### Ejemplo de Request con curl

```bash
# Sin filtro de fecha
curl -X GET "http://localhost:8000/api/v4/invoices/issuers?limit=10&offset=0" \
  -H "Authorization: Bearer your_jwt_token_here" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-1"

# Con filtro de fecha (emisores actualizados desde 2024)
curl -X GET "http://localhost:8000/api/v4/invoices/issuers?limit=10&offset=0&update_date_from=2024-01-01T00:00:00Z" \
  -H "Authorization: Bearer your_jwt_token_here" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-2"

# Con filtro de fecha (últimos 30 días)
THIRTY_DAYS_AGO=$(date -d '30 days ago' --iso-8601=seconds)
curl -X GET "http://localhost:8000/api/v4/invoices/issuers?limit=10&offset=0&update_date_from=$THIRTY_DAYS_AGO" \
  -H "Authorization: Bearer your_jwt_token_here" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-3"
```

## 📊 Performance

- **Tiempo típico:** < 50ms para datasets normales
- **Optimización:** Índices en `public.invoice_header(user_id, issuer_ruc, issuer_name)`
- **Escalabilidad:** Soporta usuarios con miles de facturas
- **Memory:** Paginación previene carga excesiva de memoria

## 🔗 Endpoints Relacionados

- `GET /api/v4/invoices/headers` - Headers de facturas del usuario
- `GET /api/v4/invoices/details` - Detalles de facturas específicas
- `GET /api/v4/userdata` - Datos demográficos del usuario

## 📋 Casos de Uso

1. **Dashboard del Usuario:** Mostrar lista de empresas frecuentadas
2. **Análisis de Gastos:** Categorizar gastos por emisor
3. **Filtros de Facturas:** Permitir filtrado por empresa específica
4. **Reporting:** Generar reportes por merchant/sector
5. **Loyalty Programs:** Identificar empresas para programas de fidelidad

## 🛠️ Implementación Técnica

### Archivos Creados/Modificados

1. `/src/api/templates/user_issuers_templates.rs` - Templates y tipos
2. `/src/api/user_issuers_v4.rs` - Handler principal
3. `/src/api/mod.rs` - Registro del módulo y router
4. `/src/api/root_v4.rs` - Documentación del endpoint
5. `/API_ENDPOINTS.md` - Documentación completa
6. `/test_user_issuers_api.sh` - Script de testing

### Dependencias

- `axum` - Web framework
- `sqlx` - Database queries
- `serde` - JSON serialization
- `tracing` - Structured logging
- `uuid` - Request ID generation

## 📈 Métricas y Monitoreo

- **Logs:** Cada request incluye user_id, request_id, execution_time
- **Headers:** `X-Response-Time-Ms` en cada respuesta
- **Error Tracking:** Logs detallados para debugging
- **Usage Analytics:** Métricas de uso disponibles en logs

---

**Última actualización:** September 13, 2025  
**Versión API:** v4  
**Estado:** ✅ Implementado y funcional
