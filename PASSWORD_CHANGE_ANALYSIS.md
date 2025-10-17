# 🔐 ANÁLISIS: Sistema de Cambio de Contraseña

## 📋 Situación Actual

### ✅ Lo que YA existe:

El sistema **YA tiene** un flujo completo de gestión de contraseñas unificado:

#### **1. Cambio de Contraseña (Método Actual - Con Verificación por Email)**

**Endpoint 1:** `POST /api/v4/passwords/request-code`
```json
{
  "email": "usuario@ejemplo.com",
  "purpose": "change_password"
}
```
**Respuesta:**
- ✅ Envía código de 6 dígitos al email
- ✅ Código válido por 15 minutos
- ✅ Máximo 3 códigos por hora

**Endpoint 2:** `POST /api/v4/passwords/set-with-code`
```json
{
  "email": "usuario@ejemplo.com",
  "verification_code": "123456",
  "new_password": "NuevaContraseña123!",
  "confirmation_password": "NuevaContraseña123!"
}
```
**Respuesta:**
- ✅ Actualiza contraseña
- ✅ Retorna nuevo JWT token
- ✅ Invalida código usado

---

## 🎯 Propuesta: Endpoint Adicional Directo

### **Opción A: Cambio Directo con JWT + Contraseña Actual (Recomendado)**

Crear un nuevo endpoint que permita cambiar la contraseña directamente si el usuario:
1. Está autenticado (JWT válido)
2. Conoce su contraseña actual

**Ventajas:**
- ✅ Más rápido (no requiere email)
- ✅ Mejor UX para usuarios que conocen su contraseña
- ✅ Doble verificación (JWT + contraseña actual)
- ✅ No interfiere con el flujo de recuperación existente

**Endpoint Propuesto:**
```
PUT /api/v4/userdata/password
```

**Autenticación:** JWT requerido

**Request Body:**
```json
{
  "current_password": "MiContraseñaActual123!",
  "new_password": "MiNuevaContraseña456!",
  "confirmation_password": "MiNuevaContraseña456!"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "user_id": 42,
    "email": "usuario@ejemplo.com",
    "password_updated_at": "2025-10-04T10:30:45-05:00",
    "message": "Contraseña actualizada exitosamente"
  },
  "error": null,
  "request_id": "abc-123...",
  "timestamp": "2025-10-04T15:30:45Z",
  "execution_time_ms": 45,
  "cached": false
}
```

**Validaciones:**
- ✅ JWT válido y activo
- ✅ Contraseña actual correcta
- ✅ Nueva contraseña cumple requisitos:
  - 8-128 caracteres
  - Al menos 1 mayúscula
  - Al menos 1 minúscula
  - Al menos 1 número
  - Al menos 1 carácter especial
- ✅ Nueva contraseña diferente a la actual
- ✅ Contraseñas de confirmación coinciden

**Códigos de Error:**
- `400 BAD REQUEST` - Validación fallida (contraseñas no coinciden, no cumple requisitos)
- `401 UNAUTHORIZED` - JWT inválido o contraseña actual incorrecta
- `500 INTERNAL SERVER ERROR` - Error de servidor

**Seguridad:**
- ✅ Requiere JWT (usuario autenticado)
- ✅ Requiere contraseña actual (doble verificación)
- ✅ Hash bcrypt para almacenamiento
- ✅ Actualiza `updated_at` con timezone GMT-5
- ✅ Log de auditoría completo
- ✅ Rate limiting (opcional: 5 intentos/hora)

---

### **Opción B: Mantener Solo el Flujo Actual (Email Verification)**

**NO crear endpoint nuevo** y usar siempre el flujo existente:
1. `POST /api/v4/passwords/request-code` (purpose: "change_password")
2. `POST /api/v4/passwords/set-with-code`

**Ventajas:**
- ✅ Más seguro (siempre requiere email verification)
- ✅ Protege contra tokens JWT comprometidos
- ✅ Usuario recibe notificación por email
- ✅ Sistema ya implementado y probado

**Desventajas:**
- ❌ Requiere acceso al email
- ❌ Más pasos para el usuario
- ❌ Menos conveniente si el usuario conoce su contraseña

---

## 📊 Comparación de Enfoques

| Aspecto | Opción A (Directo) | Opción B (Email) | Actual |
|---------|-------------------|------------------|---------|
| **Pasos** | 1 request | 2 requests | 2 requests |
| **Autenticación** | JWT + Contraseña | Email verification | Email verification |
| **Requiere Email** | No | Sí | Sí |
| **Seguridad** | Alta (doble factor) | Muy Alta (triple factor) | Muy Alta |
| **UX** | Excelente | Buena | Buena |
| **Implementación** | Nueva | Ya existe | Ya existe |
| **Notificación** | Opcional | Automática | Automática |

---

## 🎨 Diseño de Implementación (Opción A)

### Archivo Nuevo: `src/api/userdata_v4.rs` (Agregar función)

```rust
/// PUT /api/v4/userdata/password - Cambiar contraseña con autenticación
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<PasswordChangeResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    info!("Password change request for user_id: {}", current_user.user_id);

    // 1. Validar que las contraseñas nuevas coincidan
    if payload.new_password != payload.confirmation_password {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 2. Validar formato de nueva contraseña
    validate_password_strength(&payload.new_password)?;

    // 3. Verificar contraseña actual
    let user = sqlx::query!(
        "SELECT password_hash FROM public.dim_users WHERE id = $1",
        current_user.user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let password_hash = user.password_hash
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let is_valid = bcrypt::verify(&payload.current_password, &password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 4. Verificar que nueva contraseña sea diferente
    let same_password = bcrypt::verify(&payload.new_password, &password_hash)
        .unwrap_or(false);
    if same_password {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 5. Hash nueva contraseña
    let new_hash = bcrypt::hash(&payload.new_password, bcrypt::DEFAULT_COST)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 6. Actualizar contraseña con timestamp GMT-5
    let gmt_minus_5 = FixedOffset::west_opt(5 * 3600).unwrap();
    let now_gmt_minus_5 = Utc::now().with_timezone(&gmt_minus_5);

    sqlx::query!(
        r#"
        UPDATE public.dim_users
        SET password_hash = $1,
            updated_at = $2
        WHERE id = $3
        "#,
        new_hash,
        now_gmt_minus_5,
        current_user.user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 7. Retornar respuesta
    let response_data = PasswordChangeResponse {
        user_id: current_user.user_id,
        email: current_user.email.clone(),
        password_updated_at: now_gmt_minus_5.to_rfc3339(),
        message: "Contraseña actualizada exitosamente".to_string(),
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response_data),
        error: None,
        request_id: Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        execution_time_ms: Some(start_time.elapsed().as_millis() as u64),
        cached: false,
    }))
}
```

### Estructuras Necesarias

```rust
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
    pub confirmation_password: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordChangeResponse {
    pub user_id: i64,
    pub email: String,
    pub password_updated_at: String,
    pub message: String,
}
```

### Actualizar Router

```rust
pub fn create_userdata_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/userdata", get(get_user_data).put(update_user_data))
        .route("/api/v4/userdata/password", put(change_password))
        .route_layer(from_fn(extract_current_user))
}
```

---

## 🔒 Consideraciones de Seguridad

### Opción A (Endpoint Directo)
- ✅ **Doble Factor:** JWT + contraseña actual
- ✅ **Sin email comprometido:** No depende de acceso al email
- ⚠️ **Token JWT comprometido:** Si un atacante tiene el JWT y la contraseña, puede cambiarla
- ✅ **Rate Limiting:** Limitar intentos por hora
- ✅ **Auditoría:** Log completo en audit_logs

### Opción B (Solo Email)
- ✅ **Triple Factor:** JWT (opcional) + Email + Código
- ✅ **Notificación Automática:** Usuario siempre sabe del cambio
- ✅ **Token comprometido:** No permite cambio sin acceso al email
- ❌ **Email comprometido:** Atacante con acceso al email puede cambiar contraseña

---

## 💡 Recomendación Final

### **Implementar AMBOS enfoques:**

1. **Endpoint Directo** (`PUT /api/v4/userdata/password`)
   - Para usuarios que conocen su contraseña actual
   - Requiere JWT + contraseña actual
   - Experiencia de usuario óptima

2. **Flujo de Email** (Ya existe)
   - Para recuperación de contraseña olvidada
   - Para cambios desde dispositivos no confiables
   - Mayor seguridad

### **Casos de Uso:**

```
┌─────────────────────────────────────────────┐
│ ¿El usuario conoce su contraseña actual?   │
└─────────────────┬───────────────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
       SÍ                  NO
        │                   │
        ▼                   ▼
┌───────────────┐   ┌──────────────────┐
│  Opción A     │   │  Opción B        │
│  PUT password │   │  Email flow      │
│  (Directo)    │   │  (Recuperación)  │
└───────────────┘   └──────────────────┘
```

---

## 📝 Próximos Pasos

### Si eliges **Opción A (Recomendado)**:
1. ✅ Agregar función `change_password()` en `userdata_v4.rs`
2. ✅ Crear estructuras `ChangePasswordRequest` y `PasswordChangeResponse`
3. ✅ Actualizar router con nueva ruta
4. ✅ Agregar función de validación `validate_password_strength()`
5. ✅ Documentar en `API_ENDPOINTS.md`
6. ✅ Crear script de testing
7. ✅ Compilar y probar

### Si eliges **Opción B (Mantener actual)**:
- ✅ Ya está implementado
- ✅ Documentación existente en `API_ENDPOINTS.md` (líneas 1420-1600)
- ✅ No requiere cambios

---

## 🎯 ¿Cuál prefieres?

**Opción A:** Implementar endpoint directo + mantener flujo de email  
**Opción B:** Mantener solo el flujo de email existente

**Mi recomendación:** **Opción A** - Mejor experiencia de usuario sin sacrificar seguridad.

