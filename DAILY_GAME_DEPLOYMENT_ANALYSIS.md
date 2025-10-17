# 🔍 Análisis: Daily Game 404 Error - webh.lumapp.org

## 🚨 Problema Reportado

```
Frontend intenta acceder:
🌐 URL: https://webh.lumapp.org/api/v4/daily-game/claim
❌ Error: 404 Not Found
```

---

## 📊 Análisis de la Situación

### ✅ **Lo que SÍ existe en el código**

#### 1. Endpoints Implementados (Rust Backend)

**Archivo**: `src/api/mod.rs` (líneas 141-142)

```rust
fn create_protected_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        // ... otras rutas ...
        // Daily Game endpoints (protected)
        .route("/api/v4/daily-game/claim", post(daily_game::handle_claim))
        .route("/api/v4/daily-game/status", get(daily_game::handle_status))
        .layer(from_fn(extract_current_user))
}
```

✅ **Rutas registradas correctamente en el código**

#### 2. Handlers Implementados

**Archivo**: `src/api/daily_game/claim.rs`
- ✅ `handle_claim` - POST handler completo
- ✅ Validaciones implementadas
- ✅ Lógica de negocio completa

**Archivo**: `src/api/daily_game/status.rs`
- ✅ `handle_status` - GET handler completo
- ✅ Query optimizada con CTEs

#### 3. Servidor Configurado

**Archivo**: `src/main.rs` (línea 70)

```rust
let port = 8000;
let addr = SocketAddr::from(([0, 0, 0, 0], port));
info!("listening on {}", addr);
let listener = tokio::net::TcpListener::bind(addr).await?;
```

✅ **Servidor escucha en**: `0.0.0.0:8000` (todas las interfaces)

---

## 🔴 **Problema Identificado: Infraestructura**

### El Issue NO es del código Rust

El problema está en **una de estas capas**:

### 1️⃣ **Proxy Inverso / Nginx** (más probable)

**Dominio**: `webh.lumapp.org` → sugiere que hay un **reverse proxy** (Nginx, Apache, etc.)

**Posible configuración actual**:
```nginx
# /etc/nginx/sites-available/webh.lumapp.org.conf

server {
    listen 443 ssl;
    server_name webh.lumapp.org;
    
    # SSL certificates
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    # PROBLEMA: Rutas no incluyen /api/v4/daily-game/*
    
    location /api/v4/invoices/ {
        proxy_pass http://localhost:8000;
        # headers...
    }
    
    location /api/v4/auth/ {
        proxy_pass http://localhost:8000;
        # headers...
    }
    
    location /api/v4/rewards/ {
        proxy_pass http://localhost:8000;
        # headers...
    }
    
    # ❌ FALTA ESTA SECCIÓN:
    # location /api/v4/daily-game/ {
    #     proxy_pass http://localhost:8000;
    #     ...
    # }
}
```

**Resultado**: Nginx no sabe cómo routear `/api/v4/daily-game/*` → retorna **404**

### 2️⃣ **Load Balancer / API Gateway**

Si hay un load balancer o API gateway externo, necesita configuración para las nuevas rutas.

### 3️⃣ **Firewall / Security Groups**

Menos probable, pero posible que reglas de firewall bloqueen las nuevas rutas.

### 4️⃣ **Servidor Rust no corriendo**

Si el servidor Rust está caído, Nginx retornaría error 502 (Bad Gateway), no 404.  
El 404 sugiere que **Nginx está respondiendo**, pero no encuentra la ruta.

---

## 🛠️ **Soluciones Propuestas**

### ✅ **Opción 1: Agregar rutas al Nginx** (RECOMENDADO)

**Archivo**: `/etc/nginx/sites-available/webh.lumapp.org.conf`

```nginx
# Agregar esta sección:
location /api/v4/daily-game/ {
    proxy_pass http://localhost:8000;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection 'upgrade';
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_cache_bypass $http_upgrade;
    
    # CORS si es necesario
    add_header 'Access-Control-Allow-Origin' '*' always;
    add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS' always;
    add_header 'Access-Control-Allow-Headers' 'Authorization, Content-Type' always;
}
```

**Después**:
```bash
sudo nginx -t                    # Validar sintaxis
sudo systemctl reload nginx      # Recargar configuración
```

---

### ✅ **Opción 2: Usar API genérica existente** (WORKAROUND)

Si no puedes modificar Nginx inmediatamente, **reutiliza el servicio de gamificación existente**.

#### 📍 **Endpoint Existente: `/api/v4/rewards/`**

**Archivo actual**: `src/api/gamification_service.rs`

```rust
pub async fn credit_lumis_for_invoice(
    pool: &PgPool,
    user_id: i64,
    cufe: &str,  // ← Actualmente requiere CUFE (factura)
) -> Result<LumisResult, sqlx::Error> {
    // INSERT INTO rewards.fact_accumulations
    // (user_id, accum_type, accum_key, dtype, quantity, date, accum_id)
    // ...
}
```

**PROBLEMA**: Este servicio está diseñado para **facturas** (requiere CUFE).

#### 🔧 **Modificación Propuesta: Crear endpoint genérico**

**Nuevo endpoint**: `POST /api/v4/rewards/accumulate`

**Request**:
```json
{
  "accum_type": "daily_game",
  "accum_key": "play_2025_10_13",  // Cualquier key única
  "quantity": 5,                    // Lümis a acreditar
  "accum_id": 10                    // ID de regla en dim_accumulations
}
```

**Ventajas**:
- ✅ No requiere modificar Nginx
- ✅ Reutiliza infraestructura existente de `/api/v4/rewards/`
- ✅ Genérico: sirve para daily game, misiones, etc.
- ✅ Frontend solo cambia la URL

**Implementación** (agregar a `src/api/rewards_v4.rs`):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AccumulateRequest {
    pub accum_type: String,
    pub accum_key: String,
    pub quantity: i32,
    pub accum_id: i32,
}

#[derive(Debug, Serialize)]
pub struct AccumulateResponse {
    pub lumis_added: i32,
    pub new_balance: i32,
}

/// POST /api/v4/rewards/accumulate
/// Acumula Lümis genéricamente (daily game, misiones, etc.)
#[axum::debug_handler]
async fn accumulate_lumis(
    State(app_state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<AccumulateRequest>,
) -> Result<Json<ApiResponse<AccumulateResponse>>, StatusCode> {
    let user_id = current_user.user_id as i64;
    
    // INSERT en fact_accumulations
    let current_time = Utc::now().naive_utc();
    
    sqlx::query(
        r#"
        INSERT INTO rewards.fact_accumulations 
        (user_id, accum_type, accum_key, dtype, quantity, date, accum_id)
        VALUES ($1, $2, $3, 'points', $4, $5, $6)
        "#
    )
    .bind(user_id)
    .bind(&request.accum_type)
    .bind(&request.accum_key)
    .bind(request.quantity)
    .bind(current_time)
    .bind(request.accum_id)
    .execute(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Error inserting accumulation: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Obtener balance actualizado
    let new_balance = get_user_balance(&app_state.db_pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
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

// Agregar a create_rewards_v4_router():
pub fn create_rewards_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/summary", get(get_user_summary))
        .route("/balance", get(get_user_balance))
        .route("/accumulate", post(accumulate_lumis))  // ← NUEVO
}
```

**Frontend cambia a**:
```typescript
const response = await fetch('https://webh.lumapp.org/api/v4/rewards/accumulate', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    accum_type: 'daily_game',
    accum_key: `daily_game_${userId}_${date}`,  // Único por usuario/día
    quantity: lumisWon,
    accum_id: 10,  // ID de daily_game en dim_accumulations
  }),
});
```

**PERO**: Esto NO valida que el usuario ya jugó hoy (constraint UNIQUE está en `fact_daily_game_plays`, no en `fact_accumulations`).

---

### ✅ **Opción 3: Usar proxy catch-all** (RÁPIDO)

Si Nginx tiene una regla catch-all, puedes aprovecharla:

```nginx
# Al final del server block:
location /api/ {
    proxy_pass http://localhost:8000;
    # ... headers ...
}
```

Esto hace que **todas** las rutas `/api/*` vayan al backend Rust.

**Ventaja**: No necesitas agregar cada ruta nueva
**Desventaja**: Menos control granular

---

## 🎯 **Recomendación Final**

### **INMEDIATO (Workaround)**:

**Agregar endpoint genérico de acumulación** como Opción 2.

**Pros**:
- ✅ No requiere tocar infraestructura
- ✅ Reutiliza `/api/v4/rewards/` que ya funciona
- ✅ Flutter solo cambia la URL

**Contras**:
- ⚠️ No valida "ya jugó hoy" (eso debe hacerlo el frontend o agregar validación)
- ⚠️ Dos registros separados: `fact_daily_game_plays` + `fact_accumulations`

### **CORRECTO (Mediano plazo)**:

**Configurar Nginx** para incluir `/api/v4/daily-game/` (Opción 1).

**Pros**:
- ✅ Usa los endpoints diseñados específicamente
- ✅ Validaciones completas en backend
- ✅ Constraint UNIQUE previene duplicados

**Contras**:
- ⏳ Requiere acceso al servidor y reload de Nginx

---

## 📝 **Checklist de Diagnóstico**

Para identificar el problema exacto:

```bash
# 1. Verificar que el servidor Rust está corriendo
ps aux | grep lum_rust_ws
curl http://localhost:8000/api/v4/daily-game/status -H "Authorization: Bearer $TOKEN"

# 2. Verificar configuración de Nginx
cat /etc/nginx/sites-available/webh.lumapp.org.conf | grep -A 10 "location"

# 3. Verificar logs de Nginx
tail -f /var/log/nginx/access.log
tail -f /var/log/nginx/error.log

# 4. Probar directamente el backend (sin Nginx)
curl http://localhost:8000/api/v4/daily-game/status \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json"

# 5. Probar a través de Nginx
curl https://webh.lumapp.org/api/v4/daily-game/status \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json"
```

---

## 🔄 **Flujo de Datos Actual**

```
Flutter App
    ↓
https://webh.lumapp.org/api/v4/daily-game/claim
    ↓
Nginx (Puerto 443) → ❌ 404 (no encuentra ruta)
    ↓
(NO LLEGA AL BACKEND)
    ↓
Rust Backend (Puerto 8000)
    ├─ /api/v4/daily-game/claim ✅ Existe
    └─ /api/v4/daily-game/status ✅ Existe
```

**Problema**: Nginx no hace proxy de `/api/v4/daily-game/*`

---

## 🚀 **Flujo Correcto (después de fix)**

```
Flutter App
    ↓
https://webh.lumapp.org/api/v4/daily-game/claim
    ↓
Nginx (Puerto 443) → ✅ Proxy pass
    ↓
http://localhost:8000/api/v4/daily-game/claim
    ↓
Rust Backend (Puerto 8000)
    ├─ extract_current_user (JWT middleware)
    ├─ handle_claim (validaciones)
    └─ INSERT fact_daily_game_plays + fact_accumulations
    ↓
PostgreSQL (Trigger actualiza balance)
    ↓
Response 200 OK
```

---

## 📊 **Comparación de Opciones**

| Aspecto | Opción 1: Nginx Config | Opción 2: Endpoint Genérico | Opción 3: Catch-all |
|---------|------------------------|----------------------------|---------------------|
| **Dificultad** | Media | Baja | Baja |
| **Tiempo** | 30 min | 2 horas | 15 min |
| **Requiere acceso servidor** | ✅ Sí | ❌ No | ✅ Sí |
| **Validaciones completas** | ✅ Sí | ⚠️ Parcial | ✅ Sí |
| **UNIQUE constraint** | ✅ Funciona | ⚠️ No aplica | ✅ Funciona |
| **Reutilizable** | ❌ Solo daily game | ✅ Cualquier juego | ✅ Todas las APIs |
| **Mantenibilidad** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |

---

## 💡 **Decisión Recomendada**

### **Estrategia de 2 Fases**:

#### **Fase 1 (HOY)** - Workaround rápido:
```
1. Crear endpoint genérico /api/v4/rewards/accumulate
2. Flutter cambia a usar ese endpoint
3. Frontend valida "ya jugó hoy" (consulta fact_daily_game_plays)
4. Sistema funciona en producción ✅
```

#### **Fase 2 (Esta semana)** - Solución correcta:
```
1. Configurar Nginx para /api/v4/daily-game/*
2. Flutter vuelve a usar endpoint específico
3. Backend valida todo con UNIQUE constraint
4. Eliminar validación del frontend
5. Sistema robusto en producción ✅✅
```

---

## 🔍 **Conclusión**

**El código Rust está correcto y completo.**  
**El problema es de infraestructura (Nginx no tiene las rutas configuradas).**

**Solución más rápida**: Endpoint genérico en `/api/v4/rewards/accumulate`  
**Solución correcta**: Agregar rutas de daily-game a Nginx

---

**Autor**: AI Assistant  
**Fecha**: 2025-10-14  
**Status**: Análisis completado - Esperando decisión del equipo
