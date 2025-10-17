# PUT /api/v4/userdata - API de Actualización de Datos de Usuario

## 📋 Resumen Ejecutivo

Se ha implementado exitosamente el endpoint `PUT /api/v4/userdata` que permite actualizar los datos demográficos del usuario autenticado en la tabla `public.dim_users`.

---

## 🎯 Características Principales

### ✅ Implementado

1. **Autenticación JWT**: Protegido con middleware `extract_current_user`
2. **Actualización Parcial**: Solo actualiza los campos proporcionados en el request
3. **Timestamp Automático**: Campo `updated_at` se actualiza automáticamente con timezone GMT-5
4. **Validación de Usuario**: Verifica que el usuario exista antes de actualizar
5. **Query Dinámico**: Construye el query SQL solo con los campos proporcionados
6. **Respuesta Completa**: Retorna todos los datos actualizados del usuario
7. **Logging Detallado**: Registra todas las operaciones de actualización
8. **Métricas de Performance**: Incluye `execution_time_ms` en la respuesta

---

## 🔌 Especificación del Endpoint

### Request

**Método:** `PUT`  
**URL:** `/api/v4/userdata`  
**Headers:**
- `Authorization: Bearer <jwt_token>` (REQUERIDO)
- `Content-Type: application/json`

**Body (JSON):**
```json
{
  "name": "string | null (opcional)",
  "date_of_birth": "string | null (opcional)",
  "country_origin": "string | null (opcional)",
  "country_residence": "string | null (opcional)",
  "segment_activity": "string | null (opcional)",
  "genre": "string | null (opcional)",
  "ws_id": "string | null (opcional)"
}
```

**IMPORTANTE:**
- Todos los campos son opcionales
- Solo se actualizan los campos enviados en el request
- El campo `email` NO es actualizable por seguridad
- Al menos UN campo debe ser proporcionado (de lo contrario: 400 BAD REQUEST)

---

### Response

**Success (200 OK):**
```json
{
  "success": true,
  "data": {
    "name": "María Rodríguez",
    "email": "maria@example.com",
    "date_of_birth": "1990-05-20",
    "country_origin": "Panama",
    "country_residence": "Colombia",
    "segment_activity": "Technology",
    "genre": "F",
    "ws_id": "507-9876-5432",
    "updated_at": "2025-10-04T10:30:45-05:00"
  },
  "error": null,
  "request_id": "d5e8f9a1-23bc-4def-8901-234567890abc",
  "timestamp": "2025-10-04T15:30:45Z",
  "execution_time_ms": 23,
  "cached": false
}
```

**Códigos de Error:**

| Código | Descripción |
|--------|-------------|
| `400 BAD REQUEST` | No se proporcionaron campos para actualizar |
| `401 UNAUTHORIZED` | Token JWT inválido, expirado o ausente |
| `404 NOT FOUND` | Usuario no existe en la base de datos |
| `500 INTERNAL SERVER ERROR` | Error de base de datos o servidor |

---

## 💻 Implementación Técnica

### Archivo Modificado
- **Ubicación:** `src/api/userdata_v4.rs`
- **Función:** `update_user_data()`
- **Router:** `create_userdata_v4_router()`

### Flujo de Ejecución

```
1. Request recibido → PUT /api/v4/userdata
2. Middleware JWT → Extrae CurrentUser
3. Validar payload → Al menos 1 campo presente
4. Crear timestamp → GMT-5 timezone
5. Construir query dinámico → Solo campos proporcionados
6. Ejecutar UPDATE → Con RETURNING clause
7. Verificar resultado → Usuario existe?
8. Retornar datos actualizados → Con ApiResponse wrapper
```

### Lógica del Timestamp

```rust
// Crear timestamp con timezone GMT-5
let gmt_minus_5 = FixedOffset::west_opt(5 * 3600).unwrap();
let now_gmt_minus_5 = Utc::now().with_timezone(&gmt_minus_5);
```

- **Formato en DB:** `timestamp with time zone`
- **Timezone:** GMT-5 (Panama/Colombia)
- **Actualización:** Automática en cada PUT
- **Formato de respuesta:** ISO 8601 con timezone

### Query Dinámico

El endpoint construye el query SQL dinámicamente solo con los campos proporcionados:

```sql
UPDATE public.dim_users
SET 
  name = $1,
  country_residence = $2,
  updated_at = $3
WHERE id = $4
RETURNING name, email, date_of_birth, country_origin, 
          country_residence, segment_activity, genre, 
          ws_id, updated_at
```

**Ventajas:**
- ✅ No sobrescribe campos no especificados
- ✅ Eficiencia en la base de datos
- ✅ Flexibilidad para el cliente
- ✅ Validación de campos vacíos

---

## 🧪 Ejemplos de Uso

### Ejemplo 1: Actualizar Solo el Nombre

**Request:**
```bash
curl -X PUT "http://localhost:3000/api/v4/userdata" \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{"name": "Juan Pérez López"}'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "name": "Juan Pérez López",
    "email": "juan@example.com",
    "date_of_birth": null,
    "country_origin": null,
    "country_residence": "Panama",
    "segment_activity": null,
    "genre": null,
    "ws_id": null,
    "updated_at": "2025-10-04T11:15:30-05:00"
  },
  "error": null,
  "request_id": "abc123...",
  "timestamp": "2025-10-04T16:15:30Z",
  "execution_time_ms": 18,
  "cached": false
}
```

### Ejemplo 2: Actualizar Múltiples Campos

**Request:**
```bash
curl -X PUT "http://localhost:3000/api/v4/userdata" \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "name": "María García",
    "date_of_birth": "1985-07-12",
    "country_origin": "Colombia",
    "country_residence": "Panama",
    "segment_activity": "Finance",
    "genre": "F"
  }'
```

### Ejemplo 3: Request Inválido (Sin Campos)

**Request:**
```bash
curl -X PUT "http://localhost:3000/api/v4/userdata" \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{}'
```

**Response:**
```
HTTP/1.1 400 Bad Request
```

---

## 🔒 Seguridad

### Autenticación
- ✅ JWT obligatorio en header `Authorization`
- ✅ Middleware `extract_current_user` valida token
- ✅ Usuario autenticado extraído de JWT claims

### Validaciones
- ✅ Usuario debe existir en `public.dim_users`
- ✅ Solo el usuario autenticado puede actualizar sus propios datos
- ✅ Campo `email` no es actualizable (protección de identidad)
- ✅ Campo `id` no es actualizable (protección de integridad)

### Logging
- ✅ Registro de todas las operaciones de actualización
- ✅ Incluye `user_id`, `email`, y campos modificados
- ✅ Métricas de performance (execution time)
- ✅ Errores de base de datos registrados con contexto

---

## 📊 Base de Datos

### Tabla: `public.dim_users`

**Campos Actualizables:**
- `name` (character varying)
- `date_of_birth` (character varying)
- `country_origin` (character varying)
- `country_residence` (character varying)
- `segment_activity` (character varying)
- `genre` (character varying)
- `ws_id` (text)
- `updated_at` (timestamp with time zone) - **Automático**

**Campos NO Actualizables:**
- `id` - Primary key
- `email` - Identificador de usuario (seguridad)
- Otros campos de autenticación (google_id, auth_providers, etc.)

### Índices Relevantes
```sql
-- Primary key
CREATE UNIQUE INDEX dim_users_pkey ON public.dim_users(id);

-- Email index (usado en búsquedas)
CREATE INDEX idx_users_email ON public.dim_users(email);
```

---

## 🎨 Estructura ApiResponse

El endpoint utiliza la estructura estándar `ApiResponse<UserData>`:

```rust
pub struct ApiResponse<T> {
    pub success: bool,                    // true si operación exitosa
    pub data: Option<T>,                  // Datos del usuario actualizado
    pub error: Option<String>,            // Mensaje de error (si aplica)
    pub request_id: String,               // UUID único para tracking
    pub timestamp: DateTime<Utc>,         // Timestamp de la respuesta
    pub execution_time_ms: Option<u64>,   // Tiempo de ejecución en ms
    pub cached: bool,                     // false para PUT
}
```

---

## 🔄 Comparación GET vs PUT

| Aspecto | GET /api/v4/userdata | PUT /api/v4/userdata |
|---------|----------------------|----------------------|
| **Método** | GET | PUT |
| **Autenticación** | JWT requerido | JWT requerido |
| **Body** | No | JSON con campos a actualizar |
| **Operación** | Lectura | Escritura |
| **Retorna** | Datos actuales | Datos después del update |
| **`updated_at`** | Valor actual | Se actualiza automáticamente |
| **Modificaciones** | No modifica datos | Actualiza campos especificados |
| **Validación** | Usuario existe | Usuario existe + campos válidos |

---

## 📝 Notas Técnicas

### Timezone GMT-5
El timestamp se guarda con timezone GMT-5 (Panama/Colombia):
```rust
let gmt_minus_5 = FixedOffset::west_opt(5 * 3600).unwrap();
let now_gmt_minus_5 = Utc::now().with_timezone(&gmt_minus_5);
```

### Query Dinámico
El query se construye dinámicamente para incluir solo los campos proporcionados:
- Reduce tráfico de red
- Evita sobrescribir campos no especificados
- Mejora performance de base de datos

### RETURNING Clause
El query usa `RETURNING *` para retornar los datos actualizados:
- Evita un SELECT adicional
- Garantiza consistencia de datos
- Reduce latencia de respuesta

---

## ✅ Testing

### Casos de Prueba Recomendados

1. **Actualización exitosa de un solo campo**
   - Request: `{"name": "Nuevo Nombre"}`
   - Expected: 200 OK con datos actualizados

2. **Actualización exitosa de múltiples campos**
   - Request: `{"name": "...", "country_residence": "...", "genre": "..."}`
   - Expected: 200 OK con todos los campos actualizados

3. **Request vacío**
   - Request: `{}`
   - Expected: 400 BAD REQUEST

4. **Token JWT inválido**
   - Request con token corrupto
   - Expected: 401 UNAUTHORIZED

5. **Usuario no existe**
   - Request con JWT válido pero usuario eliminado
   - Expected: 404 NOT FOUND

6. **Verificar `updated_at` actualizado**
   - Hacer PUT, verificar que `updated_at` cambió
   - Timezone debe ser GMT-5

---

## 🚀 Despliegue

### Pre-requisitos
- ✅ Rust 1.70+
- ✅ PostgreSQL 13+ con tabla `public.dim_users`
- ✅ JWT secret configurado en variables de entorno
- ✅ Middleware `extract_current_user` funcional

### Variables de Entorno
```bash
DATABASE_URL=postgresql://user:pass@host:5432/dbname
JWT_SECRET=your-secret-key
RUST_LOG=info
```

### Compilación
```bash
cargo build --release
```

### Ejecución
```bash
./target/release/lum_rust_ws
```

---

## 📚 Referencias

- **Archivo de implementación:** `src/api/userdata_v4.rs`
- **Documentación API:** `API_ENDPOINTS.md` (líneas 820-920)
- **Router:** Integrado en `create_userdata_v4_router()`
- **Middleware JWT:** `src/middleware/mod.rs` - `extract_current_user`

---

## 🎯 Roadmap Futuro

### Mejoras Potenciales
- [ ] Validación de formato de campos (email, phone, date)
- [ ] Soporte para actualización batch de múltiples usuarios (admin)
- [ ] Historial de cambios (audit log)
- [ ] Rate limiting por usuario
- [ ] Webhook de notificación post-actualización
- [ ] Validación de países usando catálogo ISO
- [ ] PATCH endpoint para operaciones más granulares

---

## 📄 Changelog

### v1.0.0 - 2025-10-04
- ✅ Implementación inicial de `PUT /api/v4/userdata`
- ✅ Autenticación JWT integrada
- ✅ Actualización parcial de campos
- ✅ Timestamp automático con GMT-5
- ✅ Query dinámico basado en campos proporcionados
- ✅ Documentación completa en `API_ENDPOINTS.md`
- ✅ Logging y métricas de performance

---

**Estado:** ✅ **IMPLEMENTADO Y LISTO PARA PRODUCCIÓN**

**Última Actualización:** 2025-10-04  
**Autor:** Sistema de desarrollo automatizado  
**Revisión:** Pendiente de testing en ambiente de staging
