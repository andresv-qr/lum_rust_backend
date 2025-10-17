# Lüm API v4 - Rust Implementation
## Documentación Completa de Endpoints

**🎉 IMPLEMENTACIÓN COMPLETA FINALIZADA - SISTEMA PRODUCTIVO**

**Servidor Principal:** Puerto 8000 (Rust)
**Servidor Fallback QR:** Puerto 8008 (Python QReader API)  
**Arquitectura:** Sistema Híbrido Rust + Python con Pipeline QR Completo + Encuestas + Unified Authentication
**Estado:** Production Ready - Sistema Multi-Capa: 3 Detectores Rust + **2 Modelos ONNX ML** + **Python QReader Optimizado** + Encuestas + OAuth2
**Fecha:** 2025-09-19

---

## 🔐 UNIFIED AUTHENTICATION SYSTEM (NEW)

### POST /api/v4/auth/unified
**Purpose**: Single endpoint for all authentication methods (Google OAuth2, Email)

**Features**:
- ✅ Google OAuth2 ID token validation
- ✅ Email/password authentication  
- ✅ Automatic account creation
- ✅ Account linking with conflict detection
- ✅ Comprehensive audit logging
- ✅ Rate limiting and security
- ✅ JWT token generation

**Request Body**:
```json
{
  "provider": "email|google",
  "email": "user@example.com",     // For email provider
  "password": "secure_password",   // For email provider  
  "name": "User Name",            // Optional for registration
  "id_token": "google_jwt_token", // For Google provider
  "create_if_not_exists": true,   // Create account if not exists
  "linking_token": "abc123",      // For account linking flows
  "client_info": {                // Optional client metadata
    "user_agent": "...",
    "ip_address": "...",
    "device_id": "...",
    "app_version": "..."
  }
}
```

**Response Types**:

**Success (200)**:
```json
{
  "status": "success",
  "user": {
    "id": 123,
    "email": "user@example.com",
    "name": "User Name",                    // ✅ NUEVO: Traído de la BD
    "avatar_url": null,                     // Campo disponible para futuro
    "providers": ["email"],                 // ✅ ACTUALIZADO: Desde auth_providers BD
    "primary_provider": "email",            // ✅ ACTUALIZADO: Desde last_login_provider BD
    "email_verified": true,                 // ✅ ACTUALIZADO: Basado en email_verified_at BD
    "account_status": "Active",             // ✅ ACTUALIZADO: Inferido de is_active BD
    "created_at": "2025-09-19T12:48:36Z",  // ✅ ACTUALIZADO: Desde created_at BD
    "last_login_at": "2025-09-19T12:48:36Z" // ✅ ACTUALIZADO: Desde last_login_at BD
  },
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",  // JWT válido por 24h
  "expires_at": "2025-09-20T12:48:36Z",
  "metadata": {
    "request_id": "uuid-here",
    "provider_used": "email",
    "is_new_user": false,
    "linking_performed": false,
    "execution_time_ms": 891,              // ✅ Tiempo real de BD
    "timestamp": "2025-09-19T12:48:36Z"
  }
}
```

**Account Linking Required (200)**:
```json
{
  "status": "account_linking_required",
  "message": "Email already exists with different provider",
  "linking_token": "temp_token_here",
  "expires_at": "2025-09-19T...",
  "existing_providers": ["email"],
  "new_provider": "google",
  "metadata": { ... }
}
```

### GET /api/v4/auth/unified/health
**Purpose**: Health check for unified auth system

### GET /api/v4/auth/unified/config  
**Purpose**: Configuration info (debugging)

---

## 📖 **EJEMPLOS DE USO - AUTENTICACIÓN UNIFICADA**

### **Caso 1: Registro de nuevo usuario con email**
```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "email": "nuevo@empresa.com",
    "password": "MiPassword123!",
    "name": "Juan Pérez",
    "create_if_not_exists": true,
    "client_info": {
      "ip": "192.168.1.100",
      "user_agent": "Mozilla/5.0...",
      "device_id": "mobile-app-v1.2"
    }
  }'
```

**Respuesta esperada**: Usuario creado con token JWT válido por 24h.

### **Caso 2: Login con email existente**
```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "email": "usuario@empresa.com",
    "password": "MiPassword123!",
    "create_if_not_exists": false,
    "client_info": {
      "ip": "192.168.1.100",
      "user_agent": "Mozilla/5.0...",
      "device_id": "web-app-v2.1"
    }
  }'
```

**Respuesta esperada**: Token JWT + datos completos del usuario (nombre, fecha creación, etc.)

### **Caso 3: Autenticación con Google OAuth2**
```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "google",
    "id_token": "eyJhbGciOiJSUzI1NiIsImtpZCI6IjE2N...",
    "create_if_not_exists": true,
    "client_info": {
      "ip": "192.168.1.100",
      "user_agent": "Mobile App",
      "device_id": "android-app-v1.0"
    }
  }'
```

**Respuesta esperada**: Usuario creado/autenticado con datos de Google + token JWT.

### **Caso 4: Manejo de errores - Credenciales inválidas**
```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "email": "usuario@empresa.com",
    "password": "password_incorrecto",
    "create_if_not_exists": false
  }'
```

**Respuesta esperada**:
```json
{
  "status": "error",
  "message": "Invalid credentials",
  "error_code": "AUTH_FAILED",
  "retry_after": null,
  "metadata": {
    "request_id": "uuid-here",
    "provider_used": "unknown",
    "execution_time_ms": 891,
    "timestamp": "2025-09-19T..."
  }
}
```

### **Campos de Usuario Disponibles**
La API ahora retorna todos los campos disponibles del usuario desde la base de datos:

- ✅ **name**: Nombre completo del usuario (desde BD)
- ✅ **email**: Email verificado 
- ✅ **providers**: Array de proveedores de autenticación (email, google, etc.)
- ✅ **primary_provider**: Último proveedor usado para login
- ✅ **email_verified**: Boolean basado en email_verified_at
- ✅ **account_status**: Estado de la cuenta (Active, Suspended, etc.)
- ✅ **created_at**: Fecha de creación real desde BD
- ✅ **last_login_at**: Última fecha de login desde BD
- 🔄 **avatar_url**: Campo preparado para implementación futura
- 🔄 **country_residence**: Disponible en BD, se puede añadir si es necesario
- 🔄 **date_of_birth**: Disponible en BD, se puede añadir si es necesario
- 🔄 **trust_score**: Disponible en BD (puntaje de confianza)

### **Beneficios para el Negocio**
1. **Single Sign-On (SSO)**: Un solo endpoint para todas las autenticaciones
2. **Datos Completos**: Información real del usuario desde la primera llamada
3. **Seguridad**: Tokens JWT con expiración, validación robusta
4. **Escalabilidad**: Compatible con múltiples proveedores de autenticación
5. **Auditoría**: Metadatos completos para seguimiento y debugging
6. **Performance**: Tiempo de respuesta optimizado (< 1000ms típicamente)

---

---

## 🚀 **CARACTERÍSTICAS IMPLEMENTADAS**

### **🔍 Pipeline QR Híbrido Avanzado:**
- ✅ **3 Detectores Rust Nativos** - rqrr, quircs, rxing optimizados (~5-15ms)
- ✅ **2 Modelos ONNX ML Activos** - Small (94% precisión), Medium (96% precisión) (~100-150ms) 
- ✅ **Python QReader Fallback** - API optimizada puerto 8008 (~255ms, 3.9 RPS, 100% éxito)
  - **Hybrid Detection Engine**: CV2 + PYZBAR + QReader Small + Medium
  - **PyTorch Optimizado**: inference_mode(), singleton pattern, memoria eficiente
  - **Validado**: 400+ requests, concurrencia 100 usuarios, 91% menos memoria
- ✅ **Detección Cascada** - Optimizado por velocidad (5ms - 500ms)

### **📊 Observabilidad & Monitoreo:**
- ✅ **Métricas Prometheus** - `/metrics` endpoint completo
- ✅ **Health Checks Detallados** - `/health/detailed` con dependencias
- ✅ **Pipeline QR Health** - `/api/v4/qr/health` específico
- ✅ **Headers de Performance** - X-Response-Time-Ms, X-RateLimit-*

### **⚡ Cache & Performance:**
- ✅ **ETag/If-None-Match** - Respuestas 304 automáticas
- ✅ **Cache Redis Inteligente** - TTL dinámico, invalidación selectiva
- ✅ **Cache por Detector** - Optimización específica por algoritmo
- ✅ **Versionado de Cache** - Cache keys versionadas

### **🔒 Seguridad Avanzada:**
- ✅ **Rate Limiting Granular** - Por endpoint, IP y usuario
- ✅ **Headers de Seguridad** - CSRF, XSS, HSTS, etc.
- ✅ **Validación MIME** - Upload seguro de imágenes
- ✅ **Idempotencia** - Prevención operaciones duplicadas

### **📋 Sistema de Encuestas:**
- ✅ **Auto-asignación Inteligente** - Targeting automático por grupos/usuarios
- ✅ **Respuestas Parciales** - Guardado en progreso
- ✅ **Auto-scoring** - Cálculo automático de puntajes
- ✅ **Tracking Temporal** - Control de tiempos y intentos
- ✅ **4 Encuestas Panamá** - Estudio de mercado completo

### **🎮 Sistema de Gamificación:**
- ✅ **Sistema de Lumis (XP)** - Puntos de experiencia por acciones
- ✅ **Niveles Dinámicos** - Progresión automática con beneficios
- ✅ **Streaks Inteligentes** - Rastreo de actividades diarias
- ✅ **Misiones Temporales** - Desafíos diarios/semanales/mensuales
- ✅ **Eventos Happy Hour** - Multiplicadores temporales
- ✅ **Logros & Badges** - Sistema de reconocimientos
- ✅ **Leaderboards** - Tablas de posición dinámicas
- ✅ **Anti-Gaming** - Detección de fraude y gaming del sistema

---

---

## 📊 **MONITORING & OBSERVABILITY ENDPOINTS**

### **Basic Health Check**
```http
GET /health
```
**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "service": "lum_rust_ws"
}
```

### **Detailed Health Check**
```http
GET /health/detailed
```
**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "0.1.0",
  "uptime_seconds": 86400,
  "database": {
    "status": "healthy",
    "connection_pool_size": 10,
    "active_connections": 3,
    "last_query_duration_ms": 23
  },
  "redis": {
    "status": "healthy", 
    "connection_count": 5,
    "last_ping_duration_ms": 1
  },
  "memory_usage": {
    "allocated_bytes": 1048576,
    "heap_size_bytes": 2097152,
    "peak_allocated_bytes": 1572864
  }
}
```

### **Prometheus Metrics**
```http
GET /metrics
Content-Type: text/plain; version=0.0.4
```
**Response:**
```
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",status="200"} 1234
http_requests_total{method="POST",status="200"} 567

# HELP http_request_duration_seconds HTTP request duration in seconds
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_sum 45.2
http_request_duration_seconds_count 1890

# HELP database_connections_active Active database connections
# TYPE database_connections_active gauge
database_connections_active 3
```

### **JSON Metrics**
```http
GET /metrics/json
```
**Response:**
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "service": "lum_rust_ws",
  "version": "0.1.0",
  "metrics": {
    "http_requests": {
      "total": 1890,
      "success_rate": 0.953,
      "avg_duration_ms": 23.9,
      "p95_duration_ms": 87.2,
      "p99_duration_ms": 156.8
    },
    "database": {
      "pool_size": 10,
      "active_connections": 3,
      "query_count": 15420,
      "avg_query_duration_ms": 12.5
    },
    "business_metrics": {
      "invoices_processed_today": 1250,
      "qr_codes_detected_today": 890,
      "user_sessions_active": 45
    }
  }
}
```

### **Kubernetes Probes**
```http
GET /ready   # Readiness probe
GET /live    # Liveness probe
```

## 🔍 **OBSERVABILITY HEADERS**

Todos los endpoints ahora incluyen headers automáticos:

```http
X-Response-Time-Ms: 156          # Tiempo de respuesta en ms
X-RateLimit-Limit-Hour: 1000     # Límite por hora
X-RateLimit-Remaining-Hour: 847  # Requests restantes por hora
X-RateLimit-Limit-Day: 10000     # Límite por día
X-RateLimit-Remaining-Day: 8653  # Requests restantes por día
ETag: "d4f2c8e1a0b3f7d9"        # Para cache validation
Cache-Control: private, max-age=300, must-revalidate
```

### Analytics de Migración
- Registro completo de uso v3 con IP, User-Agent, timestamps
- Métricas almacenadas en Redis por 30 días
- Contadores diarios y por endpoint para análisis

---

## 🔗 WhatsApp Webhook Endpoints

### Verificación del Webhook
- **Endpoint:** `GET /webhookws`
- **Descripción:** Verificación del webhook de WhatsApp para configuración inicial
- **Parámetros:** Query parameters de verificación de WhatsApp
- **Respuesta:** Challenge token para verificación

### Recepción de Mensajes
- **Endpoint:** `POST /webhookws`
- **Descripción:** Recibe y procesa mensajes de WhatsApp (texto, imágenes, documentos)
- **Body:** JSON con estructura de mensaje de WhatsApp
- **Funcionalidades:**
  - Procesamiento de comandos de texto
  - Detección QR en imágenes
  - OCR de facturas
  - Gestión de estados de usuario
  - Sistema de recompensas

### Estadísticas del Webhook
- **Endpoint:** `GET /webhook/stats`
- **Descripción:** Métricas y estadísticas del sistema de webhook
- **Respuesta:** JSON con contadores y performance metrics

---

## 🌐 REST API Endpoints (v4) - OPTIMIZADO

### 🚀 Optimizaciones v4 Implementadas

#### Caching Inteligente
- **Redis Caching:** Automático para todos los endpoints GET v4
- **TTL Diferenciado:**
  - User profiles: 5 minutos (300s)
  - Invoice data: 10 minutos (600s)
  - QR health: 1 minuto (60s)
  - System info: 30 minutos (1800s)
- **Headers:** `X-Cache: HIT/MISS`, `Cache-Control: public, max-age=XXX`
- **Cache Keys:** SHA256 hash de path + query + user context

#### HTTP Caching Avanzado (ETag + 304) ✅
- **Soportado en:** `GET /api/v4/invoices/details` y `GET /api/v4/invoices/headers` (unificado)
- **Headers Enviados:** `ETag` (weak hash W/"<16bytes>")
- **Headers Cliente:** `If-None-Match: <etag>`
- **Flujo:**
  1. Cliente guarda `ETag` de respuesta previa.
  2. Re-envía petición con `If-None-Match`.
  3. Si contenido no cambió → `304 Not Modified` sin body (ahorro de ancho de banda).
- **Control de Invalidez:** Versión por usuario (`inv:v:{user_id}`) incrementada en inserciones → fuerza nuevo payload y nuevo `ETag`.
- **Beneficio:** Reducción de payloads repetidos / latencia perceptible.
- **Nota:** Ruta interna legacy `GET /api/v4/invoice_headers/search` eliminada (ahora redirigida / deprecada) para evitar duplicidad.

#### Invalidation Dirigida por Versión (Namespace Versioning) ✅
- **Clave de versión por usuario:** `inv:v:{user_id}` (Redis integer)
- **Se incrementa cuando:** Procesamiento de nueva factura (URL u OCR) exitoso.
- **Uso en Cache Keys:** Prefijo versión → cuando incrementa, todas las páginas previas quedan obsoletas sin necesidad de barrido (`SETEX` natural expira residuos).
- **Ventaja:** Invalidación O(1) sin SCAN masivo.

#### Compresión Automática
- **Gzip:** Automático para respuestas > 1KB
- **Content-Types:** JSON y text responses
- **Headers:** `Content-Encoding: gzip`, `Vary: Accept-Encoding`

#### Monitoreo de Performance
- **Métricas en Tiempo Real:** Todas las respuestas v4
- **Headers:** `X-Response-Time: XXXms` en cada respuesta
- **Alertas:** Log automático para respuestas > 1000ms
- **Storage:** Redis (1 hora TTL) + performance_manager

#### Rate Limiting Runtime (Middleware) ✅ EXPANDIDO
- **Middleware:** `rate_limit_middleware` (Redis, ventanas deslizantes hora/día).
- **Scope Expandido:** 
  - **Endpoints Auth:** `POST /api/v4/auth/login` (3/hora, 10/día), `POST /api/v4/auth/register` (5/hora, 20/día)
  - **Endpoints QR:** `POST /api/v4/qr/detect` (30/hora, 300/día)
  - **Endpoints Encuestas:** `GET /api/v4/surveys` (60/hora), `GET /api/v4/surveys/{id}` (120/hora), `PATCH /api/v4/surveys/responses` (30/hora por encuesta)
  - **Endpoints Mutantes:** `POST /api/v4/invoices/process-from-url`, `POST /api/v4/invoices/upload-ocr` (dinámico por trust score)
  - **Otros endpoints:** 100/hora, 1000/día (generoso para GET)
- **Límites Dinámicos:** Basados en trust score para endpoints de procesamiento.
- **IP-Based Auth:** Login/register usan IP como identificador (antes de JWT).
- **Headers de Observabilidad ✅ IMPLEMENTADOS:**
  - `X-RateLimit-Limit-Hour`: Límite por hora para el endpoint
  - `X-RateLimit-Remaining-Hour`: Requests restantes en la ventana horaria
  - `X-RateLimit-Limit-Day`: Límite por día para el endpoint
  - `X-RateLimit-Remaining-Day`: Requests restantes en la ventana diaria
- **Respuesta al Exceso:** `429 Too Many Requests`
```json
{
  "error": "RATE_LIMIT_HOURLY",
  "message": "Hourly limit 3 exceeded",
  "details": "retry_after=123s"
}
```
- **Claves Redis:** `rl:<scope>:u:<user_id_or_ip>:h:<YYYYMMDDHH>` y `rl:<scope>:u:<user_id_or_ip>:d:<YYYYMMDD>`

#### Validación de Uploads y MIME ✅ NUEVO
- **Middleware:** `validate_upload_middleware` aplicado a endpoints de upload.
- **Scope:** `POST /api/v4/invoices/upload-ocr`, `POST /api/v4/qr/detect`
- **Validaciones:**
  - **Tamaño máximo:** 10MB por archivo
  - **MIME types permitidos:** image/jpeg, image/png, image/gif, image/webp, application/pdf, application/json
  - **Magic bytes:** Validación de contenido real vs header declarado
  - **Filename safety:** No paths relativos (../), caracteres especiales
- **Respuestas de Error:**
  - `415 Unsupported Media Type` para tipos no permitidos
  - `413 Payload Too Large` para archivos > 10MB
  - `400 Bad Request` para filename inseguro o type mismatch
- **Headers:** Content-Type y Content-Length validados antes de procesamiento

#### Idempotencia para Operaciones Mutantes ✅ NUEVO
- **Middleware:** `idempotency_middleware` aplicado a endpoints mutantes.
- **Scope:** `POST /api/v4/invoices/process-from-url`, `POST /api/v4/invoices/upload-ocr`
- **Header Requerido:** `Idempotency-Key` (obligatorio)
- **TTL:** 24 horas en Redis
- **Claves Redis:** `idem:<path>:<key>` → `<status_code>|<json_body>`
- **Headers Respuesta:**
  - `X-Idempotent-Replay: true` (respuesta cache)
  - `X-Idempotent-Replay: false` (respuesta nueva)
- **Beneficio:** Prevención de procesamiento duplicado, operaciones seguras on retry.

#### Formato Estándar de Errores ✅
- **Estructura JSON:** `{ "error": "<CODE>", "message": "Descripción legible", "details": "opcional" }`
- **Ejemplos:**
  - Auth faltante → `{"error":"AUTH_REQUIRED","message":"Authentication required"}`
  - Token inválido → `{"error":"INVALID_TOKEN","message":"JWT invalid or expired"}`
  - Rate limit → ver arriba
  - Validación → `{"error":"VALIDATION_ERROR","message":"Campo X inválido"}`
- **Ventaja:** Uniformidad para clientes (manejo centralizado).

#### Roadmap Próximo (Inmediato)
- ✅ **Idempotencia implementada** para operaciones mutantes con `Idempotency-Key`
- ✅ **Rate limiting granular** por endpoint (auth, QR, invoices con políticas específicas)
- ✅ **Validación MIME avanzada** con magic bytes y filename safety
- ✅ **Security headers mejorados** (CSP, Permissions-Policy, preload HSTS)
- ✅ **Headers de observabilidad** expuestos en todas las respuestas (X-RateLimit-*)
- ✅ **Keyset pagination (cursor)** implementado en detalles de facturas
- 🔄 **Próximo:** Extender keyset pagination a headers de facturas
- 🔄 **Próximo:** Documentar ejemplos de 304/ETag en sección facturas

### Root
#### Información de la API
- **Endpoint:** `GET /`
- **Descripción:** Información básica de la API y health check
- **Respuesta:** JSON con información del sistema

---

### 🔐 Autenticación v4

#### Login de Usuario ✅ MIGRADO + JWT IMPLEMENTADO + RATE LIMITING
- **Endpoint:** `POST /api/v4/auth/login`
- **Descripción:** Autenticación de usuario con JWT (compatible con frontend)
- **Rate Limiting:** 3 intentos/hora, 10/día por IP
- **Body:** 
```json
{
  "email": "usuario@ejemplo.com",
  "password": "contraseña123",
  "remember_me": false
}
```
- **Respuesta:** TokenResponse directo (compatible con v3)
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "bearer",
  "expires_in": 86400,
  "user_id": 123,
  "email": "usuario@ejemplo.com"
}
```
- **Características:**
  - JWT tokens seguros con HS256
  - Expiración configurable (24h normal, 7 días remember_me)
  - **Rate limiting estricto por IP** (protección anti-brute force)
  - Validación de contraseñas con bcrypt
  - Logging completo de intentos
  - Formato compatible con cliente frontend

#### Registro de Usuario ✅ IMPLEMENTADO + JWT AUTOMÁTICO
- **Endpoint:** `POST /api/v4/auth/register`
- **Descripción:** Registro de nuevo usuario con validación completa y token JWT automático
- **Rate Limiting:** 5 registros/hora, 15/día por IP
- **Body:**
```json
{
  "email": "nuevo@ejemplo.com",
  "password": "MiContraseña123!",
  "name": "Juan Pérez",
  "phone": "+507 6123-4567",
  "country": "PA"
}
```
- **Respuesta Exitosa:** TokenResponse directo (usuario registrado y autenticado automáticamente)
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "bearer", 
  "expires_in": 86400,
  "user_id": 456,
  "email": "nuevo@ejemplo.com"
}
```
- **Errores Comunes:**
```json
{
  "error": "EMAIL_EXISTS",
  "message": "Email already registered"
}
```
```json
{
  "error": "VALIDATION_ERROR", 
  "message": "Invalid input data",
  "details": {
    "password": ["Password must be at least 8 characters", "Password must contain uppercase letter"],
    "email": ["Invalid email format"]
  }
}
```
- **Validaciones:**
  - Email: Formato válido, único en sistema
  - Contraseña: Mínimo 8 caracteres, mayúscula, minúscula, número
  - Nombre: 1-100 caracteres 
  - Teléfono: Formato internacional opcional
  - País: Código ISO de 2 letras
- **Características:**
  - Validación robusta con mensajes específicos
  - Hash de contraseña con bcrypt
  - JWT token automático tras registro exitoso
  - Sanitización de datos de entrada
  - Logging completo con request_id
  - Usuario creado con estado activo por defecto

#### Envío de Código de Verificación ✅ UNIFICADO + PÚBLICO
- **Endpoint:** `POST /api/v4/users/send-verification` *(Compatible - redirige a sistema unificado)*
- **Endpoint Unificado:** `POST /api/v4/passwords/request-code` con `purpose: "email_verification"`
- **Descripción:** Enviar código de verificación por email (sistema unificado PostgreSQL)
- **Autenticación:** ❌ **NO requiere JWT** (endpoint público)
- **Uso:** Pre-autenticación (usuario aún no está logueado)
- **Body:**
```json
{
  "email": "usuario@example.com",
  "method": "email"  // opcional: "email" o "whatsapp" (whatsapp no implementado aún)
}
```
- **Respuesta Exitosa:**
```json
{
  "success": true,
  "method": "email",
  "message": "Verification code sent successfully"
}
```
- **Respuesta de Error:**
```json
{
  "success": false,
  "error": "User not found"
}
```
- **Características:**
  - Genera código de 6 dígitos aleatorio
  - Almacena en Redis con TTL de 1 hora
  - Determina tipo de código: `reset_password` o `set_password`
  - Métodos soportados: Email (SendGrid API o SMTP)
  - Fallback a simulación si no hay configuración de email
  - Validación de email requerida
  - Logging completo con request_id

#### Verificación de Código ✅ UNIFICADO + PÚBLICO
- **Endpoint:** `POST /api/v4/users/verify-account` *(Compatible - usa sistema unificado)*
- **Descripción:** Verificar código de verificación recibido por email (sistema unificado PostgreSQL)
- **Autenticación:** ❌ **NO requiere JWT** (endpoint público)
- **Uso:** Pre-autenticación (confirmar email sin establecer contraseña)
- **Body:**
```json
{
  "email": "usuario@example.com",
  "verification_code": "123456"
}
```
- **Respuesta Exitosa:**
 ```json
{
  "success": true,
  "user_id": 123,
  "message": "Account verified successfully"
}
```
- **Respuesta de Error:**
```json
{
  "success": false,
  "error": "Invalid verification code"
}
```
- **Características (Sistema Unificado):**
  - ✅ **Almacenamiento:** PostgreSQL (tabla `password_verification_codes`)
  - ✅ **Propósito:** `email_verification` (solo verificar email)
  - ✅ **Límite:** 3 intentos fallidos por código
  - ✅ **Código de uso único:** Se marca como usado tras verificación exitosa
  - ✅ **Validación:** Case-insensitive de emails
  - ✅ **Control de expiración:** Automática con timestamps
  - ✅ **Rate limiting:** Máximo 3 códigos por hora por email
  - ✅ **Retorna:** user_id, email verificado, timestamp

#### Establecer Contraseña con Código de Email ✅ NUEVO + PÚBLICO
- **Endpoint:** `POST /api/v4/users/set-password-with-email-code`
- **Descripción:** Establecer contraseña usando código de `send-verification` (mismo código)
- **Autenticación:** ❌ **NO requiere JWT** (endpoint público)
- **Uso:** Flujo completo email + contraseña con UN SOLO código ⭐
- **Body:**
```json
{
  "email": "usuario@example.com",
  "verification_code": "123456",
  "new_password": "MiNuevaContraseña123!",
  "confirmation_password": "MiNuevaContraseña123!"
}
```
- **Respuesta Exitosa:**
```json
{
  "success": true,
  "data": {
    "user_id": 42,
    "email": "usuario@example.com",
    "email_verified": true,
    "password_set": true,
    "password_updated_at": "2025-09-26T15:30:00Z",
    "login_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```
- **Características:**
  - ✅ **Flujo optimal:** send-verification → set-password-with-email-code (2 pasos)
  - ✅ **Un código:** Mismo código verifica email Y establece contraseña
  - ✅ **Auto-login:** Retorna JWT token para login inmediato
  - ✅ **Validaciones:** Password strength, confirmación, usuario sin contraseña
  - ✅ **Seguridad:** Rate limiting, expiración, intentos máximos
  - ✅ **Purpose:** Usa códigos con `purpose="email_verification"`

#### Verificación de Cuenta (LEGACY)
- **Endpoint:** `POST /api/v4/auth/verify`
- **Descripción:** Verificar cuenta con código de verificación (endpoint legacy)
- **Body:** JSON con código
- **Respuesta:** Confirmación de verificación
- **Estado:** ⚠️ DEPRECADO - Usar `/api/v4/verify-account` en su lugar

---

### 👥 Gestión de Usuarios v4

#### Verificar Disponibilidad de Email ✅ MIGRADO
- **Endpoint:** `POST /api/v4/users/check-email`
- **Descripción:** Verificar si un email está disponible para registro
- **Body:** `{"email": "test@ejemplo.com"}`
- **Respuesta:** `{"available": true/false, "message": "..."}`
- **Optimizaciones:** Caching (5min), validación mejorada

#### Perfil de Usuario ✅ MIGRADO + JWT PROTEGIDO
- **Endpoint:** `GET /api/v4/users/profile`
- **Descripción:** Obtener perfil del usuario autenticado
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Respuesta:** Perfil completo del usuario (sin datos sensibles)
- **Características:**
  - Autenticación JWT obligatoria
  - Datos seguros (sin password_hash)
  - Caching inteligente (5min)
  - Logging de accesos

#### Perfil de Usuario por ID ✅ MIGRADO + JWT PROTEGIDO
- **Endpoint:** `GET /api/v4/users/profile/:id`
- **Descripción:** Obtener perfil de usuario específico (admin o propio)
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Parámetros:** `id` - ID del usuario
- **Respuesta:** Perfil del usuario solicitado
- **Características:**
  - Autenticación JWT obligatoria
  - Control de acceso (solo perfil propio o admin)
  - Validación de permisos automática

#### Datos de Usuario desde Dimensión ✅ NUEVO + JWT PROTEGIDO
- **Endpoint:** `GET /api/v4/userdata`
- **Descripción:** Obtener datos demográficos del usuario desde public.dim_users
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Respuesta:** Datos demográficos del usuario autenticado
- **Campos retornados:**
  - `name` (`string` | `null`) - Nombre completo (character varying)
  - `email` (`string` | `null`) - Correo electrónico (text)
  - `date_of_birth` (`string` | `null`) - Fecha de nacimiento (character varying)
  - `country_origin` (`string` | `null`) - País de origen (character varying)
  - `country_residence` (`string` | `null`) - País de residencia (character varying)
  - `segment_activity` (`string` | `null`) - Segmento de actividad (character varying)
  - `genre` (`string` | `null`) - Género del usuario (character varying)
  - `ws_id` (`string` | `null`) - ID de WhatsApp (text)
  - `updated_at` (`string` | `null`, ISO 8601) - Última actualización (timestamp with time zone)

- **Ejemplo de Respuesta:**
```json
{
  "success": true,
  "data": {
    "name": "Juan Carlos Pérez",
    "email": "juan.perez@example.com",
    "date_of_birth": "1985-03-15",
    "country_origin": "Panama",
    "country_residence": "Panama",
    "segment_activity": "Retail",
    "genre": "M",
    "ws_id": "507-1234-5678",
    "updated_at": "2025-08-15T10:30:45Z"
  },
  "error": null,
  "request_id": "af259f7f-96ad-4175-a46e-3105465b627b",
  "timestamp": "2025-08-18T15:30:00Z",
  "execution_time_ms": 12,
  "cached": false
}
```

- **Características:**
  - Autenticación JWT obligatoria
  - Datos desde tabla public.dim_users
  - Estructura ApiResponse estándar v4
  - Manejo de usuarios sin datos (respuesta vacía)
  - Logging de accesos y métricas de performance
  - Datos seguros sin información sensible

#### Actualizar Datos de Usuario ✅ NUEVO + JWT PROTEGIDO
- **Endpoint:** `PUT /api/v4/userdata`
- **Descripción:** Actualizar datos demográficos del usuario en public.dim_users
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Método:** `PUT`
- **Content-Type:** `application/json`

- **Body (JSON):** Todos los campos son opcionales. Solo los campos enviados serán actualizados.
```json
{
  "name": "Juan Carlos Pérez",
  "date_of_birth": "1985-03-15",
  "country_origin": "Panama",
  "country_residence": "Panama",
  "segment_activity": "Retail",
  "genre": "M",
  "ws_id": "507-1234-5678"
}
```

- **Campos actualizables:**
  - `name` (`string` | `null`) - Nombre completo
  - `date_of_birth` (`string` | `null`) - Fecha de nacimiento (formato libre)
  - `country_origin` (`string` | `null`) - País de origen
  - `country_residence` (`string` | `null`) - País de residencia
  - `segment_activity` (`string` | `null`) - Segmento de actividad
  - `genre` (`string` | `null`) - Género (M/F/Otro)
  - `ws_id` (`string` | `null`) - ID de WhatsApp

**NOTA:** El campo `email` NO es actualizable desde este endpoint por seguridad.

- **Ejemplo de Request:**
```bash
curl -X PUT "https://api.example.com/api/v4/userdata" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "name": "María Rodríguez",
    "country_residence": "Colombia",
    "segment_activity": "Technology"
  }'
```

- **Ejemplo de Respuesta Exitosa (200 OK):**
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

- **Códigos de Error:**
  - `400 BAD REQUEST` - No se proporcionaron campos para actualizar
  - `401 UNAUTHORIZED` - Token JWT inválido o ausente
  - `404 NOT FOUND` - Usuario no existe en la base de datos
  - `500 INTERNAL SERVER ERROR` - Error de base de datos

- **Características:**
  - ✅ Autenticación JWT obligatoria
  - ✅ Actualización parcial (solo campos enviados se actualizan)
  - ✅ Campo `updated_at` se actualiza automáticamente con timezone GMT-5
  - ✅ Retorna datos actualizados completos después del UPDATE
  - ✅ Validación de usuario existente
  - ✅ Query dinámico construido solo con campos proporcionados
  - ✅ Logging detallado de operaciones
  - ✅ Métricas de performance incluidas
  - ⚠️ El campo `email` no es actualizable por seguridad

- **Comportamiento del Timestamp:**
  - El campo `updated_at` se actualiza automáticamente en cada operación PUT
  - Formato: `timestamp with time zone` en PostgreSQL
  - Timezone: GMT-5 (Panama/Colombia)
  - Se retorna en formato ISO 8601 en la respuesta

#### Cambiar Contraseña (Directo) ✅ NUEVO + JWT PROTEGIDO
- **Endpoint:** `PUT /api/v4/userdata/password`
- **Descripción:** Cambiar contraseña del usuario autenticado con verificación de contraseña actual
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Método:** `PUT`
- **Content-Type:** `application/json`

**🎯 Ventajas vs Flujo de Email:**
- ✅ Un solo request (más rápido)
- ✅ No requiere acceso al email
- ✅ Doble verificación: JWT + contraseña actual
- ✅ Mejor UX para usuarios que conocen su contraseña

**Body (JSON):**
```json
{
  "current_password": "ContraseñaActual123!",
  "new_password": "NuevaContraseña456!",
  "confirmation_password": "NuevaContraseña456!"
}
```

**Validaciones de Contraseña:**
- ✅ **Longitud:** 8-128 caracteres
- ✅ **Mayúsculas:** Al menos 1 letra mayúscula
- ✅ **Minúsculas:** Al menos 1 letra minúscula
- ✅ **Números:** Al menos 1 dígito
- ✅ **Caracteres Especiales:** Al menos 1 de `!@#$%^&*()_+-=[]{}|;:,.<>?`
- ✅ **Confirmación:** Las contraseñas deben coincidir exactamente
- ✅ **Diferente:** Nueva contraseña debe ser diferente de la actual

**Ejemplo de Request:**
```bash
curl -X PUT "https://api.example.com/api/v4/userdata/password" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "MiPassword123!",
    "new_password": "MiNuevoPassword456!",
    "confirmation_password": "MiNuevoPassword456!"
  }'
```

**Ejemplo de Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "data": {
    "user_id": 42,
    "email": "usuario@ejemplo.com",
    "password_updated_at": "2025-10-04T10:45:30-05:00",
    "message": "Contraseña actualizada exitosamente"
  },
  "error": null,
  "request_id": "a1b2c3d4-5678-90ab-cdef-1234567890ab",
  "timestamp": "2025-10-04T15:45:30Z",
  "execution_time_ms": 234,
  "cached": false
}
```

**Códigos de Error:**

| Código | Descripción | Causa |
|--------|-------------|-------|
| **200** | ✅ Contraseña actualizada | Operación exitosa |
| **400** | ❌ Bad Request | Contraseñas no coinciden, no cumple requisitos, o nueva contraseña igual a actual |
| **401** | ❌ Unauthorized | Token JWT inválido o contraseña actual incorrecta |
| **404** | ❌ Not Found | Usuario no existe en la base de datos |
| **500** | ❌ Internal Server Error | Error de base de datos o servidor |

**Casos Especiales:**
- **Usuario OAuth (sin contraseña):** Retorna `400 BAD REQUEST` - El usuario debe usar el flujo de email para establecer una contraseña primero
- **Contraseña nueva = contraseña actual:** Retorna `400 BAD REQUEST`
- **Contraseña actual incorrecta:** Retorna `401 UNAUTHORIZED`

**Características de Seguridad:**
- ✅ **Doble Factor:** Requiere JWT válido + contraseña actual correcta
- ✅ **Hash Bcrypt:** Contraseña hasheada con bcrypt (cost=12)
- ✅ **Timestamp GMT-5:** Campo `updated_at` actualizado automáticamente
- ✅ **Validación Robusta:** Verifica fortaleza de contraseña antes de actualizar
- ✅ **Logging Completo:** Todos los intentos registrados con request_id
- ✅ **No Expone Hash:** Nunca retorna el hash de la contraseña
- ✅ **Audit Trail:** Cambios registrados en audit_logs

**Logging de Eventos:**
```
✅ SUCCESS: Password changed successfully - user_id: 42, execution_time: 234ms
⚠️  WARNING: Password confirmation mismatch - user_id: 42
⚠️  WARNING: Current password incorrect - user_id: 42
⚠️  WARNING: New password same as current - user_id: 42
❌ ERROR: User does not have password set (OAuth user) - user_id: 42
```

**Comparación de Métodos de Cambio de Contraseña:**

| Aspecto | PUT /userdata/password (Directo) | POST /passwords/request-code + set-with-code (Email) |
|---------|----------------------------------|-----------------------------------------------------|
| **Requests** | 1 | 2 |
| **Autenticación** | JWT + Contraseña actual | Email verification code |
| **Requiere Email** | ❌ No | ✅ Sí |
| **Velocidad** | ⚡ Rápido (1 request) | 🐢 Más lento (2 requests) |
| **Seguridad** | ⭐⭐⭐⭐ Alta | ⭐⭐⭐⭐⭐ Muy Alta |
| **UX** | ⭐⭐⭐⭐⭐ Excelente | ⭐⭐⭐ Buena |
| **Uso Recomendado** | Usuario conoce contraseña | Usuario olvidó contraseña |
| **Notificación** | Opcional (configurable) | Automática (email) |

**Flujos Recomendados:**

```
┌──────────────────────────────────────────┐
│ ¿Usuario conoce su contraseña actual?   │
└─────────────┬────────────────────────────┘
              │
     ┌────────┴────────┐
     │                 │
    SÍ                NO
     │                 │
     ▼                 ▼
┌─────────────┐   ┌──────────────────────┐
│ Método 1    │   │ Método 2             │
│ PUT         │   │ POST request-code    │
│ /password   │   │ + set-with-code      │
│ (Directo)   │   │ (Email recovery)     │
└─────────────┘   └──────────────────────┘
```

**Mejores Prácticas:**
1. **Para cambios rutinarios:** Usar endpoint directo (`PUT /userdata/password`)
2. **Para recuperación:** Usar flujo de email (`POST /passwords/request-code`)
3. **Para nuevos usuarios OAuth:** Usar flujo de email para establecer primera contraseña
4. **Rate Limiting:** Considerar límite de 5 intentos por hora por usuario
5. **Notificaciones:** Enviar email de confirmación después del cambio (opcional)

#### Obtener Emisores del Usuario ✅ NUEVO + JWT PROTEGIDO
- **Endpoint:** `GET /api/v4/invoices/issuers`
- **Descripción:** Obtener todos los emisores (companies) que tienen facturas asociadas con el usuario autenticado
- **Headers:** 
  - `Authorization: Bearer <jwt_token>` **REQUERIDO**
  - `Content-Type: application/json` (opcional)
  - `x-request-id: <unique-id>` (opcional, para tracing)

**Query Parameters:**

| Parameter | Type | Required | Default | Min | Max | Description |
|-----------|------|----------|---------|-----|-----|-------------|
| `limit` | `integer` | No | `20` | `1` | `100` | Número máximo de emisores a retornar por página |
| `offset` | `integer` | No | `0` | `0` | - | Número de emisores a omitir (para paginación) |
| `update_date_from` | `string` | No | - | - | - | Filtrar emisores actualizados desde esta fecha (ISO 8601) |

**Formatos de fecha aceptados para `update_date_from`:**
- `2024-01-15T10:00:00Z` (UTC)
- `2024-01-15T10:00:00-05:00` (Con timezone)
- `2024-01-15T10:00:00.123Z` (Con milisegundos)

**Tipos de Datos de Respuesta:**

| Campo | Tipo | Nullable | Descripción |
|-------|------|----------|-------------|
| `issuer_ruc` | `string` | Yes | RUC/Identificación fiscal del emisor |
| `issuer_name` | `string` | Yes | Nombre oficial registrado del emisor |
| `issuer_best_name` | `string` | Yes | Nombre comercial o "mejor nombre" del emisor |
| `issuer_l1` | `string` | Yes | Clasificación nivel 1 (sector principal) |
| `issuer_l2` | `string` | Yes | Clasificación nivel 2 (subsector) |
| `issuer_l3` | `string` | Yes | Clasificación nivel 3 (categoría específica) |
| `issuer_l4` | `string` | Yes | Clasificación nivel 4 (subcategoría) |
| `update_date` | `string` | Yes | Fecha de última actualización (ISO 8601) |

**Estructura de Paginación:**

| Campo | Tipo | Description |
|-------|------|-------------|
| `total` | `integer` | Total de emisores disponibles para el usuario |
| `limit` | `integer` | Límite aplicado en esta consulta |
| `offset` | `integer` | Offset aplicado en esta consulta |
| `has_next` | `boolean` | `true` si hay más resultados disponibles |
| `has_previous` | `boolean` | `true` si hay resultados anteriores |
| `total_pages` | `integer` | Total de páginas con el límite actual |
| `current_page` | `integer` | Página actual (basada en offset/limit) |

**Consulta SQL Implementada:**

*Sin filtro de fecha:*
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
    SELECT 1 FROM public.invoice_header ih 
    WHERE ih.user_id = $1 
    AND a.issuer_ruc = ih.issuer_ruc 
    AND a.issuer_name = ih.issuer_name
)
ORDER BY a.issuer_name ASC
LIMIT $2 OFFSET $3;
```

*Con filtro de fecha:*
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
    SELECT 1 FROM public.invoice_header ih 
    WHERE ih.user_id = $1 
    AND a.issuer_ruc = ih.issuer_ruc 
    AND a.issuer_name = ih.issuer_name
)
AND a.update_date >= $4
ORDER BY a.issuer_name ASC
LIMIT $2 OFFSET $3;
``` 
    AND a.issuer_name = ih.issuer_name
AND a.update_date >= $4
ORDER BY a.issuer_name ASC
LIMIT $2 OFFSET $3;
```

**Ejemplos de Request:**

```bash
# 1. Petición básica (sin filtros)
GET /api/v4/invoices/issuers?limit=10&offset=0
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...

# 2. Con filtro de fecha (emisores actualizados desde 2024)
GET /api/v4/invoices/issuers?limit=10&offset=0&update_date_from=2024-01-01T00:00:00Z
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...

# 3. Paginación - segunda página
GET /api/v4/invoices/issuers?limit=20&offset=20
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...

# 4. Filtro de fecha con paginación
GET /api/v4/invoices/issuers?limit=5&offset=10&update_date_from=2024-06-01T12:00:00Z
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```

**Ejemplo de Respuesta Exitosa (200 OK):**

```json
{
  "success": true,
  "data": {
    "issuers": [
      {
        "issuer_ruc": "155112341-2-DV",
        "issuer_name": "Super 99",
        "issuer_best_name": "Super99 Panamá - Líder en Retail",
        "issuer_l1": "Retail",
        "issuer_l2": "Supermercados",
        "issuer_l3": "Alimentación y Consumo",
        "issuer_l4": "General y Especializado",
        "update_date": "2024-08-10T14:30:00Z"
      },
      {
        "issuer_ruc": "155223456-1-DV",
        "issuer_name": "Farmacia Arrocha",
        "issuer_best_name": "Farmacias Arrocha - Salud y Bienestar",
        "issuer_l1": "Healthcare",
        "issuer_l2": "Farmacias",
        "issuer_l3": "Medicamentos y Productos de Salud",
        "issuer_l4": "Retail Farmacéutico",
        "update_date": "2024-07-25T16:45:00Z"
      },
      {
        "issuer_ruc": "155334567-3-DV", 
        "issuer_name": "Restaurante Casa Vegetariana",
        "issuer_best_name": "Casa Vegetariana - Comida Saludable",
        "issuer_l1": "Food & Beverage",
        "issuer_l2": "Restaurantes",
        "issuer_l3": "Comida Especializada",
        "issuer_l4": "Vegetariano/Vegano",
        "update_date": "2024-09-01T08:20:00Z"
      }
    ],
    "pagination": {
      "total": 25,
      "limit": 10,
      "offset": 0,
      "has_next": true,
      "has_previous": false,
      "total_pages": 3,
      "current_page": 1
    }
  },
  "error": null,
  "request_id": "user-issuers-f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "timestamp": "2025-09-13T15:30:45Z",
  "execution_time_ms": 42,
  "cached": false
}
```

**Ejemplo de Respuesta Vacía (200 OK):**

```json
{
  "success": true,
  "data": {
    "issuers": [],
    "pagination": {
      "total": 0,
      "limit": 20,
      "offset": 0,
      "has_next": false,
      "has_previous": false,
      "total_pages": 0,
      "current_page": 0
    }
  },
  "error": null,
  "request_id": "user-issuers-empty-12345",
  "timestamp": "2025-09-13T15:31:00Z",
  "execution_time_ms": 15,
  "cached": false
}
```

**Ejemplos de Respuestas de Error:**

*400 Bad Request (Formato de fecha inválido):*
```json
{
  "error": "BAD_REQUEST",
  "message": "Invalid date format in update_date_from parameter",
  "request_id": "user-issuers-error-400",
  "timestamp": "2025-09-13T15:32:00Z"
}
```

*401 Unauthorized (JWT faltante o inválido):*
```json
{
  "error": "UNAUTHORIZED", 
  "message": "Authentication required. Please provide a valid JWT token.",
  "request_id": "user-issuers-error-401",
  "timestamp": "2025-09-13T15:32:15Z"
}
```

*500 Internal Server Error (Error de base de datos):*
```json
{
  "error": "INTERNAL_SERVER_ERROR",
  "message": "Database query failed",
  "request_id": "user-issuers-error-500",
  "timestamp": "2025-09-13T15:32:30Z"
}
```

**Características Técnicas:**

**🔒 Seguridad y Autenticación:**
- Autenticación JWT obligatoria (user_id extraído automáticamente del token)
- Validación automática de permisos por usuario
- Headers de seguridad estándar v4
- Rate limiting compatible (estructura preparada)

**📊 Paginación y Filtrado:**
- Paginación estándar v4 con límites de seguridad (max 100 por página)
- Filtro opcional por fecha: `update_date_from` (ISO 8601)
- Validación estricta de formato de fecha (400 Bad Request si es inválida)
- Ordenamiento alfabético consistente por `i\ssuer_name`

**🏗️ Estructura de Datos:**
- Campos completos desde `public.dim_issuer`
- Clasificación jerárquica (L1→L2→L3→L4) para categorización avanzada
- Solo emisores con facturas asociadas al usuario (EXISTS optimization)
- Campos nullable para flexibilidad de datos

**⚡ Performance y Optimización:**
- Query optimizada con `DISTINCT + EXISTS` en lugar de JOINs costosos
- Índices aprovechados en `user_id`, `issuer_ruc`, `issuer_name`, `update_date`
- Paginación eficiente con `LIMIT/OFFSET`
- Consultas condicionales (ejecuta filtro de fecha solo cuando es necesario)

**📈 Observabilidad y Monitoreo:**
- Request ID único para tracing completo (`x-request-id`)
- Logging estructurado con contexto del usuario y filtros aplicados
- Headers de performance: `X-Response-Time-Ms`
- Métricas de ejecución incluidas en response
- Error tracking detallado para debugging

**💾 Caching y Persistencia:**
- Cache TTL configurado (10 minutos - emisores cambian poco)
- Cache key incluye user_id, pagination y filtros para precisión
- Estructura preparada para invalidación dirigida
- ApiResponse estándar v4 con flag de cache

**🔧 Integración y Compatibilidad:**
- Endpoint RESTful estándar siguiendo convenciones v4
- JSON response con estructura consistente
- Compatible con sistemas de monitoreo externos
- Headers CORS configurados automáticamente

**Casos de Uso Comunes:**

1. **Dashboard del Usuario:** Mostrar lista de empresas frecuentadas
   ```bash
   GET /api/v4/invoices/issuers?limit=10
   ```

2. **Análisis de Gastos por Sector:** Filtrar y categorizar por L1-L4
   ```bash
   GET /api/v4/invoices/issuers?limit=50&offset=0
   # Procesar client-side por issuer_l1, issuer_l2, etc.
   ```

3. **Auditoría de Datos Recientes:** Solo emisores actualizados recientemente
   ```bash
   GET /api/v4/invoices/issuers?update_date_from=2024-09-01T00:00:00Z
   ```

4. **Filtros de Facturas:** Permitir selección de empresa específica
   ```bash
   GET /api/v4/invoices/issuers  # Para llenar dropdown/lista
   ```

5. **Reporting Paginado:** Para exportar datos masivos
   ```bash
   GET /api/v4/invoices/issuers?limit=100&offset=0
   GET /api/v4/invoices/issuers?limit=100&offset=100
   # Continuar hasta has_next: false
   ```

---

#### Obtener Productos del Usuario ✅ NUEVO + JWT PROTEGIDO
- **Endpoint:** `GET /api/v4/invoices/products`
- **Descripción:** Obtener todos los productos distintos que el usuario ha comprado según sus facturas
- **Headers:** 
  - `Authorization: Bearer <jwt_token>` **REQUERIDO**
  - `Content-Type: application/json` (opcional)
  - `x-request-id: <unique-id>` (opcional, para tracing)

**Query Parameters:**

| Parameter | Type | Required | Default | Min | Max | Description |
|-----------|------|----------|---------|-----|-----|-------------|
| `update_date` | `string` | No | - | - | - | Filtrar productos actualizados desde esta fecha (ISO 8601) |

**Formatos de fecha aceptados para `update_date`:**
- `2024-01-15T10:00:00Z` (UTC)
- `2024-01-15T10:00:00-05:00` (Con timezone)
- `2024-01-15T10:00:00.123Z` (Con milisegundos)
- `2024-01-15` (Solo fecha, asume 00:00:00 UTC)

**Tipos de Datos de Respuesta:**

| Campo | Tipo | Nullable | Descripción |
|-------|------|----------|-------------|
| `code` | `string` | Yes | Código único del producto |
| `issuer_name` | `string` | Yes | Nombre del emisor de la factura |
| `description` | `string` | Yes | Descripción detallada del producto |
| `l1` | `string` | Yes | Clasificación nivel 1 (categoría principal) |
| `l2` | `string` | Yes | Clasificación nivel 2 (subcategoría) |
| `l3` | `string` | Yes | Clasificación nivel 3 (categoría específica) |
| `l4` | `string` | Yes | Clasificación nivel 4 (subcategoría específica) |
| `process_date` | `string` | Yes | Fecha de procesamiento del producto |

**SQL Query Ejecutada:**
```sql
-- Sin filtro de fecha
SELECT 
    p.code,
    p.issuer_name,
    p.description,
    p.l1_gemini as l1,
    p.l2_gemini as l2,
    p.l3_gemini as l3,
    p.l4_gemini as l4,
    p.process_date
FROM public.dim_product p
JOIN (
    SELECT DISTINCT d.code, h.issuer_name, d.description
    FROM public.invoice_detail d
    JOIN public.invoice_header h
      ON d.cufe = h.cufe
    WHERE h.user_id = $1
) u
  ON p.code = u.code
 AND p.issuer_name = u.issuer_name
 AND p.description = u.description
ORDER BY p.description ASC;

-- Con filtro de fecha
SELECT 
    p.code,
    p.issuer_name,
    p.description,
    p.l1_gemini as l1,
    p.l2_gemini as l2,
    p.l3_gemini as l3,
    p.l4_gemini as l4,
    p.process_date
FROM public.dim_product p
JOIN (
    SELECT DISTINCT d.code, h.issuer_name, d.description
    FROM public.invoice_detail d
    JOIN public.invoice_header h
      ON d.cufe = h.cufe
    WHERE h.user_id = $1
) u
  ON p.code = u.code
 AND p.issuer_name = u.issuer_name
 AND p.description = u.description
WHERE p.process_date >= $2
ORDER BY p.description ASC;
```

**Respuesta exitosa (200 OK):**
```json
{
  "success": true,
  "message": "Successfully retrieved user products",
  "data": [
    {
      "code": "PROD001",
      "issuer_name": "Super 99",
      "description": "Laptop Dell Inspiron 15",
      "l1": "Tecnología",
      "l2": "Computadoras",
      "l3": "Laptops",
      "l4": "Laptops Personales",
      "process_date": "2024-08-20"
    },
    {
      "code": "PROD002",
      "issuer_name": "Farmacia Arrocha",
      "description": "Vitamina C 1000mg",
      "l1": "Salud",
      "l2": "Suplementos",
      "l3": "Vitaminas",
      "l4": "Vitamina C",
      "process_date": "2024-08-15"
    },
    {
      "code": "PROD003",
      "issuer_name": "Restaurante Casa Vegetariana",
      "description": "Ensalada Mediterránea",
      "l1": "Alimentación",
      "l2": "Comida Preparada",
      "l3": "Ensaladas",
      "l4": "Ensaladas Gourmet",
      "process_date": "2024-09-01"
    }
  ],
  "timestamp": "2024-08-26T15:30:45Z",
  "user_id": 123
}
```

**Errores posibles:**

| Status | Error | Descripción |
|--------|-------|-------------|
| `401` | `UNAUTHORIZED` | Token JWT inválido o faltante |
| `400` | `BAD_REQUEST` | Formato de fecha inválido |
| `500` | `INTERNAL_SERVER_ERROR` | Error de base de datos o servidor |

**Ejemplo de Error 401:**
```json
{
  "success": false,
  "message": "Invalid or missing token",
  "data": null,
  "timestamp": "2024-08-26T15:30:45Z"
}
```

**Ejemplo de Error 400:**
```json
{
  "success": false,
  "message": "Invalid date format. Use ISO 8601 format (e.g., 2024-01-15T10:00:00Z)",
  "data": null,
  "timestamp": "2024-08-26T15:30:45Z"
}
```

**Casos de uso:**

1. **Historial de Compras:** Ver todos los productos que el usuario ha comprado
   ```bash
   GET /api/v4/invoices/products
   ```

2. **Actualizaciones Incrementales:** Obtener solo productos actualizados desde fecha específica
   ```bash
   GET /api/v4/invoices/products?update_date=2024-09-01
   ```

3. **Análisis de Preferencias:** Para sistemas de recomendación basados en historial
   ```bash
   GET /api/v4/invoices/products  # Analizar patrones de compra
   ```

4. **Personalización de Ofertas:** Mostrar productos relacionados o descuentos
   ```bash
   GET /api/v4/invoices/products  # Para ofertas personalizadas
   ```

5. **Reporting de Productos:** Análisis de productos más comprados por usuario
   ```bash
   GET /api/v4/invoices/products  # Para dashboards y analytics
   ```

**Ejemplos de cURL:**

```bash
# Obtener todos los productos del usuario
curl -H "Authorization: Bearer your_jwt_token" \
     "http://localhost:8000/api/v4/invoices/products"

# Obtener productos actualizados desde una fecha específica
curl -H "Authorization: Bearer your_jwt_token" \
     "http://localhost:8000/api/v4/invoices/products?update_date=2024-01-15"

# Con fecha completa ISO 8601
curl -H "Authorization: Bearer your_jwt_token" \
     "http://localhost:8000/api/v4/invoices/products?update_date=2024-09-01"
```

**Características de Seguridad:**
- ✅ **JWT Obligatorio:** Endpoint protegido con autenticación
- ✅ **Filtrado por Usuario:** Solo datos del usuario autenticado
- ✅ **Validación de Entrada:** Formato de fechas validado
- ✅ **Rate Limiting:** Límites de requests por usuario
- ✅ **Logging:** Todas las peticiones son logged para auditoría
- ✅ **Datos Seguros:** Solo códigos y descripciones de productos (no precios)

**Performance y Cache:**
- ⚡ **Query Optimizada:** SELECT DISTINCT con ORDER BY eficiente
- 📊 **Métricas:** Response time tracking automático
- 🔄 **Cache Potencial:** Resultados cacheables por user_id + filtros
- 📝 **Logging Detallado:** Tracking de performance y errores

---

## 🔒 **GESTIÓN UNIFICADA DE CONTRASEÑAS** ✅ MIGRADO

### **Solicitar Código de Verificación** ✅ NUEVO
- **Endpoint:** `POST /api/v4/passwords/request-code`
- **Descripción:** Solicitar código de verificación para operaciones de contraseña
- **Autenticación:** No requerida

**Request Body:**
```json
{
  "email": "usuario@ejemplo.com",
  "purpose": "reset_password|first_time_setup|change_password"
}
```

**Response Body:**
```json
{
  "success": true,
  "data": {
    "email": "usuario@ejemplo.com",
    "code_expires_at": "2025-09-18T15:30:00Z",
    "purpose": "reset_password",
    "instructions": "Use este código para restablecer tu contraseña. El código expira en 15 minutos."
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "processing_time_ms": 156,
  "cached": false
}
```

**Validaciones y Características:**
- ✅ **Rate Limiting:** Máximo 3 códigos por hora por email
- ✅ **Validación Purpose:** Solo acepta purposes válidos según estado del usuario
- ✅ **Código Temporal:** 15 minutos de validez, 6 dígitos
- ✅ **Invalidación Automática:** Códigos previos se invalidan al generar uno nuevo
- ✅ **Máximo Intentos:** 3 intentos por código antes de invalidación

### **Establecer Contraseña con Código** ✅ NUEVO
- **Endpoint:** `POST /api/v4/passwords/set-with-code`
- **Descripción:** Establecer contraseña usando código de verificación
- **Autenticación:** No requerida

**Request Body:**
```json
{
  "email": "usuario@ejemplo.com",
  "verification_code": "123456",
  "new_password": "MiNuevaContraseña123!",
  "confirmation_password": "MiNuevaContraseña123!"
}
```

**Response Body:**
```json
{
  "success": true,
  "data": {
    "user_id": 42,
    "email": "usuario@ejemplo.com",
    "password_updated_at": "2025-09-18T15:30:00Z",
    "login_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "processing_time_ms": 234,
  "cached": false
}
```

**Validaciones de Contraseña:**
- ✅ **Longitud:** 8-128 caracteres
- ✅ **Mayúsculas:** Al menos 1 letra mayúscula
- ✅ **Minúsculas:** Al menos 1 letra minúscula
- ✅ **Números:** Al menos 1 dígito
- ✅ **Caracteres Especiales:** Al menos 1 de !@#$%^&*()_+-=[]{}|;:,.<>?
- ✅ **Confirmación:** Passwords deben coincidir exactamente

### 🔒 **IMPORTANTE: TODOS LOS CAMBIOS DE CONTRASEÑA USAN EMAIL VERIFICATION**

**🚪 Por Seguridad, NO existe endpoint directo para cambiar contraseña.**

**Para cambiar contraseña (incluso si el usuario ya está autenticado):**
1. **POST** `/api/v4/passwords/request-code` con `purpose="change_password"`
2. **POST** `/api/v4/passwords/set-with-code` con el código del email

**✅ Beneficios de Seguridad:**
- 🔒 **Doble Factor:** JWT + Email verification
- 🚫 **Anti-Hijacking:** Tokens comprometidos no pueden cambiar passwords
- 📧 **Notificación:** Usuario recibe email de cualquier cambio
- 📅 **Auditoría:** Todos los cambios quedan registrados con códigos

### **📛 ENDPOINTS DEPRECATED (Serán Removidos)**

#### ~~Establecer Contraseña~~ ❌ DEPRECATED
- ~~**Endpoint:** `POST /api/v4/users/set-password`~~
- **Motivo:** Reemplazado por flujo unificado `/api/v4/passwords/set-with-code`
- **Migración:** Usar nuevo endpoint con purpose="first_time_setup" o "reset_password"

#### ~~Resetear Contraseña~~ ❌ DEPRECATED  
- ~~**Endpoint:** `POST /api/v4/users/reset-password`~~
- **Motivo:** Reemplazado por flujo unificado `/api/v4/passwords/request-code` + `/api/v4/passwords/set-with-code`
- **Migración:** 
  1. POST `/api/v4/passwords/request-code` con `purpose="reset_password"`
  2. POST `/api/v4/passwords/set-with-code` con el código recibido

#### ~~Cambiar Contraseña (Directo)~~ ❌ DEPRECATED
- ~~**Endpoint:** `POST /api/v4/passwords/change`~~
- **Motivo:** **SEGURIDAD** - Reemplazado por flujo de email verification
- **Migración:**
  1. POST `/api/v4/passwords/request-code` con `purpose="change_password"`
  2. POST `/api/v4/passwords/set-with-code` con el código del email
- **🔒 Por qué:** Previene ataques con tokens JWT comprometidos

---

## 🔗 SISTEMA UNIFICADO DE VERIFICACIÓN

### **📋 Flujos Unificados (Septiembre 2025)**

El sistema ahora usa **PostgreSQL** para todos los códigos de verificación, eliminando la complejidad de tener Redis y PostgreSQL por separado.

#### **🎯 Caso 1: Solo Verificar Email**
```
1. POST /api/v4/passwords/request-code
   └── purpose: "email_verification"
   └── Almacena código en PostgreSQL

2. POST /api/v4/users/verify-account
   └── Busca código en PostgreSQL  
   └── Resultado: Email verificado ✅
```

#### **🎯 Caso 2: Establecer Contraseña Primera Vez**
```
1. POST /api/v4/passwords/request-code
   └── purpose: "first_time_setup"
   └── Almacena código en PostgreSQL

2. POST /api/v4/passwords/set-with-code
   └── Busca código en PostgreSQL
   └── Resultado: Contraseña establecida + JWT token ✅
```

#### **🎯 Caso 3: Verificar Email + Establecer Contraseña**
```
OPCIÓN A (Un código - RECOMENDADO ⭐):
1. POST /api/v4/users/send-verification
2. POST /api/v4/users/set-password-with-email-code
   └── Usa MISMO código para verificar email + establecer contraseña ✅

OPCIÓN B (Un código - directo):
1. POST /api/v4/passwords/request-code (purpose: "first_time_setup")
2. POST /api/v4/passwords/set-with-code
   └── Automáticamente verifica email + establece contraseña

OPCIÓN C (Dos códigos - más seguro pero complejo):
1. POST /api/v4/passwords/request-code (purpose: "email_verification")
2. POST /api/v4/users/verify-account 
3. POST /api/v4/passwords/request-code (purpose: "first_time_setup")
4. POST /api/v4/passwords/set-with-code
```

### **🔄 Compatibilidad con Endpoints Existentes**

Los endpoints antiguos siguen funcionando pero **redirigen internamente** al sistema unificado:

- ✅ `POST /api/v4/users/send-verification` → `request-code` con `purpose="email_verification"`
- ✅ `POST /api/v4/users/verify-account` → Usa sistema PostgreSQL unificado
- 🆕 `POST /api/v4/users/set-password-with-email-code` → Nuevo endpoint para flujo optimal

### **⚡ Ventajas del Sistema Unificado**

- 🗄️ **Un solo almacén:** PostgreSQL (eliminamos Redis para códigos)
- 🔒 **Más seguro:** Rate limiting y auditoría completa
- 🛠️ **Más robusto:** Validaciones avanzadas por purpose
- 📊 **Mejor UX:** Códigos con propósitos claros
- 🔧 **Mantenimiento:** Un solo sistema que mantener

---

#### Perfil de Usuario ✅ MIGRADO
- **Endpoint:** `GET /api/v4/users/profile`
- **Descripción:** Obtener perfil del usuario autenticado
- **Headers:** `Authorization: Bearer <jwt_token>`
- **Respuesta:** Datos completos del perfil + estadísticas
- **Optimizaciones:** Caching (5min), datos comprensivos

#### Perfil de Usuario por ID ✅ MIGRADO
- **Endpoint:** `GET /api/v4/users/profile/{id}`
- **Descripción:** Obtener perfil de usuario específico (solo admins)
- **Headers:** `Authorization: Bearer <jwt_token>`
- **Respuesta:** Datos del perfil solicitado
- **Seguridad:** Control de acceso admin, validación de permisos

---

### 🎁 Rewards & Métricas v4

#### Resumen de Usuario ✅ NUEVO
- **Endpoint:** `GET /api/v4/rewards/summary`
- **Descripción:** Obtener resumen completo de métricas y facturas del usuario desde `rewards.user_invoice_summary`
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Query Parameters:**
  - `include_trends`: Incluir análisis de tendencias (default: true)
  - `include_projections`: Incluir proyecciones (default: true)
  - `currency`: Moneda para mostrar (default: USD)
- **Respuesta:** 
```json
{
  "success": true,
  "data": {
    "summary": {
      "user_id": 7,
      "total_facturas": 6,
      "total_monto": 50.45,
      "total_items": 10,
      "n_descuentos": 2,
      "total_descuento": 2.85,
      "top_emisores": [
        {"monto": 16.64, "issuer": "PRETELT GOURMET MEATS"}, 
        {"monto": 13.9, "issuer": "FSL TIENDA 1, S.A"},
        {"monto": 13.73, "issuer": "SUPERMERCADOS REY"},
        {"monto": 6.18, "issuer": "BARCENAS GROUP INTERNATIONAL INC"}
      ],
      "top_categorias": [
        {"monto": 21.82, "categoria": "ALIMENTOS Y BEBIDAS"},
        {"monto": 20.08, "categoria": "OTRO"},
        {"monto": 8.55, "categoria": "SALUD Y BIENESTAR"}
      ],
      "serie_mensual": {
        "issuer": [
          {"mes": "2025-03-01T00:00:00", "monto": 16.64, "issuer": "PRETELT GOURMET MEATS"},
          {"mes": "2025-03-01T00:00:00", "monto": 13.9, "issuer": "FSL TIENDA 1, S.A"},
          {"mes": "2025-04-01T00:00:00", "monto": 13.73, "issuer": "SUPERMERCADOS REY"},
          {"mes": "2025-04-01T00:00:00", "monto": 6.18, "issuer": "BARCENAS GROUP INTERNATIONAL INC"}
        ],
        "summary": [
          {"mes": "2025-03-01T00:00:00", "monto": 30.54, "descuento": 0, "tot_items": 4, "n_descuentos": 0},
          {"mes": "2025-04-01T00:00:00", "monto": 19.91, "descuento": 0, "tot_items": 6, "n_descuentos": 0}
        ],
        "category": [
          {"mes": "2025-03-01T00:00:00", "monto": 16.64, "categoria": "ALIMENTOS Y BEBIDAS"},
          {"mes": "2025-03-01T00:00:00", "monto": 13.9, "categoria": "OTRO"},
          {"mes": "2025-04-01T00:00:00", "monto": 8.55, "categoria": "SALUD Y BIENESTAR"},
          {"mes": "2025-04-01T00:00:00", "monto": 6.18, "categoria": "OTRO"},
          {"mes": "2025-04-01T00:00:00", "monto": 5.18, "categoria": "ALIMENTOS Y BEBIDAS"}
        ],
        "issuer_category": [
          {"mes": "2025-03-01T00:00:00", "monto": 16.64, "issuer_l2": "CARNICERÍAS"},
          {"mes": "2025-03-01T00:00:00", "monto": 13.9, "issuer_l2": "OTRO"},
          {"mes": "2025-04-01T00:00:00", "monto": 13.73, "issuer_l2": "SUPERMERCADOS"},
          {"mes": "2025-04-01T00:00:00", "monto": 6.18, "issuer_l2": "OTRO"}
        ]
      },
      "updated_at": "2025-05-19T20:03:51.142Z",
      "comparativo_categoria": [
        {
          "categoria": "OTRO", 
          "pct_cliente": 39.80, 
          "pct_general": 16.77, 
          "var_relativa": 23.03,
          "monto_cliente": 20.08,
          "monto_promedio_general": 2192.36
        },
        {
          "categoria": "SALUD Y BIENESTAR", 
          "pct_cliente": 16.95, 
          "pct_general": 6.61, 
          "var_relativa": 10.34,
          "monto_cliente": 8.55,
          "monto_promedio_general": 865.42
        }
      ]
    },
    "performance_metrics": {
      "month_over_month_growth": 15.0,
      "invoice_frequency_score": 80.0,
      "spending_tier": "Gold",
      "lumis_efficiency": 85.0
    },
    "trend_analysis": {
      "monthly_trend": "increasing",
      "avg_monthly_invoices": 15.5,
      "seasonal_pattern": "Q4 peak",
      "projected_next_month": 17.05
    }
  },
  "message": "user_id: 7, query_time_ms: 45",
  "timestamp": "2024-08-12T10:30:00Z"
}
```
- **Optimizaciones:** Caching (10min), métricas calculadas, performance headers

#### Balance de Lümis ✅ NUEVO
- **Endpoint:** `GET /api/v4/rewards/balance`
- **Descripción:** Obtener balance actual de Lümis del usuario
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Respuesta:**
```json
{
  "success": true,
  "data": {
    "balance": 910,
    "currency": "Lümis",
    "user_id": 7
  },
  "message": "query_time_ms: 23",
  "timestamp": "2024-08-12T10:30:00Z"
}
```
- **Optimizaciones:** Respuesta rápida, cache, datos en tiempo real

    #### Historial de Recompensas ✅ IMPLEMENTADO + JWT PROTEGIDO
    - **Endpoint:** `GET /api/v4/rewards/history`
    - **Descripción:** Obtener historial de acumulaciones y redenciones del usuario desde `rewards.vw_hist_accum_redem`
    - **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
    - **Estado:** ✅ **COMPLETAMENTE FUNCIONAL** - Endpoint implementado y probado exitosamente
    - **Query Parameters:**
      - `limit`: Límite de resultados (default: 50, max: 500)
      - `offset`: Posición inicial para paginación (default: 0)
      - `date_from`: Fecha desde (formato: YYYY-MM-DD)
      - `date_to`: Fecha hasta (formato: YYYY-MM-DD)
      - `source_type_filter`: Filtro por tipo de fuente (búsqueda parcial)

    - **Ejemplo de Uso:**
    ```bash
    # Consulta básica
    GET /api/v4/rewards/history?limit=20

    # Con filtros de fecha
    GET /api/v4/rewards/history?date_from=2024-01-01&date_to=2024-12-31&limit=100

    # Con filtro por tipo
    GET /api/v4/rewards/history?source_type_filter=Acumulación&limit=50
    ```

    - **Respuesta:**
    ```json
    {
      "success": true,
      "data": {
        "items": [
          {
            "source_type": "Acumulación",
            "user_id": 1,
            "name_friendly": "Compra en Supermercado Rey",
            "description_friendly": "Acumulación por compra de productos",
            "quantity": 150,
            "date": "2024-08-15",
            "img": "https://example.com/images/accumulation.png"
          },
          {
            "source_type": "Redención",
            "user_id": 1,
            "name_friendly": "Canje de productos",
            "description_friendly": "Redención de 100 Lümis por descuento",
            "quantity": -100,
            "date": "2024-08-10",
            "img": "https://example.com/images/redemption.png"
          }
        ],
        "pagination": {
          "total": 245,
          "limit": 20,
          "offset": 0,
          "has_next": true,
          "has_previous": false
        },
        "summary": {
          "total_items": 245,
          "total_acumulaciones": 180,
          "total_redenciones": 65,
          "sum_quantity": 2150
        }
      },
      "error": null,
      "request_id": "b8f3c9e2-4f5a-4b6c-9d8e-1f2a3b4c5d6e",
      "timestamp": "2024-08-18T16:45:00Z",
      "execution_time_ms": 23,
      "cached": false
    }
    ```

    - **Características:**
      - Autenticación JWT obligatoria
      - Filtrado automático por usuario del token
      - Paginación eficiente con limit/offset
      - Filtros avanzados por fechas y tipo de fuente
      - Estadísticas de resumen incluidas
      - Ordenamiento por fecha descendente
      - Queries optimizadas ejecutadas en paralelo
      - Logging de performance y métricas
      - Estructura ApiResponse estándar v4

    ---

### 📄 Facturas v4

#### Subir para OCR ✅ IMPLEMENTADO + JWT PROTEGIDO + VALIDACIÓN COMPLETA + SIN COSTO
- **Estado Actual:** Completamente implementado con servicio OCR común extraído de WhatsApp.
- **Características:** Pipeline OCR con Gemini 2.0-flash, validación de campos requeridos, guardado en `invoice_header` e `invoice_detail`.
- **Costo:** 0 Lümis (funcionalidad gratuita)
- **Endpoint:** `POST /api/v4/invoices/upload-ocr`
- **Descripción:** Subir imagen/PDF de factura para procesamiento OCR con validación de campos críticos

#### **Headers Requeridos**
```http
Authorization: Bearer <jwt_token>  # OBLIGATORIO
Content-Type: multipart/form-data  # Automático en clients
```

#### **Parámetros del Body (multipart/form-data)**

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `image` o `file` | File | ✅ | Imagen de la factura (max 10MB) |
| `mode` | String | ❌ | Modo OCR: `"1"` (básico) o `"2"` (combinado). Default: `"1"` |

#### **Formatos Soportados**
- **JPEG** (`.jpg`, `.jpeg`) - Magic bytes: `FF D8 FF`
- **PNG** (`.png`) - Magic bytes: `89 50 4E 47`
- **PDF** (`.pdf`) - Magic bytes: `25 50 44 46`
- **Límite de tamaño:** 10MB máximo

#### **Ejemplos de Uso**

**Ejemplo básico:**
```bash
curl -X POST "http://localhost:8000/api/v4/invoices/upload-ocr" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -F "image=@factura.jpg"
```

**Ejemplo con modo combinado:**
```bash
curl -X POST "http://localhost:8000/api/v4/invoices/upload-ocr" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -F "file=@factura.pdf" \
  -F "mode=2"
```

**Ejemplo con archivo desde URL:**
```bash
curl -X POST "http://localhost:8000/api/v4/invoices/upload-ocr" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -F "image=@/path/to/invoice.png" \
  -F "mode=1"
```

#### **Estructura de Respuesta**

**✅ Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "data": {
    "success": true,
    "cufe": "OCR-123456712-20240115-00001",
    "invoice_number": "00001",
    "issuer_name": "Supermercado Rey",
    "issuer_ruc": "1234567",
    "issuer_dv": "12",
    "issuer_address": "Calle 50, Ciudad de Panamá",
    "date": "2024-01-15",
    "total": "150.50",
    "tot_itbms": "0.0",
    "products": [
      {
        "name": "Arroz Integral 1kg",
        "quantity": "2",
        "unit_price": "3.50",
        "total_price": "7.00",
        "partkey": "OCR-123456712-20240115-00001|1"
      },
      {
        "name": "Aceite Vegetal 1L",
        "quantity": "1",
        "unit_price": "5.25",
        "total_price": "5.25",
        "partkey": "OCR-123456712-20240115-00001|2"
      }
    ],
    "message": "Factura procesada exitosamente"
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-10-01T12:51:50.184Z",
  "cached": false
}
```

**Campos de la Respuesta Exitosa:**
- **`cufe`**: Identificador único de factura (formato: `OCR-{RUC+DV}-{FECHA}-{NUMERO}`)
  - Ejemplo: `OCR-123456712-20240115-00001` donde `123456712` es RUC+DV (1234567 + 12), `20240115` es la fecha (2024-01-15), y `00001` es el número de factura
- **`invoice_number`**: Número de factura extraído
- **`issuer_name`**: Nombre del comercio/emisor
- **`issuer_ruc`**: RUC del emisor
- **`issuer_dv`**: Dígito verificador del RUC
- **`issuer_address`**: Dirección del emisor
- **`date`**: Fecha de la factura (formato YYYY-MM-DD)
- **`total`**: Monto total de la factura
- **`tot_itbms`**: Total de ITBMS (calculado desde productos)
- **`products`**: Array de productos extraídos con:
  - `name`: Nombre del producto
  - `quantity`: Cantidad
  - `unit_price`: Precio unitario
  - `total_price`: Precio total del ítem
  - `partkey`: Clave única del producto (formato: `{cufe}|{índice}`)

**❌ Error de Validación - Campos Faltantes (422 Unprocessable Entity):**
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Campos requeridos faltantes: comercio, RUC, fecha",
    "details": {
      "success": false,
      "cufe": null,
      "invoice_number": null,
      "issuer_name": null,
      "issuer_ruc": null,
      "issuer_dv": null,
      "issuer_address": null,
      "date": null,
      "total": null,
      "tot_itbms": null,
      "products": null,
      "message": "Validación fallida: campos requeridos faltantes: comercio, RUC, fecha"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440001",
  "timestamp": "2025-10-01T12:51:50.184Z"
}
```

**❌ Error de Procesamiento OCR (500 Internal Server Error):**
```json
{
  "success": false,
  "error": {
    "code": "OCR_PROCESSING_FAILED",
    "message": "Error procesando OCR",
    "details": {
      "success": false,
      "cufe": null,
      "invoice_number": null,
      "issuer_name": null,
      "issuer_ruc": null,
      "issuer_dv": null,
      "issuer_address": null,
      "date": null,
      "total": null,
      "tot_itbms": null,
      "products": null,
      "message": "Error al procesar la imagen con OCR"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440001",
  "timestamp": "2025-10-01T12:51:50.184Z"
}
```

**❌ Archivo muy grande (413 Payload Too Large):**
```json
{
  "success": false,
  "error": {
    "code": "FILE_TOO_LARGE",
    "message": "Image file too large (max 10MB)"
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440002",
  "timestamp": "2025-09-30T12:51:50.184Z"
}
```

**❌ Formato no soportado (415 Unsupported Media Type):**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_FORMAT",
    "message": "Invalid image format. Supported: JPEG, PNG, PDF"
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440003",
  "timestamp": "2025-09-30T12:51:50.184Z"
}
```

**❌ Sin autorización (401 Unauthorized):**
```json
{
  "error": "Missing Authorization header",
  "message": "Authentication required. Please provide a valid Bearer token."
}
```

#### **Funcionalidades Implementadas**
- **✅ Autenticación JWT obligatoria**
- **✅ Rate limiting personalizado por trust score**
- **✅ Sin costo en Lümis (funcionalidad gratuita)**
- **✅ Procesamiento OCR avanzado con Gemini 2.0-flash API**
- **✅ Validación de campos críticos requeridos:**
  - Nombre del comercio (issuer_name)
  - RUC del emisor (issuer_ruc)
  - Fecha de la factura (date)
  - Total de la factura (total)
  - Productos con detalle completo (products)
- **✅ Respuesta completa con todos los campos extraídos:**
  - Información del emisor (comercio, RUC, DV, dirección)
  - Detalles de la factura (fecha, total, ITBMS)
  - Array completo de productos con partkeys
- **✅ Validación de formato completa (JPEG, PNG, PDF)**
- **✅ Guardado transaccional en BD (invoice_header, invoice_detail)**
- **✅ Manejo de errores con códigos HTTP apropiados**
- **✅ Logging completo para auditoría**
- **✅ Validación magic bytes para seguridad**
- **✅ Límite de 10MB por archivo**
- **✅ Generación automática de CUFE con formato: OCR-{RUC+DV}-{FECHA}-{NUMERO}**
  - Ejemplo: `OCR-123456712-20240115-00001` (RUC+DV normalizado + fecha YYYYMMDD + número de factura)
- **✅ Generación de partkeys únicos por producto: {cufe}|{índice}**

#### **Validación de Campos Requeridos**
El sistema valida que la extracción OCR incluya todos los campos críticos:
1. **Nombre del comercio** (`issuer_name`) - Requerido
2. **RUC del emisor** (`issuer_ruc`) - Requerido
3. **Fecha de la factura** (`date`) - Requerido
4. **Total de la factura** (`total`) - Requerido
5. **Lista de productos** (`products`) - Requerido (al menos 1 producto)

Si alguno de estos campos no se puede extraer, el sistema **rechaza la factura** con un error 422 y un mensaje detallado indicando qué campos faltan.

#### **Códigos de Estado HTTP**
  - `200 OK` - Procesamiento exitoso con todos los campos extraídos
  - `400 Bad Request` - Archivo faltante o datos inválidos
  - `401 Unauthorized` - Token JWT faltante o inválido
  - `413 Payload Too Large` - Archivo muy grande (>10MB)
  - `415 Unsupported Media Type` - Formato no soportado
  - `422 Unprocessable Entity` - Validación fallida (campos requeridos faltantes)
  - `429 Too Many Requests` - Límite de rate excedido
  - `500 Internal Server Error` - Error interno del servidor o error en procesamiento OCR

#### Procesar desde URL ✅ MIGRADO + JWT PROTEGIDO + IDEMPOTENCIA
- **Endpoint:** `POST /api/v4/invoices/process-from-url`
- **Descripción:** Procesar factura desde URL de DGI Panamá
- **Headers Requeridos:** 
  - `Authorization: Bearer <jwt_token>` **REQUERIDO**
  - `Idempotency-Key: <unique_key>` **REQUERIDO**
- **Body:** `{"url": "https://dgi.mef.gob.pa/...", "source": "APP"}`
- **Funcionalidades:**
  - **Autenticación JWT obligatoria**
  - **Idempotencia:** Prevención de procesamiento duplicado (24h TTL)
  - **Rate limiting granular:** Basado en trust score del usuario
  - **Timeouts:** 30s máximo por request
  - Validación de URL de DGI
  - Web scraping nativo
  - Persistencia en base de datos
  - Logging de actividad por usuario
- **Respuesta:** Datos de la factura procesada
- **Middlewares aplicados:** validate_upload → rate_limit → idempotency → request_limits

#### Consultar Detalles ✅ MIGRADO + JWT PROTEGIDO + PAGINACIÓN OPTIMIZADA ⚡
- **Endpoint:** `GET /api/v4/invoices/details`
- **Descripción:** Consultar facturas con filtros avanzados y paginación eficiente de clase empresarial
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Query Parameters:**
  - `from_date`: Fecha desde (YYYY-MM-DD) **REQUERIDO**
  - `to_date`: Fecha hasta (YYYY-MM-DD)
  - `invoice_type`: Tipo de factura individual
  - `invoice_types`: Tipos múltiples separados por comas
  - `min_amount`: Monto mínimo de filtro
  - `max_amount`: Monto máximo de filtro
  - `limit`: Límite de resultados (default: 100, max: 1000)
  - `offset`: Posición inicial para paginación offset/limit (default: 0)
  - `page`: Número de página (alternativo a offset, se calcula automáticamente)
  - `order_by`: Campo de ordenamiento (`date`, `reception_date`, `amount`, `issuer_name`)
  - `order_direction`: Dirección `ASC`/`DESC` (default: "DESC")
  - **✅ NUEVO:** `cursor`: Token de cursor para keyset pagination (reemplaza offset)
  - **✅ NUEVO:** `direction`: Dirección de navegación (`next`/`prev`) para cursors

- **Ejemplo de Uso:**
```bash
# Paginación básica
GET /api/v4/invoices/details?from_date=2024-01-01&limit=50&offset=100

# Por número de página
GET /api/v4/invoices/details?from_date=2024-01-01&limit=50&page=3

# Con filtros avanzados
GET /api/v4/invoices/details?from_date=2024-01-01&to_date=2024-12-31&min_amount=100&max_amount=5000&order_by=amount&order_direction=DESC

# Múltiples tipos de factura
GET /api/v4/invoices/details?from_date=2024-01-01&invoice_types=FACTURA,NOTA_CREDITO&limit=200
```

- **Respuesta Optimizada:**
```json
{
  "data": [
    {
      "id": 1,
      "cufe": "ABC123...",
      "quantity": 2.0,
      "code": "PROD001",
      "description": "Producto ejemplo",
      "unit_price": 25.50,
      "amount": 51.00,
      "unit_discount": "0%",
      "date": "2024-08-01T10:30:00",
      "total": 51.00,
      "issuer_name": "Empresa Ejemplo S.A.",
      "reception_date": "2024-08-01T15:45:30Z"
    }
  ],
  "pagination": {
    "total": 1250,
    "limit": 100,
    "offset": 200,
    "page": 3,
    "total_pages": 13,
    "has_next": true,
    "has_previous": true,
    "next_offset": 300,
    "previous_offset": 100
  },
  "performance": {
    "query_time_ms": 45,
    "cached": false
  }
}
```

- **Headers de Respuesta Automáticos:**
  - `X-Total-Count: 1250` - Total de registros disponibles
  - `X-Page-Count: 13` - Total de páginas
  - `X-Current-Page: 3` - Página actual
  - `Link: </api/v4/invoices/details?offset=300&limit=100>; rel="next", </api/v4/invoices/details?offset=100&limit=100>; rel="prev"` - Enlaces de navegación
  - `ETag` + soporte `If-None-Match` (304 cuando aplica)
  - `X-Cache: HIT/MISS`

##### (BETA) Paginación Keyset / Cursor (Próxima Iteración)
- **Motivación:** Escalabilidad superior a OFFSET en volúmenes altos y cambios concurrentes.
- **Orden Base:** `(order_by DESC/ASC, id DESC/ASC)` garantiza unicidad y orden estable.
- **✅ Parámetros Implementados:**
  - `cursor`: string base64 con múltiples campos (`date:ISO8601|amount:decimal|id:integer|reception_date:ISO8601`)
  - `direction`: `next` (default) / `prev` para navegación bidireccional
  - `limit`: tamaño página (compatible con offset pagination)
- **✅ Ejemplos de Uso:**
```bash
# Primera página (sin cursor)
GET /api/v4/invoices/details?from_date=2024-01-01&limit=50

# Página siguiente usando cursor
GET /api/v4/invoices/details?cursor=ZGF0ZToyMDI0LTA4LTE1VDEwOjAwOjAwWnxhbW91bnQ6MTI1MC4wMHxpZDoxMjM0NQ==&direction=next&limit=50

# Página anterior usando cursor  
GET /api/v4/invoices/details?cursor=ZGF0ZToyMDI0LTA4LTE1VDEwOjAwOjAwWnxhbW91bnQ6MTI1MC4wMHxpZDoxMjM0NQ==&direction=prev&limit=50
```
- **✅ Respuesta Expandida:**
```json
{
  "pagination": {
    "cursor_pagination": {
      "next_cursor": "ZGF0ZToyMDI0LTA4LTE1VDA5OjAwOjAwWnxhbW91bnQ6MTEwMC4wMHxpZDoxMjM0Ng==",
      "previous_cursor": "ZGF0ZToyMDI0LTA4LTE1VDExOjAwOjAwWnxhbW91bnQ6MTQwMC4wMHxpZDoxMjM0NA==",
      "has_next_page": true,
      "has_previous_page": true,
      "page_size": 50,
      "direction": "next"
    }
  }
}
```
- **Headers de Navegación:**
  - `X-Pagination-Type: cursor` (indica que se usó keyset pagination)
  - `X-Has-Next-Page: true/false`
  - `Link: </api/v4/invoices/details?cursor=...&direction=next&limit=50>; rel="next"`
- **Convivencia:** OFFSET+PAGE se mantiene (keyset usado cuando `cursor` está presente).

#### Consultar Headers ✅ MIGRADO + JWT PROTEGIDO + FILTROS AVANZADOS ⚡
- **(Próximo)** Se añadirá la misma estrategia de keyset pagination que en detalles (MISMA RUTA - sin duplicados).
- **Endpoint:** `GET /api/v4/invoices/headers`
- **Descripción:** Consultar headers de facturas con filtros avanzados y paginación eficiente
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**

##### Query Parameters
| Parámetro | Tipo | Requerido | Descripción | Ejemplo |
|-----------|------|-----------|-------------|---------|
| `from_date` | `DateTime` | No | Fecha desde (YYYY-MM-DD) | `2024-01-01` |
| `to_date` | `DateTime` | No | Fecha hasta (YYYY-MM-DD) | `2024-12-31` |
| `min_amount` | `f64` | No | Monto mínimo de filtro | `100.00` |
| `max_amount` | `f64` | No | Monto máximo de filtro | `5000.00` |
| `issuer_name` | `String` | No | Nombre del emisor (búsqueda parcial con ILIKE) | `Empresa` |
| `limit` | `i32` | No | Límite de resultados (default: 100, max: 1000) | `50` |
| `offset` | `i32` | No | Posición inicial para paginación (default: 0) | `0` |
| `cursor` | `String` | No | Cursor para keyset pagination (alternativa a offset) | `base64_encoded` |
| `direction` | `String` | No | Dirección de navegación: `next` o `prev` (con cursor) | `next` |
| `order_by` | `String` | No | Campo para ordenar (default: `reception_date`) | `tot_amount` |
| `order_direction` | `String` | No | Dirección de orden: `ASC` o `DESC` (default: `DESC`) | `DESC` |

##### Ejemplo de Uso
```bash
# Filtros básicos con paginación offset
GET /api/v4/invoices/headers?from_date=2024-01-01&limit=50

# Con filtros avanzados múltiples
GET /api/v4/invoices/headers?from_date=2024-01-01&to_date=2024-12-31&min_amount=100&issuer_name=Empresa&limit=100

# Con keyset pagination (recomendado para datasets grandes)
GET /api/v4/invoices/headers?cursor=eyJyZWNlcHRpb25fZGF0ZSI6IjIwMjQtMDEtMTVUMTI6MzA6MDAiLCJpZCI6MTIzfQ&direction=next&limit=50

# Búsqueda por emisor con orden personalizado
GET /api/v4/invoices/headers?issuer_name=Panama&order_by=tot_amount&order_direction=DESC&limit=20
```

##### Respuesta Exitosa (200 OK)
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "no": "FE01-00012345",
      "date": "2024-01-15T10:30:00",
      "tot_itbms": 12.50,
      "cufe": "FE01200000000434-15-9379...",
      "issuer_name": "Empresa Ejemplo S.A.",
      "tot_amount": 112.50,
      "url": "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=...",
      "process_date": "2024-01-15T10:35:00Z",
      "reception_date": "2024-01-15T10:30:00Z",
      "type": "01",
      "issuer_ruc": "1234567890",
      "issuer_dv": "12",
      "issuer_address": "Calle 50, Ciudad de Panamá",
      "issuer_phone": "+507 123-4567",
      "time": "",
      "auth_date": "",
      "receptor_name": "",
      "details_count": 5,
      "payments_count": 1
    }
  ],
  "total": 150,
  "page_info": {
    "current_page": 1,
    "page_size": 50,
    "total_pages": 3,
    "has_next": true,
    "has_previous": false,
    "cursor_pagination": {
      "next_cursor": "eyJyZWNlcHRpb25fZGF0ZSI6IjIwMjQtMDEtMTBUMDk6MDA6MDBaIiwiaWQiOjUwfQ",
      "prev_cursor": null,
      "has_more": true
    }
  },
  "summary": {
    "total_invoices": 150,
    "total_amount": 15750.80,
    "unique_issuers": 12,
    "date_range": {
      "earliest": "2024-01-01T08:00:00Z",
      "latest": "2024-12-31T18:30:00Z"
    },
    "amount_range": {
      "minimum": 25.00,
      "maximum": 2500.00,
      "average": 105.01
    }
  }
}
```

##### Estructura de Campos (`InvoiceHeaderItem`)

| Campo | Tipo | Nullable | Descripción |
|-------|------|----------|-------------|
| `id` | `i64` | No | ID secuencial generado por ROW_NUMBER() |
| `no` | `String` | Sí | Número de factura (ej: "FE01-00012345") |
| `date` | `NaiveDateTime` | Sí | Fecha de emisión de la factura |
| `tot_itbms` | `f64` | Sí | Total de ITBMS (impuesto) |
| `cufe` | `String` | Sí | Código Único de Factura Electrónica |
| `issuer_name` | `String` | Sí | Nombre del emisor/proveedor |
| `tot_amount` | `f64` | Sí | Monto total de la factura |
| `url` | `String` | Sí | URL del QR de consulta DGI |
| `process_date` | `DateTime<Utc>` | Sí | Fecha de procesamiento del sistema |
| `reception_date` | `DateTime<Utc>` | Sí | Fecha de recepción (usado para ordenar) |
| `type` | `String` | Sí | Tipo de factura ("01" = Factura, etc.) |
| `details_count` | `i64` | No | Cantidad de líneas de detalle (JOIN con `invoice_detail`) |
| `payments_count` | `i64` | No | Cantidad de pagos asociados (JOIN con `invoice_payment`) |
| `issuer_ruc` | `String` | Sí | RUC (Registro Único de Contribuyente) del emisor |
| `issuer_dv` | `String` | Sí | Dígito verificador del RUC del emisor |
| `issuer_address` | `String` | Sí | Dirección física del emisor/comercio |
| `issuer_phone` | `String` | Sí | Teléfono de contacto del emisor |
| `time` | `String` | Sí | **Campo legacy** (vacío, mantener por compatibilidad) |
| `auth_date` | `String` | Sí | **Campo legacy** (vacío, mantener por compatibilidad) |
| `receptor_name` | `String` | Sí | **Campo legacy** (vacío, mantener por compatibilidad) |

##### Estructura del Summary

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `total_invoices` | `i64` | Total de facturas en el resultado |
| `total_amount` | `f64` | Suma de todos los montos |
| `unique_issuers` | `i64` | Cantidad de emisores únicos |
| `date_range.earliest` | `DateTime<Utc>` | Fecha de factura más antigua |
| `date_range.latest` | `DateTime<Utc>` | Fecha de factura más reciente |
| `amount_range.minimum` | `f64` | Monto mínimo encontrado |
| `amount_range.maximum` | `f64` | Monto máximo encontrado |
| `amount_range.average` | `f64` | Promedio de montos |

##### Características
- ✅ **Autenticación JWT obligatoria** con filtrado automático por `user_id`
- ⚡ **Filtros múltiples** combinables (fechas AND montos AND emisor)
- 🎯 **Búsqueda por emisor** con ILIKE pattern matching (case-insensitive, búsqueda parcial)
- 📊 **Summary automático** con estadísticas agregadas (totales, promedios, rangos)
- 🚀 **Performance optimizada** con queries eficientes y LEFT JOINs para conteos
- 📈 **Doble paginación** soportada: 
  - **Offset/Limit** (simple, compatible con APIs tradicionales)
  - **Keyset/Cursor** (recomendado para datasets grandes, más eficiente)
- 🔍 **Tipos de dato correctos** (`DateTime<Utc>` vs `NaiveDateTime` vs `String`)
- 🧩 **Unificación Completa:** Eliminado endpoint interno `invoice_headers/search`; toda la funcionalidad vive aquí
- 📋 **Contadores automáticos:** `details_count` y `payments_count` calculados con LEFT JOIN
- 🔗 **Link headers** para navegación con cursor (siguiendo estándar RFC 8288)

##### Notas Técnicas
- Los **campos de emisor** (`issuer_ruc`, `issuer_dv`, `issuer_address`, `issuer_phone`) traen datos reales de la base de datos cuando están disponibles
- Los **campos legacy** (`time`, `auth_date`, `receptor_name`) retornan strings vacíos para mantener compatibilidad con versiones anteriores
- El campo `id` es generado por `ROW_NUMBER()` y **NO es persistente** (cambia con filtros/orden)
- Para identificación única usar `cufe` (Código Único de Factura Electrónica)
- `reception_date` es el campo por defecto para ordenamiento (más reciente primero)
- Los filtros de fecha usan `reception_date` (no `date`) para consistencia
- La búsqueda de `issuer_name` usa `ILIKE` con patrón `%texto%` (búsqueda parcial case-insensitive)
- Los conteos (`details_count`, `payments_count`) usan `COALESCE(..., 0)` para evitar NULLs
- `issuer_ruc` y `issuer_dv` pueden ser NULL si la factura no tiene estos datos (facturas antiguas o incompletas)

---

#### Procesar Factura desde URL ✅ JWT PROTEGIDO + WEB SCRAPING
- **Endpoint:** `POST /api/v4/invoices/process-from-url`
- **Descripción:** Extrae y procesa datos de factura desde URL de DGI Panamá mediante web scraping
- **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
- **Content-Type:** `application/json`

##### Request Body
```json
{
  "url": "string",                    // ✅ REQUERIDO - URL de la factura DGI
  "type": "string",                   // ⚪ OPCIONAL - Tipo: "QR" o "CUFE" (default: auto-detect)
  "origin": "string",                 // ⚪ OPCIONAL - Origen: "app", "whatsapp", "telegram"
  "user_email": "string",             // ⚪ OPCIONAL - Email del usuario
  "user_phone_number": "string",      // ⚪ OPCIONAL - Número de teléfono
  "user_telegram_id": "string",       // ⚪ OPCIONAL - ID de Telegram
  "user_ws": "string"                 // ⚪ OPCIONAL - ID de WhatsApp
}
```

##### Campos del Request

| Campo | Tipo | Requerido | Descripción | Ejemplo |
|-------|------|-----------|-------------|---------|
| `url` | `String` | ✅ Sí | URL completa de la factura electrónica de DGI Panamá | `https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=...` |
| `type` | `String` | ⚪ No | Tipo de URL: `"QR"` o `"CUFE"` (auto-detectado si no se provee) | `"QR"` |
| `origin` | `String` | ⚪ No | Canal de origen de la solicitud | `"app"`, `"whatsapp"`, `"telegram"` |
| `user_email` | `String` | ⚪ No | Email del usuario que registra la factura | `"user@example.com"` |
| `user_phone_number` | `String` | ⚪ No | Teléfono del usuario | `"+507-1234-5678"` |
| `user_telegram_id` | `String` | ⚪ No | ID de usuario de Telegram | `"123456789"` |
| `user_ws` | `String` | ⚪ No | ID de WhatsApp | `"507-6123-4567"` |

##### Validaciones
- ✅ URL debe comenzar con `http://` o `https://`
- ✅ URL no debe exceder 2048 caracteres
- ✅ URL debe ser de dominio permitido (DGI Panamá)
- ✅ Usuario autenticado mediante JWT (user_id extraído del token)

##### Ejemplo de Uso
```bash
# Procesar factura desde URL de QR
POST /api/v4/invoices/process-from-url
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "url": "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=FE01200000000434-15-9379...",
  "type": "QR",
  "origin": "app"
}
```

```bash
# Procesar factura desde URL con metadatos completos
POST /api/v4/invoices/process-from-url
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "url": "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorCUFE?cufe=FE012024...",
  "type": "CUFE",
  "origin": "whatsapp",
  "user_email": "user@example.com",
  "user_phone_number": "+507-6123-4567"
}
```

##### Respuesta Exitosa (200 OK)
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "Tu factura de Super 99 por valor de $45.80 fue procesada exitosamente. Tu historial de compras está tomando forma... ¡Vamos por más!",
    "process_type": "QR",
    "invoice_id": null,
    "cufe": "FE01200000000434-15-9379-001-000-20240115-12345-67890",
    "processing_time_ms": 1250,
    "issuer_name": "Super 99",
    "tot_amount": 45.80
  },
  "error": null,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-15T10:30:00Z",
  "execution_time_ms": 1250,
  "cached": false
}
```

##### Respuesta - Factura Duplicada (200 OK)
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "Esta factura ya fue procesada recientemente (CUFE: FE01200000000434...)",
    "process_type": "DUPLICATE",
    "invoice_id": null,
    "cufe": "FE01200000000434-15-9379-001-000-20240115-12345-67890",
    "processing_time_ms": 45,
    "issuer_name": null,
    "tot_amount": null
  },
  "error": null,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-15T10:30:00Z",
  "execution_time_ms": 45,
  "cached": false
}
```

##### Respuesta - Error de Validación (400 Bad Request)
```json
{
  "error": "VALIDATION_ERROR",
  "message": "URL must start with http:// or https://",
  "details": {
    "field": "url",
    "provided_value": "dgi-fep.mef.gob.pa/..."
  }
}
```

##### Respuesta - Error de Scraping (200 OK con fallback a mef_pending)
```json
{
  "success": false,
  "data": {
    "success": false,
    "message": "No pudimos procesar la factura automáticamente. Nuestro equipo la revisará manualmente y te notificaremos cuando esté lista.",
    "process_type": null,
    "invoice_id": null,
    "cufe": null,
    "processing_time_ms": 3500,
    "issuer_name": null,
    "tot_amount": null
  },
  "error": null,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-15T10:30:00Z",
  "execution_time_ms": 3500,
  "cached": false
}
```

**Nota:** Cuando ocurre error de scraping o guardado, la factura se guarda automáticamente en `public.mef_pending` para procesamiento manual posterior.

##### Estructura de Respuesta (`ProcessUrlResponse`)

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `success` | `boolean` | Indica si el procesamiento fue exitoso |
| `message` | `String` | Mensaje descriptivo del resultado en español |
| `process_type` | `String` (nullable) | Tipo de procesamiento: `"QR"`, `"CUFE"`, o `"DUPLICATE"` |
| `invoice_id` | `i32` (nullable) | ID de la factura en la base de datos (si fue guardada) |
| `cufe` | `String` (nullable) | Código Único de Factura Electrónica extraído |
| `processing_time_ms` | `u64` (nullable) | Tiempo total de procesamiento en milisegundos |
| `issuer_name` | `String` (nullable) | Nombre del emisor de la factura |
| `tot_amount` | `f64` (nullable) | Monto total de la factura |

##### Proceso de Web Scraping

El endpoint realiza las siguientes operaciones:

1. **Validación de URL**
   - Verifica formato válido (http/https)
   - Valida dominio permitido (DGI Panamá)
   - Normaliza la URL

2. **Web Scraping**
   - Descarga el HTML de la factura
   - Extrae datos del header (emisor, receptor, totales)
   - Extrae líneas de detalle de la factura
   - Extrae información de pagos

3. **Persistencia en Base de Datos**
   - Verifica duplicados (mismo CUFE en última hora)
   - Inserta en `invoice_header`
   - Inserta detalles en `invoice_detail`
   - Inserta pagos en `invoice_payment`
   - Transacción atómica (rollback si falla)

4. **Logging**
   - Registra intento en `url_processing_logs`
   - Incluye tiempo de ejecución
   - Registra errores si ocurren

##### Datos Extraídos de la Factura

**Header (`invoice_header`):**
- `no` - Número de factura
- `date` - Fecha de emisión
- `cufe` - Código Único de Factura Electrónica
- `issuer_name` - Nombre del emisor
- `issuer_ruc` - RUC del emisor
- `issuer_dv` - Dígito verificador
- `issuer_address` - Dirección del emisor
- `issuer_phone` - Teléfono del emisor
- `tot_amount` - Monto total
- `tot_itbms` - Total de impuestos ITBMS
- `url` - URL de la factura
- `type` - Tipo de factura
- `user_id` - ID del usuario (del JWT)
- `origin` - Canal de origen
- `process_date` - Fecha de procesamiento
- `reception_date` - Fecha de recepción

**Detalles (`invoice_detail`):**
- `cufe` - Referencia a la factura
- `quantity` - Cantidad
- `code` - Código del producto/servicio
- `description` - Descripción
- `unit_price` - Precio unitario
- `unit_discount` - Descuento unitario
- `itbms` - ITBMS del ítem
- `amount` - Subtotal
- `information_of_interest` - Información adicional

**Pagos (`invoice_payment`):**
- `cufe` - Referencia a la factura
- `forma_de_pago` - Forma de pago
- `forma_de_pago_otro` - Otra forma de pago
- `valor_pago` - Valor del pago
- `efectivo` - Monto en efectivo
- `tarjeta_debito` - Monto con tarjeta débito
- `tarjeta_credito` - Monto con tarjeta crédito

##### Características
- ✅ **Autenticación JWT obligatoria** con extracción de `user_id`
- 🌐 **Web scraping robusto** con manejo de errores
- 🔄 **Detección de duplicados** (previene reprocesar misma factura en 1 hora)
- 💾 **Transacciones atómicas** (todo o nada en DB)
- ⚡ **Rate limiting** configurable por usuario
- 📊 **Logging completo** de intentos y errores
- 🔍 **Auto-detección** de tipo de URL (QR vs CUFE)
- 🎯 **Validación de dominio** (solo URLs oficiales de DGI)
- 📱 **Soporte multi-canal** (app, WhatsApp, Telegram)
- 🔐 **Idempotencia** con header `x-request-id`
- 🛡️ **Fallback a `mef_pending`** cuando falla procesamiento (permite revisión manual)

##### Rate Limiting
- **Máximo por hora:** 50 solicitudes
- **Máximo por minuto:** 10 solicitudes
- Configurable por usuario según trust score

##### Notas Técnicas
- El `user_id` se extrae **automáticamente del JWT**, no del request body (seguridad)
- Las URLs deben ser del dominio oficial de DGI Panamá (`dgi-fep.mef.gob.pa`)
- Los campos opcionales (`type`, `origin`, etc.) se almacenan como metadatos adicionales
- Si la factura ya fue procesada en la última hora, retorna `process_type: "DUPLICATE"`
- El web scraping usa selectores CSS robustos con fallbacks
- Tiempo típico de procesamiento: 1-3 segundos (incluye HTTP request + parsing + DB)
- Maneja diferentes formatos de facturas DGI (FE, FEE, NC, etc.)
- **Fallback automático:** Si falla el procesamiento (scraping o guardado), la factura se guarda en `public.mef_pending` para revisión manual del equipo
- Las facturas en `mef_pending` se procesan posteriormente y el usuario es notificado

##### Sistema de Fallback a `mef_pending`

Cuando el procesamiento de la factura falla (scraping o guardado en DB), el sistema automáticamente guarda la información en la tabla `public.mef_pending` para procesamiento manual posterior.

**Campos guardados en `mef_pending`:**
- `url` - URL de la factura
- `user_id` - ID del usuario (del JWT)
- `user_email` - Email del usuario (si se proporcionó)
- `user_ws` / `chat_id` - WhatsApp ID (si se proporcionó)
- `origin` - Canal de origen ("API", "app", "whatsapp", etc.)
- `type_document` - Tipo de documento ("QR", "CUFE", "URL")
- `error_message` - Descripción detallada del error
- `reception_date` - Timestamp del intento

**Beneficios:**
- ✅ **Trazabilidad completa** de todos los intentos de procesamiento
- ✅ **Recuperación automática** posterior por el equipo de soporte
- ✅ **Notificación al usuario** cuando la factura es procesada manualmente
- ✅ **Análisis de errores** para mejorar el sistema
- ✅ **Sin pérdida de datos** incluso en casos de fallo

##### Errores Comunes

| Código | Error | Solución | Fallback |
|--------|-------|----------|----------|
| `400` | `URL is required` | Proporcionar campo `url` en el request | ❌ No |
| `400` | `URL must start with http:// or https://` | Usar protocolo válido | ❌ No |
| `400` | `URL is too long` | URL no debe exceder 2048 caracteres | ❌ No |
| `401` | `Missing Authorization header` | Incluir JWT token válido | ❌ No |
| `403` | `Rate limit exceeded` | Esperar antes de reintentar | ❌ No |
| `409` | Factura duplicada | La factura ya fue procesada recientemente | ❌ No |
| `200` | `SCRAPING_ERROR` | Error al extraer datos del HTML | ✅ Sí → mef_pending |
| `200` | `Database error` | Error al guardar en base de datos | ✅ Sí → mef_pending |

---

### 📱 Pipeline QR Híbrido v4

#### Detección QR Avanzada ✅ IMPLEMENTADO
- **Endpoint:** `POST /api/v4/qr/detect`
- **Descripción:** Pipeline híbrido con 7 detectores + ONNX + Python fallback
- **Content-Type:** `multipart/form-data` o `application/json`

**Request (JSON):**
```json
{
  "image_data": "base64_encoded_image",
  "options": {
    "max_detectors": 3,
    "timeout_ms": 5000,
    "enable_preprocessing": true,
    "prefer_speed": false
  }
}
```

**Request (Multipart):**
```http
POST /api/v4/qr/detect
Content-Type: multipart/form-data

file: [image_file]
max_detectors: 3
timeout_ms: 5000
```

**Response:**
```json
{
  "success": true,
  "data": "https://example.com/invoice?id=12345",
  "detector_model": "rqrr",
  "processing_time_ms": 15,
  "confidence": 0.95,
  "pipeline_stats": {
    "detectors_tried": 1,
    "total_time_ms": 15,
    "cache_hit": false,
    "image_size": "1024x768",
    "preprocessing_time_ms": 3
  },
  "metadata": {
    "qr_type": "url",
    "error_correction_level": "M",
    "version": 7
  }
}
```

**Pipeline de Detección (Cascada Mejorada):**
1. **rqrr** (~5ms) - Rust nativo, más rápido
2. **quircs** (~10ms) - Alta precisión QR
3. **rxing** (~15ms) - Port ZXing, muy preciso
4. **🤖 ONNX Small** (~100ms) - YOLOv8 ML model (12MB, 94% precisión)
5. **🤖 ONNX Medium** (~150ms) - YOLOv8 ML model (25MB, 96% precisión)
6. **Rotation Correction** (~50ms) - 90°/180°/270° con detectores Rust
7. **Python Fallback** (~255ms) - QReader API optimizada en puerto 8008
   - **Hybrid Detection Engine**: CV2 → PYZBAR → QReader Small → QReader Medium
   - **PyTorch Optimizations**: inference_mode(), torch.set_grad_enabled(False)
   - **Performance**: 3.9 req/s, 255ms avg, 100% success rate
   - **Concurrency**: Supports up to 100 concurrent requests
   - **Memory**: 708MB total (91% reduction vs baseline)

**Modelos ONNX Disponibles:**
- `qreader_detector_nano.onnx` - 5MB, ~50ms, precisión 90%
- `qreader_detector_small.onnx` - 12MB, ~100ms, precisión 94%
- `qreader_detector_medium.onnx` - 25MB, ~150ms, precisión 96%
- `qreader_detector_large.onnx` - 45MB, ~300ms, precisión 98%

#### Detección en Lote ✅ IMPLEMENTADO
- **Endpoint:** `POST /api/v4/qr/batch`
- **Descripción:** Procesar múltiples imágenes en paralelo

**Request:**
```json
{
  "images": [
    {"image_data": "base64_1", "id": "img_1"},
    {"image_data": "base64_2", "id": "img_2"}
  ],
  "options": {
    "max_concurrent": 4,
    "timeout_per_image_ms": 3000
  }
}
```

**Response:**
```json
{
  "success": true,
  "results": [
    {
      "id": "img_1",
      "success": true,
      "data": "qr_content_1",
      "detector_model": "rqrr",
      "processing_time_ms": 12
    },
    {
      "id": "img_2", 
      "success": false,
      "error": "No QR detected",
      "processing_time_ms": 500
    }
  ],
  "summary": {
    "total_images": 2,
    "successful_detections": 1,
    "total_time_ms": 512,
    "average_time_ms": 256
  }
}
```

#### Estadísticas del Pipeline ✅ IMPLEMENTADO
- **Endpoint:** `GET /api/v4/qr/stats`
- **Descripción:** Métricas de rendimiento del pipeline

**Response:**
```json
{
  "detector_stats": {
    "rqrr": {
      "total_attempts": 15420,
      "successful_detections": 12336,
      "success_rate": 0.80,
      "avg_time_ms": 5.2,
      "p95_time_ms": 8.1
    },
    "bardecoder": {
      "total_attempts": 3084,
      "successful_detections": 2467,
      "success_rate": 0.80,
      "avg_time_ms": 9.8,
      "p95_time_ms": 15.2
    }
  },
  "cache_stats": {
    "hit_rate": 0.34,
    "total_hits": 5248,
    "total_misses": 10172,
    "evictions": 234
  },
  "model_stats": {
    "onnx_models_loaded": 4,
    "active_model": "small",
    "memory_usage_mb": 45.2
  }
}
```

#### Health Check del Pipeline ✅ IMPLEMENTADO
- **Endpoint:** `GET /api/v4/qr/health`
- **Descripción:** Estado detallado de todos los detectores

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2025-08-11T10:30:00Z",
  "detectors": {
    "rqrr": {"status": "healthy", "last_used": "2025-08-11T10:29:45Z"},
    "bardecoder": {"status": "healthy", "last_used": "2025-08-11T10:28:22Z"},
    "zbar": {"status": "healthy", "last_used": "2025-08-11T10:29:01Z"},
    "quircs": {"status": "healthy", "last_used": "2025-08-11T10:27:15Z"},
    "rxing": {"status": "healthy", "last_used": "2025-08-11T10:26:33Z"},
    "rust_qreader": {
      "status": "healthy",
      "loaded_models": ["nano", "small", "medium", "large"],
      "active_model": "small",
      "onnx_runtime_version": "1.16.3",
      "last_used": "2025-08-11T10:25:12Z"
    },
    "python_fallback": {
      "status": "healthy",
      "endpoint": "http://localhost:8008/qr/hybrid-fallback",
      "last_ping": "2025-10-05T21:05:00Z",
      "response_time_ms": 255,
      "implementation": "QReader PyTorch Optimized",
      "features": [
        "QReader Small + Medium models",
        "PyTorch optimizations (inference_mode, threads=4)",
        "Singleton pattern (91% memory reduction)",
        "Multi-strategy preprocessing (3 approaches)",
        "CV2 + PYZBAR + QReader hybrid engine",
        "Real-time metrics and monitoring"
      ],
      "performance": {
        "avg_latency_ms": 255,
        "throughput_rps": 3.9,
        "success_rate": 100.0,
        "memory_usage_mb": 708,
        "supported_concurrency": 100
      }
    }
  },
  "performance": {
    "avg_detection_time_ms": 23.5,
    "success_rate": 0.94,
    "cache_hit_rate": 0.34
  }
}
```

---

## 🐍 Python QReader Fallback API (Puerto 8008)

### Arquitectura del Sistema Híbrido
La aplicación Rust utiliza una **API Python optimizada como fallback** cuando los detectores Rust nativos no logran detectar códigos QR. Esta API implementa un **Hybrid Detection Engine** con QReader + optimizaciones PyTorch.

### 🚀 Rendimiento Comprobado
- **✅ Latencia**: 255ms promedio
- **✅ Throughput**: 3.9 req/s
- **✅ Concurrencia**: Hasta 100 usuarios simultáneos  
- **✅ Tasa de éxito**: 100% con las 5 imágenes de test
- **✅ Memoria**: 708MB total (91% reducción vs baseline)

---

#### GET /health
**Descripción**: Health check para verificar el estado de la API Python
**Puerto**: 8008
**Usado por**: Sistema Rust para verificar disponibilidad del fallback

**Response**:
```json
{
  "status": "ok",
  "service": "qreader_api"
}
```

#### GET /qr-hybrid-metrics
**Descripción**: Métricas detalladas del Hybrid Detection Engine
**Puerto**: 8008
**Usado por**: Sistema Rust para monitoreo y debugging

**Response**:
```json
{
  "total_requests": 1250,
  "successful_detections": 1238,
  "success_rate": 99.04,
  "avg_latency_ms": 255.3,
  "current_concurrent": 0,
  "peak_concurrent": 45,
  "detector_stats": {
    "qreader_small_success": 892,
    "qreader_medium_success": 346,
    "cv2_success": 0,
    "pyzbar_success": 0
  },
  "performance": {
    "p95_latency_ms": 460.4,
    "p99_latency_ms": 521.1,
    "throughput_rps": 3.9,
    "memory_usage_mb": 708
  },
  "engine_type": "hybrid_optimized"
}
```

#### POST /qr/hybrid-fallback
**Descripción**: Endpoint principal para detección QR como fallback del sistema Rust
**Puerto**: 8008
**Content-Type**: `multipart/form-data`
**Usado por**: Sistema Rust cuando detectores nativos fallan

**Request**:
```http
POST /qr/hybrid-fallback HTTP/1.1
Host: localhost:8008
Content-Type: multipart/form-data

file: [binary_image_data]
```

**Response (Éxito)**:
```json
{
  "success": true,
  "qr_data": "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=FE01...",
  "detector_model": "QREADER_S_PRIORITY",
  "pipeline": "Python Hybrid Fallback",
  "methods_tried": [
    "QREADER_S_PRIORITY",
    "QREADER_M_PRIORITY"
  ],
  "processing_time_ms": 255,
  "confidence": 0.98
}
```

**Response (No QR detectado)**:
```json
{
  "success": false,
  "qr_data": null,
  "detector_model": "NONE",
  "pipeline": "Python Hybrid Fallback", 
  "methods_tried": [
    "CV2",
    "CV2_CURVED", 
    "PYZBAR",
    "PYZBAR_ENHANCED",
    "QREADER_S_PRIORITY",
    "QREADER_M_PRIORITY"
  ],
  "processing_time_ms": 890,
  "error": "No QR code detected by any method"
}
```

### 🔧 Hybrid Detection Engine
La API implementa un **motor de detección híbrido** que ejecuta múltiples estrategias en orden de prioridad:

#### Fase 1: QReader Prioritario (Para máximo rendimiento)
1. **QREADER_S_PRIORITY**: QReader Small model (100MB, ~200ms)
2. **QREADER_M_PRIORITY**: QReader Medium model (250MB, ~300ms)

#### Fase 2: Detectores Tradicionales (Fallback)
3. **CV2**: OpenCV QR detector nativo
4. **CV2_CURVED**: OpenCV con corrección de curvatura
5. **PYZBAR**: Librería PYZBAR estándar
6. **PYZBAR_ENHANCED**: PYZBAR con preprocessing mejorado

### 🎯 Optimizaciones Implementadas

#### PyTorch Optimizations
- `torch.set_grad_enabled(False)` - Deshabilita gradientes innecesarios
- `torch.inference_mode()` - Modo inferencia puro para máximo rendimiento  
- `torch.set_num_threads(4)` - Optimización de threads CPU
- **Singleton Pattern** - Evita recargar modelos (91% menos memoria)

#### Preprocessing Inteligente
- **3 estrategias de preprocessing** por cada detector

### 🚀 Inicialización del Servidor Python

#### Código de Arranque (api_main.py)
El servidor Python QReader se ejecuta en puerto 8008 con el siguiente código:

```python
if __name__ == "__main__":
    # Este bloque es para pruebas locales y no se ejecutará en producción con uvicorn
    # Para ejecutar: python api_main.py
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8008)
```

#### Comandos de Ejecución

**Desarrollo/Testing Local:**
```bash
cd /home/client_1099_1/scripts/qreader_server
python api_main.py
```

**Producción (Recomendado):**
```bash
cd /home/client_1099_1/scripts/qreader_server
uvicorn api_main:app --host 0.0.0.0 --port 8008 --workers 1
```

**Con Logging Detallado:**
```bash
uvicorn api_main:app --host 0.0.0.0 --port 8008 --log-level debug --workers 1
```

**Background Process:**
```bash
nohup uvicorn api_main:app --host 0.0.0.0 --port 8008 --workers 1 > qreader_api.log 2>&1 &
```

#### Verificación de Estado
Una vez iniciado, verifica que el servidor esté funcionando:

```bash
# Health Check
curl http://localhost:8008/health

# Métricas del Engine
curl http://localhost:8008/qr-hybrid-metrics

# Verificar proceso
ps aux | grep "api_main.py" | grep -v grep
```

#### Dependencias del Proyecto
Asegúrate de que estén instaladas las dependencias necesarias:

```bash
pip install fastapi uvicorn qreader torch torchvision opencv-python pyzbar pillow
```

#### Configuración de Memoria
Para optimizar el uso de memoria en producción:

```bash
# Variables de entorno para PyTorch
export PYTORCH_CUDA_ALLOC_CONF=max_split_size_mb:512
export OMP_NUM_THREADS=4
export MKL_NUM_THREADS=4

# Ejecutar servidor con configuración optimizada
uvicorn api_main:app --host 0.0.0.0 --port 8008 --workers 1
```
- **Corrección automática de orientación** 
- **Ajuste de contraste y brillo** adaptativo
- **Detección de bordes mejorada** para QRs dañados

#### Concurrencia y Escalabilidad
- **Thread-safe** - Soporta hasta 100 usuarios simultáneos
- **Memory management** - GC optimizado y cleanup automático
- **Metrics collection** - Monitoreo en tiempo real de rendimiento
- **Singleton models** - Un solo modelo en memoria para todas las requests

### 📊 Integración con Sistema Rust
El sistema Rust utiliza esta API como **última línea de defensa**:

1. **Rust intenta 5 detectores nativos** (rqrr, bardecoder, zbar, quircs, rxing)
2. **Si fallan, usa ONNX** (4 modelos YOLOv8)
3. **Como último recurso, llama a la API Python** en puerto 8008
4. **La API Python ejecuta el Hybrid Engine** con QReader optimizado
5. **Retorna el resultado al sistema Rust** para respuesta al cliente

### ✅ Validación de Rendimiento
Probado exitosamente con:
- **400 requests** bajo diferentes cargas de concurrencia
- **5 imágenes específicas** de facturas panameñas 
- **100% tasa de éxito** en detección
- **Latencia consistente** ~255ms promedio
- **Sin memory leaks** después de 400+ requests

---

### 📊 APIs de Encuestas v4 ✅ NUEVO

#### Lista de Encuestas del Usuario ✅ IMPLEMENTADO
- **Endpoint:** `GET /api/v4/surveys`
- **Autenticación:** JWT requerido
- **Descripción:** Obtiene todas las encuestas asignadas al usuario autenticado
- **🔐 Nota:** El `user_id` se extrae automáticamente del JWT token - NO se envía manualmente

**Query Parameters (Opcionales):**
```
?status_filter=pending|completed|overdue|due_soon
?limit=50 (máximo resultados)
?offset=0 (paginación)
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Ejemplo de Request:**
```bash
curl -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
  http://localhost:8000/api/v4/surveys?status_filter=pending
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "status_id": 1,
      "user_id": 123,
      "survey_id": 1,
      "campaign_id": 1,
      "survey_title": "Hábitos Alimenticios y de Consumo",
      "survey_description": "Comprender los hábitos diarios de alimentación...",
      "instructions": "Por favor responde todas las preguntas de manera honesta...",
      "total_questions": 10,
      "max_attempts": 1,
      "time_limit_minutes": 15,
      "points_per_question": 10,
      "points_per_survey": 100,
      "difficulty": "easy",
      "campaign_name": "Estudio de Mercado Panamá 2025",
      "campaign_category": "Market Research",
      "status": "pending",
      "assigned_at": "2025-08-26T10:00:00Z",
      "due_date": "2025-09-25T23:59:59Z",
      "completed_at": null,
      "is_mandatory": false,
      "responses": null,
      "total_score": null,
      "correct_answers": null,
      "attempts_made": 0,
      "total_time_minutes": null,
      "days_until_due": 30.5,
      "accuracy_percentage": null,
      "priority_order": 2,
      "created_at": "2025-08-26T10:00:00Z",
      "updated_at": "2025-08-26T10:00:00Z"
    }
  ],
  "error": null
}
```

**Campos Destacados:**
- `points_per_question`: Puntos por cada pregunta correcta (int)
- `points_per_survey`: Puntos totales que se pueden ganar completando toda la encuesta (int) ✅ NUEVO
- `status`: Estado calculado dinámicamente (`pending`, `completed`, `overdue`, `due_soon`)
- `accuracy_percentage`: Porcentaje de respuestas correctas (solo para completadas)

**Ordenamiento:**
1. Encuestas completadas primero
2. Por fecha de vencimiento (due_date)
3. Por prioridad (priority_order)

**Rate Limiting:** 60 requests/hora por usuario

#### Detalle de Encuesta Específica ✅ IMPLEMENTADO
- **Endpoint:** `GET /api/v4/surveys/{survey_id}`
- **Autenticación:** JWT requerido
- **Descripción:** Obtiene el detalle completo de una encuesta con todas sus preguntas

**Path Parameters:**
```
survey_id: integer (required)
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Response:**
```json
{
  "success": true,
  "data": {
    "survey": {
      "survey_id": 2,
      "campaign_id": 1,
      "title": "Preferencias y Experiencias en Citas",
      "survey_description": "Comprender hábitos y preferencias en salidas sociales...",
      "instructions": "Responde con sinceridad sobre tus experiencias...",
      "total_questions": 10,
      "max_attempts": 1,
      "time_limit_minutes": 12,
      "points_per_question": 10,
      "points_per_survey": 100,
      "difficulty": "easy",
      "questions": {
        "questions": [
          {
            "question_id": 1,
            "question_text": "¿Actualmente estás?",
            "question_type": "single_choice",
            "options": [
              {"value": "A", "text": "Soltero/a", "is_correct": null},
              {"value": "B", "text": "En una relación", "is_correct": null},
              {"value": "C", "text": "Casado/a", "is_correct": null},
              {"value": "D", "text": "Es complicado", "is_correct": null}
            ],
            "explanation": "Estado civil actual."
          },
          {
            "question_id": 2,
            "question_text": "¿Con qué frecuencia sales a citas?",
            "question_type": "single_choice",
            "options": [
              {"value": "A", "text": "Una vez a la semana o más", "is_correct": null},
              {"value": "B", "text": "1-2 veces al mes", "is_correct": null},
              {"value": "C", "text": "Raramente", "is_correct": null},
              {"value": "D", "text": "Nunca", "is_correct": null}
            ],
            "explanation": "Frecuencia de citas o salidas románticas."
          }
        ]
      }
    },
    "user_status": null
  },
  "error": null
}
```

**Validaciones:**
- Usuario debe tener acceso a la encuesta (debe estar asignada)
- Encuesta debe estar activa

**Rate Limiting:** 120 requests/hora por usuario

#### Guardar Respuestas de Encuesta ✅ IMPLEMENTADO
- **Endpoint:** `PATCH /api/v4/surveys/responses`
- **Autenticación:** JWT requerido
- **Descripción:** Guarda respuestas de encuesta (parciales o completas)
- **🔐 Nota:** El `user_id` se extrae automáticamente del JWT token y se envía a la base de datos - NO se incluye en el request body

**Headers:**
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

**Flujo Interno:**
```
Cliente envía: {survey_id, responses} 
      ↓
Servidor agrega: user_id (del JWT)
      ↓  
Base de datos recibe: (user_id, survey_id, responses, time)
```

**Request Body:**
```json
{
  "survey_id": 1,
  "responses": {
    "answers": [
      {
        "question_id": 1,
        "answer": "A",
        "answered_at": "2025-08-26T10:30:00Z"
      },
      {
        "question_id": 2,
        "answer": ["A", "C"],
        "answered_at": "2025-08-26T10:31:15Z"
      }
    ]
  },
  "is_completed": false,
  "total_time_minutes": 5
}
```

**Response (Respuesta Parcial):**
```json
{
  "success": true,
  "data": {
    "status_id": 1,
    "survey_id": 1,
    "status": "in_progress",
    "total_score": null,
    "correct_answers": null,
    "completed_at": null
  },
  "error": null
}
```

**Response (Encuesta Completada):**
```json
{
  "success": true,
  "data": {
    "status_id": 1,
    "survey_id": 1,
    "status": "completed",
    "total_score": 85,
    "correct_answers": 7,
    "completed_at": "2025-08-26T10:45:00Z"
  },
  "error": null
}
```

**Validaciones:**
- Usuario debe tener acceso a la encuesta
- Encuesta no debe estar ya completada
- No exceder el máximo de intentos
- Formato válido de respuestas

**Características:**
- ✅ **Respuestas parciales** - `is_completed: false`
- ✅ **Auto-scoring** - Cálculo automático de puntajes
- ✅ **Tracking de tiempo** - Registro de tiempo total
- ✅ **Prevención duplicados** - No permite completar encuesta ya finalizada

**Rate Limiting:** 30 requests/hora por encuesta por usuario

#### Errores Comunes de APIs de Encuestas

**SURVEY_NOT_FOUND** (404):
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "SURVEY_NOT_FOUND",
    "message": "Encuesta no encontrada o sin acceso",
    "details": null
  }
}
```

**SURVEY_ALREADY_COMPLETED** (400):
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "SURVEY_ALREADY_COMPLETED",
    "message": "Esta encuesta ya fue completada",
    "details": null
  }
}
```

**MAX_ATTEMPTS_EXCEEDED** (400):
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "MAX_ATTEMPTS_EXCEEDED",
    "message": "Máximo de 1 intentos excedido",
    "details": null
  }
}
```

**SURVEY_NOT_ASSIGNED** (403):
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "SURVEY_NOT_ASSIGNED",
    "message": "Encuesta no asignada al usuario",
    "details": null
  }
}
```

---

### �🖥️ Sistema y Monitoreo v4

#### Health Check Comprensivo ✅ NUEVO
- **Endpoint:** `GET /api/v4/system/health`
- **Descripción:** Health check completo del sistema
- **Respuesta:** Estado de DB, Redis, QR service, webhook system
- **Checks Incluidos:**
  - Database connectivity
  - Redis connectivity  
  - QR service availability
  - Webhook system status
  - Memory and performance metrics

#### Información del Sistema ✅ NUEVO
- **Endpoint:** `GET /api/v4/system/info`
- **Descripción:** Información detallada del sistema
- **Respuesta:** Versión, uptime, configuración, estadísticas
- **Optimizaciones:** Caching (30min), datos comprensivos

#### Estado del Sistema ✅ NUEVO
- **Endpoint:** `GET /api/v4/system/status`
- **Descripción:** Estado actual del sistema en tiempo real
- **Respuesta:** Métricas de performance, carga, conexiones activas

#### Métricas del Sistema ✅ NUEVO
- **Endpoint:** `GET /api/v4/system/metrics`
- **Descripción:** Métricas detalladas para monitoreo
- **Respuesta:** Prometheus-style metrics, performance data
- **Uso:** Integración con sistemas de monitoreo externos

---

## 🏗️ Arquitectura de Middlewares

### Stack de Middlewares v4 (Aplicado en Orden)
1. **Request Limits Middleware** - Timeouts globales (30s)
2. **Security Headers Middleware** - Headers de seguridad reforzados
3. **Performance Middleware** - Métricas de tiempo de respuesta
4. **Compression Middleware** - Compresión gzip automática  
5. **Caching Middleware** - Redis caching inteligente
6. **Rate Limiting Middleware** - Control granular por endpoint y trust score
7. **Upload Validation Middleware** - MIME types y magic bytes (endpoints upload)
8. **Idempotency Middleware** - Prevención duplicados (endpoints mutantes)
9. **JWT Auth Middleware** - Autenticación (endpoints protegidos)

### Stack de Middlewares v3 (Aplicado en Orden)
1. **Deprecation Middleware** - Headers de deprecación y analytics
2. **Rate Limiting Middleware** - Control de tasa básico
3. **Security Headers Middleware** - Headers de seguridad básicos

### Separación de Rutas
- **v4 Routes:** `/api/v4/*` - Stack completo de optimización
- **v3 Routes:** `/api/v3/*` - Stack de deprecación y migración
- **Webhook Routes:** `/webhook*` - Sin middleware (performance crítico)
- **System Routes:** `/health`, `/metrics`, `/status`, `/info` - Acceso directo

---

## 📊 Características Técnicas Avanzadas

### Performance Optimizada
- **Throughput:** >400 RPS validado
- **Latency v4:** <5ms promedio (mejorado con caching)
- **Latency v3:** <8ms promedio (sin optimizaciones)
- **Concurrencia:** Excelente escalado bajo carga
- **Cache Hit Rate:** ~85% en endpoints de consulta

### Caching Inteligente
- **Redis Backend:** Conexiones pooled para alta concurrencia
- **Cache Invalidation:** TTL automático por tipo de endpoint
- **Cache Keys:** SHA256 hash seguro con contexto de usuario
- **Cache Headers:** X-Cache, Cache-Control para debugging
- **Skip Logic:** Endpoints críticos excluidos automáticamente

### Compresión Avanzada
- **Gzip Compression:** Automático para respuestas > 1KB
- **Content-Type Aware:** JSON y text responses
- **Client Detection:** Accept-Encoding header validation
- **Bandwidth Savings:** ~60-80% reducción en respuestas grandes

### Rate Limiting Avanzado
- **Trust Score Dinámico:** 0-50 puntos
- **Límites Personalizados:**
  - Usuarios muy confiables (40+ pts): 5/hora, 20/día
  - Usuarios confiables (25+ pts): 3/hora, 12/día
  - Usuarios nuevos activos (10+ pts): 2/hora, 8/día
  - Usuarios nuevos/sospechosos (<10 pts): 1/hora, 3/día
- **Nota:** El procesamiento OCR no tiene costo en Lümis (funcionalidad gratuita)

### Seguridad Reforzada - JWT IMPLEMENTADO ✅

#### Autenticación JWT Completa
- **JWT Middleware:** Implementado en `src/middleware/auth.rs`
- **Algoritmo:** HS256 con secret key seguro
- **Endpoints Protegidos:** Todos los endpoints críticos v4
  - `GET /api/v4/users/profile` - Perfil del usuario autenticado
  - `GET /api/v4/users/profile/:id` - Perfil por ID (admin/propio)
  - `POST /api/v4/invoices/upload-ocr` - Upload OCR (implementado)
  - `POST /api/v4/invoices/process-from-url` - Procesamiento URL
  - `GET /api/v4/invoices/details` - Consulta de facturas
  - `GET /api/v4/invoices/headers` - Headers de facturas
  - **NOTA:** Cambios de contraseña ahora usan email verification (sin JWT)

#### Características de Seguridad
- **Token Validation:** Extracción y validación automática de Bearer tokens
- **User Injection:** Inyección de `CurrentUser` en request extensions
- **Error Handling:** Manejo completo de tokens expirados/inválidos
- **Frontend Compatibility:** Login v4 devuelve formato TokenResponse directo
- **Bcrypt Hashing:** Passwords con salt seguro
- **Input Validation:** Sanitización completa de inputs + MIME validation
- **Security Headers:** CORS, CSP mejorado, HSTS con preload, Permissions-Policy
- **Rate Limiting:** Protección granular anti-brute force y DDoS
- **Idempotency:** Prevención de operaciones duplicadas
- **Upload Safety:** Magic bytes validation, filename sanitization
- **Logging Completo:** Actividad y errores trackeados
- **Paridad v3:** Misma seguridad que implementación Python + mejoras

### Observabilidad Completa
- **Structured Logging:** Tracing con request IDs únicos
- **Performance Metrics:** Tiempo de respuesta por endpoint
- **Health Checks:** Multi-service monitoring
- **Error Tracking:** Logging detallado de errores
- **Usage Analytics:** Métricas de deprecación v3
- **Real-time Monitoring:** Dashboard metrics disponibles

---

## 🚀 Estado del Proyecto - PAGINACIÓN AVANZADA IMPLEMENTADA

### ✅ FASE 5: PAGINACIÓN OPTIMIZADA Y PERFORMANCE ENTERPRISE-GRADE

**⚡ Paginación Avanzada Implementada:**
- ✅ **Paginación Eficiente:** LIMIT/OFFSET optimizado con performance < 200ms
- ✅ **Filtros Múltiples:** Combinación de fechas, montos, tipos con validación
- ✅ **Ordenamiento Dinámico:** Por cualquier campo con validación de seguridad
- ✅ **Headers HTTP:** X-Total-Count, X-Page-Count, Links de navegación automáticos
- ✅ **Metadatos Completos:** Pagination object con next/prev navigation
- ✅ **Performance Monitoring:** Query time tracking en cada respuesta
- ✅ **Frontend Ready:** Estructura optimizada para UIs de paginación

**🎯 Endpoint Mejorado:**
- ✅ `GET /api/v4/invoices/details` - **Paginación completa implementada**
  - Soporte para `offset` y `page` alternativo
  - Límites configurables (100 default, 1000 max)
  - Filtros avanzados (fechas, montos, tipos múltiples)
  - Ordenamiento por cualquier campo válido
  - Headers de respuesta automáticos para navegación
  - Performance metrics en tiempo real

**🔐 Seguridad y Performance:**
- ✅ **JWT Authentication:** Filtrado automático por usuario del token
- ✅ **Input Validation:** Validación de todos los parámetros de entrada
- ✅ **SQL Injection Protection:** Prepared statements con parámetros seguros
- ✅ **Rate Limiting Ready:** Estructura preparada para rate limiting por usuario
- ✅ **Caching Framework:** Base implementada para caching por página
- ✅ **Index Optimization:** Queries optimizadas para índices de DB

### ✅ FASE 4: AUTENTICACIÓN JWT Y COMPATIBILIDAD FRONTEND FINALIZADA

**🔐 Autenticación JWT Implementada:**
- ✅ **JWT Middleware:** Completo en `src/middleware/auth.rs`
- ✅ **Login v4:** Formato TokenResponse compatible con frontend
- ✅ **Endpoints Protegidos:** Todos los endpoints críticos v4 con JWT obligatorio
- ✅ **Token Validation:** Extracción y validación automática de Bearer tokens
- ✅ **Error Handling:** Manejo completo de autenticación fallida
- ✅ **Frontend Compatibility:** Respuesta directa sin ApiResponse wrapper
- ✅ **Security Parity:** Misma seguridad que implementación Python v2/v3

**🎯 Endpoints JWT Protegidos:**
- ✅ `GET /api/v4/users/profile` - Perfil del usuario autenticado
- ✅ `GET /api/v4/users/profile/:id` - Perfil por ID (admin/propio)
- ✅ `GET /api/v4/userdata` - Datos demográficos desde public.dim_users
- ✅ `POST /api/v4/invoices/process-from-url` - Procesamiento desde URL
- ✅ `GET /api/v4/invoices/details` - Consulta de facturas
- ✅ `GET /api/v4/invoices/headers` - Headers de facturas
- ✅ `POST /api/v4/invoices/upload-ocr` - Upload OCR (implementado con servicio común)

**🏗️ Arquitectura y Performance:**
- ✅ **Migración Completa:** Todos los endpoints críticos migrados a v4
- ✅ **Deprecación Activa:** Headers y analytics en todos los v3
- ✅ **Performance Optimizada:** Caching, compresión, monitoring
- ✅ **Compilación:** Exitosa con solo advertencias menores
- ✅ **Arquitectura:** Modular por dominios con middlewares
- ✅ **Endpoints:** 18+ REST v4 + 12+ REST v3 + 3 webhook
- ✅ **Performance:** Validado >400 RPS con optimizaciones
- ✅ **Observabilidad:** Monitoring completo implementado
- ✅ **Seguridad:** JWT completo, rate limiting, headers de seguridad
- ✅ **Caching:** Redis inteligente con TTL diferenciado
- ✅ **Compresión:** Gzip automático para respuestas grandes  

### Próximos Pasos - Fase 5
- **Keyset Pagination:** Implementar cursores en `details` y `headers` (orden `(reception_date DESC, id DESC)`).
- **Rate Limit Headers:** Exponer `X-RateLimit-*` en respuestas exitosas mutantes.
- **Caching Por Página:** Activar Redis caching por página (usar versión usuario + parámetros normalizados).
- **ETag Ejemplos:** Añadir ejemplos concretos de petición condicional en docs de facturas.
- **OCR Pipeline:** Implementar pipeline real para `upload-ocr` + invalidación versión.
- **Trust Score:** Persistencia real y ajuste dinámico de límites.

**Sistema Híbrido v3/v4 con Paginación Enterprise - Listo para Producción** 🚀⚡

---

## 📋 Endpoints Deprecated (v3) - USAR v4

⚠️ **IMPORTANTE:** Todos estos endpoints están deprecated y serán removidos. Migrar a v4.

### 🔐 Autenticación v3 (DEPRECATED)
- `POST /api/v3/auth/login` → `POST /api/v4/auth/login`
- `POST /api/v3/auth/register` → `POST /api/v4/auth/register`
- `POST /api/v3/auth/verify` → `POST /api/v4/auth/verify`

### 👥 Usuarios v3 (DEPRECATED)
- `POST /api/v3/users/check-email` → `POST /api/v4/users/check-email`
- `POST /api/v3/users/set-password` → `POST /api/v4/users/set-password`
- `POST /api/v3/users/reset-password` → `POST /api/v4/users/reset-password`
- `GET /api/v3/users/profile` → `GET /api/v4/users/profile`

### 📄 Facturas v3 (DEPRECATED)
- `POST /api/v3/invoices/upload-ocr` → `POST /api/v4/invoices/upload-ocr`

---

## 🎮 **GAMIFICACIÓN API v4 - ENGAGEMENT SYSTEM**

### **🔥 Características del Sistema de Gamificación**

**Sistema Completo de Engagement** diseñado para maximizar la retención y participación de usuarios a través de mecánicas de juego comprobadas:

- **🏆 Sistema de Puntos (Lumis):** Experiencia gamificada con múltiples fuentes de recompensas
- **📈 Niveles Dinámicos:** Progresión automática con beneficios exclusivos por nivel
- **🔥 Streaks Inteligentes:** Rastreo de actividades consecutivas con recompensas crecientes
- **🎯 Misiones Temporales:** Desafíos personalizados diarios, semanales y mensuales
- **⚡ Happy Hour Events:** Multiplicadores temporales para maximizar rewards
- **🏅 Sistema de Logros:** Badges y achievements con criterios dinámicos
- **🏁 Leaderboards:** Competencia saludable entre usuarios
- **🛡️ Anti-Gaming:** Detección automática de fraude y gaming del sistema

---

### **📊 Track User Action - Registro de Actividades**

**Endpoint principal para registrar cualquier actividad del usuario y obtener rewards.**

```http
POST /api/v4/gamification/track
Authorization: Bearer {jwt_token}
Content-Type: application/json
```

**Request Body:**
```json
{
  "action": "daily_login",
  "channel": "mobile_app",
  "metadata": {
    "session_duration": 1200,
    "device_info": "iOS 17.1",
    "app_version": "2.1.0"
  }
}
```

**Acciones Soportadas:**
- `daily_login` - Login diario del usuario
- `invoice_upload` - Carga de factura
- `survey_complete` - Completar encuesta

**Canales Soportados:**
- `mobile_app` - Aplicación móvil
- `whatsapp` - Bot de WhatsApp  
- `web_app` - Aplicación web

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "lumis_earned": 25,
    "total_lumis": 1250,
    "xp_earned": 10,
    "current_level": 3,
    "level_name": "Silver Explorer",
    "streaks": {
      "daily_login": {
        "current": 7,
        "bonus_applied": true,
        "next_bonus_at": 14
      }
    },
    "achievements_unlocked": [
      {
        "code": "week_warrior",
        "name": "Guerrero Semanal",
        "description": "7 días consecutivos de actividad",
        "lumis_reward": 100
      }
    ],
    "active_events": [
      {
        "code": "happy_hour_evening",
        "name": "Happy Hour Vespertino",
        "multiplier": 2.0,
        "ends_in_minutes": 47
      }
    ],
    "message": "¡Increíble! 7 días seguidos. ¡Desbloqueaste Guerrero Semanal!"
  },
  "error": null,
  "request_id": "req_gam_89f2c3d1",
  "timestamp": "2025-08-29T14:30:00Z",
  "execution_time_ms": 45,
  "cached": false
}
```

---

### **📱 User Dashboard - Dashboard Completo**

**Obtiene toda la información gamificada del usuario en una sola llamada.**

```http
GET /api/v4/gamification/dashboard
Authorization: Bearer {jwt_token}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "user_id": 1001,
    "email": "usuario@example.com",
    "total_lumis": 2450,
    "current_level": 4,
    "level_name": "Gold Hunter",
    "level_description": "Cazador experimentado con bonificaciones premium",
    "level_color": "#FFD700",
    "level_benefits": [
      "10% bonus en todas las acciones",
      "Acceso a misiones premium",
      "Badge dorado personalizable"
    ],
    "next_level_hint": "Faltan 550 Lumis para Platinum Master",
    "lumis_to_next_level": 550,
    "next_level_name": "Platinum Master",
    "active_streaks": {
      "daily_login": {
        "current": 12,
        "max": 28,
        "bonus_multiplier": 1.5
      },
      "invoice_upload": {
        "current": 3,
        "max": 15,
        "bonus_multiplier": 1.2
      }
    },
    "active_missions_count": 3,
    "completed_missions_count": 18,
    "total_achievements": 12,
    "recent_activity": [
      {
        "action": "survey_complete",
        "lumis_earned": 50,
        "timestamp": "2025-08-29T13:15:00Z"
      },
      {
        "action": "achievement_unlock",
        "achievement": "Survey Master",
        "timestamp": "2025-08-29T13:15:00Z"
      }
    ]
  },
  "error": null,
  "request_id": "req_dash_f3a8b2c9",
  "timestamp": "2025-08-29T14:30:00Z",
  "execution_time_ms": 67,
  "cached": true
}
```

---

### **🎯 User Missions - Misiones Activas**

**Obtiene todas las misiones disponibles y el progreso del usuario.**

```http
GET /api/v4/gamification/missions
Authorization: Bearer {jwt_token}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "mission_code": "daily_login_streak",
      "mission_name": "Racha Diaria",
      "mission_type": "daily",
      "description": "Inicia sesión durante 5 días consecutivos",
      "current_progress": 3,
      "target_count": 5,
      "reward_lumis": 100,
      "due_date": "2025-08-30",
      "status": "active",
      "progress_percentage": 60.0
    },
    {
      "mission_code": "invoice_master_weekly",
      "mission_name": "Maestro de Facturas",
      "mission_type": "weekly",
      "description": "Sube 10 facturas esta semana",
      "current_progress": 7,
      "target_count": 10,
      "reward_lumis": 250,
      "due_date": "2025-08-31",
      "status": "active", 
      "progress_percentage": 70.0
    },
    {
      "mission_code": "survey_champion",
      "mission_name": "Campeón de Encuestas",
      "mission_type": "special",
      "description": "Completa 3 encuestas diferentes en un día",
      "current_progress": 3,
      "target_count": 3,
      "reward_lumis": 200,
      "due_date": null,
      "status": "completed",
      "progress_percentage": 100.0
    }
  ],
  "error": null,
  "request_id": "req_mis_d8e7a1b4",
  "timestamp": "2025-08-29T14:30:00Z",
  "execution_time_ms": 32,
  "cached": false
}
```

---

### **⚡ Active Events - Eventos y Happy Hours**

**Obtiene eventos activos y próximos que afectan las recompensas.**

```http
GET /api/v4/gamification/events
Authorization: Bearer {jwt_token}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "event_code": "happy_hour_evening",
      "event_name": "Happy Hour Vespertino",
      "event_type": "daily",
      "starts_in_minutes": -30,
      "ends_in_minutes": 90,
      "multiplier": 2.0,
      "description": "Duplica tus Lumis entre 6:00 PM y 8:00 PM",
      "is_active_now": true
    },
    {
      "event_code": "weekend_warrior",
      "event_name": "Guerrero de Fin de Semana",
      "event_type": "weekly",
      "starts_in_minutes": 2160,
      "ends_in_minutes": 4320,
      "multiplier": 1.5,
      "description": "Bonus del 50% en actividades de fin de semana",
      "is_active_now": false
    },
    {
      "event_code": "christmas_bonanza",
      "event_name": "Bonanza Navideña",
      "event_type": "seasonal",
      "starts_in_minutes": 8640,
      "ends_in_minutes": 20160,
      "multiplier": 3.0,
      "description": "¡Triple Lumis durante la temporada navideña!",
      "is_active_now": false
    }
  ],
  "error": null,
  "request_id": "req_evt_a2b9c4f1",
  "timestamp": "2025-08-29T14:30:00Z",
  "execution_time_ms": 28,
  "cached": true
}
```

---

### **🏅 User Achievements - Logros del Usuario**

**Obtiene todos los logros disponibles y los desbloqueados por el usuario.**

```http
GET /api/v4/gamification/achievements
Authorization: Bearer {jwt_token}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "achievement_code": "first_invoice",
      "achievement_name": "Primera Factura",
      "description": "Sube tu primera factura al sistema",
      "category": "invoices",
      "difficulty": "bronze",
      "reward_lumis": 50,
      "unlocked_at": "2025-08-15T10:20:00Z",
      "is_unlocked": true
    },
    {
      "achievement_code": "survey_master",
      "achievement_name": "Maestro de Encuestas",
      "description": "Completa 50 encuestas exitosamente",
      "category": "surveys",
      "difficulty": "gold",
      "reward_lumis": 500,
      "unlocked_at": "2025-08-29T13:15:00Z",
      "is_unlocked": true
    },
    {
      "achievement_code": "platinum_explorer",
      "achievement_name": "Explorador Platino",
      "description": "Alcanza el nivel Platinum Master",
      "category": "progression",
      "difficulty": "platinum",
      "reward_lumis": 1000,
      "unlocked_at": null,
      "is_unlocked": false
    },
    {
      "achievement_code": "social_butterfly",
      "achievement_name": "Mariposa Social",
      "description": "Refiere 10 amigos exitosamente",
      "category": "social",
      "difficulty": "silver",
      "reward_lumis": 300,
      "unlocked_at": null,
      "is_unlocked": false
    }
  ],
  "error": null,
  "request_id": "req_ach_c7d2f8a3",
  "timestamp": "2025-08-29T14:30:00Z",
  "execution_time_ms": 41,
  "cached": false
}
```

---

### **ℹ️ Mechanics Info - Información de Mecánicas**

**Obtiene explicaciones detalladas de todas las mecánicas de gamificación.**

```http
GET /api/v4/gamification/mechanics
Authorization: Bearer {jwt_token}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "mechanic_code": "streak_system",
      "mechanic_name": "Sistema de Rachas",
      "mechanic_type": "streak",
      "description": "Recompensas por actividades consecutivas",
      "display_name": "Rachas Diarias",
      "short_description": "Mantén tu racha activa para bonificaciones",
      "long_description": "El sistema de rachas recompensa la consistencia. Cada día que realizas una actividad específica, tu racha aumenta y recibes bonificaciones progresivas.",
      "how_it_works": {
        "steps": [
          "Realiza una actividad (login, subir factura, etc.)",
          "Tu racha aumenta en 1 día",
          "Recibes bonificaciones progresivas",
          "Si faltas un día, la racha se reinicia"
        ],
        "bonuses": {
          "day_3": "10% bonus",
          "day_7": "25% bonus",
          "day_14": "50% bonus",
          "day_30": "100% bonus"
        }
      },
      "rewards": {
        "base_lumis": 10,
        "max_multiplier": 2.0,
        "milestone_rewards": [50, 100, 250, 500]
      },
      "tips": [
        "Usa recordatorios para mantener tu racha",
        "Las rachas se mantienen por 48 horas de gracia",
        "Combina múltiples rachas para máximos beneficios"
      ]
    },
    {
      "mechanic_code": "mission_system",
      "mechanic_name": "Sistema de Misiones",
      "mechanic_type": "mission",
      "description": "Desafíos temporales con objetivos específicos",
      "display_name": "Misiones Dinámicas",
      "short_description": "Completa desafíos para recompensas especiales",
      "long_description": "Las misiones son desafíos temporales que aparecen diariamente, semanalmente o en eventos especiales. Cada misión tiene objetivos claros y recompensas generosas.",
      "how_it_works": {
        "types": ["Diarias", "Semanales", "Mensuales", "Especiales"],
        "assignment": "Automática basada en tu actividad",
        "progression": "Progreso en tiempo real",
        "completion": "Recompensas automáticas al completar"
      },
      "rewards": {
        "daily_missions": "50-150 Lumis",
        "weekly_missions": "200-500 Lumis", 
        "special_missions": "500-1000 Lumis"
      },
      "tips": [
        "Revisa misiones diariamente para nuevos desafíos",
        "Prioriza misiones con mayor recompensa",
        "Algunas misiones tienen tiempo límite"
      ]
    }
  ],
  "error": null,
  "request_id": "req_mech_e5f8a9b2",
  "timestamp": "2025-08-29T14:30:00Z",
  "execution_time_ms": 35,
  "cached": true
}
```

---

### **🏆 Leaderboard - Tabla de Posiciones**

**Obtiene la tabla de posiciones ordenada por Lumis totales.**

```http
GET /api/v4/gamification/leaderboard?limit=50&offset=0
Authorization: Bearer {jwt_token}
```

**Query Parameters:**
- `limit` (optional): Número de usuarios a retornar (default: 50, max: 100)
- `offset` (optional): Offset para paginación (default: 0)

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "rank": 1,
      "user_id": 1001,
      "username": "LumChampion",
      "total_lumis": 15750,
      "current_level": 8,
      "level_name": "Diamond Master"
    },
    {
      "rank": 2,
      "user_id": 2034,
      "username": "FacturaNinja",
      "total_lumis": 14200,
      "current_level": 7,
      "level_name": "Platinum Expert"
    },
    {
      "rank": 3,
      "user_id": 3456,
      "username": "SurveyKing",
      "total_lumis": 12800,
      "current_level": 7,
      "level_name": "Platinum Expert"
    },
    {
      "rank": 4,
      "user_id": 4789,
      "username": "StreakMaster",
      "total_lumis": 11400,
      "current_level": 6,
      "level_name": "Gold Elite"
    },
    {
      "rank": 5,
      "user_id": 5012,
      "username": "QuestHero",
      "total_lumis": 10950,
      "current_level": 6,
      "level_name": "Gold Elite"
    }
  ],
  "error": null,
  "request_id": "req_lead_b4c8d1a7",
  "timestamp": "2025-08-29T14:30:00Z",
  "execution_time_ms": 52,
  "cached": true
}
```

---

### **🔒 Autenticación Requerida**

Todos los endpoints de gamificación requieren autenticación JWT válida:

```http
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```

### **📈 Headers de Rendimiento**

Todos los endpoints incluyen headers informativos:

```http
X-Response-Time-Ms: 45
X-Request-ID: req_gam_89f2c3d1
X-Cached: false
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1693312200
```

### **⚡ Caching Inteligente**

- **Dashboard:** Cache de 2 minutos
- **Eventos:** Cache de 5 minutos
- **Mecánicas:** Cache de 1 hora
- **Leaderboard:** Cache de 30 segundos
- **ETag Support:** Headers condicionales soportados

### **🛡️ Rate Limiting**

- **Track Action:** 100 requests/min por usuario
- **Dashboard:** 200 requests/min por usuario
- **Otros endpoints:** 300 requests/min por usuario

### **🎯 Sistema de Progresión**

**Niveles Disponibles:**
1. **Chispa Lüm** (0 - 99 Lumis) - Usuario nuevo
2. **Bronze Explorer** (100 - 299 Lumis) - 5% bonus
3. **Silver Hunter** (300 - 699 Lumis) - 10% bonus
4. **Gold Elite** (700 - 1499 Lumis) - 15% bonus + misiones premium
5. **Platinum Expert** (1500 - 2999 Lumis) - 20% bonus + eventos exclusivos
6. **Diamond Master** (3000 - 5999 Lumis) - 25% bonus + badges premium
7. **Legendary Hero** (6000+ Lumis) - 30% bonus + beneficios máximos

**🎮 Sistema de Gamificación v4 - Production Ready** 🚀
- `POST /api/v3/invoices/process-from-url` → `POST /api/v4/invoices/process-from-url`
- `GET /api/v3/invoices/details` → `GET /api/v4/invoices/details`
- `GET /api/v3/invoices/headers` → `GET /api/v4/invoices/headers`

### 📱 QR v3 (DEPRECATED)
- `POST /api/v3/qr/detect` → `POST /api/v4/qr/detect`
- `GET /api/v3/qr/health` → `GET /api/v4/qr/health`

**Todos los endpoints v3 incluyen headers de deprecación automáticos y analytics de uso.**
