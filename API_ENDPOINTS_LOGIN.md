# API ENDPOINTS - SISTEMA DE AUTENTICACIÓN UNIFICADO

**Fecha de actualización**: 26 de Septiembre, 2025  
**Versión**: 4.2 - Gestión de Contraseñas y Verificación de Email  
**Base URL**: `http://localhost:8000`

> 🚀 **Nuevo en v4.2**: Sistema completo de gestión de contraseñas y verificación de email implementado.

---

## 📋 ÍNDICE

1. [Visión General](#visión-general)
2. [Autenticación Unificada](#autenticación-unificada)
3. [Endpoints Auxiliares](#endpoints-auxiliares)
4. [Modelos de Datos](#modelos-de-datos)
5. [Códigos de Error](#códigos-de-error)
6. [Ejemplos de Uso](#ejemplos-de-uso)
7. [Integración con Frontend](#integración-con-frontend)
8. [Migración JWT Completa](#migración-jwt-completa)
9. [Auditoría y Logs](#auditoría-y-logs-automáticos)
10. [Seguridad](#seguridad)

---

## 🎯 VISIÓN GENERAL

El sistema de autenticación unificado permite a los usuarios autenticarse usando diferentes proveedores (email/password, Google OAuth2) a través de un único endpoint. El sistema maneja:

- ✅ **Registro de nuevos usuarios**
- ✅ **Autenticación con email/password**
- ✅ **Autenticación con Google OAuth2**
- ✅ **Generación de tokens JWT unificados**
- ✅ **Validación de email case-insensitive**
- ✅ **Validación de password case-sensitive**
- ✅ **Gestión de contraseñas** con códigos de verificación
- ✅ **Verificación de email** automática
- ✅ **Vinculación de cuentas** (futuro)
- ✅ **Auditoría y logging automático en base de datos**

### 🆕 Nuevas Características v4.2

- **🔑 Gestión de Contraseñas**: Sistema completo de request-code + set-with-code
- **📧 Verificación de Email**: Endpoints públicos para verificar cuentas
- **🛡️ Seguridad Mejorada**: Rate limiting y expiración de códigos
- **🔄 Auto-login**: Generación automática de JWT tras establecer contraseña

### 🔧 Características v4.1 (Previas)

- **🔑 JWT Unificado**: El token contiene toda la información (user_id, email, permisos)
- **🔄 Migración Completa**: Todas las APIs eliminaron dependencia de `user_id` en headers
- **🔒 Seguridad Mejorada**: Imposible inconsistencia entre token y user_id
- **⚡ Performance Optimizado**: Una sola validación por request

---

## 🔐 AUTENTICACIÓN UNIFICADA

### POST `/api/v4/auth/unified`

**Descripción**: Endpoint unificado para autenticación con múltiples proveedores.

**Características**:
- Soporte para email/password y Google OAuth2
- Registro automático de nuevos usuarios
- Tokens JWT con expiración configurable
- Metadatos de auditoría incluidos
- Manejo robusto de errores

#### 📝 Formato de Petición

**Headers**:
```
Content-Type: application/json
```

**Body (Email/Password)**:
```json
{
  "provider": "email",
  "email": "usuario@ejemplo.com",
  "password": "contraseña123",
  "name": "Nombre Usuario",           // Solo para registro
  "client_info": {
    "ip": "192.168.1.100",
    "user_agent": "Mozilla/5.0...",
    "device_id": "device-uuid-123"
  },
  "create_if_not_exists": true,       // true = registro, false = solo login
  "linking_token": null               // Para vincular cuentas (futuro)
}
```

**Body (Google OAuth2)**:
```json
{
  "provider": "google",
  "id_token": "eyJhbGciOiJSUzI1NiIs...",
  "access_token": "ya29.a0AfH6SMA...",  // Opcional
  "client_info": {
    "ip": "192.168.1.100",
    "user_agent": "Mozilla/5.0...",
    "device_id": "device-uuid-123"
  },
  "create_if_not_exists": true,
  "linking_token": null
}
```

#### ✅ Respuesta Exitosa (200 OK)

```json
{
  "status": "success",
  "user": {
    "id": 68,
    "email": "usuario@ejemplo.com",
    "name": "Nombre Usuario",
    "avatar_url": null,
    "providers": ["email"],
    "primary_provider": "email",
    "email_verified": true,
    "account_status": "Active",
    "created_at": "2025-09-19T03:12:48.363444341Z",
    "last_login_at": "2025-09-19T03:12:48.363444341Z"
  },
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_at": "2025-09-20T03:12:48.363445073Z",
  "metadata": {
    "request_id": "81a8cb09-4e66-4419-8771-fadb18255e43",
    "provider_used": "email",
    "is_new_user": true,
    "linking_performed": false,
    "execution_time_ms": 906,
    "timestamp": "2025-09-19T03:12:48.363446535Z"
  }
}
```

#### ❌ Respuesta de Error (401 Unauthorized)

```json
{
  "status": "error",
  "message": "Invalid credentials",
  "error_code": "AUTH_FAILED",
  "retry_after": null,
  "metadata": {
    "request_id": "28d50b5b-5e77-488e-84a8-25bcffe89d21",
    "provider_used": "unknown",
    "is_new_user": false,
    "linking_performed": false,
    "execution_time_ms": 0,
    "timestamp": "2025-09-19T03:13:59.835660236Z"
  }
}
```

---

## 🛠️ ENDPOINTS AUXILIARES

### GET `/api/v4/auth/health`

**Descripción**: Verificación del estado del sistema de autenticación.

**Respuesta**:
```json
{
  "status": "healthy",
  "services": {
    "database": "connected",
    "redis": "connected", 
    "google_oauth": "configured"
  },
  "timestamp": "2025-09-19T03:15:28.067Z"
}
```

## 🔑 GESTIÓN DE CONTRASEÑAS

### POST `/api/v4/passwords/request-code`

**Descripción**: Solicitar código de verificación para establecer/cambiar contraseña.
**Autenticación**: ❌ No requerida (endpoint público)

**Request Body**:
```json
{
  "email": "usuario@ejemplo.com",
  "purpose": "first_time_setup"  // "first_time_setup", "reset_password", "change_password"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Verification code sent to email",
    "expires_in_minutes": 15,
    "max_attempts": 3
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### POST `/api/v4/passwords/set-with-code`

**Descripción**: Establecer contraseña usando código de verificación.
**Autenticación**: ❌ No requerida (endpoint público)

**Request Body**:
```json
{
  "email": "usuario@ejemplo.com",
  "verification_code": "123456",
  "new_password": "MiNuevaContraseña123!",
  "confirmation_password": "MiNuevaContraseña123!"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "user_id": 42,
    "email": "usuario@ejemplo.com",
    "password_updated_at": "2025-09-26T15:30:00Z",
    "login_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

## 📧 VERIFICACIÓN DE EMAIL

### POST `/api/v4/users/send-verification`

**Descripción**: Enviar código de verificación para confirmar email.
**Autenticación**: ❌ No requerida (endpoint público)

**Request Body**:
```json
{
  "email": "usuario@ejemplo.com"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "message": "Verification code sent to email",
    "expires_in_minutes": 15
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### POST `/api/v4/users/verify-account`

**Descripción**: Verificar código de verificación recibido por email.
**Autenticación**: ❌ No requerida (endpoint público)

**Request Body**:
```json
{
  "email": "usuario@ejemplo.com",
  "verification_code": "123456"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "user_id": 42,
    "email": "usuario@ejemplo.com",
    "verified": true,
    "verified_at": "2025-09-26T15:30:00Z"
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### GET `/api/v4/auth/config`

**Descripción**: Configuración pública del sistema de autenticación.

**Respuesta**:
```json
{
  "providers": {
    "email": {
      "enabled": true,
      "registration_enabled": true
    },
    "google": {
      "enabled": true,
      "client_id_masked": "123456789...xyz",
      "registration_enabled": true
    }
  },
  "token": {
    "expiry_hours": 24,
    "refresh_enabled": false
  }
}
```

---

## 📊 MODELOS DE DATOS

### Usuario Autenticado
```typescript
interface AuthenticatedUser {
  id: number;
  email: string;
  name: string | null;
  avatar_url: string | null;
  providers: string[];           // ["email", "google"]
  primary_provider: string;      // "email" | "google"
  email_verified: boolean;
  account_status: "Active" | "Suspended" | "PendingVerification";
  created_at: string;           // ISO 8601
  last_login_at: string | null; // ISO 8601
}
```

### Información del Cliente
```typescript
interface ClientInfo {
  ip: string;
  user_agent: string;
  device_id: string;
}
```

### Metadatos de Auditoría
```typescript
interface AuthMetadata {
  request_id: string;           // UUID único por petición
  provider_used: string;        // "email" | "google"
  is_new_user: boolean;         // true si es registro
  linking_performed: boolean;   // true si se vinculó cuenta
  execution_time_ms: number;    // Tiempo de procesamiento
  timestamp: string;           // ISO 8601
}
```

---

## ⚠️ CÓDIGOS DE ERROR

| Código | Descripción | Solución |
|--------|-------------|----------|
| `AUTH_FAILED` | Credenciales inválidas | Verificar email/password o token |
| `NOT_IMPLEMENTED` | Funcionalidad no implementada | Usar otro proveedor |
| `VALIDATION_ERROR` | Error en formato de datos | Revisar estructura JSON |
| `PROVIDER_ERROR` | Error del proveedor externo | Verificar token de Google |
| `DATABASE_ERROR` | Error de base de datos | Reintentar o contactar soporte |
| `INTERNAL_ERROR` | Error interno del servidor | Contactar soporte |

---

## 💡 EJEMPLOS DE USO

### 1. Registro de Nuevo Usuario con Email

```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "email": "nuevo@usuario.com",
    "password": "MiPassword123!",
    "name": "Nuevo Usuario",
    "client_info": {
      "ip": "192.168.1.100",
      "user_agent": "MyApp/1.0",
      "device_id": "device-abc-123"
    },
    "create_if_not_exists": true
  }'
```

**Respuesta esperada**: Usuario creado con `is_new_user: true`

### 2. Login con Usuario Existente

```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "email": "usuario@existente.com",
    "password": "MiPassword123!",
    "client_info": {
      "ip": "192.168.1.100",
      "user_agent": "MyApp/1.0",
      "device_id": "device-abc-123"
    },
    "create_if_not_exists": false
  }'
```

**Respuesta esperada**: Login exitoso con `is_new_user: false`

### 3. Autenticación con Google OAuth2

```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "google",
    "id_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
    "client_info": {
      "ip": "192.168.1.100",
      "user_agent": "MyApp/1.0",
      "device_id": "device-abc-123"
    },
    "create_if_not_exists": true
  }'
```

**Respuesta esperada**: Login/registro con datos de Google

### 4. Verificación de Token JWT

Una vez obtenido el token, usarlo en requests posteriores. **El JWT ya contiene toda la información del usuario (incluyendo user_id), NO es necesario enviar user_id por separado**:

```bash
# ✅ MÉTODO NUEVO (Correcto) - Solo el token JWT
curl -X GET http://localhost:8000/api/v4/users/profile \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."

# ❌ MÉTODO VIEJO (Deprecado) - NO hacer esto
curl -X GET http://localhost:8000/api/v4/users/profile \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "user_id: 69"
```

**Contenido del JWT**:
```json
{
  "sub": "69",                    // user_id extraído automáticamente
  "email": "usuario@ejemplo.com",
  "iat": 1758258371,              // timestamp de creación
  "exp": 1758344771               // timestamp de expiración
}
```

### 5. Manejo de Errores - Credenciales Inválidas

```bash
curl -X POST http://localhost:8000/api/v4/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "email": "usuario@existente.com",
    "password": "PasswordIncorrecto",
    "create_if_not_exists": false
  }'
```

**Respuesta esperada**: Error 401 con `error_code: "AUTH_FAILED"`

---

## 🔧 INTEGRACIÓN CON FRONTEND

### JavaScript/TypeScript Example

```typescript
// Función de login
async function login(email: string, password: string) {
  try {
    const response = await fetch('http://localhost:8000/api/v4/auth/unified', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        provider: 'email',
        email,
        password,
        client_info: {
          ip: await getClientIP(),
          user_agent: navigator.userAgent,
          device_id: getDeviceId()
        },
        create_if_not_exists: false
      })
    });

    const data = await response.json();
    
    if (data.status === 'success') {
      // Guardar SOLO el token (contiene toda la info del usuario)
      localStorage.setItem('auth_token', data.token);
      localStorage.setItem('user_data', JSON.stringify(data.user));
      return { success: true, user: data.user };
    } else {
      return { success: false, error: data.message };
    }
  } catch (error) {
    return { success: false, error: 'Network error' };
  }
}

// Función para hacer requests autenticados
async function apiCall(endpoint: string, options: RequestInit = {}) {
  const token = localStorage.getItem('auth_token');
  
  return fetch(`http://localhost:8000${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`,  // ✅ Solo el token
      ...options.headers
    }
  });
}

// Función de registro
async function register(email: string, password: string, name: string) {
  const response = await fetch('http://localhost:8000/api/v4/auth/unified', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      provider: 'email',
      email,
      password,
      name,
      client_info: {
        ip: await getClientIP(),
        user_agent: navigator.userAgent,
        device_id: getDeviceId()
      },
      create_if_not_exists: true
    })
  });
  
  return response.json();
}
```

### React Hook Example

```typescript
import { useState, useCallback } from 'react';

interface User {
  id: number;
  email: string;
  name: string;
  // ... otros campos
}

export const useAuth = () => {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const login = useCallback(async (email: string, password: string) => {
    setLoading(true);
    setError(null);
    
    try {
      const response = await fetch('/api/v4/auth/unified', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: 'email',
          email,
          password,
          create_if_not_exists: false
        })
      });
      
      const data = await response.json();
      
      if (data.status === 'success') {
        setUser(data.user);
        localStorage.setItem('auth_token', data.token);
      } else {
        setError(data.message);
      }
    } catch (err) {
      setError('Error de conexión');
    } finally {
      setLoading(false);
    }
  }, []);

  return { user, login, loading, error };
};
```

---

## 📈 MÉTRICAS Y MONITORING

El sistema incluye métricas automáticas para:

- **Tiempo de respuesta** (`execution_time_ms`)
- **Proveedores utilizados** (`provider_used`)
- **Registros vs logins** (`is_new_user`)
- **IDs únicos de petición** (`request_id`)
- **Timestamps precisos** para auditoría

---

## 📝 AUDITORÍA Y LOGS AUTOMÁTICOS

### Sistema de Auditoría

El sistema registra **automáticamente** todos los eventos de autenticación en la tabla `public.auth_audit_log` de la base de datos:

#### Eventos Registrados

- ✅ **login_attempt**: Intento de login iniciado
- ✅ **login_success**: Login exitoso  
- ✅ **login_failure**: Login fallido (usuario no encontrado, password incorrecto, etc.)
- ✅ **register_attempt**: Intento de registro iniciado
- ✅ **register_success**: Registro exitoso
- ✅ **register_failure**: Registro fallido
- ✅ **google_auth**: Autenticación con Google (éxito/fallo)

#### Información Capturada

```sql
-- Estructura de la tabla de auditoría
public.auth_audit_log (
    id SERIAL PRIMARY KEY,
    user_id INTEGER,              -- ID del usuario (si disponible)
    event_type VARCHAR(50),       -- Tipo de evento
    provider VARCHAR(50),         -- "email" | "google"
    ip_address INET,              -- IP del cliente
    user_agent TEXT,              -- User agent del navegador
    success BOOLEAN,              -- Si el evento fue exitoso
    error_code VARCHAR(50),       -- Código de error (si aplica)
    error_message TEXT,           -- Mensaje de error detallado
    metadata JSONB,               -- Información adicional
    session_id VARCHAR(100),      -- ID de sesión (futuro)
    request_id VARCHAR(100),      -- ID único de request
    created_at TIMESTAMP          -- Timestamp del evento
);
```

#### Consultas Útiles

```sql
-- Ver últimos 10 eventos
SELECT * FROM public.auth_audit_log 
ORDER BY created_at DESC LIMIT 10;

-- Ver intentos fallidos por IP
SELECT ip_address, COUNT(*), MAX(created_at) 
FROM public.auth_audit_log 
WHERE success = false 
GROUP BY ip_address 
ORDER BY count DESC;

-- Ver estadísticas por proveedor
SELECT provider, event_type, success, COUNT(*) 
FROM public.auth_audit_log 
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY provider, event_type, success;
```

#### Beneficios

- 🔍 **Detección de ataques**: Identificar intentos de fuerza bruta
- 📊 **Análisis de uso**: Estadísticas de proveedores más usados
- 🐛 **Debugging**: Rastrear problemas con request_id único
- 📈 **Métricas**: Tasas de éxito/fallo por proveedor
- 🚨 **Alertas**: Base para sistemas de alerta automáticos

---

## � MIGRACIÓN JWT COMPLETA

### ¡Cambio Importante!

**A partir de esta versión, todas las APIs han migrado al nuevo sistema JWT unificado**:

#### ✅ Nuevo Método (Implementado)
```javascript
// Solo necesitas el token JWT
fetch('/api/v4/cualquier-endpoint', {
  headers: {
    'Authorization': 'Bearer ' + jwt_token  // Contiene user_id + email + permisos
  }
});
```

#### ❌ Método Anterior (Deprecado)
```javascript
// Ya NO es necesario enviar user_id por separado
fetch('/api/v4/cualquier-endpoint', {
  headers: {
    'Authorization': 'Bearer ' + jwt_token,
    'user_id': user_id  // ❌ ELIMINADO - redundante
  }
});
```

### Beneficios de la Migración

1. **🔒 Mayor Seguridad**: Imposible inconsistencia entre JWT y user_id
2. **⚡ Mejor Performance**: Una sola validación en lugar de dos
3. **🧩 Simplicidad**: Los desarrolladores solo gestionan un token
4. **🛡️ Menos Errores**: No se puede enviar user_id incorrecto

### APIs Actualizadas

Todas estas APIs ahora usan **solo JWT** (sin user_id separado):

- ✅ `/api/v4/users/*` - Perfil y gestión de usuarios
- ✅ `/api/v4/invoices/*` - Gestión de facturas  
- ✅ `/api/v4/rewards/*` - Sistema de recompensas
- ✅ `/api/v4/surveys/*` - Encuestas y formularios
- ✅ `/api/v4/gamification/*` - Sistema de gamificación
- ✅ Todos los endpoints protegidos

### Decodificación del JWT

```typescript
// El JWT contiene automáticamente:
interface JWTPayload {
  sub: string;        // user_id ("69")
  email: string;      // "usuario@ejemplo.com" 
  iat: number;        // timestamp creación
  exp: number;        // timestamp expiración
}

// El servidor extrae automáticamente el user_id del campo 'sub'
const userId = jwt.sub;  // No necesitas enviarlo por separado
```

---

## �🔒 SEGURIDAD

### Medidas Implementadas

- ✅ **Hashing bcrypt** para contraseñas
- ✅ **Validación de tokens Google** 
- ✅ **Tokens JWT con expiración**
- ✅ **Validación de formato de email**
- ✅ **Logging de auditoría**
- ✅ **Sanitización de inputs**

### Buenas Prácticas

1. **Usar HTTPS** en producción
2. **Rotación de secrets** JWT regularmente  
3. **Rate limiting** por IP
4. **Validación de tokens** en cada request
5. **Logging** de intentos fallidos
6. **Timeout** de sesiones inactivas

---

## 🚀 PRÓXIMOS DESARROLLOS

### ✅ Completado Recientemente

- [x] **JWT Unificado** - Eliminación de user_id separado en headers
- [x] **Email Case-Insensitive** - Login funciona con cualquier combinación de mayúsculas/minúsculas
- [x] **Password Case-Sensitive** - Mantiene seguridad en contraseñas
- [x] **Audit Logging Automático** - Registro completo de eventos de autenticación
- [x] **Migración API Completa** - Todas las APIs usan solo JWT

### Funcionalidades Implementadas Adicionales

- [x] **Reset de contraseña** por email - `POST /api/v4/passwords/request-code` + `POST /api/v4/passwords/set-with-code`
- [x] **Verificación de email** automática - `POST /api/v4/users/send-verification` + `POST /api/v4/users/verify-account`
- [x] **Rate limiting** implementado (3 códigos por hora)
- [x] **Códigos de verificación** con expiración automática

### Funcionalidades Planificadas

- [ ] **Refresh tokens** para renovación automática
- [ ] **2FA/MFA** con SMS o authenticator apps
- [ ] **Social logins** adicionales (Facebook, GitHub)
- [ ] **Vinculación de cuentas** existentes
- [ ] **Rate limiting** avanzado para todas las APIs
- [ ] **Geolocalización** de logins

### Mejoras Técnicas

- [ ] **Cache** de tokens en Redis
- [ ] **Métricas** avanzadas con Prometheus
- [ ] **Tests** automatizados
- [ ] **Documentación** OpenAPI/Swagger
- [ ] **SDK** para diferentes lenguajes

---

## 📞 SOPORTE

Para consultas sobre la API de autenticación:

- **Documentación técnica**: Este archivo
- **Logs del servidor**: Revisar logs con `request_id`
- **Ambiente de desarrollo**: `http://localhost:8000`
- **Health check**: `GET /api/v4/auth/health`
- **Audit logs**: `./test_audit_logging.sh` para probar logging automático

---

*Documento actualizado: 19 de Septiembre, 2025 - v4.1 JWT Unificado*

**Cambios importantes en esta versión:**
- ✅ Migración completa a JWT unificado
- ✅ Eliminación de `user_id` en headers
- ✅ Email case-insensitive implementation
- ✅ Password case-sensitive mantained
- ✅ Sistema de auditoría automático implementado