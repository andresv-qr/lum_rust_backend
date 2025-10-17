# 🛍️ API de Ofertas WS - Documentación de Endpoints

## 📋 Información General

**Base URL**: `https://webh.lumapp.org` (producción) o `http://localhost:8000` (desarrollo)

**Versión API**: v4

**Autenticación**: JWT Bearer Token (requerido en todos los endpoints)

**Formato de Respuesta**: JSON comprimido con GZIP

---

## 🔐 Autenticación

Todos los endpoints requieren un token JWT válido en el header:

```http
Authorization: Bearer <jwt_token>
```

**Obtener Token**: Usar el endpoint de login `/api/v4/auth/login`

**Expiración**: 1 hora desde la generación

---

## 📡 Endpoints Disponibles

### 1. GET /api/v4/ofertasws

Obtiene todas las ofertas disponibles con diferencias de precio superiores a $3.00.

#### Request

**Método**: `GET`

**URL**: `/api/v4/ofertasws`

**Headers Requeridos**:
```http
Authorization: Bearer <jwt_token>
Content-Type: application/json
Accept-Encoding: gzip
```

**Headers Opcionales**:
```http
If-None-Match: "<etag_value>"
```

**Query Parameters**: Ninguno

**Body**: Ninguno

#### Response

**Status Codes**:
- `200 OK` - Ofertas obtenidas exitosamente
- `304 Not Modified` - Contenido no ha cambiado (cuando se envía E-Tag válido)
- `401 Unauthorized` - Token inválido o expirado
- `503 Service Unavailable` - Base de datos WS no disponible

**Response Headers**:
```http
Content-Type: application/json
Content-Encoding: gzip
ETag: "ofertas-2025-10-16-15:00"
Cache-Control: public, max-age=18000
```

**Response Body (200 OK)**:

```typescript
{
  success: boolean;
  data: {
    ofertasws: Oferta[];
    metadata: OfertasMetadata;
  };
}
```

**Tipo de Datos - Oferta**:

```typescript
interface Oferta {
  comercio: string;                    // Nombre del comercio
  producto: string;                    // Nombre del producto
  codigo: string | null;               // Código de barras o SKU (opcional)
  precio_actual: number;               // Precio actual en USD (float)
  fecha_actual: string;                // Fecha del precio actual (ISO 8601: "2025-10-15")
  dias_con_precio_actual: number;      // Días que lleva el precio actual (entero)
  precio_anterior: number;             // Precio anterior en USD (float)
  fecha_anterior: string;              // Fecha del precio anterior (ISO 8601: "2025-10-14")
  precio_minimo_60d: number | null;    // Precio mínimo últimos 60 días (opcional)
  precio_maximo_60d: number | null;    // Precio máximo últimos 60 días (opcional)
  precio_promedio_60d: number | null;  // Precio promedio últimos 60 días (opcional)
  es_precio_mas_bajo: boolean;         // Indica si es el precio más bajo registrado
  porc: number;                        // Porcentaje de descuento (0-100)
  diferencia: number;                  // Diferencia de precio (precio_anterior - precio_actual)
  link: string | null;                 // URL de la oferta (opcional)
  imagen: string | null;               // URL de la imagen del producto (opcional)
}
```

**Tipo de Datos - OfertasMetadata**:

```typescript
interface OfertasMetadata {
  total_count: number;                 // Total de ofertas en la respuesta
  generated_at: string;                // Timestamp de generación (ISO 8601: "2025-10-16T15:00:00Z")
  next_update: string;                 // Timestamp del próximo update (ISO 8601)
  version: string;                     // Versión del cache ("ofertasws:cache:2025-10-16-15:00")
}
```

**Ejemplo de Respuesta Completa**:

```json
{
  "success": true,
  "data": {
    "ofertasws": [
      {
        "comercio": "Rodelag",
        "producto": "XTECH XTC-515 ADAPTADOR TIPO C A USB NEGRO",
        "codigo": "798302162105",
        "precio_actual": 0.1,
        "fecha_actual": "2025-10-15",
        "dias_con_precio_actual": 1,
        "precio_anterior": 18.99,
        "fecha_anterior": "2025-10-14",
        "precio_minimo_60d": 0.1,
        "precio_maximo_60d": 18.99,
        "precio_promedio_60d": 5.18,
        "es_precio_mas_bajo": true,
        "porc": 99.47,
        "diferencia": 18.89,
        "link": "https://rodelag.com/cdn/shop/files/MIGPS0015331_1.jpg?v=1723816445",
        "imagen": "https://rodelag.com/cdn/shop/files/MIGPS0015331_1.jpg?v=1723816445"
      },
      {
        "comercio": "Rodelag",
        "producto": "XTECH XTC-331 ADAPTADOR DISPLAY A HDMI BLANCO",
        "codigo": "798302167391",
        "precio_actual": 0.1,
        "fecha_actual": "2025-10-15",
        "dias_con_precio_actual": 1,
        "precio_anterior": 14.99,
        "fecha_anterior": "2025-10-14",
        "precio_minimo_60d": 0.1,
        "precio_maximo_60d": 14.99,
        "precio_promedio_60d": 4.25,
        "es_precio_mas_bajo": true,
        "porc": 99.33,
        "diferencia": 14.89,
        "link": "https://rodelag.com/cdn/shop/files/MIGPS0015342_1.jpg?v=1723816446",
        "imagen": "https://rodelag.com/cdn/shop/files/MIGPS0015342_1.jpg?v=1723816446"
      }
      // ... hasta ~7,000 ofertas
    ],
    "metadata": {
      "total_count": 7000,
      "generated_at": "2025-10-16T15:00:00Z",
      "next_update": "2025-10-16T20:00:00Z",
      "version": "ofertasws:cache:2025-10-16-15:00"
    }
  }
}
```

**Response Body (304 Not Modified)**:

Sin body (0 bytes). El cliente debe usar su cache local.

**Response Body (401 Unauthorized)**:

```json
**Ejemplo de Respuesta de Error de Token Inválido**:

```json
{
  "success": false,
  "message": "Invalid token"
}
```

---

## 📋 Registro de Cambios

### v1.1.0 (2025-01-16)
- **Schema Update**: Migrado a métricas de 60 días
  - Campos eliminados: `precio_minimo_2m`, `porcentaje_descuento`, `ahorro`, `latest_date`
  - Campos agregados: `fecha_actual`, `fecha_anterior`, `precio_minimo_60d`, `precio_maximo_60d`, `precio_promedio_60d`, `porc`
  - Mejora en precisión de datos históricos
  - Mayor transparencia en evolución de precios

### v1.0.0 (2025-01-13)
```

**Response Body (503 Service Unavailable)**:

```json
{
  "success": false,
  "error": {
    "code": "SERVICE_UNAVAILABLE",
    "message": "WS database not configured. Ofertas API is unavailable."
  }
}
```

#### Características de Cache

- **Cache Redis**: Automático, TTL 12 horas
- **E-Tag Support**: Enviar `If-None-Match` header para obtener 304 si no hay cambios
- **Compresión GZIP**: Respuesta automáticamente comprimida (~563 KB para 7k ofertas)
- **Updates Automáticos**: 
  - Scheduler interno ejecuta a las 10am y 3pm (hora Panamá)
  - No requiere autenticación (proceso del servidor)
  - Llama directamente a la función de cache sin pasar por HTTP

#### Ejemplo de Request con cURL

**Primera llamada (sin E-Tag)**:
```bash
curl -X GET "https://webh.lumapp.org/api/v4/ofertasws" \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Accept-Encoding: gzip" \
  --compressed \
  -v
```

**Llamada subsecuente (con E-Tag)**:
```bash
ETAG="ofertas-2025-10-16-15:00"

curl -X GET "https://webh.lumapp.org/api/v4/ofertasws" \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "If-None-Match: \"${ETAG}\"" \
  -H "Accept-Encoding: gzip" \
  --compressed \
  -v
```

---

### 2. POST /api/v4/ofertasws/refresh

Invalida el cache actual y fuerza una regeneración inmediata de las ofertas. 

**Uso**: Endpoint administrativo para refresh manual fuera de los horarios programados.

**Nota**: Los refreshes automáticos (10am y 3pm) se ejecutan mediante un scheduler interno del servidor que no requiere autenticación ni pasa por este endpoint HTTP.

#### Request

**Método**: `POST`

**URL**: `/api/v4/ofertasws/refresh`

**Headers Requeridos**:
```http
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

**Query Parameters**: Ninguno

**Body**: Ninguno (JSON vacío `{}` opcional)

#### Response

**Status Codes**:
- `200 OK` - Cache refrescado exitosamente
- `401 Unauthorized` - Token inválido o expirado
- `500 Internal Server Error` - Error al refrescar cache
- `503 Service Unavailable` - Base de datos WS no disponible

**Response Body (200 OK)**:

```typescript
{
  success: boolean;
  data: {
    message: string;
    records_count: number;
    compressed_size_bytes: number;
    execution_time_ms: number;
    cache_key: string;
  };
}
```

**Tipo de Datos - RefreshResponse**:

```typescript
interface RefreshResponse {
  message: string;                     // Mensaje de confirmación
  records_count: number;               // Cantidad de ofertas procesadas
  compressed_size_bytes: number;       // Tamaño del payload comprimido en bytes
  execution_time_ms: number;           // Tiempo de ejecución en milisegundos
  cache_key: string;                   // Key de Redis utilizada
}
```

**Ejemplo de Respuesta Completa**:

```json
{
  "success": true,
  "data": {
    "message": "Cache refreshed successfully",
    "records_count": 7000,
    "compressed_size_bytes": 257298,
    "execution_time_ms": 145,
    "cache_key": "ofertasws:cache:2025-10-16-15:00"
  }
}
```

**Response Body (500 Internal Server Error)**:

```json
{
  "success": false,
  "error": {
    "code": "INTERNAL_ERROR",
    "message": "Cache refresh failed: Database connection error"
  }
}
```

#### Ejemplo de Request con cURL

```bash
curl -X POST "https://webh.lumapp.org/api/v4/ofertasws/refresh" \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{}' \
  | jq
```

---

## 📊 Criterios de Filtrado

Las ofertas incluidas en la respuesta cumplen con los siguientes criterios:

1. **Diferencia de precio**: `abs(precio_anterior - precio_actual) > 3`
2. **Precios válidos**: Ambos precios deben ser != NULL y != 0
3. **Precios diferentes**: `precio_actual <> precio_anterior`
4. **Sin valores NaN**: No se aceptan valores NaN en ningún campo de precio
5. **Ordenamiento**: Por diferencia descendente (mejores ofertas primero)
6. **Límite**: Máximo 7,000 ofertas

---

## 🕐 Horarios de Actualización

El cache se actualiza automáticamente mediante un **scheduler interno** del servidor:

- **10:00 AM** hora Panamá (UTC-5) = **15:00 UTC**
- **03:00 PM** hora Panamá (UTC-5) = **20:00 UTC**

**Frecuencia**: 2 veces al día

**Mecanismo**: 
- El scheduler usa `tokio-cron-scheduler` con expresiones cron
- Llama directamente a la función `get_ofertasws_cached()` (sin pasar por HTTP)
- No requiere autenticación JWT (proceso interno del servidor)
- Logs en `ofertasws_cache_refresh_log` con métricas de performance

**Refresh Manual**: Usar el endpoint `POST /api/v4/ofertasws/refresh` (requiere JWT)

**Próximo Update**: Verificar campo `metadata.next_update` en la respuesta

---

## 📈 Performance

| Métrica | Valor Típico |
|---------|--------------|
| **Tamaño sin comprimir** | ~2.0 MB |
| **Tamaño con GZIP** | ~563 KB |
| **Ratio de compresión** | ~72% |
| **Tiempo de respuesta (cache hit)** | 5-15 ms |
| **Tiempo de respuesta (cache miss)** | 400-600 ms |
| **Tiempo refresh automático** | 150-200 ms |
| **Tiempo de respuesta (304)** | 3-5 ms |
| **Transferencia (304)** | 0 bytes |
| **Ofertas típicas** | ~7,000 |

---

## 🏗️ Arquitectura del Sistema de Cache

### Flujo de Actualización Automática

```
┌─────────────────────────────────────────────────────────────┐
│  SERVIDOR RUST (lum_rust_ws)                                │
│                                                              │
│  ┌──────────────────────────────────────┐                   │
│  │ Tokio Cron Scheduler                 │                   │
│  │  - 10am Panamá (15:00 UTC)           │                   │
│  │  - 03pm Panamá (20:00 UTC)           │                   │
│  └──────────┬───────────────────────────┘                   │
│             │                                                │
│             │ (Sin JWT - proceso interno)                   │
│             ▼                                                │
│  ┌──────────────────────────────────────┐                   │
│  │ get_ofertasws_cached()               │                   │
│  │  1. Query PostgreSQL (wsf_consolidado)│                  │
│  │  2. Serializar JSON                  │                   │
│  │  3. Comprimir GZIP                   │                   │
│  │  4. Guardar en Redis                 │                   │
│  │  5. Log métricas en PostgreSQL       │                   │
│  └──────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

### Flujo de Refresh Manual (vía HTTP)

```
┌─────────────┐      JWT      ┌──────────────────────┐
│   Cliente   │──────────────▶│ POST /refresh        │
│  (Admin)    │               │ (requiere auth)      │
└─────────────┘               └──────────┬───────────┘
                                         │
                                         ▼
                              ┌──────────────────────┐
                              │ Middleware Auth      │
                              │ (valida JWT)         │
                              └──────────┬───────────┘
                                         │
                                         ▼
                              ┌──────────────────────┐
                              │ refresh_ofertasws_   │
                              │ cache()              │
                              │  - Invalida Redis    │
                              │  - Llama función     │
                              │    interna           │
                              └──────────────────────┘
```

### Componentes Clave

1. **Scheduler Interno** (`src/tasks/ofertasws_refresh.rs`)
   - Ejecuta automáticamente sin autenticación
   - Cron: `0 0 15 * * *` y `0 0 20 * * *`
   - Llama directamente a `get_ofertasws_cached()`

2. **Endpoint HTTP** (`POST /api/v4/ofertasws/refresh`)
   - Requiere JWT Bearer token
   - Para refresh manual por administradores
   - Pasa por middleware de autenticación

3. **Función de Cache** (`get_ofertasws_cached()`)
   - Compartida por scheduler y endpoint HTTP
   - Genera cache en Redis con TTL 12h
   - Registra métricas en PostgreSQL

4. **Redis Cache**
   - Key: `ofertasws:cache:YYYY-MM-DD:HH:00`
   - Valor: JSON comprimido con GZIP
   - TTL: 43200 segundos (12 horas)

5. **Logging** (tabla `ofertasws_cache_refresh_log`)
   - `executed_at`: Timestamp
   - `status`: success/error
   - `records_count`: Cantidad de ofertas
   - `execution_time_ms`: Performance
   - `request_size_kb`: Tamaño comprimido

---

## 🔍 Códigos de Error

| Código | Status HTTP | Descripción |
|--------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Token JWT inválido, expirado o ausente |
| `SERVICE_UNAVAILABLE` | 503 | Base de datos WS no disponible |
| `INTERNAL_ERROR` | 500 | Error interno del servidor |
| `REDIS_ERROR` | 500 | Error de conexión con Redis |
| `DATABASE_ERROR` | 500 | Error de consulta a base de datos |

---

## 💡 Mejores Prácticas

### 1. **Usar E-Tag para Optimizar Transferencia**

Almacena el E-Tag de la primera respuesta y envíalo en requests subsecuentes:

```javascript
// Primera llamada
const response = await fetch('/api/v4/ofertasws', {
  headers: {
    'Authorization': `Bearer ${token}`,
    'Accept-Encoding': 'gzip'
  }
});

const etag = response.headers.get('etag');
localStorage.setItem('ofertas_etag', etag);

// Llamadas subsecuentes
const cachedEtag = localStorage.getItem('ofertas_etag');
const response2 = await fetch('/api/v4/ofertasws', {
  headers: {
    'Authorization': `Bearer ${token}`,
    'If-None-Match': cachedEtag
  }
});

if (response2.status === 304) {
  // Usar cache local
  const cachedData = localStorage.getItem('ofertas_data');
  return JSON.parse(cachedData);
}
```

### 2. **Cache Local en Cliente**

Implementa cache local con expiración:

```javascript
const CACHE_DURATION = 5 * 60 * 60 * 1000; // 5 horas

function getCachedOfertas() {
  const cached = localStorage.getItem('ofertas_data');
  const timestamp = localStorage.getItem('ofertas_timestamp');
  
  if (cached && timestamp) {
    const age = Date.now() - parseInt(timestamp);
    if (age < CACHE_DURATION) {
      return JSON.parse(cached);
    }
  }
  return null;
}

function setCachedOfertas(data) {
  localStorage.setItem('ofertas_data', JSON.stringify(data));
  localStorage.setItem('ofertas_timestamp', Date.now().toString());
}
```

### 3. **Manejo de Errores**

```javascript
async function fetchOfertas(token) {
  try {
    const response = await fetch('/api/v4/ofertasws', {
      headers: {
        'Authorization': `Bearer ${token}`,
        'Accept-Encoding': 'gzip'
      }
    });

    if (response.status === 304) {
      return getCachedOfertas();
    }

    if (response.status === 401) {
      // Refrescar token o re-login
      await refreshToken();
      return fetchOfertas(newToken);
    }

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();
    setCachedOfertas(data);
    return data;

  } catch (error) {
    console.error('Error fetching ofertas:', error);
    // Usar cache local como fallback
    return getCachedOfertas() || { success: false, error };
  }
}
```

### 4. **Paginación en Cliente (Opcional)**

Para grandes datasets, implementa paginación virtual en el cliente:

```javascript
function paginateOfertas(ofertas, page = 1, itemsPerPage = 50) {
  const start = (page - 1) * itemsPerPage;
  const end = start + itemsPerPage;
  
  return {
    items: ofertas.slice(start, end),
    currentPage: page,
    totalPages: Math.ceil(ofertas.length / itemsPerPage),
    totalItems: ofertas.length
  };
}
```

### 5. **Filtrado por Comercio/Categoría**

Implementa filtros locales después de obtener los datos:

```javascript
function filterByComercio(ofertas, comercioName) {
  return ofertas.filter(o => 
    o.comercio.toLowerCase().includes(comercioName.toLowerCase())
  );
}

function filterByMinDescuento(ofertas, minPercentage) {
  return ofertas.filter(o => o.porcentaje_descuento >= minPercentage);
}

function sortByDescuento(ofertas, ascending = false) {
  return [...ofertas].sort((a, b) => 
    ascending 
      ? a.porcentaje_descuento - b.porcentaje_descuento
      : b.porcentaje_descuento - a.porcentaje_descuento
  );
}
```

---

## 🧪 Testing

### Test de Conectividad

```bash
# Verificar que el endpoint responde
curl -I "https://webh.lumapp.org/api/v4/ofertasws" \
  -H "Authorization: Bearer ${TOKEN}"

# Debe devolver: HTTP/1.1 200 OK o 401 Unauthorized
```

### Test de Performance

```bash
# Medir tiempo de respuesta
time curl -X GET "https://webh.lumapp.org/api/v4/ofertasws" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Accept-Encoding: gzip" \
  --compressed \
  -o /dev/null \
  -w "HTTP: %{http_code}\nTime: %{time_total}s\nSize: %{size_download} bytes\n"
```

### Test de E-Tag

```bash
# Primera llamada: obtener E-Tag
ETAG=$(curl -s -I "https://webh.lumapp.org/api/v4/ofertasws" \
  -H "Authorization: Bearer ${TOKEN}" \
  | grep -i "etag" \
  | cut -d' ' -f2 \
  | tr -d '\r')

echo "E-Tag: ${ETAG}"

# Segunda llamada: usar E-Tag
curl -X GET "https://webh.lumapp.org/api/v4/ofertasws" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "If-None-Match: ${ETAG}" \
  -w "\nStatus: %{http_code}\n"

# Debe devolver: Status: 304
```

---

## 📝 Registro de Cambios

### v1.0.0 (2025-10-16)

**Nuevos Endpoints**:
- ✅ `GET /api/v4/ofertasws` - Obtener ofertas
- ✅ `POST /api/v4/ofertasws/refresh` - Refresh manual

**Características**:
- ✅ Cache Redis con TTL 12 horas
- ✅ Compresión GZIP automática
- ✅ E-Tag support para 304 Not Modified
- ✅ Auto-refresh programado (10am y 3pm Panamá)
- ✅ Logging en PostgreSQL con métricas de performance
- ✅ Campo `request_size_kb` en logs

**Métricas de Logging**:
- `executed_at`: Timestamp de ejecución
- `status`: success/error/partial
- `records_count`: Cantidad de ofertas
- `execution_time_ms`: Tiempo de ejecución
- `request_size_kb`: Tamaño comprimido en KB
- `error_message`: Mensaje de error (si aplica)
- `redis_key`: Key de Redis utilizada

---

## 📞 Soporte

Para reportar problemas o solicitar nuevas características:

- **Logs de aplicación**: `/home/client_1099_1/scripts/lum_rust_ws/nohup_ofertasws.out`
- **Logs de base de datos**: Tabla `ofertasws_cache_refresh_log` en DB `ws`
- **Monitoreo Redis**: Usar `redis-cli KEYS ofertasws:cache:*`

**Queries útiles para debugging**:

```sql
-- Ver últimas ejecuciones
SELECT * FROM ofertasws_cache_refresh_log 
ORDER BY executed_at DESC LIMIT 10;

-- Ver errores
SELECT * FROM ofertasws_cache_refresh_log 
WHERE status = 'error' 
ORDER BY executed_at DESC;

-- Ver métricas de performance
SELECT 
  AVG(execution_time_ms) as avg_time_ms,
  AVG(request_size_kb) as avg_size_kb,
  COUNT(*) as total_executions
FROM ofertasws_cache_refresh_log
WHERE status = 'success'
  AND executed_at >= NOW() - INTERVAL '7 days';
```

---

**Fecha**: 17 de Octubre, 2025  
**Versión**: 1.1.0  
**Autor**: Lüm App Development Team
