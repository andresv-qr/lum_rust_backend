# 🎁 API DE REDENCIÓN DE LÜMIS - DOCUMENTACIÓN COMPLETA

**Versión**: 2.0  
**Fecha**: 2025-10-18  
**Estado**: ✅ Producción  

## 📋 Tabla de Contenidos

1. [Arquitectura del Sistema](#arquitectura-del-sistema)
2. [Explicación Conceptual](#explicación-conceptual)
3. [Modelo de Datos](#modelo-de-datos)
4. [Diagramas de Flujo](#diagramas-de-flujo)
5. [API Endpoints - Usuarios](#api-endpoints---usuarios)
6. [API Endpoints - Merchant](#api-endpoints---merchant)
7. [Autenticación y Seguridad](#autenticación-y-seguridad)
8. [Códigos de Error](#códigos-de-error)
9. [Ejemplos de Integración](#ejemplos-de-integración)

---

## 🏗️ Arquitectura del Sistema

### Stack Tecnológico

```
┌─────────────────────────────────────────────────────────────┐
│  Backend: Rust + Axum (Puerto 8000)                         │
│  Base de Datos: PostgreSQL 14+                              │
│  Autenticación: JWT (HS256)                                 │
│  Passwords: bcrypt                                           │
│  QR Generation: qrcode-generator crate                      │
└─────────────────────────────────────────────────────────────┘
```

### Arquitectura de Módulos

```
lum_rust_ws/
├── src/
│   ├── domains/
│   │   └── rewards/
│   │       ├── models.rs          → Structs (RedemptionOffer, UserRedemption)
│   │       ├── offer_service.rs   → Lógica de catálogo y balance
│   │       └── redemption_service.rs → Lógica de redención
│   │
│   ├── api/
│   │   ├── rewards/
│   │   │   ├── offers.rs         → GET /api/v1/rewards/offers
│   │   │   ├── redeem.rs         → POST /api/v1/rewards/redeem
│   │   │   └── user.rs           → GET /api/v1/rewards/history|stats
│   │   │
│   │   └── merchant/
│   │       ├── auth.rs           → POST /api/v1/merchant/auth/login
│   │       ├── validate.rs       → POST /api/v1/merchant/validate
│   │       └── stats.rs          → GET /api/v1/merchant/stats
│   │
│   └── middleware/
│       └── auth.rs               → extract_current_user(), extract_merchant()
│
└── database/
    └── schema: rewards
        ├── redemption_offers
        ├── user_redemptions
        ├── fact_accumulations
        ├── fact_balance_points
        └── merchants
```

---

## 💡 Explicación Conceptual

### ¿Qué es el Sistema de Redención?

El sistema de redención permite a los usuarios de la app Lümis **canjear sus puntos Lümis** por productos o servicios reales en comercios aliados.

### Conceptos Clave

#### 1. **Lümis (Puntos)**
- Moneda virtual de la app
- Los usuarios acumulan Lümis comprando facturas
- 1 Lümi = unidad base (no fraccionable)
- Balance se guarda en `rewards.fact_balance_points`

#### 2. **Oferta de Redención (Redemption Offer)**
- Producto o servicio ofrecido por un comercio
- Ejemplos: "Café Americano - 55 Lümis", "Entrada al Cine - 150 Lümis"
- Almacenado en `rewards.redemption_offers`
- Tiene costo en Lümis, imagen, descripción, términos y condiciones

#### 3. **Redención (User Redemption)**
- Instancia de un usuario canjeando una oferta
- Se crea cuando el usuario presiona "Redimir"
- Genera un **código único** (ej: `LUMS-967E-F893-7EC2`)
- Estados: `pending`, `confirmed`, `cancelled`, `expired`

#### 4. **Código de Redención**
- Código alfanumérico único de 19 caracteres
- Formato: `LUMS-XXXX-XXXX-XXXX`
- Válido por tiempo limitado (configurable, típicamente 15 minutos)
- El comercio lo escanea/ingresa para validar

#### 5. **QR Code**
- Generado automáticamente para cada redención
- Contiene el código de redención
- Landing URL: `https://app.lumis.pa/redeem/{code}`
- Image URL: `https://cdn.lumis.pa/qr/{code}.png`

#### 6. **Merchant (Comercio Aliado)**
- Negocio que acepta redenciones
- Tiene API key hasheado con bcrypt
- Recibe JWT token tras login
- Puede validar y confirmar redenciones

---

## 🗄️ Modelo de Datos

### Diagrama ER

```
┌─────────────────────────┐
│  redemption_offers      │
│─────────────────────────│
│ ✓ offer_id (UUID, PK)   │
│   name_friendly         │
│   lumis_cost (INT)      │
│   merchant_id (UUID, FK)│
│   terms_and_conditions  │
│   is_active (BOOLEAN)   │
└──────────┬──────────────┘
           │ 1
           │
           │ N
┌──────────┴──────────────┐       ┌─────────────────────────┐
│  user_redemptions       │       │  merchants              │
│─────────────────────────│       │─────────────────────────│
│ ✓ redemption_id (UUID)  │       │ ✓ merchant_id (UUID, PK)│
│   user_id (INT, FK)     │──────>│   merchant_name         │
│   offer_id (UUID, FK)   │       │   api_key_hash          │
│   redemption_code       │       │   is_active (BOOLEAN)   │
│   redemption_status     │       │   total_redemptions     │
│   lumis_spent (INT)     │       │   total_lumis_redeemed  │
│   code_expires_at       │       └─────────────────────────┘
│   validated_at          │
│   validated_by_merchant │
└──────────┬──────────────┘
           │
           │ 1
           │
           │ 1
┌──────────┴──────────────┐
│  fact_accumulations     │
│─────────────────────────│
│ ✓ id (SERIAL, PK)       │
│   user_id (INT)         │
│   accum_type (TEXT)     │  -- 'earn' | 'spend'
│   dtype (TEXT)          │  -- 'points'
│   quantity (NUMERIC)    │
│   date (TIMESTAMPTZ)    │
│   redemption_id (UUID)  │
└─────────────────────────┘

┌─────────────────────────┐
│  fact_balance_points    │
│─────────────────────────│
│ ✓ user_id (INT, PK)     │
│   balance (NUMERIC)     │
│   latest_update         │
└─────────────────────────┘
```

### Tabla: `redemption_offers`

**Descripción**: Catálogo de ofertas disponibles para redención

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `offer_id` | UUID | Identificador único (PK) |
| `name_friendly` | VARCHAR(255) | Nombre para mostrar al usuario |
| `description_friendly` | VARCHAR(500) | Descripción de la oferta |
| `lumis_cost` | INTEGER | Costo en Lümis |
| `points` | INTEGER | Alias legacy (mismo que lumis_cost) |
| `merchant_id` | UUID | FK a tabla merchants |
| `merchant_name` | VARCHAR(255) | Nombre del comercio |
| `offer_category` | VARCHAR(50) | food, entertainment, shopping, etc |
| `img` | VARCHAR(500) | URL de imagen |
| `terms_and_conditions` | TEXT | Términos y condiciones |
| `is_active` | BOOLEAN | Si la oferta está disponible |
| `valid_from` | TIMESTAMPTZ | Fecha inicio vigencia |
| `valid_to` | TIMESTAMPTZ | Fecha fin vigencia |
| `stock_quantity` | INTEGER | NULL = ilimitado |
| `max_redemptions_per_user` | INTEGER | Límite por usuario |

**Ejemplo**:
```json
{
  "offer_id": "550e8400-e29b-41d4-a716-446655440000",
  "name_friendly": "Café Americano",
  "description_friendly": "Disfruta un delicioso café americano",
  "lumis_cost": 55,
  "merchant_id": "a1726cd2-dd94-45c6-b996-3c89fa927a0c",
  "merchant_name": "Starbucks Panamá",
  "offer_category": "food",
  "img": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg",
  "terms_and_conditions": "Válido solo en sucursales participantes...",
  "is_active": true
}
```

### Tabla: `user_redemptions`

**Descripción**: Registro de redenciones de usuarios

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `redemption_id` | UUID | Identificador único (PK) |
| `user_id` | INTEGER | ID del usuario |
| `offer_id` | UUID | FK a redemption_offers |
| `redemption_code` | VARCHAR(50) | Código único LUMS-XXXX-XXXX-XXXX |
| `redemption_status` | VARCHAR(20) | pending, confirmed, cancelled, expired |
| `lumis_spent` | INTEGER | Lümis gastados |
| `qr_landing_url` | TEXT | URL de landing page |
| `qr_image_url` | TEXT | URL de imagen QR |
| `created_at` | TIMESTAMPTZ | Cuando se creó |
| `code_expires_at` | TIMESTAMPTZ | Cuando expira el código |
| `validated_at` | TIMESTAMPTZ | Cuando el merchant lo validó |
| `validated_by_merchant_id` | UUID | Qué merchant lo validó |

**Estados del ciclo de vida**:
```
pending ──> confirmed  (merchant confirma)
   │
   ├──> cancelled  (usuario cancela)
   │
   └──> expired    (timeout sin usar)
```

### Tabla: `fact_accumulations`

**Descripción**: Registro de transacciones de Lümis

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `id` | SERIAL | ID autoincremental |
| `user_id` | INTEGER | ID del usuario |
| `accum_type` | TEXT | 'earn' (ganar) o 'spend' (gastar) |
| `dtype` | TEXT | Tipo de operación ('points') |
| `quantity` | NUMERIC | Cantidad de Lümis (positivo) |
| `date` | TIMESTAMPTZ | Fecha de la transacción |
| `redemption_id` | UUID | FK opcional a user_redemptions |

**Lógica del Balance**:
```sql
balance = SUM(
  CASE 
    WHEN accum_type = 'earn' THEN quantity
    WHEN accum_type = 'spend' THEN -quantity
  END
)
```

### Tabla: `merchants`

**Descripción**: Comercios aliados que aceptan redenciones

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `merchant_id` | UUID | Identificador único (PK) |
| `merchant_name` | VARCHAR(255) | Nombre del comercio (UNIQUE) |
| `merchant_type` | VARCHAR(50) | restaurant, cafe, cinema, etc |
| `contact_email` | VARCHAR(255) | Email de contacto |
| `api_key_hash` | VARCHAR(255) | API key hasheado con bcrypt |
| `is_active` | BOOLEAN | Si puede operar |
| `total_redemptions` | INTEGER | Contador de redenciones |
| `total_lumis_redeemed` | BIGINT | Total de Lümis canjeados |

---

## 📊 Diagramas de Flujo

### Flujo 1: Usuario Redime Oferta

```mermaid
sequenceDiagram
    participant U as Usuario (App)
    participant API as API Rust
    participant DB as PostgreSQL
    participant QR as QR Generator

    U->>API: POST /api/v1/rewards/redeem<br/>{offer_id, JWT}
    
    API->>DB: Verificar balance >= lumis_cost
    
    alt Balance insuficiente
        DB-->>API: balance < cost
        API-->>U: 400 Saldo insuficiente
    else Balance suficiente
        DB-->>API: balance OK
        
        API->>API: Generar código único<br/>LUMS-XXXX-XXXX-XXXX
        
        API->>QR: Generar QR code
        QR-->>API: QR image data
        
        API->>DB: BEGIN TRANSACTION
        
        API->>DB: INSERT INTO user_redemptions<br/>(pending, code, expires_at)
        
        API->>DB: INSERT INTO fact_accumulations<br/>(spend, -lumis_cost)
        
        DB->>DB: TRIGGER: fun_update_balance_points()<br/>recalcula balance
        
        API->>DB: COMMIT TRANSACTION
        
        API-->>U: 200 OK<br/>{redemption_id, code, qr_urls, new_balance}
    end
```

### Flujo 2: Merchant Valida y Confirma

```mermaid
sequenceDiagram
    participant M as Merchant
    participant API as API Rust
    participant DB as PostgreSQL
    participant U as Usuario

    M->>API: POST /api/v1/merchant/auth/login<br/>{merchant_name, api_key}
    
    API->>DB: SELECT api_key_hash FROM merchants
    DB-->>API: Hashed key
    
    API->>API: bcrypt::verify(api_key, hash)
    
    alt Valid credentials
        API->>API: Generate JWT token<br/>(exp: 8 hours)
        API-->>M: 200 OK {token, merchant_id}
    else Invalid
        API-->>M: 401 Unauthorized
    end
    
    M->>API: POST /api/v1/merchant/validate<br/>{code, Bearer token}
    
    API->>API: Decode & verify JWT
    
    API->>DB: SELECT * FROM user_redemptions<br/>WHERE code AND status='pending'
    
    alt Valid code
        DB-->>API: Redemption data
        API->>API: Check expiration
        API-->>M: 200 OK {valid: true, offer_name, lumis_spent}
    else Invalid
        API-->>M: 200 OK {valid: false, error}
    end
    
    M->>API: POST /api/v1/merchant/confirm/:id<br/>{Bearer token}
    
    API->>DB: BEGIN TRANSACTION
    API->>DB: SELECT FOR UPDATE<br/>WHERE id AND status='pending'
    
    alt Still pending
        API->>DB: UPDATE SET status='confirmed',<br/>validated_at=NOW()
        
        DB->>DB: TRIGGER: update_merchant_stats()<br/>increment counters
        
        API->>DB: COMMIT
        API-->>M: 200 OK {success: true, confirmed_at}
        
        API->>U: 📱 Push notification<br/>"Tu redención fue confirmada"
    else Already confirmed
        API->>DB: ROLLBACK
        API-->>M: 400 Ya fue confirmada
    end
```

### Flujo 3: Cálculo de Balance (Triggers)

```mermaid
graph TD
    A[INSERT INTO fact_accumulations] --> B{Trigger:}
    B --> C[fun_update_balance_points]
    
    C --> D[Calcular nuevo balance]
    D --> E["SUM(CASE<br/>WHEN accum_type='earn' THEN +quantity<br/>WHEN accum_type='spend' THEN -quantity<br/>END)"]
    
    E --> F[UPDATE fact_balance_points<br/>SET balance = calculated]
    
    F --> G{Balance existe?}
    G -->|No| H[INSERT nuevo registro<br/>en fact_balance_points]
    G -->|Sí| I[UPDATE registro existente]
```

---

## 🔌 API Endpoints - Usuarios

### Base URL
```
https://api.lumis.pa/api/v1/rewards
```

---

### 1. Listar Ofertas Disponibles

```http
GET /api/v1/rewards/offers
```

**Autenticación**: ✅ JWT Required

**Headers**:
```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Query Parameters**:
| Parámetro | Tipo | Requerido | Descripción |
|-----------|------|-----------|-------------|
| `category` | string | No | Filtrar por categoría (food, entertainment, etc) |
| `min_cost` | integer | No | Costo mínimo en Lümis |
| `max_cost` | integer | No | Costo máximo en Lümis |
| `sort` | string | No | Ordenar: `cost_asc`, `cost_desc`, `newest` |
| `limit` | integer | No | Límite de resultados (default: 20) |
| `offset` | integer | No | Offset para paginación (default: 0) |

**Ejemplo Request**:
```bash
curl -X GET "https://api.lumis.pa/api/v1/rewards/offers?category=food&sort=cost_asc&limit=10" \
  -H "Authorization: Bearer eyJ..."
```

**Response 200 OK**:
```json
{
  "success": true,
  "offers": [
    {
      "offer_id": "550e8400-e29b-41d4-a716-446655440000",
      "name_friendly": "Café Americano",
      "description_friendly": "Disfruta un delicioso café americano en Starbucks",
      "lumis_cost": 55,
      "category": "food",
      "merchant_name": "Starbucks Panamá",
      "image_url": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg",
      "terms_and_conditions": "Válido solo en sucursales participantes. No acumulable con otras promociones.",
      "stock_available": true,
      "user_can_redeem": true,
      "user_redemptions_count": 0,
      "max_redemptions": 5
    },
    {
      "offer_id": "660e8400-e29b-41d4-a716-446655440001",
      "name_friendly": "Entrada al Cine 2D",
      "description_friendly": "Boleto para cualquier película en formato 2D",
      "lumis_cost": 150,
      "category": "entertainment",
      "merchant_name": "Cinépolis Panamá",
      "image_url": "https://cdn.lumis.pa/offers/cinepolis-2d.jpg",
      "terms_and_conditions": "Válido lunes a jueves. No aplica estrenos.",
      "stock_available": true,
      "user_can_redeem": true,
      "user_redemptions_count": 1,
      "max_redemptions": 5
    }
  ],
  "pagination": {
    "total": 25,
    "limit": 10,
    "offset": 0,
    "has_more": true
  }
}
```

**Errores**:
- `401 Unauthorized`: JWT inválido o expirado
- `500 Internal Server Error`: Error de base de datos

---

### 2. Detalle de Oferta

```http
GET /api/v1/rewards/offers/:offer_id
```

**Autenticación**: ✅ JWT Required

**Path Parameters**:
| Parámetro | Tipo | Descripción |
|-----------|------|-------------|
| `offer_id` | UUID | ID de la oferta |

**Ejemplo Request**:
```bash
curl -X GET "https://api.lumis.pa/api/v1/rewards/offers/550e8400-e29b-41d4-a716-446655440000" \
  -H "Authorization: Bearer eyJ..."
```

**Response 200 OK**:
```json
{
  "success": true,
  "offer": {
    "offer_id": "550e8400-e29b-41d4-a716-446655440000",
    "name_friendly": "Café Americano",
    "description_friendly": "Disfruta un delicioso café americano caliente o frío",
    "lumis_cost": 55,
    "category": "food",
    "merchant_id": "a1726cd2-dd94-45c6-b996-3c89fa927a0c",
    "merchant_name": "Starbucks Panamá",
    "merchant_type": "cafe",
    "image_url": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg",
    "terms_and_conditions": "Válido solo en sucursales participantes. No acumulable con otras promociones. El café debe ser consumido en el local.",
    "valid_from": "2025-01-01T00:00:00Z",
    "valid_to": "2026-12-31T23:59:59Z",
    "stock_available": true,
    "is_active": true,
    "user_balance": 945,
    "user_can_afford": true,
    "user_redemptions_count": 0,
    "max_redemptions_per_user": 5,
    "user_can_redeem": true
  }
}
```

**Errores**:
- `404 Not Found`: Oferta no existe
- `401 Unauthorized`: JWT inválido

---

### 3. Crear Redención (Canjear Oferta)

```http
POST /api/v1/rewards/redeem
```

**Autenticación**: ✅ JWT Required

**Headers**:
```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json
```

**Request Body**:
```json
{
  "offer_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Ejemplo Request**:
```bash
curl -X POST "https://api.lumis.pa/api/v1/rewards/redeem" \
  -H "Authorization: Bearer eyJ..." \
  -H "Content-Type: application/json" \
  -d '{
    "offer_id": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

**Response 200 OK**:
```json
{
  "success": true,
  "redemption": {
    "redemption_id": "969b8c90-57f8-421d-9db9-4627456b19b7",
    "redemption_code": "LUMS-967E-F893-7EC2",
    "offer_name": "Café Americano",
    "lumis_spent": 55,
    "qr_landing_url": "https://app.lumis.pa/redeem/LUMS-967E-F893-7EC2",
    "qr_image_url": "https://cdn.lumis.pa/qr/LUMS-967E-F893-7EC2.png",
    "code_expires_at": "2025-10-18T18:42:25Z",
    "expires_at": "2025-10-18T18:42:25Z",
    "status": "pending",
    "merchant_name": "Starbucks Panamá",
    "message": "¡Redención creada! Presenta este código en el comercio.",
    "new_balance": 890
  }
}
```

**Errores**:

**400 Bad Request - Saldo insuficiente**:
```json
{
  "error": "Insufficient balance",
  "message": "No tienes suficientes Lümis. Necesitas 55 pero tienes 30.",
  "details": {
    "required": 55,
    "current_balance": 30,
    "missing": 25
  }
}
```

**404 Not Found - Oferta no existe**:
```json
{
  "error": "Offer not found",
  "message": "La oferta solicitada no existe o no está disponible"
}
```

**400 Bad Request - Stock agotado**:
```json
{
  "error": "Out of stock",
  "message": "Esta oferta ya no tiene stock disponible"
}
```

**400 Bad Request - Límite alcanzado**:
```json
{
  "error": "Redemption limit reached",
  "message": "Ya alcanzaste el límite máximo de redenciones para esta oferta (5/5)"
}
```

---

### 4. Historial de Redenciones

```http
GET /api/v1/rewards/history
```

**Autenticación**: ✅ JWT Required

**Query Parameters**:
| Parámetro | Tipo | Descripción |
|-----------|------|-------------|
| `status` | string | Filtrar por estado: pending, confirmed, cancelled, expired |
| `limit` | integer | Límite de resultados (default: 20) |
| `offset` | integer | Offset para paginación |

**Ejemplo Request**:
```bash
curl -X GET "https://api.lumis.pa/api/v1/rewards/history?status=pending&limit=10" \
  -H "Authorization: Bearer eyJ..."
```

**Response 200 OK**:
```json
{
  "success": true,
  "redemptions": [
    {
      "redemption_id": "969b8c90-57f8-421d-9db9-4627456b19b7",
      "redemption_code": "LUMS-967E-F893-7EC2",
      "offer_name": "Café Americano",
      "merchant_name": "Starbucks Panamá",
      "lumis_spent": 55,
      "status": "confirmed",
      "created_at": "2025-10-18T18:27:25Z",
      "code_expires_at": "2025-10-18T18:42:25Z",
      "validated_at": "2025-10-18T18:32:49Z",
      "qr_landing_url": "https://app.lumis.pa/redeem/LUMS-967E-F893-7EC2",
      "qr_image_url": "https://cdn.lumis.pa/qr/LUMS-967E-F893-7EC2.png"
    },
    {
      "redemption_id": "12bd8782-9e63-4d29-9cba-17494201ae67",
      "redemption_code": "LUMS-0B2E-F885-610D",
      "offer_name": "Café Americano",
      "merchant_name": "Starbucks Panamá",
      "lumis_spent": 55,
      "status": "pending",
      "created_at": "2025-10-18T18:11:32Z",
      "code_expires_at": "2025-10-18T18:26:32Z",
      "validated_at": null,
      "qr_landing_url": "https://app.lumis.pa/redeem/LUMS-0B2E-F885-610D",
      "qr_image_url": "https://cdn.lumis.pa/qr/LUMS-0B2E-F885-610D.png"
    }
  ],
  "pagination": {
    "total": 5,
    "limit": 10,
    "offset": 0,
    "has_more": false
  }
}
```

---

### 5. Detalle de Redención

```http
GET /api/v1/rewards/history/:redemption_id
```

**Autenticación**: ✅ JWT Required

**Ejemplo Request**:
```bash
curl -X GET "https://api.lumis.pa/api/v1/rewards/history/969b8c90-57f8-421d-9db9-4627456b19b7" \
  -H "Authorization: Bearer eyJ..."
```

**Response 200 OK**:
```json
{
  "success": true,
  "redemption": {
    "redemption_id": "969b8c90-57f8-421d-9db9-4627456b19b7",
    "redemption_code": "LUMS-967E-F893-7EC2",
    "offer": {
      "offer_id": "550e8400-e29b-41d4-a716-446655440000",
      "name_friendly": "Café Americano",
      "description_friendly": "Disfruta un delicioso café americano",
      "image_url": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg"
    },
    "merchant_name": "Starbucks Panamá",
    "lumis_spent": 55,
    "status": "confirmed",
    "created_at": "2025-10-18T18:27:25Z",
    "code_expires_at": "2025-10-18T18:42:25Z",
    "validated_at": "2025-10-18T18:32:49Z",
    "qr_landing_url": "https://app.lumis.pa/redeem/LUMS-967E-F893-7EC2",
    "qr_image_url": "https://cdn.lumis.pa/qr/LUMS-967E-F893-7EC2.png",
    "can_cancel": false
  }
}
```

---

### 6. Cancelar Redención

```http
DELETE /api/v1/rewards/history/:redemption_id
```

**Autenticación**: ✅ JWT Required

**Ejemplo Request**:
```bash
curl -X DELETE "https://api.lumis.pa/api/v1/rewards/history/969b8c90-57f8-421d-9db9-4627456b19b7" \
  -H "Authorization: Bearer eyJ..."
```

**Response 200 OK**:
```json
{
  "success": true,
  "message": "Redención cancelada. Tus Lümis han sido devueltos.",
  "refunded_lumis": 55,
  "new_balance": 945
}
```

**Errores**:

**400 Bad Request - Ya confirmada**:
```json
{
  "error": "Cannot cancel",
  "message": "No puedes cancelar una redención que ya fue confirmada por el comercio"
}
```

**400 Bad Request - Ya expirada**:
```json
{
  "error": "Already expired",
  "message": "Esta redención ya expiró y no puede ser cancelada"
}
```

---

### 7. Estadísticas del Usuario

```http
GET /api/v1/rewards/stats
```

**Autenticación**: ✅ JWT Required

**Ejemplo Request**:
```bash
curl -X GET "https://api.lumis.pa/api/v1/rewards/stats" \
  -H "Authorization: Bearer eyJ..."
```

**Response 200 OK**:
```json
{
  "success": true,
  "balance": 890,
  "total_redemptions": 2,
  "pending_redemptions": 1,
  "confirmed_redemptions": 1,
  "cancelled_redemptions": 0,
  "expired_redemptions": 0,
  "total_lumis_spent": 110
}
```

---

## 🏪 API Endpoints - Merchant

### Base URL
```
https://api.lumis.pa/api/v1/merchant
```

---

### 1. Login de Merchant

```http
POST /api/v1/merchant/auth/login
```

**Autenticación**: ❌ No requiere (endpoint público)

**Headers**:
```http
Content-Type: application/json
```

**Request Body**:
```json
{
  "merchant_name": "Starbucks Test",
  "api_key": "test_merchant_key_12345"
}
```

**Ejemplo Request**:
```bash
curl -X POST "https://api.lumis.pa/api/v1/merchant/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "merchant_name": "Starbucks Test",
    "api_key": "test_merchant_key_12345"
  }'
```

**Response 200 OK**:
```json
{
  "success": true,
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3NjA4NDAzNzUsImlhdCI6MTc2MDgxMTU3NSwibWVyY2hhbnRfbmFtZSI6IlN0YXJidWNrcyBUZXN0Iiwicm9sZSI6Im1lcmNoYW50Iiwic3ViIjoiYTE3MjZjZDItZGQ5NC00NWM2LWI5OTYtM2M4OWZhOTI3YTBjIn0.zosCGq4RslPPExgWwkVJp1Z9CirpkoKZ9BQtdEVTRLk",
  "merchant": {
    "merchant_id": "a1726cd2-dd94-45c6-b996-3c89fa927a0c",
    "merchant_name": "Starbucks Test",
    "expires_in": 28800
  }
}
```

**JWT Token Payload**:
```json
{
  "sub": "a1726cd2-dd94-45c6-b996-3c89fa927a0c",
  "merchant_name": "Starbucks Test",
  "role": "merchant",
  "exp": 1760840375,
  "iat": 1760811575
}
```

**Errores**:

**401 Unauthorized - Credenciales inválidas**:
```json
{
  "error": "Invalid credentials",
  "message": "Credenciales inválidas"
}
```

**404 Not Found - Merchant no existe**:
```json
{
  "error": "Merchant not found",
  "message": "Comercio no encontrado"
}
```

**403 Forbidden - Merchant inactivo**:
```json
{
  "error": "Merchant inactive",
  "message": "Comercio desactivado. Contacta soporte."
}
```

---

### 2. Validar Código de Redención

```http
POST /api/v1/merchant/validate
```

**Autenticación**: ✅ JWT Required (Merchant Token)

**Headers**:
```http
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
Content-Type: application/json
```

**Request Body**:
```json
{
  "code": "LUMS-967E-F893-7EC2"
}
```

**Tipos aceptados para `code`**:
- Código de redención: `"LUMS-967E-F893-7EC2"`
- UUID redemption_id: `"969b8c90-57f8-421d-9db9-4627456b19b7"`

**Ejemplo Request**:
```bash
curl -X POST "https://api.lumis.pa/api/v1/merchant/validate" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
  -H "Content-Type: application/json" \
  -d '{
    "code": "LUMS-967E-F893-7EC2"
  }'
```

**Response 200 OK - Código válido**:
```json
{
  "success": true,
  "valid": true,
  "redemption": {
    "redemption_id": "969b8c90-57f8-421d-9db9-4627456b19b7",
    "redemption_code": "LUMS-967E-F893-7EC2",
    "offer_name": "Café Americano",
    "lumis_spent": 55,
    "status": "pending",
    "created_at": "2025-10-18T18:27:25.950899+00:00",
    "expires_at": "2025-10-18T18:42:25.950733+00:00",
    "can_confirm": true
  },
  "message": "Código válido. Puedes confirmar la redención."
}
```

**Response 200 OK - Código inválido**:
```json
{
  "success": true,
  "valid": false,
  "redemption": null,
  "message": "Código no encontrado o inválido"
}
```

**Response 200 OK - Código expirado**:
```json
{
  "success": true,
  "valid": false,
  "redemption": null,
  "message": "Este código expiró"
}
```

**Response 200 OK - Ya fue usado**:
```json
{
  "success": true,
  "valid": false,
  "redemption": {
    "redemption_id": "969b8c90-57f8-421d-9db9-4627456b19b7",
    "redemption_code": "LUMS-967E-F893-7EC2",
    "offer_name": "Café Americano",
    "lumis_spent": 55,
    "status": "confirmed",
    "created_at": "2025-10-18T18:27:25.950899+00:00",
    "expires_at": "2025-10-18T18:42:25.950733+00:00",
    "can_confirm": false
  },
  "message": "Este código ya fue usado"
}
```

**Errores**:
- `401 Unauthorized`: JWT de merchant inválido o expirado
- `403 Forbidden`: Token no tiene role="merchant"

---

### 3. Confirmar Redención

```http
POST /api/v1/merchant/confirm/:redemption_id
```

**Autenticación**: ✅ JWT Required (Merchant Token)

**Headers**:
```http
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
Content-Type: application/json
```

**Path Parameters**:
| Parámetro | Tipo | Descripción |
|-----------|------|-------------|
| `redemption_id` | UUID | ID de la redención |

**Ejemplo Request**:
```bash
curl -X POST "https://api.lumis.pa/api/v1/merchant/confirm/969b8c90-57f8-421d-9db9-4627456b19b7" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
  -H "Content-Type: application/json"
```

**Response 200 OK**:
```json
{
  "success": true,
  "message": "Redención confirmada exitosamente",
  "redemption_id": "969b8c90-57f8-421d-9db9-4627456b19b7",
  "confirmed_at": "2025-10-18T18:32:49.977009201+00:00"
}
```

**Errores**:

**404 Not Found**:
```json
{
  "error": "Redemption not found",
  "message": "Redención no encontrada"
}
```

**400 Bad Request - Ya confirmada**:
```json
{
  "error": "Already confirmed",
  "message": "Esta redención ya fue confirmada"
}
```

**400 Bad Request - Ya cancelada**:
```json
{
  "error": "Already cancelled",
  "message": "Esta redención fue cancelada por el usuario"
}
```

**400 Bad Request - Expirada**:
```json
{
  "error": "Expired",
  "message": "Código expirado"
}
```

---

### 4. Estadísticas del Merchant

```http
GET /api/v1/merchant/stats
```

**Autenticación**: ✅ JWT Required (Merchant Token)

**Headers**:
```http
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```

**Ejemplo Request**:
```bash
curl -X GET "https://api.lumis.pa/api/v1/merchant/stats" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..."
```

**Response 200 OK**:
```json
{
  "success": true,
  "stats": {
    "total_redemptions": 2,
    "pending_redemptions": 1,
    "confirmed_redemptions": 1,
    "today_redemptions": 2,
    "this_week_redemptions": 2,
    "this_month_redemptions": 2,
    "total_lumis_redeemed": 55,
    "recent_redemptions": [
      {
        "redemption_id": "969b8c90-57f8-421d-9db9-4627456b19b7",
        "redemption_code": "LUMS-967E-F893-7EC2",
        "offer_name": "Café Americano",
        "lumis_spent": 55,
        "status": "confirmed",
        "created_at": "2025-10-18T18:27:25.950899+00:00",
        "validated_at": "2025-10-18T18:32:49.974960+00:00"
      },
      {
        "redemption_id": "12bd8782-9e63-4d29-9cba-17494201ae67",
        "redemption_code": "LUMS-0B2E-F885-610D",
        "offer_name": "Café Americano",
        "lumis_spent": 55,
        "status": "pending",
        "created_at": "2025-10-18T18:11:32.783191+00:00",
        "validated_at": null
      }
    ]
  }
}
```

---

## 🔐 Autenticación y Seguridad

### JWT Tokens - Usuarios

**Header**:
```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Payload**:
```json
{
  "sub": "12345",
  "email": "test@example.com",
  "name": "Test User",
  "iat": 1760812027,
  "exp": 1760819227
}
```

**Campos**:
- `sub`: User ID como string
- `email`: Email del usuario
- `name`: Nombre del usuario
- `iat`: Issued at timestamp
- `exp`: Expiration timestamp

**Algoritmo**: HS256 (HMAC with SHA-256)  
**Secret**: Configurado en variable de entorno `JWT_SECRET`

### JWT Tokens - Merchants

**Header**:
```http
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```

**Payload**:
```json
{
  "sub": "a1726cd2-dd94-45c6-b996-3c89fa927a0c",
  "merchant_name": "Starbucks Test",
  "role": "merchant",
  "exp": 1760840375,
  "iat": 1760811575
}
```

**Campos**:
- `sub`: Merchant ID (UUID)
- `merchant_name`: Nombre del comercio
- `role`: Siempre "merchant"
- `iat`: Issued at timestamp
- `exp`: Expiration timestamp (8 horas después de iat)

**Validación**:
- Middleware verifica que `role === "merchant"`
- Valida firma con el mismo secret que usuarios
- Verifica expiración

### API Keys - Merchants

**Formato**: String arbitrario (ej: `test_merchant_key_12345`)

**Almacenamiento**: Hasheado con bcrypt en columna `api_key_hash`

**Validación**:
```rust
let is_valid = bcrypt::verify(&payload.api_key, &merchant.api_key_hash)
    .unwrap_or(false);
```

**Best Practices**:
- Nunca almacenar API keys en texto plano
- Usar bcrypt con cost factor 12
- Rotar keys periódicamente
- Un merchant = un API key único

### Rate Limiting

**Usuarios**:
- 100 requests por minuto por IP
- 1000 requests por hora por user_id

**Merchants**:
- 500 requests por minuto por merchant_id
- Sin límite de confirmaciones (operación crítica)

### HTTPS

**Obligatorio en producción**:
- Todos los endpoints deben usar HTTPS
- Certificado SSL/TLS válido
- HSTS habilitado

---

## ❌ Códigos de Error

### Códigos HTTP

| Código | Significado | Cuándo ocurre |
|--------|-------------|---------------|
| 200 | OK | Request exitoso |
| 400 | Bad Request | Datos inválidos, saldo insuficiente, límite alcanzado |
| 401 | Unauthorized | JWT inválido, expirado o faltante |
| 403 | Forbidden | Permisos insuficientes, merchant inactivo |
| 404 | Not Found | Recurso no encontrado (oferta, redención) |
| 409 | Conflict | Race condition en confirmación |
| 500 | Internal Server Error | Error de base de datos o servidor |

### Estructura de Errores

```json
{
  "error": "error_code",
  "message": "Mensaje legible para el usuario",
  "details": {
    "campo_adicional": "información extra"
  }
}
```

**Ejemplos**:

**Saldo insuficiente**:
```json
{
  "error": "Insufficient balance",
  "message": "No tienes suficientes Lümis. Necesitas 55 pero tienes 30.",
  "details": {
    "required": 55,
    "current_balance": 30,
    "missing": 25
  }
}
```

**Token expirado**:
```json
{
  "error": "Token expired",
  "message": "Your session has expired. Please log in again.",
  "details": "JWT error: ExpiredSignature"
}
```

**Oferta no encontrada**:
```json
{
  "error": "Offer not found",
  "message": "La oferta solicitada no existe o no está disponible"
}
```

---

## 📱 Ejemplos de Integración

### Flujo Completo - App de Usuario

```javascript
// 1. Login del usuario (obtener JWT)
const loginResponse = await fetch('https://api.lumis.pa/api/v1/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'password123'
  })
});
const { token } = await loginResponse.json();

// 2. Listar ofertas disponibles
const offersResponse = await fetch('https://api.lumis.pa/api/v1/rewards/offers?category=food&limit=10', {
  headers: { 'Authorization': `Bearer ${token}` }
});
const { offers } = await offersResponse.json();

// 3. Usuario selecciona una oferta y la canjea
const redeemResponse = await fetch('https://api.lumis.pa/api/v1/rewards/redeem', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    offer_id: '550e8400-e29b-41d4-a716-446655440000'
  })
});

const redemption = await redeemResponse.json();

if (redemption.success) {
  // 4. Mostrar QR al usuario
  console.log('Código:', redemption.redemption.redemption_code);
  console.log('QR Image:', redemption.redemption.qr_image_url);
  console.log('Nuevo balance:', redemption.redemption.new_balance);
  
  // Mostrar UI con QR y countdown hasta expiración
  showQRCode(redemption.redemption.qr_image_url);
  startCountdown(redemption.redemption.code_expires_at);
}
```

### Flujo Completo - App de Merchant

```javascript
// 1. Login del merchant (obtener JWT)
const merchantLoginResponse = await fetch('https://api.lumis.pa/api/v1/merchant/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    merchant_name: 'Starbucks Test',
    api_key: 'test_merchant_key_12345'
  })
});
const { token: merchantToken } = await merchantLoginResponse.json();

// 2. Escanear QR del usuario (obtener código)
const scannedCode = 'LUMS-967E-F893-7EC2';

// 3. Validar el código (preview)
const validateResponse = await fetch('https://api.lumis.pa/api/v1/merchant/validate', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${merchantToken}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({ code: scannedCode })
});

const validation = await validateResponse.json();

if (validation.valid && validation.redemption.can_confirm) {
  // 4. Mostrar preview al merchant
  console.log('Oferta:', validation.redemption.offer_name);
  console.log('Lümis:', validation.redemption.lumis_spent);
  
  // 5. Merchant confirma
  const confirmResponse = await fetch(
    `https://api.lumis.pa/api/v1/merchant/confirm/${validation.redemption.redemption_id}`,
    {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${merchantToken}`,
        'Content-Type': 'application/json'
      }
    }
  );
  
  const confirmation = await confirmResponse.json();
  
  if (confirmation.success) {
    console.log('✅ Redención confirmada!');
    playSuccessSound();
    showSuccessMessage();
  }
} else {
  // Mostrar mensaje de error
  console.error('❌', validation.message);
  showErrorMessage(validation.message);
}
```

### Manejo de Errores

```javascript
async function redeemOffer(offerId, userToken) {
  try {
    const response = await fetch('https://api.lumis.pa/api/v1/rewards/redeem', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${userToken}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ offer_id: offerId })
    });
    
    const data = await response.json();
    
    if (!response.ok) {
      // Manejar errores específicos
      switch (response.status) {
        case 400:
          if (data.error === 'Insufficient balance') {
            alert(`No tienes suficientes Lümis. Te faltan ${data.details.missing} Lümis.`);
          } else if (data.error === 'Redemption limit reached') {
            alert('Ya alcanzaste el límite de redenciones para esta oferta.');
          }
          break;
        
        case 401:
          // Token expirado, redirigir a login
          redirectToLogin();
          break;
        
        case 404:
          alert('Esta oferta ya no está disponible.');
          break;
        
        case 500:
          alert('Error del servidor. Intenta nuevamente.');
          logError(data);
          break;
      }
      
      return null;
    }
    
    return data.redemption;
    
  } catch (error) {
    console.error('Network error:', error);
    alert('Error de conexión. Verifica tu internet.');
    return null;
  }
}
```

---

## 🔄 Triggers y Lógica de Negocio

### Trigger: `fun_update_balance_points()`

**Cuándo se dispara**: AFTER INSERT OR DELETE OR UPDATE en `fact_accumulations`

**Función**:
```sql
CREATE OR REPLACE FUNCTION rewards.fun_update_balance_points()
RETURNS trigger
LANGUAGE plpgsql
AS $function$
BEGIN
  -- Calculate balance from fact_accumulations only
  UPDATE rewards.fact_balance_points rs
  SET balance = (
    SELECT COALESCE(
      SUM(CASE 
        WHEN accum_type = 'earn' THEN quantity
        WHEN accum_type = 'spend' THEN -quantity
        ELSE 0
      END),
      0
    )
    FROM rewards.fact_accumulations 
    WHERE user_id = NEW.user_id AND dtype = 'points'
  ),
  latest_update = NOW()
  WHERE rs.user_id = NEW.user_id;
  
  -- If the record didn't exist, insert it
  IF NOT FOUND THEN
    INSERT INTO rewards.fact_balance_points (user_id, balance, latest_update)
    VALUES (
      NEW.user_id,
      (SELECT COALESCE(SUM(...), 0) FROM rewards.fact_accumulations ...),
      NOW()
    );
  END IF;
  
  RETURN NEW;
END;
$function$;
```

### Trigger: `update_merchant_stats()`

**Cuándo se dispara**: AFTER UPDATE en `user_redemptions`

**Función**:
```sql
CREATE OR REPLACE FUNCTION rewards.update_merchant_stats()
RETURNS trigger
LANGUAGE plpgsql
AS $function$
BEGIN
    IF NEW.redemption_status = 'confirmed' AND OLD.redemption_status != 'confirmed' THEN
        UPDATE rewards.merchants
        SET total_redemptions = total_redemptions + 1,
            total_lumis_redeemed = total_lumis_redeemed + NEW.lumis_spent,
            updated_at = NOW()
        WHERE merchant_id = NEW.validated_by_merchant_id;
    END IF;
    
    RETURN NEW;
END;
$function$;
```

### Trigger: `refund_lumis_on_cancel()`

**Cuándo se dispara**: AFTER UPDATE en `user_redemptions`

**Función**: Devuelve Lümis al usuario cuando cancela una redención

---

## 📊 Métricas y Monitoreo

### KPIs Recomendados

1. **Conversion Rate**: % de ofertas vistas que se convierten en redenciones
2. **Redemption Confirmation Rate**: % de redenciones pending que llegan a confirmed
3. **Expiration Rate**: % de redenciones que expiran sin usar
4. **Average Redemption Value**: Promedio de Lümis por redención
5. **Top Offers**: Ofertas más redimidas
6. **Merchant Performance**: Velocidad de confirmación por merchant

### Logs Importantes

```
✅ Redemption created: user_id=12345, offer_id=550e8400, code=LUMS-967E
✅ Redemption confirmed: redemption_id=969b8c90, merchant=Starbucks Test
❌ Insufficient balance: user_id=12345, required=55, current=30
❌ Validation failed: code=LUMS-INVALID, reason=not_found
🔐 JWT authentication successful: user_id=12345, email=test@example.com
🏪 Merchant authentication successful: Starbucks Test (a1726cd2...)
```

---

## 🚀 Próximos Pasos

### Funcionalidades Planificadas

1. **Webhooks para Merchants**
   - Notificar a merchants cuando se crea una redención
   - POST a `merchant.webhook_url` con datos de redención

2. **Push Notifications**
   - Notificar usuario cuando redención es confirmada
   - Alertas de expiración próxima

3. **Analytics Dashboard**
   - Panel para merchants con estadísticas en tiempo real
   - Gráficos de redenciones por día/semana/mes

4. **Scheduled Jobs**
   - Expiración automática de códigos vencidos
   - Recordatorios antes de expiración

5. **Fraud Detection**
   - Detectar patrones de uso sospechoso
   - Rate limiting más sofisticado

---

## 📞 Soporte

**Equipo de Desarrollo**:
- API Issues: api@lumis.pa
- Merchant Onboarding: merchants@lumis.pa
- Documentación: docs@lumis.pa

**Status Page**: https://status.lumis.pa

---

**Última actualización**: 2025-10-18  
**Versión del documento**: 2.0  
**Estado**: ✅ Producción
