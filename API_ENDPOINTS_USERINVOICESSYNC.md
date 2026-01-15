# 📊 User Invoices Sync API - Incremental Synchronization
## Sistema de Sincronización Incremental con Integridad de Datos

**Versión:** 3.0 (Production - UTC Timestamps + Full Sync + Sync Status)  
**Fecha:** 2026-01-14  
**Estado:** ✅ EN PRODUCCIÓN

---

## 🆕 Novedades v3.0

| Feature | Descripción |
|---------|-------------|
| `full_sync` | Parámetro para forzar sincronización completa |
| `sync-status` | Nuevo endpoint para verificar estado de sincronización |
| `recovery` | Endpoints POST para sincronización por CUFE/IDs conocidos |
| UTC unificado | Todos los timestamps en DateTime<Utc> |

---

## 📋 Resumen de Endpoints

| # | Endpoint | Método | Descripción |
|---|----------|--------|-------------|
| 0 | `/api/v4/invoices/sync-status` | GET | ⭐ Estado de sincronización (count + max_date) |
| 1 | `/api/v4/invoices/products` | GET | Productos del usuario |
| 2 | `/api/v4/invoices/issuers` | GET | Emisores/tiendas del usuario |
| 3 | `/api/v4/invoices/headers` | GET | Encabezados de facturas |
| 3b | `/api/v4/invoices/headers/recovery` | **POST** | ⭐ Recovery por CUFE |
| 4 | `/api/v4/invoices/details` | GET | Detalles/líneas de facturas |
| 4b | `/api/v4/invoices/details/recovery` | **POST** | ⭐ Recovery por ID |
| 5 | `/api/v4/invoices/integrity-summary` | GET | Validación de integridad |

### Estrategias de Sincronización

| Estrategia | Método | Endpoint | Cuándo usar |
|------------|--------|----------|-------------|
| **Incremental** | GET | `/headers?update_date_from=...` | Sync diario/frecuente |
| **Full Sync** | GET | `/headers?full_sync=true` | Reinstalación, corrupción |
| **Recovery** | POST | `/headers/recovery` | Verificación de consistencia |

### Parámetros GET (Endpoints 1-4)

| Parámetro | Tipo | Default | Descripción |
|-----------|------|---------|-------------|
| `update_date_from` | `string` | - | ISO 8601 UTC. Filtrar registros desde esta fecha |
| `full_sync` | `boolean` | `false` | Si `true`, ignora `update_date_from` |
| `limit` | `integer` | `20` | Max 100 registros por página |
| `offset` | `integer` | `0` | Para paginación |

### Body POST (Recovery Endpoints)

```typescript
// POST /api/v4/invoices/headers/recovery
{
  "known_cufes": ["CUFE1", "CUFE2", ...],  // CUFEs que el cliente ya tiene
  "limit": 100                              // Máximo de faltantes a retornar
}

// POST /api/v4/invoices/details/recovery
{
  "known_ids": ["CUFE1_CODE1", "CUFE1_CODE2", ...],  // IDs compuestos
  "limit": 100
}
```

---

## ⏰ Timestamps y Zonas Horarias (UTC)

> **IMPORTANTE:** Todos los timestamps en el sistema de sincronización están en **UTC**.

### Formato de timestamps:
```
2025-01-14T15:30:00Z
```

### Parámetros de filtrado:
| Parámetro | Formato | Ejemplo |
|-----------|---------|---------|
| `update_date_from` | ISO 8601 UTC | `?update_date_from=2025-01-14T00:00:00Z` |
| `since` | ISO 8601 UTC | `?since=2025-01-14T10:30:00Z` |

### Campos de respuesta:
| Campo | Descripción |
|-------|-------------|
| `max_update_date` | Timestamp UTC más reciente en el dataset |
| `update_date` | Timestamp UTC de última modificación por registro |
| `deleted_at` | Timestamp UTC de eliminación (soft delete) |

### Notas para clientes:
- Siempre enviar timestamps con sufijo `Z` (UTC)
- Guardar `max_update_date` para la siguiente sincronización incremental
- Panamá está en UTC-5 (convertir para mostrar al usuario)

---

## 🎯 Objetivo del Sistema

Garantizar que los datos de facturas, productos, emisores y detalles entre el backend y frontend estén **siempre sincronizados** con:
- ✅ **Actualización incremental** (solo descargar cambios nuevos/modificados)
- ✅ **Detección de eliminaciones** (tracking de deletes)
- ✅ **Validación de integridad** (checksums SHA256, Materialized Views con XOR hash)
- ✅ **Detección de desincronización** (validación diaria a las 3:15 AM UTC)
- ✅ **Escalable a 50K usuarios activos** (6-18ms por request)

---

## 📐 Arquitectura de Sincronización

### Componentes Clave

```
┌─────────────────────────────────────────────────────────┐
│                    FRONTEND                              │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Local Storage / IndexedDB                       │   │
│  │  - products_data                                 │   │
│  │  - products_last_sync: "2025-11-08T10:30:00Z"   │   │
│  │  - products_checksum: "sha256:abc123..."        │   │
│  │  - products_hash: 1651528645 (from MV)          │   │
│  │  - products_count: 1475                          │   │
│  └──────────────────────────────────────────────────┘   │
│                         ↕                                │
│     Sync Incremental (frecuente):                       │
│     GET /api/v4/invoices/products                        │
│     ?update_date_from=2025-11-08T10:30:00Z              │
│                         ↕                                │
│     Validación Integridad (1x día):                     │
│     GET /api/v4/invoices/integrity-summary               │
└─────────────────────────────────────────────────────────┘
                          ↕
┌─────────────────────────────────────────────────────────┐
│                    BACKEND (Rust)                        │
│  ┌──────────────────────────────────────────────────┐   │
│  │  PostgreSQL Database                             │   │
│  │  - dim_product (update_date, is_deleted)        │   │
│  │  - dim_issuer (update_date, is_deleted)         │   │
│  │  - invoice_header (update_date)                 │   │
│  │  - invoice_detail (update_date)                 │   │
│  │                                                  │   │
│  │  Materialized Views (refresh 3:15 AM UTC):      │   │
│  │  - user_product_integrity_daily                 │   │
│  │  - user_issuer_integrity_daily                  │   │
│  │  - user_header_integrity_daily                  │   │
│  │  - user_detail_integrity_daily                  │   │
│  └──────────────────────────────────────────────────┘   │
│                         ↓                                │
│  Response: IncrementalSyncResponse<T>                    │
│  - data: [nuevos/modificados]                           │
│  - pagination: {total, limit, offset, has_more}         │
│  - sync_metadata: {                                     │
│      max_update_date,                                   │
│      data_checksum (SHA256),                            │
│      deleted_since: [...]                               │
│    }                                                     │
└─────────────────────────────────────────────────────────┘
```

---

## 🔄 Flujo de Sincronización Incremental

### Escenario 1: Primera Carga (Cold Start)

```javascript
// 1. Frontend pide datos iniciales
GET /api/v4/invoices/products?limit=100

// 2. Backend responde con datos completos
{
  "data": [...100 productos],
  "pagination": {
    "total_records": 1524,
    "returned_records": 100,
    "has_more": true
  },
  "sync_metadata": {
    "max_update_date": "2025-10-31T05:40:50.209311",
    "server_timestamp": "2025-11-08T10:31:00.000000",
    "data_checksum": "sha256:e6d71ceff801c357fdd719531736aaf1fc6511cbfa1e4db8112a70e1a3cb8e08",
    "record_ids": ["PROD001", "PROD002", ...],
    "returned_records": 100,
    "deleted_since": {
      "enabled": true,
      "count": 0,
      "items": []
    }
  }
}

// 3. Frontend guarda estado
localStorage.setItem('products_last_sync', '2025-10-31T05:40:50.209311');
localStorage.setItem('products_checksum', 'sha256:e6d71ceff801...');
```

````

### Escenario 2: Sync Incremental (Warm Update)

```javascript
// 1. Frontend recupera último sync
const lastSync = localStorage.getItem('products_last_sync'); // "2025-11-07T10:30:45.123Z"

// 2. Pide solo cambios desde entonces
GET /api/v4/invoices/products?update_date_from=2025-11-07T10:30:45.123Z&limit=100

// 3. Backend retorna solo nuevos/modificados + deletes
{
  "data": [
    {
      "code": "PROD999",
      "description": "Nuevo producto",
      "update_date": "2025-11-07T11:15:00Z"
    },
    {
      "code": "PROD001",
      "description": "Producto modificado",
      "update_date": "2025-11-07T11:20:00Z"
    }
  ],
  "pagination": {
    "total_records": 1525,  // Total global (creció)
    "returned_records": 2,   // Solo 2 cambios desde last sync
    "has_more": false
  },
  "sync_metadata": {
    "max_update_date": "2025-11-07T11:20:00Z",  // 🔑 Nuevo timestamp para próximo sync
    "server_timestamp": "2025-11-07T11:21:00.000Z",
    "data_checksum": "sha256:xyz789...",
    "record_ids": ["PROD999", "PROD001"],
    "dataset_version": 147,  // Version incrementó (hubo cambios)
    "deleted_since": {
      "enabled": true,
      "count": 1,
      "items": [
        {
          "id": "PROD500",
          "deleted_at": "2025-11-07T11:10:00Z"
        }
      ]
    }
  }
}

// 4. Frontend aplica cambios incrementales
// a) Eliminar PROD500
// b) Upsert PROD999 (nuevo)
// c) Upsert PROD001 (modificado)
// d) Guardar nuevo estado
localStorage.setItem('products_last_sync', '2025-11-07T11:20:00Z');
localStorage.setItem('products_version', 147);
```

### Escenario 3: Detección de Desincronización

```javascript
// 1. Frontend hace lightweight version check (periódico)
GET /api/v4/invoices/products/version

Response:
{
  "dataset_version": 150,  // Cambió desde 147!
  "last_modified": "2025-11-07T12:00:00Z",
  "server_timestamp": "2025-11-07T12:05:00Z"
}

// 2. Frontend detecta desync
if (serverVersion > localVersion) {
  console.warn('⚠️ Dataset desactualizado, iniciando sync incremental...');
  syncIncremental(); // Ejecuta Escenario 2
}
```

---

## � Estrategias de Sincronización

El sistema soporta **3 estrategias** de sincronización:

| Estrategia | Cuándo usar | Parámetros |
|------------|-------------|------------|
| **Incremental** | Sync diario/frecuente | `update_date_from=<timestamp>` |
| **Full Sync** | Reinstalación, corrupción, soporte | `full_sync=true` |
| **Verificación** | Antes de decidir estrategia | `GET /sync-status` |

### Flujo Recomendado

```
┌─────────────────────────────────────────────────────────────┐
│                    CLIENTE (Flutter)                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. GET /api/v4/invoices/sync-status                        │
│     → Obtiene: headers_count, headers_max_update_date       │
│                                                              │
│  2. Comparar con estado local:                              │
│     ┌────────────────────────────────────────────────────┐  │
│     │ if (local_count == 0) {                            │  │
│     │   // Primera vez → Full Sync                       │  │
│     │   GET /headers?full_sync=true&limit=100            │  │
│     │ }                                                  │  │
│     │ else if (local_count == server_count &&            │  │
│     │          local_max_date >= server_max_date) {      │  │
│     │   // Sincronizado → No hacer nada                  │  │
│     │ }                                                  │  │
│     │ else {                                             │  │
│     │   // Desactualizado → Incremental                  │  │
│     │   GET /headers?update_date_from=<local_max_date>   │  │
│     │ }                                                  │  │
│     └────────────────────────────────────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## � Autenticación y Errores

### JWT Token

Todos los endpoints requieren un token JWT válido en el header `Authorization`:

```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Estructura del JWT:**
```typescript
interface JwtPayload {
  sub: string;      // User ID
  email: string;    // Email del usuario (requerido)
  exp: number;      // Timestamp de expiración
  iat: number;      // Timestamp de emisión
}
```

### Wrapper de Respuesta Estándar

Todas las respuestas siguen esta estructura:

```typescript
interface ApiResponse<T> {
  success: boolean;           // true si la operación fue exitosa
  data: T | null;             // Datos de la respuesta (null si error)
  error: string | null;       // Mensaje de error (null si success)
  request_id: string;         // UUID único para debugging
  timestamp: string;          // Timestamp UTC de la respuesta
  execution_time_ms: number;  // Tiempo de ejecución en milisegundos
  cached: boolean;            // Si la respuesta vino de cache
}
```

### Códigos de Error HTTP

| Código | Descripción | Causa típica |
|--------|-------------|--------------|
| `200` | ✅ OK | Operación exitosa |
| `400` | ❌ Bad Request | Parámetros inválidos, formato de fecha incorrecto |
| `401` | 🔒 Unauthorized | Token JWT faltante, expirado o inválido |
| `403` | 🚫 Forbidden | Token válido pero sin permisos |
| `404` | 🔍 Not Found | Recurso no existe |
| `500` | 💥 Internal Error | Error del servidor, reportar con `request_id` |

### Ejemplo Error 400 (Bad Request):

```json
{
  "success": false,
  "data": null,
  "error": "Invalid date format. Expected ISO 8601 UTC (e.g., 2026-01-14T00:00:00Z)",
  "request_id": "a1b2c3d4-...",
  "timestamp": "2026-01-14T12:00:00Z",
  "execution_time_ms": 1,
  "cached": false
}
```

### Ejemplo Error 401 (Unauthorized):

```json
{
  "success": false,
  "data": null,
  "error": "Invalid or expired token",
  "request_id": "...",
  "timestamp": "2026-01-14T12:00:00Z",
  "execution_time_ms": 0,
  "cached": false
}
```

---

## �📡 Endpoints API

### 0. GET /api/v4/invoices/sync-status ⭐ NUEVO

**Descripción:** Obtener estado de sincronización del usuario. Endpoint ligero (~1-5ms) para determinar estrategia de sync.

**Headers:**
| Header | Requerido | Descripción |
|--------|-----------|-------------|
| `Authorization` | ✅ Sí | `Bearer <jwt_token>` |

**Query Parameters:** Ninguno

**Response Structure:**

```typescript
interface SyncStatusResponse {
  headers_count: number;                    // Total de facturas del usuario
  headers_max_update_date: string | null;   // Timestamp más reciente (UTC)
  server_timestamp: string;                 // Hora del servidor (UTC)
}
```

**Ejemplo Request:**

```bash
curl -X GET "http://localhost:8000/api/v4/invoices/sync-status" \
     -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

**Ejemplo Response - Usuario con facturas:**

```json
{
  "success": true,
  "data": {
    "headers_count": 1234,
    "headers_max_update_date": "2026-01-14T10:30:00Z",
    "server_timestamp": "2026-01-14T12:00:00Z"
  },
  "error": null,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-01-14T12:00:00Z",
  "execution_time_ms": 3,
  "cached": false
}
```

**Ejemplo Response - Usuario sin facturas:**

```json
{
  "success": true,
  "data": {
    "headers_count": 0,
    "headers_max_update_date": null,
    "server_timestamp": "2026-01-14T12:00:00Z"
  },
  "error": null,
  "request_id": "550e8400-e29b-41d4-a716-446655440001",
  "timestamp": "2026-01-14T12:00:00Z",
  "execution_time_ms": 2,
  "cached": false
}
```

**Uso en Flutter:**

```dart
class SyncService {
  Future<SyncStrategy> determineSyncStrategy() async {
    // 1. Obtener estado del servidor
    final serverStatus = await api.getSyncStatus();
    
    // 2. Obtener estado local
    final localCount = await localDb.getHeadersCount();
    final localMaxDate = await localDb.getMaxUpdateDate();
    
    // 3. Decidir estrategia
    if (localCount == 0) {
      return SyncStrategy.fullSync;
    }
    
    if (localCount == serverStatus.headersCount &&
        localMaxDate != null &&
        !localMaxDate.isBefore(serverStatus.headersMaxUpdateDate)) {
      return SyncStrategy.alreadySynced;
    }
    
    return SyncStrategy.incremental(since: localMaxDate);
  }
}
```

---

### 1. GET /api/v4/invoices/products

**Descripción:** Obtener productos del usuario con sync incremental

**Headers:**
- `Authorization: Bearer <jwt_token>` **REQUERIDO**
- `Content-Type: application/json`
- `x-request-id: <uuid>` (opcional)

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `update_date_from` | `string` | No | - | Filtrar productos actualizados desde esta fecha (ISO 8601 UTC) |
| `full_sync` | `boolean` | No | `false` | ⭐ Si `true`, ignora `update_date_from` y retorna TODOS los registros |
| `limit` | `integer` | No | `20` | Número máximo de items por página (max 100) |
| `offset` | `integer` | No | `0` | Número de items a omitir (paginación) |

**Ejemplos de uso:**

```bash
# Incremental sync (solo cambios desde fecha)
GET /api/v4/invoices/products?update_date_from=2026-01-14T00:00:00Z&limit=50

# Full sync (todos los registros)
GET /api/v4/invoices/products?full_sync=true&limit=100
```

**Response Structure:**

```typescript
interface IncrementalSyncResponse<T> {
  data: T[];
  pagination: {
    total_records: number;        // Total global del dataset
    returned_records: number;     // Cuántos retornó esta query
    limit: number;
    offset: number;
    has_more: boolean;            // ¿Hay más páginas?
    total_pages: number;
    current_page: number;
  };
  sync_metadata: {
    // Para próximo sync incremental
    max_update_date: string | null;      // Timestamp del registro más reciente
    server_timestamp: string;            // Cuándo se generó esta respuesta
    
    // Validación de integridad
    data_checksum: string;               // SHA256 del data array
    record_ids: string[];                // IDs de los items retornados
    returned_records: number;            // Duplicado para validación
    
    // Tracking de eliminaciones
    deleted_since: {
      enabled: boolean;
      count: number;
      items: Array<{
        id: string;
        deleted_at: string;
      }>;
    };
  };
}

// Nuevo: Response de validación de integridad (1x día)
interface IntegritySummaryResponse {
  products: ResourceIntegritySummary;
  issuers: ResourceIntegritySummary;
  headers: ResourceIntegritySummary;
  details: ResourceIntegritySummary;
}

interface ResourceIntegritySummary {
  total_count: number;           // Conteo total de registros
  global_hash: number;           // XOR hash de todos los IDs (bigint)
  last_update: string | null;    // Última actualización en el dataset
  snapshot_time: string;         // Cuándo se tomó el snapshot (3:15 AM UTC)
}
```

**Product Response Fields:**

```typescript
interface UserProductsResponse {
  code: string | null;              // Código único del producto
  code_cleaned: string | null;      // Código normalizado
  issuer_name: string | null;       // Nombre del emisor
  issuer_ruc: string | null;        // RUC del emisor
  description: string | null;       // Descripción del producto
  l1: string | null;                // Clasificación nivel 1
  l2: string | null;                // Clasificación nivel 2
  l3: string | null;                // Clasificación nivel 3
  l4: string | null;                // Clasificación nivel 4
  update_date: string | null;       // Fecha última actualización
}
```

**Ejemplo Request - Primera carga:**

```bash
curl -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
     "http://localhost:8000/api/v4/invoices/products?limit=100"
```

**Ejemplo Request - Sync incremental:**

```bash
curl -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
     "http://localhost:8000/api/v4/invoices/products?update_date_from=2025-11-08T10:30:45Z&limit=100"
```

**Ejemplo Request - Validación de integridad:**

```bash
curl -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
     "http://localhost:8000/api/v4/invoices/integrity-summary"
```

**Ejemplo Response - Primera carga:**

```json
{
  "success": true,
  "data": {
    "data": [
      {
        "code": "467836",
        "code_cleaned": "467836",
        "issuer_name": "A-AMANI, S.A.",
        "issuer_ruc": "53688-13-328636",
        "description": "MALLA 1.83X7.62M 2025-56-2A",
        "l1": null,
        "l2": null,
        "l3": null,
        "l4": null,
        "update_date": null
      }
    ],
    "pagination": {
      "total_records": 1524,
      "returned_records": 5,
      "limit": 5,
      "offset": 0,
      "has_more": true,
      "total_pages": 305,
      "current_page": 1
    },
    "sync_metadata": {
      "max_update_date": null,
      "server_timestamp": "2025-11-08T13:14:51.334908862",
      "data_checksum": "sha256:e6d71ceff801c357fdd719531736aaf1fc6511cbfa1e4db8112a70e1a3cb8e08",
      "record_ids": ["467836", "", "1001002", "MF2014", "IT279"],
      "returned_records": 5,
      "deleted_since": {
        "enabled": true,
        "count": 0,
        "items": []
      }
    }
  },
  "error": null,
  "request_id": "357ad679-2e55-4103-99c1-9b6c5367b82c",
  "timestamp": "2025-11-08T13:14:51.353505548Z",
  "execution_time_ms": 18,
  "cached": false
}
```

**Ejemplo Response - Sync incremental (con cambios desde 2025-11-07):**

```json
{
  "success": true,
  "data": {
    "data": [
      {
        "cufe": "abc123def456",
      "issuer_name": "Tienda Nueva",
      "issuer_ruc": "5555555555-1-2024",
      "description": "Producto Nuevo",
      "l1": "Categoría",
      "l2": "Subcategoría",
      "l3": "Item",
      "l4": "Detalle",
      "update_date": "2025-11-07T11:15:00Z"
    }
  ],
  "pagination": {
    "total_records": 1524,
    "returned_records": 1,
    "limit": 100,
    "offset": 0,
    "has_more": false,
    "total_pages": 1,
    "current_page": 1
  },
  "sync_metadata": {
    "max_update_date": "2025-11-07T11:15:00Z",
    "server_timestamp": "2025-11-07T11:16:00.000Z",
    "data_checksum": "sha256:xyz789...",
    "record_ids": ["PROD999"],
    "returned_records": 1,
    "dataset_version": 146,
    "deleted_since": {
      "enabled": true,
      "count": 1,
      "items": [
        {
          "id": "PROD500",
          "deleted_at": "2025-11-07T11:10:00Z"
        }
      ]
    }
  }
}
```

**Ejemplo Response - Sin cambios:**

```json
{
  "data": [],
  "pagination": {
    "total_records": 1523,
    "returned_records": 0,
    "limit": 100,
    "offset": 0,
    "has_more": false,
    "total_pages": 0,
    "current_page": 0
  },
  "sync_metadata": {
    "max_update_date": null,
    "server_timestamp": "2025-11-07T11:20:00.000Z",
    "data_checksum": "sha256:empty",
    "record_ids": [],
    "returned_records": 0,
    "dataset_version": 145,
    "deleted_since": {
      "enabled": true,
      "count": 0,
      "items": []
    }
  }
}
```

---

### 2. GET /api/v4/invoices/issuers

**Descripción:** Obtener emisores (empresas) del usuario con sync incremental

**Headers:**
- `Authorization: Bearer <jwt_token>` **REQUERIDO**

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `update_date_from` | `string` | No | - | Filtrar emisores actualizados desde esta fecha (ISO 8601 UTC) |
| `full_sync` | `boolean` | No | `false` | ⭐ Si `true`, ignora `update_date_from` y retorna TODOS los registros |
| `limit` | `integer` | No | `20` | Número máximo de items por página (max 100) |
| `offset` | `integer` | No | `0` | Número de items a omitir |

**Ejemplos de uso:**

```bash
# Incremental sync
GET /api/v4/invoices/issuers?update_date_from=2026-01-14T00:00:00Z&limit=50

# Full sync
GET /api/v4/invoices/issuers?full_sync=true&limit=100
```

**Issuer Response Fields:**

```typescript
interface UserIssuersResponse {
  issuer_ruc: string | null;           // RUC/ID fiscal del emisor
  store_id: string | null;             // ID único de la tienda
  store_name: string | null;           // Nombre de la tienda
  brand_name: string | null;           // Nombre de la marca/cadena
  l1: string | null;                   // Clasificación nivel 1 (sector)
  l2: string | null;                   // Clasificación nivel 2 (subsector)
  l3: string | null;                   // Clasificación nivel 3 (categoría)
  l4: string | null;                   // Clasificación nivel 4 (subcategoría)
  update_date: string | null;          // Fecha última actualización
}
```

**Nota:** El ID único de cada issuer es la combinación de `issuer_ruc` + `store_id`. En `sync_metadata.record_ids` se retorna como `"{issuer_ruc}-{store_id}"`.

**Response:** Misma estructura `IncrementalSyncResponse<UserIssuersResponse>`

---

### 3. GET /api/v4/invoices/headers

**Descripción:** Obtener encabezados de facturas del usuario con sync incremental

**Headers:**
- `Authorization: Bearer <jwt_token>` **REQUERIDO**

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `update_date_from` | `string` | No | - | Filtrar facturas actualizadas desde esta fecha (ISO 8601 UTC) |
| `full_sync` | `boolean` | No | `false` | ⭐ Si `true`, ignora `update_date_from` y retorna TODOS los registros |
| `limit` | `integer` | No | `20` | Número máximo de items por página (max 100) |
| `offset` | `integer` | No | `0` | Número de items a omitir |

**Ejemplos de uso:**

```bash
# Incremental sync
GET /api/v4/invoices/headers?update_date_from=2026-01-14T00:00:00Z&limit=50

# Full sync
GET /api/v4/invoices/headers?full_sync=true&limit=100
```

**Invoice Header Response Fields:**

```typescript
interface InvoiceHeadersResponse {
  cufe: string;                         // Código único de factura electrónica (PK)
  issuer_name: string | null;           // Nombre del emisor
  issuer_ruc: string | null;            // RUC del emisor
  store_id: string | null;              // ID de la tienda/sucursal
  no: string | null;                    // Número de factura
  date: string | null;                  // Fecha de emisión (ISO 8601 UTC)
  tot_amount: number | null;            // Monto total de la factura
  tot_itbms: number | null;             // Total de impuestos ITBMS
  url: string | null;                   // URL de verificación DGI
  process_date: string | null;          // Fecha de procesamiento OCR/scraping (UTC)
  reception_date: string | null;        // Fecha de recepción del documento (UTC)
  type: string | null;                  // Tipo: "QR", "EMAIL", "MANUAL", etc.
  update_date: string;                  // Fecha última actualización (UTC) - SIEMPRE presente
}
```

**Ejemplo Response Headers:**

```json
{
  "success": true,
  "data": {
    "data": [
      {
        "cufe": "FE01200000000434-15-93796-2200512026010900000938190020318917814654",
        "issuer_name": "IMPORTADORA RICAMAR S A",
        "issuer_ruc": "434-15-93796",
        "store_id": "0051",
        "no": "0000093819",
        "date": "2026-01-09T18:52:46Z",
        "tot_amount": 34.11,
        "tot_itbms": 2.24,
        "url": "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorQR?chFE=...",
        "process_date": "2026-01-14T16:26:07.480828Z",
        "reception_date": "2026-01-14T16:26:06.092100Z",
        "type": "QR",
        "update_date": "2026-01-14T21:26:07.483880Z"
      }
    ],
    "pagination": { ... },
    "sync_metadata": { ... }
  }
}
```

**Response:** Estructura `IncrementalSyncResponse<InvoiceHeadersResponse>`

---

### 4. GET /api/v4/invoices/details

**Descripción:** Obtener detalles de líneas de facturas del usuario con sync incremental

**Headers:**
- `Authorization: Bearer <jwt_token>` **REQUERIDO**

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `update_date_from` | `string` | No | - | Filtrar detalles actualizados desde esta fecha (ISO 8601 UTC) |
| `full_sync` | `boolean` | No | `false` | ⭐ Si `true`, ignora `update_date_from` y retorna TODOS los registros |
| `cufe` | `string` | No | - | Filtrar por CUFE de factura específica |
| `limit` | `integer` | No | `20` | Número máximo de items por página (max 100) |
| `offset` | `integer` | No | `0` | Número de items a omitir |

**Ejemplos de uso:**

```bash
# Incremental sync
GET /api/v4/invoices/details?update_date_from=2026-01-14T00:00:00Z&limit=50

# Full sync
GET /api/v4/invoices/details?full_sync=true&limit=100
```

**Invoice Detail Response Fields:**

```typescript
interface InvoiceDetailsResponse {
  cufe: string;                         // CUFE de la factura padre (FK)
  code: string | null;                  // Código del producto
  description: string | null;           // Descripción del item/producto
  quantity: string | null;              // Cantidad (string para precisión decimal)
  unit_price: string | null;            // Precio unitario
  amount: string | null;                // Subtotal sin impuestos
  itbms: string | null;                 // Impuesto ITBMS del item
  total: string | null;                 // Total con impuestos
  unit_discount: string | null;         // Descuento unitario aplicado
  information_of_interest: string | null; // Información adicional del item
  update_date: string;                  // Fecha última actualización (UTC)
}
```

**Nota:** El ID único de cada detalle es la combinación de `cufe` + `code`. En `sync_metadata.record_ids` se retorna como `"{cufe}_{code}"`.

**Ejemplo Response Details:**

```json
{
  "success": true,
  "data": {
    "data": [
      {
        "cufe": "FE01200000000434-15-93796-2200512026010900000938190020318917814654",
        "code": "7461323129244",
        "description": "RON BARCELO ANEJO BO",
        "quantity": "1",
        "unit_price": "10.9900",
        "amount": "10.99",
        "itbms": "1.10",
        "total": "12.09",
        "unit_discount": "0.0000",
        "information_of_interest": "",
        "update_date": "2026-01-14T21:26:07.483880Z"
      }
    ],
    "pagination": { ... },
    "sync_metadata": { ... }
  }
}
```

**Response:** Estructura `IncrementalSyncResponse<InvoiceDetailsResponse>`

---

### 5. POST /api/v4/invoices/headers/recovery ⭐ NUEVO

**Descripción:** Recovery sync por comparación de CUFEs. El cliente envía los CUFEs que tiene localmente, el servidor retorna los faltantes y los eliminados.

**Headers:**
| Header | Requerido | Descripción |
|--------|-----------|-------------|
| `Authorization` | ✅ Sí | `Bearer <jwt_token>` |
| `Content-Type` | ✅ Sí | `application/json` |

**Request Body:**

```typescript
interface RecoveryRequest {
  known_cufes: string[];  // CUFEs que el cliente ya tiene (max 10,000)
  limit?: number;         // Max registros faltantes a retornar (default: 100, max: 500)
}
```

**Response:**

```typescript
interface RecoveryResponse<InvoiceHeader> {
  missing_records: InvoiceHeader[];  // Registros que el cliente no tiene
  deleted_cufes: string[];           // CUFEs que el cliente tiene pero fueron eliminados
  total_missing: number;             // Total faltantes (puede ser > missing_records.length)
  server_timestamp: string;          // UTC timestamp
}
```

**Ejemplo Request - Lista vacía (obtener todos):**

```bash
curl -X POST "http://localhost:8000/api/v4/invoices/headers/recovery" \
     -H "Authorization: Bearer <token>" \
     -H "Content-Type: application/json" \
     -d '{
       "known_cufes": [],
       "limit": 3
     }'
```

**Ejemplo Response - Lista vacía:**

```json
{
  "success": true,
  "data": {
    "missing_records": [
      {
        "cufe": "FE01200000000434-15-93796-2200512026010900000938190020318917814654",
        "issuer_name": "IMPORTADORA RICAMAR S A",
        "issuer_ruc": "434-15-93796",
        "store_id": "0051",
        "no": "0000093819",
        "date": "2026-01-09T18:52:46Z",
        "tot_amount": 34.11,
        "tot_itbms": 2.24,
        "url": "https://dgi-fep.mef.gob.pa/...",
        "process_date": "2026-01-14T16:26:07.480828Z",
        "reception_date": "2026-01-14T16:26:06.092100Z",
        "type": "QR",
        "update_date": "2026-01-14T21:26:07.483880Z"
      }
    ],
    "deleted_cufes": [],
    "total_missing": 527,
    "server_timestamp": "2026-01-14T21:50:00Z"
  },
  "error": null,
  "request_id": "35fa8af8-270c-42ec-9574-b19d0de4fa34",
  "timestamp": "2026-01-14T21:50:00Z",
  "execution_time_ms": 45,
  "cached": false
}
```

**Ejemplo Request - Con CUFEs conocidos:**

```bash
curl -X POST "http://localhost:8000/api/v4/invoices/headers/recovery" \
     -H "Authorization: Bearer <token>" \
     -H "Content-Type: application/json" \
     -d '{
       "known_cufes": [
         "FE01200000000434-15-93796-2200512026010900000938190020318917814654"
       ],
       "limit": 100
     }'
```

**Ejemplo Response - Con filtrado:**

```json
{
  "success": true,
  "data": {
    "missing_records": [...],
    "deleted_cufes": [],
    "total_missing": 526,
    "server_timestamp": "2026-01-14T21:50:00Z"
  }
}
```

**Cuándo usar:**
- Usuario reporta "faltan facturas"
- `sync-status` muestra count diferente al local
- Después de recuperar backup local corrupto
- Verificación periódica de integridad (opcional)
- Primera sincronización (enviar `known_cufes: []`)

---

### 6. POST /api/v4/invoices/details/recovery ⭐ NUEVO

**Descripción:** Recovery sync para detalles de facturas. Similar a headers pero usando IDs compuestos (cufe_code).

**Headers:**
| Header | Requerido | Descripción |
|--------|-----------|-------------|
| `Authorization` | ✅ Sí | `Bearer <jwt_token>` |
| `Content-Type` | ✅ Sí | `application/json` |

**Request Body:**

```typescript
interface DetailsRecoveryRequest {
  known_ids: string[];    // IDs compuestos "cufe_code" (max 50,000)
  limit?: number;         // Max registros (default: 100, max: 1000)
}
```

**Response:**

```typescript
interface DetailsRecoveryResponse<InvoiceDetail> {
  missing_records: InvoiceDetail[];
  deleted_ids: string[];       // Siempre vacío (details se eliminan con su header)
  total_missing: number;
  server_timestamp: string;
}
```

**Ejemplo Request - Lista vacía:**

```bash
curl -X POST "http://localhost:8000/api/v4/invoices/details/recovery" \
     -H "Authorization: Bearer <token>" \
     -H "Content-Type: application/json" \
     -d '{
       "known_ids": [],
       "limit": 3
     }'
```

**Ejemplo Response - Lista vacía:**

```json
{
  "success": true,
  "data": {
    "missing_records": [
      {
        "cufe": "FE01200000000434-15-93796-2200512026010900000938190020318917814654",
        "code": "7461323129244",
        "description": "RON BARCELO ANEJO BO",
        "quantity": "1",
        "unit_price": "10.9900",
        "amount": "10.99",
        "itbms": "1.10",
        "total": "12.09",
        "unit_discount": "0.0000",
        "information_of_interest": "",
        "update_date": "2026-01-14T21:26:07.483880Z"
      }
    ],
    "deleted_ids": [],
    "total_missing": 3429,
    "server_timestamp": "2026-01-14T21:50:00Z"
  },
  "error": null,
  "request_id": "...",
  "execution_time_ms": 50,
  "cached": false
}
```

**Ejemplo Request - Con IDs conocidos:**

```bash
curl -X POST "http://localhost:8000/api/v4/invoices/details/recovery" \
     -H "Authorization: Bearer <token>" \
     -H "Content-Type: application/json" \
     -d '{
       "known_ids": [
         "FE01200000000434-15-93796-2200512026010900000938190020318917814654_7461323129244"
       ],
       "limit": 100
     }'
```

**Ejemplo Response - Con filtrado:**

```json
{
  "success": true,
  "data": {
    "missing_records": [...],
    "deleted_ids": [],
    "total_missing": 3428,
    "server_timestamp": "2026-01-14T21:50:00Z"
  }
}
```

**Formato del ID compuesto:**
```
{cufe}_{code}
```

Ejemplo: `FE01200000000434-15-93796-2200512026010900000938190020318917814654_7461323129244`

**Cuándo usar:**
- Después de recovery de headers para obtener los detalles faltantes
- Verificación de integridad de líneas de facturas
- Corrupción de datos locales en detalles

---

### 7. GET /api/v4/invoices/{resource}/version

**Descripción:** Endpoint ligero para verificar version del dataset sin descargar datos

**Resources:** `products`, `issuers`, `headers`, `details`

**Headers:**
- `Authorization: Bearer <jwt_token>` **REQUERIDO**

**Response:**

```typescript
interface VersionResponse {
  dataset_version: number;              // Version actual del dataset
  last_modified: string;                // Timestamp de última modificación
  server_timestamp: string;             // Timestamp del servidor
  total_records: number;                // Total de registros (opcional)
}
```

**Ejemplo:**

```bash
GET /api/v4/invoices/products/version
Authorization: Bearer eyJ0eXAiOiJKV1Q...

Response:
{
  "dataset_version": 150,
  "last_modified": "2025-11-07T12:00:00Z",
  "server_timestamp": "2025-11-07T12:05:00Z",
  "total_records": 1523
}
```

**Uso:** Frontend puede hacer polling ligero cada N minutos para detectar cambios sin descargar datos.

---

## 🔐 Validaciones de Integridad (Frontend)

### 1. Validación de Checksum

```javascript
function validateChecksum(response) {
  const { data, sync_metadata } = response;
  
  // Calcular checksum local
  const dataJson = JSON.stringify(data);
  const calculatedChecksum = sha256(dataJson);
  
  // Comparar con checksum del servidor
  if (calculatedChecksum !== sync_metadata.data_checksum) {
    console.error('❌ Checksum mismatch - data corrupted in transit');
    return false;
  }
  
  return true;
}
```

### 2. Validación de Conteo de Registros

```javascript
function validateRecordCount(response) {
  const { data, sync_metadata, pagination } = response;
  
  // Check 1: data.length vs returned_records
  if (data.length !== sync_metadata.returned_records) {
    console.error('❌ Record count mismatch in metadata');
    return false;
  }
  
  // Check 2: data.length vs pagination.returned_records
  if (data.length !== pagination.returned_records) {
    console.error('❌ Record count mismatch in pagination');
    return false;
  }
  
  // Check 3: record_ids.length vs data.length
  if (sync_metadata.record_ids.length !== data.length) {
    console.error('❌ Record IDs count mismatch');
    return false;
  }
  
  return true;
}
```

### 3. Aplicar Cambios Incrementales

```javascript
async function applyIncrementalChanges(response, datasetName) {
  const { data, sync_metadata } = response;
  
  // 1. Validar integridad
  if (!validateChecksum(response) || !validateRecordCount(response)) {
    throw new Error('Integrity validation failed');
  }
  
  // 2. Cargar datos locales
  const localData = await getLocalData(datasetName);
  
  // 3. Aplicar deletes
  for (const deleted of sync_metadata.deleted_since.items) {
    const index = localData.findIndex(item => item.id === deleted.id);
    if (index >= 0) {
      localData.splice(index, 1);
      console.log(`🗑️ Deleted ${deleted.id}`);
    }
  }
  
  // 4. Upsert nuevos/modificados
  for (const newItem of data) {
    const index = localData.findIndex(item => item.id === newItem.id);
    if (index >= 0) {
      localData[index] = newItem; // Update
      console.log(`✏️ Updated ${newItem.id}`);
    } else {
      localData.push(newItem);     // Insert
      console.log(`➕ Inserted ${newItem.id}`);
    }
  }
  
  // 5. Guardar estado actualizado
  await saveLocalData(datasetName, localData);
  await saveLastSync(datasetName, sync_metadata.max_update_date);
  await saveVersion(datasetName, sync_metadata.dataset_version);
  
  console.log(`✅ Sync complete: +${data.length} upserts, -${sync_metadata.deleted_since.count} deletes`);
}
```

---

## 🗄️ Cambios en Base de Datos

### Schema Modifications

```sql
-- 1. Agregar columnas de soft delete a todas las tablas de dimensiones
ALTER TABLE public.dim_product 
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

ALTER TABLE public.dim_issuer 
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

ALTER TABLE public.invoice_header
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

ALTER TABLE public.invoice_detail
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

-- 2. Crear tabla de versiones de datasets
CREATE TABLE IF NOT EXISTS dataset_versions (
    table_name VARCHAR(100) PRIMARY KEY,
    version BIGINT DEFAULT 0,
    last_modified TIMESTAMP DEFAULT NOW()
);

-- 3. Inicializar versiones
INSERT INTO dataset_versions (table_name, version) 
VALUES 
    ('dim_product', 1),
    ('dim_issuer', 1),
    ('invoice_header', 1),
    ('invoice_detail', 1)
ON CONFLICT (table_name) DO NOTHING;

-- 4. Function para incrementar version automáticamente
CREATE OR REPLACE FUNCTION increment_dataset_version()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE dataset_versions 
    SET version = version + 1, 
        last_modified = NOW()
    WHERE table_name = TG_TABLE_NAME;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 5. Triggers para auto-increment version en cambios
CREATE TRIGGER increment_product_version
AFTER INSERT OR UPDATE OR DELETE ON public.dim_product
FOR EACH STATEMENT EXECUTE FUNCTION increment_dataset_version();

CREATE TRIGGER increment_issuer_version
AFTER INSERT OR UPDATE OR DELETE ON public.dim_issuer
FOR EACH STATEMENT EXECUTE FUNCTION increment_dataset_version();

CREATE TRIGGER increment_header_version
AFTER INSERT OR UPDATE OR DELETE ON public.invoice_header
FOR EACH STATEMENT EXECUTE FUNCTION increment_dataset_version();

CREATE TRIGGER increment_detail_version
AFTER INSERT OR UPDATE OR DELETE ON public.invoice_detail
FOR EACH STATEMENT EXECUTE FUNCTION increment_dataset_version();

-- 6. Índices para performance
CREATE INDEX IF NOT EXISTS idx_dim_product_update_date ON public.dim_product(update_date) WHERE is_deleted = FALSE;
CREATE INDEX IF NOT EXISTS idx_dim_product_deleted ON public.dim_product(deleted_at) WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_dim_issuer_update_date ON public.dim_issuer(update_date) WHERE is_deleted = FALSE;
CREATE INDEX IF NOT EXISTS idx_dim_issuer_deleted ON public.dim_issuer(deleted_at) WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_invoice_header_update_date ON public.invoice_header(update_date) WHERE is_deleted = FALSE;
CREATE INDEX IF NOT EXISTS idx_invoice_detail_update_date ON public.invoice_detail(update_date) WHERE is_deleted = FALSE;
```

---

## 🛠️ Implementación Rust

### Estructuras Comunes

```rust
// src/api/common/sync_types.rs

use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct IncrementalSyncResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
    pub sync_metadata: SyncMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub total_records: i64,
    pub returned_records: usize,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
    pub total_pages: i64,
    pub current_page: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// Timestamp del registro más reciente en esta respuesta
    pub max_update_date: Option<NaiveDateTime>,
    
    /// Timestamp del servidor al generar la respuesta
    pub server_timestamp: NaiveDateTime,
    
    /// SHA256 checksum de los datos retornados
    pub data_checksum: String,
    
    /// Lista de IDs retornados (para validación de completitud)
    pub record_ids: Vec<String>,
    
    /// Número de registros retornados (duplicado para validación)
    pub returned_records: usize,
    
    /// Version incremental del dataset completo
    pub dataset_version: i64,
    
    /// Items eliminados desde last sync
    pub deleted_since: DeletedItems,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletedItems {
    pub enabled: bool,
    pub count: usize,
    pub items: Vec<DeletedItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletedItem {
    pub id: String,
    pub deleted_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionResponse {
    pub dataset_version: i64,
    pub last_modified: NaiveDateTime,
    pub server_timestamp: NaiveDateTime,
    pub total_records: Option<i64>,
}
```

### Query Helpers

```rust
// src/api/common/sync_queries.rs

pub fn get_deleted_items_query(table_name: &str) -> String {
    format!(
        r#"
        SELECT 
            code as id,
            deleted_at
        FROM public.{}
        WHERE is_deleted = TRUE
          AND deleted_at >= $1
        ORDER BY deleted_at DESC
        LIMIT 1000
        "#,
        table_name
    )
}

pub fn get_dataset_version_query() -> &'static str {
    r#"
    SELECT version, last_modified
    FROM dataset_versions
    WHERE table_name = $1
    "#
}

pub async fn get_dataset_version(
    pool: &PgPool,
    table_name: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT version FROM dataset_versions WHERE table_name = $1"
    )
    .bind(table_name)
    .fetch_one(pool)
    .await?;
    
    Ok(result)
}

pub fn calculate_checksum(data: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}
```

### Handler Example (Products)

```rust
// src/api/user_products_v4.rs

pub async fn get_user_products(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserProductsRequest>,
) -> Result<Json<ApiResponse<IncrementalSyncResponse<UserProductsResponse>>>, StatusCode> {
    
    let server_timestamp = chrono::Utc::now().naive_utc();
    let user_id = current_user.user_id;
    
    // 1. Query principal (con update_date_from filter si aplica)
    let products = if let Some(date_filter) = &params.update_date_from {
        sqlx::query_as::<_, UserProductsResponse>(
            UserProductsQueryTemplates::get_user_products_with_date_filter_query()
        )
        .bind(user_id)
        .bind(params.limit.unwrap_or(20))
        .bind(params.offset.unwrap_or(0))
        .bind(date_filter)
        .fetch_all(&state.db_pool)
        .await?
    } else {
        sqlx::query_as::<_, UserProductsResponse>(
            UserProductsQueryTemplates::get_user_products_query()
        )
        .bind(user_id)
        .bind(params.limit.unwrap_or(20))
        .bind(params.offset.unwrap_or(0))
        .fetch_all(&state.db_pool)
        .await?
    };
    
    // 2. Max update_date de los retornados
    let max_update_date = products
        .iter()
        .filter_map(|p| p.update_date)
        .max();
    
    // 3. Deleted items (solo si hay update_date_from)
    let deleted_items = if let Some(since) = &params.update_date_from {
        sqlx::query_as::<_, DeletedItem>(
            &get_deleted_items_query("dim_product")
        )
        .bind(since)
        .fetch_all(&state.db_pool)
        .await.unwrap_or_default()
    } else {
        vec![]
    };
    
    // 4. Dataset version
    let dataset_version = get_dataset_version(&state.db_pool, "dim_product")
        .await
        .unwrap_or(0);
    
    // 5. Total count (para pagination)
    let total_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.dim_product WHERE is_deleted = FALSE"
    )
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);
    
    // 6. Checksum
    let data_json = serde_json::to_string(&products)?;
    let checksum = calculate_checksum(&data_json);
    
    // 7. Record IDs
    let record_ids: Vec<String> = products
        .iter()
        .filter_map(|p| p.code.clone())
        .collect();
    
    // 8. Pagination info
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);
    let total_pages = (total_count as f64 / limit as f64).ceil() as i64;
    let current_page = (offset / limit) + 1;
    let has_more = (offset + limit) < total_count;
    
    // 9. Construir response
    let response = IncrementalSyncResponse {
        data: products.clone(),
        pagination: PaginationInfo {
            total_records: total_count,
            returned_records: products.len(),
            limit,
            offset,
            has_more,
            total_pages,
            current_page,
        },
        sync_metadata: SyncMetadata {
            max_update_date,
            server_timestamp,
            data_checksum: checksum,
            record_ids,
            returned_records: products.len(),
            dataset_version,
            deleted_since: DeletedItems {
                enabled: true,
                count: deleted_items.len(),
                items: deleted_items,
            },
        },
    };
    
    Ok(Json(ApiResponse::success(response)))
}
```

---

## ✅ Garantías del Sistema

| Escenario | Frontend lo detecta | Pérdida de datos | Recuperación |
|-----------|---------------------|------------------|--------------|
| **Nuevos registros** | ✅ Sí (update_date) | ❌ No | Automática |
| **Registros modificados** | ✅ Sí (update_date) | ❌ No | Automática |
| **Registros eliminados** | ✅ Sí (deleted_since) | ❌ No | Automática |
| **Datos corruptos** | ✅ Sí (checksum) | ❌ No | Retry |
| **Response truncado** | ✅ Sí (record count) | ❌ No | Retry |
| **Dataset cambió** | ✅ Sí (version check) | ❌ No | Sync incremental |
| **Red falla** | ⚠️ Retry idempotente | ❌ No | Retry |
| **Race conditions** | ✅ Sí (max_update_date) | ❌ No | Automática |

---

## � Endpoint de Validación de Integridad

### GET /api/v4/invoices/integrity-summary

**Descripción:** Endpoint ligero para validar la integridad global de todos los datasets del usuario. Lee las Materialized Views actualizadas diariamente a las 3:15 AM UTC.

**Uso recomendado:** 1 vez al día (por ejemplo, al abrir la app a las 4 AM hora local)

**Headers:**
- `Authorization: Bearer <jwt_token>` **REQUERIDO**

**Query Parameters:** Ninguno

**Response:**

```json
{
  "success": true,
  "data": {
    "products": {
      "total_count": 1475,
      "global_hash": 1651528645,
      "last_update": "2025-10-31T05:40:50.209311",
      "snapshot_time": "2025-11-08T10:15:30.050794Z"
    },
    "issuers": {
      "total_count": 109,
      "global_hash": 308545882,
      "last_update": "2025-11-03T19:45:39",
      "snapshot_time": "2025-11-08T10:15:30.050794Z"
    },
    "headers": {
      "total_count": 395,
      "global_hash": -1207530112,
      "last_update": "2025-11-07T22:18:09.953659",
      "snapshot_time": "2025-11-08T10:15:30.050794Z"
    },
    "details": {
      "total_count": 2663,
      "global_hash": 1451438421,
      "last_update": "2025-11-07T22:18:09.953659",
      "snapshot_time": "2025-11-08T10:15:30.050794Z"
    }
  },
  "error": null,
  "request_id": "a98d57b7-9e73-4f21-a972-6986eb97c0f7",
  "timestamp": "2025-11-08T13:13:41.747711123Z",
  "execution_time_ms": 6,
  "cached": false
}
```

**Lógica de Validación Frontend:**

```javascript
// 1. Obtener integrity summary del servidor
const serverIntegrity = await fetch('/api/v4/invoices/integrity-summary');

// 2. Calcular hash local (mismo algoritmo XOR)
const localProductsHash = calculateXorHash(localProducts.map(p => p.code));

// 3. Comparar
if (serverIntegrity.products.global_hash !== localProductsHash ||
    serverIntegrity.products.total_count !== localProducts.length) {
  console.warn('⚠️ Desincronización detectada en products!');
  
  // Forzar resync completo
  await fullResyncProducts();
}

// 4. Guardar snapshot time para siguiente validación
localStorage.setItem('last_integrity_check', serverIntegrity.products.snapshot_time);
```

**Performance:**
- Tiempo de respuesta: **~6ms**
- Queries ejecutados: 4 (1 por cada Materialized View)
- Datos transferidos: ~500 bytes
- Costo computacional: Mínimo (solo lectura de índices)

---

## �📝 Casos de Uso

### 1. App Startup - Full Sync
```javascript
// Al abrir la app, cargar todos los datos
const products = await syncProducts();
const issuers = await syncIssuers();
const headers = await syncHeaders();
const details = await syncDetails();
```

### 2. Periodic Refresh - Incremental Sync
```javascript
// Cada 5 minutos, solo obtener cambios
setInterval(async () => {
  await syncProductsIncremental();
  await syncIssuersIncremental();
  await syncHeadersIncremental();
  await syncDetailsIncremental();
}, 5 * 60 * 1000);
```

### 3. Daily Integrity Check (4 AM local)
```javascript
// 1 vez al día, validar integridad global
async function dailyIntegrityCheck() {
  const summary = await fetch('/api/v4/invoices/integrity-summary');
  
  // Validar cada recurso
  validateResourceIntegrity('products', summary.data.products);
  validateResourceIntegrity('issuers', summary.data.issuers);
  validateResourceIntegrity('headers', summary.data.headers);
  validateResourceIntegrity('details', summary.data.details);
}

function validateResourceIntegrity(resourceName, serverData) {
  const localData = getLocalData(resourceName);
  const localHash = calculateXorHash(localData.map(item => item.id));
  
  if (serverData.global_hash !== localHash) {
    console.error(`❌ Integrity mismatch in ${resourceName}!`);
    console.log('Server:', serverData.total_count, 'items, hash:', serverData.global_hash);
    console.log('Local:', localData.length, 'items, hash:', localHash);
    
    // Trigger full resync
    fullResync(resourceName);
  } else {
    console.log(`✅ ${resourceName} integrity OK`);
  }
}
```

### 4. User Action - Force Refresh
```javascript
// Botón "Refresh" del usuario
async function handleRefreshButton() {
  showLoading();
  await syncProductsIncremental();
  await syncIssuersIncremental();
  await syncHeadersIncremental();
  await syncDetailsIncremental();
  hideLoading();
  showToast('Datos actualizados');
}
```

---

## 🔄 Estado de Implementación

### ✅ Completado
- [x] Documentación completa del sistema
- [x] Diseño de arquitectura con Materialized Views
- [x] Modificaciones de schema en PostgreSQL (soft-delete, MVs)
- [x] Implementación de estructuras Rust (sync_types, sync_helpers)
- [x] Implementación de handlers (products, issuers, headers, details)
- [x] Endpoint de integrity-summary
- [x] Scheduled job para refresh de MVs (3:15 AM UTC diario)
- [x] Testing de integración y performance
- [x] Deployment a producción

### 📊 Métricas de Producción
- **Usuarios activos soportados:** 50,000
- **Tiempo de respuesta sync incremental:** 6-18ms
- **Tiempo de respuesta integrity check:** ~6ms
- **Refresh de Materialized Views:** 3:15 AM UTC (2-5 minutos)
- **Performance target:** ✅ Alcanzado

---

## 📞 Soporte

Para preguntas o issues relacionados con este sistema de sincronización, contactar al equipo de desarrollo backend.

**Última actualización:** 2026-01-14  
**Versión del documento:** 3.0  
**Estado:** ✅ EN PRODUCCIÓN
