# 🔧 CORRECCIONES NECESARIAS PARA ENDPOINTS

## Resumen de Errores (90 errores de compilación)

### 1. **Claims.user_id** → Usar `sub` y parsear a i32
- Archivos afectados: `offers.rs`, `redeem.rs`, `user.rs`
- Solución: Agregar helper `claims.user_id()` que parse `sub`

### 2. **Services faltan Clone**
- `OfferService` necesita `#[derive(Clone)]`
- `RedemptionService` necesita `#[derive(Clone)]`

### 3. **RedemptionService faltan métodos**:
- `pool() -> &PgPool`
- `get_redemption_by_code(code: &str) -> Result<UserRedemption>`
- `confirm_redemption(id, merchant_id, ip) -> Result<()>`
- `refresh_qr_token(id) -> Result<UserRedemption>`

### 4. **OfferFilters necesita Serialize**
- Agregar `#[derive(Serialize)]` en models.rs

### 5. **UserRedemption vs UserRedemptionItem** mismatch
- `list_user_redemptions` retorna `UserRedemptionItem`
- Endpoints esperan campos que no existen

### 6. **RedemptionError variants incorrectos**:
- `InvalidStatus` no existe → Usar match alternativo
- `OfferExpired` no existe → Es `CodeExpired`
- `DatabaseError(msg)` no existe → Es tuple variant sin mensaje
- `InsufficientBalance { available }` → Es `{ current }`
- `MaxRedemptionsReached { max_allowed }` → Es `{ max, current }`

### 7. **Cancel redemption** retorna `CancellationResponse` no `i32`

### 8. **RedemptionService::new()** recibe 3 params: `(PgPool, String, QrConfig)`
- Actualmente se pasa `(PgPool, QRGenerator)`

### 9. **Router state type mismatch**
- Rewards router retorna `Router<()>` pero lib.rs espera `Router<Arc<AppState>>`

## 🚀 PLAN DE ACCIÓN

Dado que son 90 errores y requiere refactoring significativo del código que acabamos de crear, tengo 2 opciones:

### Opción A: **REFACTORIZAR TODO** (6-8 horas)
- Adaptar TODOS los endpoints a la estructura existente
- Agregar todos los métodos faltantes en services
- Pros: endpoints 100% funcionales
- Contras: mucho tiempo, alta probabilidad de más errores

### Opción B: **CREAR ENDPOINTS SIMPLES PRIMERO** (2 horas) ✅ RECOMENDADO
- Crear UN endpoint de prueba que compile y funcione
- Validar integración end-to-end
- Luego expandir gradualmente
- Pros: validación rápida, iterativo, menos riesgo
- Contras: no tendremos todos los endpoints inmediatamente

---

## 💡 RECOMENDACIÓN: Opción B - Endpoint Incremental

Vamos a crear **SOLO** el endpoint de listado de ofertas first:

```
GET /api/v1/rewards/offers
```

**Beneficios**:
1. Valida autenticación (JWT Claims)
2. Valida servicio OfferService
3. Valida integración con DB
4. Compila y funciona en 30 minutos
5. Sirve de template para el resto

**Una vez funcione este**, extendemos:
- POST /redeem
- GET /my-redemptions  
- Merchant endpoints

**Pregunta**: ¿Procedemos con endpoint incremental (Opción B) o prefieres el refactor completo (Opción A)?
