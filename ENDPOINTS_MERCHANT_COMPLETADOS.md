# ✅ Merchant Endpoints - Implementación Completa

## 🎯 Resumen Ejecutivo

**Status**: ✅ **COMPLETADO y COMPILANDO**
- **4 endpoints** de comercio implementados
- **0 errores** de compilación
- **0 warnings**
- Listos para testing

---

## 📋 Endpoints Implementados

### 1️⃣ POST /api/v1/merchant/auth/login
**Autenticación de comercios usando API Key**

#### Request:
```json
{
  "merchant_name": "Restaurante El Buen Sabor",
  "api_key": "your-secret-api-key"
}
```

#### Response (200 OK):
```json
{
  "success": true,
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "merchant": {
    "merchant_id": "550e8400-e29b-41d4-a716-446655440000",
    "merchant_name": "Restaurante El Buen Sabor",
    "expires_in": 28800
  }
}
```

#### Detalles:
- ✅ Verifica API key con bcrypt
- ✅ Valida que el comercio esté activo
- ✅ Genera JWT con 8 horas de duración
- ✅ Incluye merchant_id en claims para auditoría

#### Errores:
- `401 Unauthorized`: Credenciales inválidas o comercio inactivo
- `500 Internal Server Error`: Error de base de datos

---

### 2️⃣ POST /api/v1/merchant/validate
**Validar código QR de redención** 🔐 Requiere JWT

#### Request:
```json
{
  "code": "QR-2024-ABC123"
}
```

O usando UUID:
```json
{
  "code": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Response - Código Válido (200 OK):
```json
{
  "success": true,
  "valid": true,
  "redemption": {
    "redemption_id": "550e8400-e29b-41d4-a716-446655440000",
    "redemption_code": "QR-2024-ABC123",
    "offer_name": "Café Gratis",
    "lumis_spent": 50,
    "status": "pending",
    "created_at": "2024-03-20T10:30:00Z",
    "expires_at": "2024-03-27T10:30:00Z",
    "can_confirm": true
  },
  "message": "Código válido. Puedes confirmar la redención."
}
```

#### Response - Código Inválido (200 OK):
```json
{
  "success": true,
  "valid": false,
  "redemption": null,
  "message": "Código no encontrado"
}
```

#### Response - Código Expirado (200 OK):
```json
{
  "success": true,
  "valid": false,
  "redemption": {
    "redemption_id": "...",
    "status": "expired",
    "can_confirm": false
  },
  "message": "Código expirado"
}
```

#### Detalles:
- ✅ Acepta código QR o UUID
- ✅ Verifica existencia y validez
- ✅ Chequea que no esté expirado
- ✅ Chequea que esté en estado "pending"
- ✅ Responde con información completa de la redención
- ✅ Indica si se puede confirmar (`can_confirm`)

#### Estados Posibles:
- `pending`: Puede confirmarse ✅
- `confirmed`: Ya fue utilizado ❌
- `cancelled`: Fue cancelado por el usuario ❌
- `expired`: Venció el tiempo límite ❌

---

### 3️⃣ POST /api/v1/merchant/confirm/:redemption_id
**Confirmar uso de redención** 🔐 Requiere JWT

#### Request:
```bash
POST /api/v1/merchant/confirm/550e8400-e29b-41d4-a716-446655440000
Authorization: Bearer <merchant_jwt>
```

#### Response (200 OK):
```json
{
  "success": true,
  "message": "Redención confirmada exitosamente",
  "redemption_id": "550e8400-e29b-41d4-a716-446655440000",
  "confirmed_at": "2024-03-20T15:45:30Z"
}
```

#### Detalles:
- ✅ Valida que la redención exista
- ✅ Verifica que esté en estado "pending"
- ✅ Actualiza estado a "confirmed"
- ✅ Registra merchant_id que validó
- ✅ Registra timestamp de validación
- ✅ Usa `SELECT ... FOR UPDATE` para evitar race conditions
- ✅ Transacción atómica

#### Errores:
- `404 Not Found`: Redención no existe
- `400 Bad Request`: Redención ya confirmada/cancelada/expirada
- `500 Internal Server Error`: Error de base de datos

---

### 4️⃣ GET /api/v1/merchant/stats
**Estadísticas del comercio** 🔐 Requiere JWT

#### Request:
```bash
GET /api/v1/merchant/stats
Authorization: Bearer <merchant_jwt>
```

#### Response (200 OK):
```json
{
  "success": true,
  "stats": {
    "total_redemptions": 458,
    "pending_redemptions": 23,
    "confirmed_redemptions": 425,
    "today_redemptions": 12,
    "this_week_redemptions": 89,
    "this_month_redemptions": 356,
    "total_lumis_redeemed": 22500,
    "recent_redemptions": [
      {
        "redemption_id": "550e8400-...",
        "redemption_code": "QR-2024-ABC123",
        "offer_name": "Café Gratis",
        "lumis_spent": 50,
        "status": "confirmed",
        "created_at": "2024-03-20T10:30:00Z",
        "validated_at": "2024-03-20T10:35:00Z"
      }
      // ... hasta 10 redenciones recientes
    ]
  }
}
```

#### Detalles:
- ✅ Agregados totales
- ✅ Filtros por estado (pending, confirmed)
- ✅ Filtros temporales:
  - Hoy (`DATE(created_at) = CURRENT_DATE`)
  - Esta semana (últimos 7 días)
  - Este mes (`DATE_TRUNC('month', ...)`)
- ✅ Total de Lümis canjeados
- ✅ Últimas 10 redenciones con detalles

---

## 🔐 Autenticación

Todos los endpoints (excepto `/auth/login`) requieren JWT en el header:

```
Authorization: Bearer <token>
```

### Estructura del JWT:
```json
{
  "sub": "merchant_id",
  "merchant_name": "Restaurante El Buen Sabor",
  "exp": 1711810800,
  "iat": 1711782000
}
```

- **Duración**: 8 horas (28800 segundos)
- **Algoritmo**: HS256
- **Secret**: Configurado en `JWT_SECRET` env variable

---

## 🧪 Testing con curl

### 1. Login
```bash
curl -X POST http://localhost:8003/api/v1/merchant/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "merchant_name": "Restaurante El Buen Sabor",
    "api_key": "your-api-key"
  }'
```

### 2. Guardar Token
```bash
TOKEN="<token_from_login_response>"
```

### 3. Validar Código
```bash
curl -X POST http://localhost:8003/api/v1/merchant/validate \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "code": "QR-2024-ABC123"
  }'
```

### 4. Confirmar Redención
```bash
curl -X POST http://localhost:8003/api/v1/merchant/confirm/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer $TOKEN"
```

### 5. Ver Estadísticas
```bash
curl http://localhost:8003/api/v1/merchant/stats \
  -H "Authorization: Bearer $TOKEN"
```

---

## 📊 Flujo Completo de Validación

```
┌─────────────┐
│   Merchant  │
│   Terminal  │
└──────┬──────┘
       │
       │ 1. Login con API Key
       ├──────────────────────────────┐
       │                              │
       │                        ┌─────▼─────┐
       │                        │   JWT     │
       │                        │  (8 hrs)  │
       │                        └─────┬─────┘
       │                              │
       │ 2. Escanear QR del cliente   │
       │ ◄────────────────────────────┘
       │
       │ 3. POST /validate con código
       ├──────────────────────────────┐
       │                              │
       │                        ┌─────▼─────┐
       │                        │  Validar  │
       │                        │  Código   │
       │                        └─────┬─────┘
       │                              │
       │ ◄────────────────────────────┘
       │ Response: valid=true, can_confirm=true
       │
       │ 4. POST /confirm/:id
       ├──────────────────────────────┐
       │                              │
       │                        ┌─────▼─────┐
       │                        │ Confirmar │
       │                        │   Uso     │
       │                        └─────┬─────┘
       │                              │
       │ ◄────────────────────────────┘
       │ Response: success=true
       │
       │ 5. Entregar producto/servicio
       ▼
```

---

## 🗄️ Base de Datos

### Tablas Principales:

#### `rewards.merchants`
```sql
- merchant_id (UUID, PK)
- merchant_name (VARCHAR, UNIQUE)
- api_key_hash (VARCHAR) -- bcrypt hash
- is_active (BOOLEAN)
- total_redemptions (INTEGER)
- total_lumis_redeemed (BIGINT)
```

#### `rewards.user_redemptions`
```sql
- redemption_id (UUID, PK)
- user_id (INTEGER)
- offer_id (UUID, FK)
- redemption_code (VARCHAR, UNIQUE)
- redemption_status (VARCHAR) -- pending|confirmed|cancelled|expired
- lumis_spent (INTEGER)
- validated_by_merchant_id (UUID)
- validated_at (TIMESTAMPTZ)
- code_expires_at (TIMESTAMPTZ)
- created_at (TIMESTAMPTZ)
```

### Queries Optimizadas:

#### Validate (con índice en redemption_code):
```sql
SELECT ur.*, ro.name_friendly
FROM rewards.user_redemptions ur
INNER JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
WHERE ur.redemption_code = $1 -- Indexed!
```

#### Confirm (con row lock):
```sql
BEGIN;
SELECT * FROM rewards.user_redemptions
WHERE redemption_id = $1
FOR UPDATE; -- Lock row

UPDATE rewards.user_redemptions
SET redemption_status = 'confirmed',
    validated_by_merchant_id = $2,
    validated_at = NOW()
WHERE redemption_id = $1;
COMMIT;
```

---

## ⚡ Performance

### Optimizaciones Implementadas:
- ✅ **sqlx compile-time verification**: Queries verificadas en tiempo de compilación
- ✅ **Índices**: redemption_code, user_id, merchant_id
- ✅ **Row-level locking**: `FOR UPDATE` en confirm endpoint
- ✅ **Connection pooling**: sqlx PgPool con límite configurable
- ✅ **Async/await**: Non-blocking I/O con Tokio runtime
- ✅ **Structured logging**: tracing con niveles (info, error, warn)

### Métricas Esperadas:
- Login: ~50ms (bcrypt + JWT generation)
- Validate: ~10ms (index scan)
- Confirm: ~15ms (transaction with lock)
- Stats: ~30ms (aggregations on indexed columns)

---

## 🔒 Seguridad

### Implementado:
✅ **Bcrypt** para API keys (cost 12)
✅ **JWT** con expiración de 8 horas
✅ **Middleware de autenticación** en rutas protegidas
✅ **Validación de estado activo** del comercio
✅ **Row-level locking** para evitar double-spending
✅ **Logging de intentos de acceso**

### Pendiente (Próximas Sprints):
- 🔲 Rate limiting por comercio
- 🔲 Audit logging de confirmaciones
- 🔲 IP whitelisting opcional
- 🔲 Webhooks para notificaciones
- 🔲 API key rotation policy
- 🔲 2FA opcional para comercios críticos

---

## 📁 Archivos Creados

```
src/api/merchant/
├── mod.rs          (115 líneas) - Router con rutas públicas y protegidas
├── auth.rs         (180 líneas) - Login con API key y JWT
├── validate.rs     (330 líneas) - Validación y confirmación de QR
└── stats.rs        (176 líneas) - Estadísticas agregadas y recientes
```

**Total**: ~800 líneas de código Rust

---

## 🚀 Próximos Pasos

### Testing (Sprint Actual):
1. ✅ Compilación exitosa
2. 🔲 Crear merchant de prueba en DB
3. 🔲 Generar API key con bcrypt
4. 🔲 Probar flujo completo end-to-end
5. 🔲 Validar edge cases (códigos inválidos, expirados, etc.)

### Mejoras (Próximos Sprints):
- 🔲 Documentación OpenAPI/Swagger
- 🔲 Tests unitarios e integración
- 🔲 Dashboard web para comercios
- 🔲 Webhooks de notificación
- 🔲 Reportes detallados (CSV/PDF)
- 🔲 Multi-tenant con roles (admin, cashier, etc.)

---

## 🎉 Resumen de Implementación Completa

### Sistema Completo de Redenciones:

**ENDPOINTS DE USUARIOS (6)** ✅ COMPLETADOS:
1. GET /api/v1/rewards/offers - Listar ofertas
2. GET /api/v1/rewards/offers/:id - Detalle de oferta
3. POST /api/v1/rewards/redeem - Canjear oferta
4. GET /api/v1/rewards/redemptions - Mis redenciones
5. GET /api/v1/rewards/redemptions/:id - Detalle de redención
6. DELETE /api/v1/rewards/redemptions/:id - Cancelar redención

**ENDPOINTS DE COMERCIOS (4)** ✅ COMPLETADOS:
1. POST /api/v1/merchant/auth/login - Login con API key
2. POST /api/v1/merchant/validate - Validar código QR
3. POST /api/v1/merchant/confirm/:id - Confirmar uso
4. GET /api/v1/merchant/stats - Estadísticas

**TOTAL: 10 ENDPOINTS** 🎯 **100% COMPLETADO**

---

## 📞 Contacto y Soporte

Para testing, debugging, o mejoras:
- Revisar logs: `tracing` con niveles info/error
- Verificar DB: Schema `rewards` en PostgreSQL
- Monitoreo: Prometheus metrics habilitado

**¡Sistema listo para pruebas de integración!** 🚀
