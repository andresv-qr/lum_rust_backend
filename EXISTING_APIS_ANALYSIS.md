# 🔍 Análisis: APIs Existentes para Acreditar Lümis

## 📊 Situación Actual

### ✅ **APIs Ya Existentes**

#### 1. **POST `/api/v4/gamification/track`** 
**Archivo**: `src/api/gamification_v4.rs` (línea 151)

**Purpose**: Track user actions and award Lumis/XP through gamification system

**Request**:
```json
{
  "action": "daily_login | invoice_upload | survey_complete",
  "channel": "mobile_app | whatsapp | web_app",
  "metadata": {}
}
```

**Response**:
```json
{
  "lumis_earned": 5,
  "total_lumis": 308,
  "xp_earned": 10,
  "current_level": 3,
  "level_name": "Estrella Lüm",
  "streaks": {...},
  "achievements_unlocked": [...],
  "active_events": [...],
  "message": "¡Genial! Has ganado 5 Lümis"
}
```

**Cómo funciona**:
- Llama a función PostgreSQL: `gamification.track_user_action(user_id, action, channel, metadata)`
- La función de BD maneja:
  - Verificar si ya hizo la acción hoy
  - Calcular Lümis según reglas de gamificación
  - Insertar en `gamification.fact_user_actions`
  - Actualizar `gamification.fact_user_progression`
  - Evaluar streaks, achievements, eventos
  - **¿Inserta en `rewards.fact_accumulations`?** 🤔 **NO** - usa schema `gamification`

**Problema**: Este endpoint usa el schema `gamification.*`, NO `rewards.fact_accumulations`

---

#### 2. **Función: `gamification_service::credit_lumis_for_invoice()`**
**Archivo**: `src/api/gamification_service.rs` (línea 18)

**Purpose**: Acredita Lümis cuando se procesa una factura (usado internamente por `/invoices/process-from-url`)

**Firma**:
```rust
pub async fn credit_lumis_for_invoice(
    pool: &PgPool,
    user_id: i64,
    cufe: &str,  // ← Requiere CUFE (factura)
) -> Result<LumisResult, sqlx::Error>
```

**Qué hace**:
```rust
// 1. Consulta regla activa (id=0) en rewards.dim_accumulations
// 2. INSERT INTO rewards.fact_accumulations 
//    (user_id, accum_type, accum_key, dtype, quantity, date, accum_id)
//    VALUES (user_id, 'Factura', cufe, 'points', lumis, NOW(), 0)
// 3. Trigger actualiza rewards.fact_balance_points
// 4. Retorna balance actualizado
```

**Usado en**: `POST /api/v4/invoices/process-from-url` (línea 81 de `url_processing_v4.rs`)

**Problema**: 
- ❌ NO es un endpoint público (es función interna)
- ❌ Requiere CUFE (diseñado para facturas)
- ❌ Usa accum_id=0 (regla de facturas), no accum_id=10 (daily_game)

---

## 🎯 **Conclusión del Análisis**

### ❌ **NO existe un endpoint genérico público para acreditar Lümis**

Las opciones existentes son:

1. **`/api/v4/gamification/track`**:
   - ✅ Es público y protegido con JWT
   - ✅ Funciona en producción
   - ❌ Inserta en `gamification.fact_user_actions`, NO en `rewards.fact_accumulations`
   - ❌ Solo acepta 3 acciones hardcodeadas: `daily_login`, `invoice_upload`, `survey_complete`
   - ❌ No inserta en `rewards.fact_accumulations` (usa otro schema)

2. **`gamification_service::credit_lumis_for_invoice()`**:
   - ❌ No es endpoint (función interna)
   - ❌ Requiere CUFE (diseñado para facturas)
   - ✅ SÍ inserta en `rewards.fact_accumulations`
   - ❌ Usa accum_id=0 (factura), no accum_id=10 (daily_game)

---

## 💡 **Opciones de Solución**

### **Opción A: Extender `/api/v4/gamification/track`** ⚡

Modificar para aceptar `"daily_game"` como acción:

**Cambio en línea 161**:
```rust
// ANTES:
if !["daily_login", "invoice_upload", "survey_complete"].contains(&request.action.as_str()) {
    return Err(ApiError::validation_error("Invalid action type"));
}

// DESPUÉS:
if !["daily_login", "invoice_upload", "survey_complete", "daily_game"].contains(&request.action.as_str()) {
    return Err(ApiError::validation_error("Invalid action type"));
}
```

**Frontend usa**:
```typescript
POST https://webh.lumapp.org/api/v4/gamification/track
{
  "action": "daily_game",
  "channel": "mobile_app",
  "metadata": {
    "star_id": "star_3",
    "lumis_won": 5
  }
}
```

**Ventajas**:
- ✅ Reutiliza endpoint que **YA funciona en producción**
- ✅ Ya tiene JWT authentication
- ✅ Ya está en `/api/v4/gamification/` que probablemente está en Nginx
- ✅ Solo 1 línea de código a cambiar

**Desventajas**:
- ⚠️ Inserta en `gamification.fact_user_actions`, no en `rewards.fact_accumulations`
- ⚠️ Necesitas modificar la función PostgreSQL `gamification.track_user_action()` para soportar `daily_game`
- ⚠️ No valida UNIQUE constraint de "ya jugó hoy" (eso está en `rewards.fact_daily_game_plays`)

---

### **Opción B: Crear endpoint genérico `/api/v4/rewards/accumulate`** 🆕

Crear nuevo endpoint específico para acumular puntos:

```rust
// En src/api/rewards_v4.rs

#[derive(Debug, Deserialize)]
pub struct AccumulateRequest {
    pub accum_type: String,      // "daily_game"
    pub accum_key: String,       // "play_2025_10_13_user_1"
    pub quantity: i32,           // 5
    pub accum_id: i32,           // 10 (id de daily_game en dim_accumulations)
}

pub async fn accumulate_lumis(
    State(app_state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<AccumulateRequest>,
) -> Result<Json<ApiResponse<AccumulateResponse>>, StatusCode> {
    let user_id = current_user.user_id as i64;
    
    // INSERT directo en fact_accumulations
    sqlx::query(
        r#"
        INSERT INTO rewards.fact_accumulations 
        (user_id, accum_type, accum_key, dtype, quantity, date, accum_id)
        VALUES ($1, $2, $3, 'points', $4, NOW(), $5)
        "#
    )
    .bind(user_id)
    .bind(&request.accum_type)
    .bind(&request.accum_key)
    .bind(request.quantity)
    .bind(request.accum_id)
    .execute(&app_state.db_pool)
    .await?;
    
    // Obtener balance
    let new_balance = get_user_balance(&app_state.db_pool, user_id).await?;
    
    Ok(Json(ApiResponse::success(
        AccumulateResponse {
            lumis_added: request.quantity,
            new_balance,
        },
        Uuid::new_v4().to_string(),
        None,
        false
    )))
}

// Agregar ruta:
pub fn create_rewards_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/summary", get(get_user_summary))
        .route("/balance", get(get_user_balance))
        .route("/accumulate", post(accumulate_lumis))  // ← NUEVO
}
```

**Frontend usa**:
```typescript
POST https://webh.lumapp.org/api/v4/rewards/accumulate
{
  "accum_type": "daily_game",
  "accum_key": "daily_game_1_2025_10_13",
  "quantity": 5,
  "accum_id": 10
}
```

**Ventajas**:
- ✅ Inserta directamente en `rewards.fact_accumulations` ✓
- ✅ Genérico: sirve para daily game, misiones futuras, etc.
- ✅ Usa ruta `/api/v4/rewards/` que probablemente está en Nginx
- ✅ Simple y directo

**Desventajas**:
- ⚠️ No valida "ya jugó hoy" (constraint UNIQUE está en `fact_daily_game_plays`)
- ⚠️ Frontend debe insertar en `fact_daily_game_plays` primero (otra API call)
- ⚠️ Requiere 2 operaciones separadas (no atómicas)

---

### **Opción C: Usar `/api/v4/daily-game/claim` (correcto pero bloqueado)** ✅❌

Esta es la solución **arquitectónicamente correcta** pero requiere configurar Nginx.

**Ventajas**:
- ✅ Endpoints ya implementados y testeados
- ✅ Validaciones completas (lumis_won, star_id)
- ✅ UNIQUE constraint previene duplicados
- ✅ Transacción atómica (jugada + acumulación)

**Desventajas**:
- ❌ Nginx no tiene la ruta configurada (404)
- ❌ Requiere acceso al servidor

---

## 📊 **Comparación de Opciones**

| Aspecto | Opción A: Extend /track | Opción B: Nuevo /accumulate | Opción C: /daily-game (correcto) |
|---------|-------------------------|----------------------------|-----------------------------------|
| **Cambios en Rust** | 1 línea | ~50 líneas | 0 (ya existe) |
| **Cambios en BD** | Modificar función PL/pgSQL | 0 | 0 |
| **Ruta en Nginx** | ✅ Probablemente existe | ✅ Probablemente existe | ❌ Falta configurar |
| **Inserta en rewards.fact_accumulations** | ❌ No (usa gamification) | ✅ Sí | ✅ Sí |
| **Valida "ya jugó hoy"** | ⚠️ Depende de BD | ❌ No | ✅ Sí (UNIQUE) |
| **Transacción atómica** | ⚠️ Depende de función | ❌ No | ✅ Sí |
| **Tiempo implementación** | 1 hora | 2 horas | 0 horas (+ config Nginx) |
| **Reutilizable futuro** | ⚠️ Solo gamification | ✅ Genérico | ❌ Solo daily game |
| **Arquitectura limpia** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

---

## 🎯 **Recomendación Final**

### **INMEDIATO (Workaround para producción HOY)**:

**Opción B: Crear `/api/v4/rewards/accumulate`**

**Por qué**:
1. ✅ Ruta `/api/v4/rewards/*` probablemente ya existe en Nginx
2. ✅ Inserta directamente en `rewards.fact_accumulations`
3. ✅ Genérico (sirve para otros juegos futuros)
4. ✅ No requiere tocar infraestructura
5. ⚠️ Frontend debe validar "ya jugó" consultando `fact_daily_game_plays`

**Flujo**:
```
1. Frontend: GET /api/v4/daily-game/status 
   → Verifica si puede jugar (consulta fact_daily_game_plays)

2. Usuario elige estrella → Frontend calcula lumis_won

3. Frontend: POST /api/v4/rewards/accumulate
   {
     "accum_type": "daily_game",
     "accum_key": "daily_game_{user_id}_{date}",
     "quantity": 5,
     "accum_id": 10
   }
   → Inserta en rewards.fact_accumulations
   → Trigger actualiza balance

4. Frontend: Marca como jugado en local storage
```

---

### **CORRECTO (Mediano plazo esta semana)**:

**Opción C: Configurar Nginx para `/api/v4/daily-game/*`**

Agregar al config:
```nginx
location /api/v4/daily-game/ {
    proxy_pass http://localhost:8000;
    # headers...
}
```

Entonces frontend usa los endpoints diseñados:
- `GET /api/v4/daily-game/status`
- `POST /api/v4/daily-game/claim`

---

## 📝 **Resumen Ejecutivo**

### **APIs Existentes para Lümis**:
1. ✅ `/api/v4/gamification/track` - Existe pero usa schema `gamification`, no `rewards`
2. ✅ `gamification_service::credit_lumis_for_invoice()` - Función interna para facturas
3. ❌ **NO existe endpoint genérico público** para `rewards.fact_accumulations`

### **Necesitamos**:
- **Crear** `/api/v4/rewards/accumulate` (workaround rápido)
- **O configurar** Nginx para `/api/v4/daily-game/*` (correcto)

### **Decisión**:
Implementar **Opción B** hoy para producción, luego migrar a **Opción C** cuando configuren Nginx.

---

**Autor**: AI Assistant  
**Fecha**: 2025-10-14  
**Status**: Análisis completado - Opción B recomendada
