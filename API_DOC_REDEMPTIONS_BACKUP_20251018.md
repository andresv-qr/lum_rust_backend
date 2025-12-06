# 🎁 API DE REDENCIÓN DE LÜMIS - DOCUMENTACIÓN COMPLETA

## 📋 Tabla de Contenidos
- [Arquitectura del Sistema](#arquitectura-del-sistema)
- [Modelo de Datos](#modelo-de-datos)
- [Flujos de Negocio](#flujos-de-negocio)
- [API Endpoints - Usuarios](#api-endpoints---usuarios)
- [API Endpoints - Comercios](#api-endpoints---comercios)
- [API Endpoints - Admin](#api-endpoints---admin)
- [Sistema de QR Codes](#sistema-de-qr-codes)
- [Seguridad](#seguridad)
- [Ejemplos de Uso](#ejemplos-de-uso)

---

## 🏗️ Arquitectura del Sistema

### Componentes

```
┌─────────────────────────────────────────────────────────────┐
│  APP PRINCIPAL (lum_rust_ws) - Puerto 8000                  │
│                                                             │
│  📦 MÓDULOS:                                                │
│  ├── domains/rewards/                                       │
│  │   ├── catalog.rs      → Catálogo de ofertas            │
│  │   ├── redemption.rs   → Lógica de redención            │
│  │   └── qr_generator.rs → Generación de QR codes         │
│  │                                                         │
│  ├── api/rewards/                                           │
│  │   ├── offers.rs       → GET /api/v1/rewards/offers     │
│  │   ├── redeem.rs       → POST /api/v1/rewards/redeem    │
│  │   └── user.rs         → GET /api/v1/rewards/my-*       │
│  │                                                         │
│  └── api/merchant/                                          │
│      ├── validate.rs     → POST /api/v1/merchant/validate │
│      └── confirm.rs      → POST /api/v1/merchant/confirm  │
└─────────────────────────────────────────────────────────────┘
```

### Base de Datos

**Schema**: `rewards` (ya existe)

```sql
-- Tablas Adaptadas/Extendidas:
✓ redemption_offers       (antes: dim_redemptions)
✓ user_redemptions        (antes: fact_redemptions)
✓ fact_accumulations      (extendida con redemption_id)
✓ fact_balance_points     (sin cambios)

-- Tablas Nuevas:
+ redemption_audit_log    (auditoría completa)
+ merchants               (comercios aliados)
```

---

## 🗄️ Modelo de Datos

### 1. `redemption_offers` - Catálogo de Ofertas

**Descripción**: Define las ofertas disponibles para redimir con Lümis.

```sql
CREATE TABLE redemption_offers (
    -- Identificación
    id INTEGER PRIMARY KEY,                      -- ID original (compatibilidad)
    offer_id UUID DEFAULT gen_random_uuid(),     -- UUID para APIs
    
    -- Información básica
    name TEXT NOT NULL,                          -- 'cafe_americano_starbucks'
    name_friendly VARCHAR(255),                  -- 'Café Americano'
    description_friendly VARCHAR(500),           -- Descripción para usuario
    
    -- Costo y disponibilidad
    points INTEGER NOT NULL,                     -- Lümis que cuesta (legacy)
    lumis_cost INTEGER,                          -- Alias moderno
    stock_quantity INTEGER,                      -- NULL = ilimitado
    max_redemptions_per_user INTEGER DEFAULT 5,  -- Límite por usuario
    
    -- Categorización
    offer_category VARCHAR(50),                  -- 'food', 'entertainment', 'books'
    
    -- Comercio
    merchant_id UUID,                            -- FK a merchants
    merchant_name VARCHAR(255),                  -- Nombre del comercio
    
    -- Vigencia
    valid_from TIMESTAMP WITH TIME ZONE,
    valid_to TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT true,
    
    -- Visual
    img VARCHAR(500),                            -- URL de imagen
    terms_and_conditions TEXT,                   -- Términos y condiciones
    
    -- Condiciones dinámicas (legacy, mantener compatibilidad)
    cond_1_key TEXT,
    cond_1_value TEXT,
    cond_2_key TEXT,
    cond_2_value TEXT,
    cond_3_key TEXT,
    cond_3_value TEXT,
    cond_4_key TEXT,
    cond_4_value TEXT,
    
    -- Duración (legacy)
    duration_value INTEGER,
    duration_unit VARCHAR(50),
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    update_date TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Índices
CREATE INDEX idx_offers_active_valid ON redemption_offers(is_active, valid_to) 
    WHERE is_active = true AND valid_to > NOW();
CREATE INDEX idx_offers_category ON redemption_offers(offer_category);
CREATE INDEX idx_offers_cost ON redemption_offers(lumis_cost);
```

**Ejemplo de Registro**:
```json
{
  "offer_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "cafe_americano_starbucks",
  "name_friendly": "Café Americano",
  "description_friendly": "Disfruta de un delicioso café americano en cualquier sucursal de Starbucks",
  "lumis_cost": 55,
  "offer_category": "food",
  "merchant_name": "Starbucks Panamá",
  "img": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg",
  "valid_from": "2025-01-01T00:00:00Z",
  "valid_to": "2026-12-31T23:59:59Z",
  "is_active": true,
  "stock_quantity": null
}
```

---

### 2. `user_redemptions` - Redenciones de Usuario

**Descripción**: Registra cada redención realizada por un usuario.

```sql
CREATE TABLE user_redemptions (
    redemption_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Usuario y oferta
    user_id INTEGER NOT NULL,
    offer_id UUID NOT NULL REFERENCES redemption_offers(offer_id),
    
    -- Redención
    lumis_spent INTEGER NOT NULL,
    redemption_method VARCHAR(50) NOT NULL DEFAULT 'qr_code',
    redemption_status VARCHAR(50) NOT NULL DEFAULT 'pending',
    
    -- QR Code
    redemption_code VARCHAR(100) UNIQUE NOT NULL,
    qr_image_url TEXT,
    qr_landing_url TEXT,
    
    -- Validación por comercio
    validated_by_merchant_id UUID,
    validated_at TIMESTAMP WITH TIME ZONE,
    validation_ip_address INET,
    
    -- Expiración
    code_expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    cancelled_at TIMESTAMP WITH TIME ZONE,
    cancellation_reason TEXT,
    
    -- Seguridad
    validation_token_hash VARCHAR(255),
    is_used BOOLEAN DEFAULT false,
    
    -- Metadata
    metadata JSONB,
    
    CONSTRAINT valid_status CHECK (
        redemption_status IN ('pending', 'confirmed', 'cancelled', 'expired')
    ),
    CONSTRAINT valid_method CHECK (
        redemption_method IN ('qr_code', 'barcode', 'nfc', 'manual')
    )
);

-- Índices críticos
CREATE INDEX idx_redemptions_user ON user_redemptions(user_id, created_at DESC);
CREATE INDEX idx_redemptions_code ON user_redemptions(redemption_code) 
    WHERE redemption_status = 'pending';
CREATE INDEX idx_redemptions_status ON user_redemptions(user_id, redemption_status);
CREATE UNIQUE INDEX idx_unique_active_redemption 
    ON user_redemptions(redemption_code) 
    WHERE redemption_status = 'pending' AND is_used = false;
```

**Estados de Redención**:
- `pending`: Creada, esperando validación
- `confirmed`: Validada y confirmada por comercio
- `cancelled`: Cancelada por usuario (Lümis devueltos)
- `expired`: Código expirado (no usado a tiempo)

**Métodos de Redención**:
- `qr_code`: Código QR (implementado)
- `barcode`: Código de barras (futuro)
- `nfc`: Near Field Communication (futuro)
- `manual`: Código manual (fallback)

---

### 3. `fact_accumulations` - Transacciones de Lümis

**Descripción**: Registra todas las transacciones de Lümis (ganar y gastar).

```sql
-- Tabla existente, extendida con:
ALTER TABLE fact_accumulations 
ADD COLUMN redemption_id UUID REFERENCES user_redemulations(redemption_id);

-- Estructura completa:
CREATE TABLE fact_accumulations (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    accum_id INTEGER,                    -- FK a dim_accumulations (ganar)
    accum_type TEXT,                     -- 'earn' o 'spend'
    accum_key TEXT,                      -- Referencia externa
    dtype TEXT,                          -- Tipo específico
    quantity NUMERIC NOT NULL,           -- Positivo=ganar, Negativo=gastar
    balance BIGINT,                      -- Balance después de transacción
    date TIMESTAMP WITH TIME ZONE,
    
    -- NUEVO:
    redemption_id UUID                   -- FK para redenciones
);

CREATE INDEX idx_fact_accumulations_redemption 
ON fact_accumulations(redemption_id) WHERE redemption_id IS NOT NULL;
```

**Ejemplo - Ganar Lümis**:
```json
{
  "user_id": 123,
  "accum_type": "earn",
  "accum_id": 1,
  "dtype": "invoice_processed",
  "quantity": 10,
  "balance": 465,
  "date": "2025-10-17T10:30:00Z",
  "redemption_id": null
}
```

**Ejemplo - Gastar Lümis (Redención)**:
```json
{
  "user_id": 123,
  "accum_type": "spend",
  "dtype": "redemption",
  "quantity": -55,
  "balance": 410,
  "date": "2025-10-17T15:45:00Z",
  "redemption_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

---

### 4. `redemption_audit_log` - Auditoría

**Descripción**: Log de todas las acciones sobre redenciones (seguridad y trazabilidad).

```sql
CREATE TABLE redemption_audit_log (
    log_id BIGSERIAL PRIMARY KEY,
    redemption_id UUID NOT NULL REFERENCES user_redemptions(redemption_id),
    
    -- Acción
    action_type VARCHAR(50) NOT NULL,    -- 'created', 'validated', 'confirmed', 'cancelled'
    
    -- Actor
    performed_by VARCHAR(50),            -- 'user', 'merchant', 'system'
    merchant_id UUID,
    
    -- Contexto técnico
    ip_address INET,
    user_agent TEXT,
    request_id UUID,
    
    -- Resultado
    success BOOLEAN NOT NULL,
    error_message TEXT,
    
    -- Timestamp
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_audit_redemption ON redemption_audit_log(redemption_id, created_at DESC);
CREATE INDEX idx_audit_action ON redemption_audit_log(action_type, created_at DESC);
```

---

### 5. `merchants` - Comercios Aliados

**Descripción**: Comercios autorizados para validar redenciones.

```sql
CREATE TABLE merchants (
    merchant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_name VARCHAR(255) NOT NULL UNIQUE,
    merchant_type VARCHAR(50),           -- 'restaurant', 'cinema', 'bookstore'
    
    -- Contacto
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),
    
    -- API Credentials
    api_key_hash VARCHAR(255) NOT NULL,  -- bcrypt hash
    webhook_url TEXT,
    
    -- Estado
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Estadísticas
    total_redemptions INTEGER DEFAULT 0,
    total_lumis_redeemed BIGINT DEFAULT 0
);

CREATE INDEX idx_merchants_active ON merchants(is_active) WHERE is_active = true;
```

---

## 🔄 Flujos de Negocio

### Flujo 1: Usuario Redime Lümis

```mermaid
sequenceDiagram
    participant U as Usuario
    participant API as API Principal
    participant DB as Database
    participant QR as QR Generator
    participant S3 as Storage

    U->>API: GET /api/v1/rewards/offers
    API->>DB: SELECT FROM redemption_offers WHERE is_active
    DB-->>API: Lista de ofertas
    API-->>U: JSON ofertas

    U->>API: POST /api/v1/rewards/redeem {offer_id}
    
    API->>DB: BEGIN TRANSACTION
    API->>DB: SELECT balance FROM fact_balance_points
    
    alt Balance suficiente
        API->>DB: UPDATE stock (si aplica)
        API->>DB: INSERT INTO user_redemptions
        API->>DB: INSERT INTO fact_accumulations (negativo)
        API->>DB: UPDATE fact_balance_points
        API->>DB: COMMIT
        
        API->>QR: Generar QR con logo
        QR->>S3: Subir imagen
        S3-->>QR: URL
        QR-->>API: QR data
        
        API->>DB: UPDATE user_redemptions SET qr_image_url
        API-->>U: {redemption_id, qr_url, landing_url, code}
    else Balance insuficiente
        API->>DB: ROLLBACK
        API-->>U: 400 Saldo insuficiente
    end
```

**Pasos**:
1. Usuario consulta ofertas disponibles
2. Selecciona oferta y confirma redención
3. API valida saldo del usuario
4. Crea registro en `user_redemptions` (status='pending')
5. Resta Lümis en `fact_accumulations` (quantity negativo)
6. Actualiza `fact_balance_points`
7. Genera código QR con logo
8. Sube imagen a storage (S3/local)
9. Retorna URL del QR y landing page

---

### Flujo 2: Comercio Valida Redención

```mermaid
sequenceDiagram
    participant C as Comercio
    participant API as API Principal
    participant DB as Database
    participant U as Usuario

    C->>API: POST /api/v1/merchant/validate<br/>{redemption_code, token}
    
    API->>DB: SELECT * FROM user_redemptions<br/>WHERE redemption_code AND status='pending'
    
    alt Código válido
        DB-->>API: Redemption data
        API->>API: Validar JWT token (1 min)
        API->>API: Verificar no expirado
        API->>API: Verificar no usado
        
        API->>DB: INSERT INTO redemption_audit_log
        API-->>C: 200 OK {offer_name, user_name, can_confirm: true}
    else Código inválido
        API->>DB: INSERT INTO redemption_audit_log (failed)
        API-->>C: 400 Código inválido/expirado/usado
    end

    C->>API: POST /api/v1/merchant/confirm<br/>{redemption_code}
    
    API->>DB: BEGIN TRANSACTION
    API->>DB: SELECT FOR UPDATE NOWAIT<br/>WHERE code AND status='pending'
    
    alt Aún disponible
        API->>DB: UPDATE SET status='confirmed',<br/>is_used=true, validated_at=NOW()
        API->>DB: INSERT INTO redemption_audit_log
        API->>DB: COMMIT
        
        API->>U: 📱 Push notification "Redención confirmada"
        API-->>C: 200 OK {success: true}
    else Ya confirmado (race condition)
        API->>DB: ROLLBACK
        API-->>C: 409 Ya fue confirmado
    end
```

**Pasos**:
1. Comercio escanea QR → Obtiene landing page
2. Comercio llama `/validate` con código + token JWT
3. API verifica código válido, no expirado, no usado
4. Retorna info de la oferta (preview)
5. Comercio confirma redención llamando `/confirm`
6. API marca como confirmado con row-level locking
7. Registra en audit log
8. Envía notificación push al usuario

---

### Flujo 3: Expiración Automática

```sql
-- Job programado (cron) que corre cada hora
UPDATE user_redemptions
SET redemption_status = 'expired'
WHERE redemption_status = 'pending'
  AND code_expires_at < NOW()
  AND is_used = false;

-- NO devuelve Lümis (ya fueron gastados al crear)
-- Usuario puede cancelar manualmente antes de expirar
```

---

## 🔌 API Endpoints - Usuarios

### 1. Listar Ofertas Disponibles

**Endpoint**: `GET /api/v1/rewards/offers`

**Autenticación**: JWT Required (usuario autenticado)

**Query Parameters**:
```
?category=food          # Filtrar por categoría
?min_cost=50           # Mínimo de Lümis
&max_cost=200          # Máximo de Lümis
&sort=cost_asc         # Ordenar: cost_asc, cost_desc, newest
&limit=20              # Límite de resultados
&offset=0              # Paginación
```

**Response 200 OK**:
```json
{
  "offers": [
    {
      "offer_id": "550e8400-e29b-41d4-a716-446655440000",
      "name_friendly": "Café Americano",
      "description_friendly": "Disfruta de un delicioso café americano en Starbucks",
      "lumis_cost": 55,
      "category": "food",
      "merchant_name": "Starbucks Panamá",
      "image_url": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg",
      "valid_until": "2026-12-31T23:59:59Z",
      "stock_available": true,
      "user_can_redeem": true,
      "user_redemptions_left": 3
    },
    {
      "offer_id": "660e8400-e29b-41d4-a716-446655440001",
      "name_friendly": "Entrada a Cine 2D",
      "description_friendly": "1 entrada para película 2D",
      "lumis_cost": 180,
      "category": "entertainment",
      "merchant_name": "Cinépolis",
      "image_url": "https://cdn.lumis.pa/offers/cine.jpg",
      "valid_until": "2026-06-30T23:59:59Z",
      "stock_available": true,
      "user_can_redeem": true,
      "user_redemptions_left": 5
    }
  ],
  "total": 12,
  "user_balance": 450
}
```

**Errors**:
- `401 Unauthorized`: JWT inválido o expirado
- `500 Internal Server Error`: Error de servidor

---

### 2. Detalle de Oferta

**Endpoint**: `GET /api/v1/rewards/offers/:offer_id`

**Autenticación**: JWT Required

**Response 200 OK**:
```json
{
  "offer_id": "550e8400-e29b-41d4-a716-446655440000",
  "name_friendly": "Café Americano",
  "description_friendly": "Disfruta de un delicioso café americano en cualquier sucursal de Starbucks en Panamá",
  "lumis_cost": 55,
  "category": "food",
  "merchant_name": "Starbucks Panamá",
  "image_url": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg",
  "valid_from": "2025-01-01T00:00:00Z",
  "valid_until": "2026-12-31T23:59:59Z",
  "terms_and_conditions": "Válido en todas las sucursales. No acumulable con otras promociones.",
  "stock_quantity": null,
  "max_redemptions_per_user": 5,
  "user_can_redeem": true,
  "user_redemptions_count": 2,
  "user_redemptions_left": 3,
  "user_balance": 450,
  "user_has_enough_balance": true
}
```

**Errors**:
- `404 Not Found`: Oferta no existe
- `410 Gone`: Oferta expirada o inactiva

---

### 3. Redimir Oferta

**Endpoint**: `POST /api/v1/rewards/redeem`

**Autenticación**: JWT Required

**Request Body**:
```json
{
  "offer_id": "550e8400-e29b-41d4-a716-446655440000",
  "redemption_method": "qr_code"
}
```

**Response 201 Created**:
```json
{
  "redemption_id": "770e8400-e29b-41d4-a716-446655440002",
  "offer_name": "Café Americano",
  "lumis_spent": 55,
  "new_balance": 395,
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "qr_image_url": "https://cdn.lumis.pa/qr/770e8400.png",
  "qr_landing_url": "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1",
  "expires_at": "2025-10-24T15:45:00Z",
  "status": "pending",
  "merchant_name": "Starbucks Panamá",
  "message": "¡Redención creada! Presenta este código en el comercio."
}
```

**Errors**:
- `400 Bad Request`: 
  ```json
  {
    "error": "insufficient_balance",
    "message": "Saldo insuficiente. Tienes 30 Lümis, necesitas 55.",
    "current_balance": 30,
    "required": 55
  }
  ```
- `404 Not Found`: Oferta no existe
- `409 Conflict`: 
  ```json
  {
    "error": "max_redemptions_reached",
    "message": "Ya alcanzaste el límite de redenciones para esta oferta",
    "max_allowed": 5,
    "current_count": 5
  }
  ```
- `410 Gone`: Oferta expirada o sin stock

---

### 4. Mis Redenciones

**Endpoint**: `GET /api/v1/rewards/my-redemptions`

**Autenticación**: JWT Required

**Query Parameters**:
```
?status=pending        # Filtrar: pending, confirmed, cancelled, expired
&limit=20
&offset=0
```

**Response 200 OK**:
```json
{
  "redemptions": [
    {
      "redemption_id": "770e8400-e29b-41d4-a716-446655440002",
      "offer_name": "Café Americano",
      "merchant_name": "Starbucks Panamá",
      "lumis_spent": 55,
      "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
      "status": "pending",
      "created_at": "2025-10-17T15:45:00Z",
      "expires_at": "2025-10-24T15:45:00Z",
      "qr_image_url": "https://cdn.lumis.pa/qr/770e8400.png",
      "qr_landing_url": "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1",
      "can_cancel": true
    },
    {
      "redemption_id": "660e8400-e29b-41d4-a716-446655440001",
      "offer_name": "Entrada a Cine 2D",
      "merchant_name": "Cinépolis",
      "lumis_spent": 180,
      "redemption_code": "LUMS-B8G3-2L4Y-C6Z2",
      "status": "confirmed",
      "created_at": "2025-10-15T10:30:00Z",
      "validated_at": "2025-10-15T18:45:00Z",
      "can_cancel": false
    }
  ],
  "total": 8,
  "stats": {
    "pending": 1,
    "confirmed": 5,
    "cancelled": 1,
    "expired": 1,
    "total_lumis_spent": 680
  }
}
```

---

### 5. Detalle de Redención

**Endpoint**: `GET /api/v1/rewards/redemptions/:redemption_id`

**Autenticación**: JWT Required

**Response 200 OK**:
```json
{
  "redemption_id": "770e8400-e29b-41d4-a716-446655440002",
  "offer": {
    "name": "Café Americano",
    "description": "Disfruta de un delicioso café americano",
    "merchant_name": "Starbucks Panamá",
    "image_url": "https://cdn.lumis.pa/offers/starbucks-cafe.jpg"
  },
  "lumis_spent": 55,
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "redemption_method": "qr_code",
  "status": "pending",
  "qr_image_url": "https://cdn.lumis.pa/qr/770e8400.png",
  "qr_landing_url": "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1",
  "created_at": "2025-10-17T15:45:00Z",
  "expires_at": "2025-10-24T15:45:00Z",
  "validated_at": null,
  "can_cancel": true,
  "timeline": [
    {
      "action": "created",
      "timestamp": "2025-10-17T15:45:00Z",
      "description": "Redención creada"
    }
  ]
}
```

**Response para redención confirmada**:
```json
{
  "redemption_id": "660e8400-e29b-41d4-a716-446655440001",
  "offer": {...},
  "status": "confirmed",
  "validated_at": "2025-10-15T18:45:00Z",
  "validated_by_merchant": "Starbucks Centro Comercial",
  "timeline": [
    {
      "action": "created",
      "timestamp": "2025-10-15T10:30:00Z",
      "description": "Redención creada"
    },
    {
      "action": "confirmed",
      "timestamp": "2025-10-15T18:45:00Z",
      "description": "Redención confirmada por comercio"
    }
  ]
}
```

---

### 6. Cancelar Redención

**Endpoint**: `POST /api/v1/rewards/redemptions/:redemption_id/cancel`

**Autenticación**: JWT Required

**Request Body** (opcional):
```json
{
  "reason": "Ya no lo necesito"
}
```

**Response 200 OK**:
```json
{
  "success": true,
  "redemption_id": "770e8400-e29b-41d4-a716-446655440002",
  "lumis_refunded": 55,
  "new_balance": 450,
  "message": "Redención cancelada. Tus Lümis han sido devueltos."
}
```

**Errors**:
- `400 Bad Request`: No se puede cancelar (ya confirmada/expirada)
  ```json
  {
    "error": "cannot_cancel",
    "message": "No puedes cancelar una redención ya confirmada",
    "status": "confirmed"
  }
  ```
- `404 Not Found`: Redención no existe o no pertenece al usuario

---

### 7. Regenerar QR (Nuevo Token)

**Endpoint**: `GET /api/v1/rewards/redemptions/:redemption_id/qr/refresh`

**Autenticación**: JWT Required

**Descripción**: Genera un nuevo token JWT de validación (1 min) sin cambiar el redemption_code.

**Response 200 OK**:
```json
{
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "new_validation_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "qr_landing_url": "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1?t=eyJ0eXAi...",
  "qr_image_url": "https://cdn.lumis.pa/qr/770e8400-refresh.png",
  "token_expires_at": "2025-10-17T16:31:00Z"
}
```

---

### 8. Landing Page Pública (QR)

**Endpoint**: `GET /r/:redemption_code`

**Autenticación**: None (público)

**Query Parameters**:
```
?t=eyJ0eXAi...    # Token JWT de validación (opcional)
```

**Response**: HTML Page

```html
<!DOCTYPE html>
<html>
<head>
    <title>🎁 Tu redención de Lümis</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {
            font-family: 'Segoe UI', sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            text-align: center;
            padding: 50px 20px;
        }
        .container {
            max-width: 500px;
            margin: 0 auto;
            background: rgba(255,255,255,0.1);
            padding: 40px;
            border-radius: 20px;
            backdrop-filter: blur(10px);
        }
        .icon { font-size: 80px; margin-bottom: 20px; }
        .code {
            font-family: 'Courier New', monospace;
            font-size: 24px;
            background: rgba(0,0,0,0.3);
            padding: 15px;
            border-radius: 10px;
            margin: 30px 0;
            letter-spacing: 2px;
        }
        .status {
            display: inline-block;
            padding: 10px 30px;
            border-radius: 25px;
            font-weight: bold;
            margin: 20px 0;
        }
        .pending { background: #ffd700; color: #000; }
        .confirmed { background: #00ff00; color: #000; }
        .expired { background: #ff4444; }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">☕</div>
        <h1>Café Americano</h1>
        <p>Starbucks Panamá</p>
        
        <div class="code">LUMS-A7F2-9K3X-B5Y1</div>
        
        <div class="status pending">
            ✅ Listo para redimir
        </div>
        
        <p style="font-size: 14px; margin-top: 30px;">
            ⏰ Válido hasta: 24 Oct 2025<br>
            🏪 Presenta este código en el comercio
        </p>
        
        <!-- Auto-refresh si está pending -->
        <script>
            const status = 'pending';
            if (status === 'pending') {
                setTimeout(() => location.reload(), 5000);
            }
        </script>
    </div>
</body>
</html>
```

---

## 🏪 API Endpoints - Comercios

### 1. Autenticación de Comercio

**Endpoint**: `POST /api/v1/merchant/auth/login`

**Autenticación**: API Key

**Request Body**:
```json
{
  "api_key": "mk_live_1234567890abcdef",
  "api_secret": "sk_live_fedcba0987654321"
}
```

**Response 200 OK**:
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "merchant": {
    "merchant_id": "880e8400-e29b-41d4-a716-446655440003",
    "merchant_name": "Starbucks Centro Comercial",
    "merchant_type": "restaurant"
  }
}
```

**Errors**:
- `401 Unauthorized`: API key inválido
- `403 Forbidden`: Comercio inactivo

---

### 2. Validar Código (Preview)

**Endpoint**: `POST /api/v1/merchant/validate`

**Autenticación**: Bearer Token (Merchant JWT)

**Request Body**:
```json
{
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "validation_token": "eyJ0eXAiOiJKV1QiLCJhbGc..."
}
```

**Response 200 OK**:
```json
{
  "valid": true,
  "redemption": {
    "redemption_id": "770e8400-e29b-41d4-a716-446655440002",
    "offer_name": "Café Americano",
    "user_name": "Juan P.",
    "lumis_spent": 55,
    "created_at": "2025-10-17T15:45:00Z",
    "expires_at": "2025-10-24T15:45:00Z"
  },
  "can_confirm": true,
  "warnings": []
}
```

**Response 400 Bad Request** (código inválido):
```json
{
  "valid": false,
  "error": "invalid_code",
  "message": "Código de redención no encontrado o ya fue usado",
  "can_confirm": false
}
```

**Response 400 Bad Request** (expirado):
```json
{
  "valid": false,
  "error": "expired",
  "message": "Este código expiró el 2025-10-20",
  "expired_at": "2025-10-20T15:45:00Z",
  "can_confirm": false
}
```

**Response 400 Bad Request** (token inválido):
```json
{
  "valid": false,
  "error": "invalid_token",
  "message": "Token de validación expirado o inválido. Solicita regenerar QR.",
  "can_confirm": false
}
```

---

### 3. Confirmar Redención

**Endpoint**: `POST /api/v1/merchant/confirm`

**Autenticación**: Bearer Token (Merchant JWT)

**Request Body**:
```json
{
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "notes": "Redimido en sucursal Centro Comercial, mesa 5"
}
```

**Response 200 OK**:
```json
{
  "success": true,
  "redemption_id": "770e8400-e29b-41d4-a716-446655440002",
  "confirmed_at": "2025-10-17T16:30:00Z",
  "offer_name": "Café Americano",
  "user_name": "Juan P.",
  "lumis_redeemed": 55,
  "message": "Redención confirmada exitosamente"
}
```

**Errors**:
- `404 Not Found`: Código no existe
- `409 Conflict`: 
  ```json
  {
    "error": "already_confirmed",
    "message": "Este código ya fue confirmado anteriormente",
    "confirmed_at": "2025-10-17T16:25:00Z",
    "confirmed_by": "Starbucks Albrook"
  }
  ```
- `410 Gone`: Código expirado o cancelado

---

### 4. Historial de Redenciones

**Endpoint**: `GET /api/v1/merchant/redemptions`

**Autenticación**: Bearer Token (Merchant JWT)

**Query Parameters**:
```
?status=confirmed      # Filtrar por status
&date_from=2025-10-01  # Desde fecha
&date_to=2025-10-31    # Hasta fecha
&limit=50
&offset=0
```

**Response 200 OK**:
```json
{
  "redemptions": [
    {
      "redemption_id": "770e8400-e29b-41d4-a716-446655440002",
      "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
      "offer_name": "Café Americano",
      "user_name": "Juan P.",
      "lumis_redeemed": 55,
      "status": "confirmed",
      "created_at": "2025-10-17T15:45:00Z",
      "confirmed_at": "2025-10-17T16:30:00Z",
      "notes": "Redimido en sucursal Centro"
    }
  ],
  "total": 156,
  "stats": {
    "today": 12,
    "this_week": 89,
    "this_month": 156,
    "total_lumis_redeemed": 8580
  }
}
```

---

### 5. Dashboard de Comercio

**Endpoint**: `GET /api/v1/merchant/dashboard`

**Autenticación**: Bearer Token (Merchant JWT)

**Response 200 OK**:
```json
{
  "merchant": {
    "merchant_id": "880e8400-e29b-41d4-a716-446655440003",
    "merchant_name": "Starbucks Centro Comercial",
    "merchant_type": "restaurant"
  },
  "stats": {
    "today": {
      "redemptions": 12,
      "lumis_redeemed": 660,
      "revenue_equivalent": 66.0
    },
    "this_week": {
      "redemptions": 89,
      "lumis_redeemed": 4895,
      "revenue_equivalent": 489.5
    },
    "this_month": {
      "redemptions": 156,
      "lumis_redeemed": 8580,
      "revenue_equivalent": 858.0
    },
    "all_time": {
      "total_redemptions": 2340,
      "total_lumis_redeemed": 128700,
      "revenue_equivalent": 12870.0
    }
  },
  "top_offers": [
    {
      "offer_name": "Café Americano",
      "redemptions_count": 89,
      "percentage": 57.1
    },
    {
      "offer_name": "Cappuccino",
      "redemptions_count": 45,
      "percentage": 28.8
    }
  ],
  "recent_redemptions": [
    {
      "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
      "offer_name": "Café Americano",
      "confirmed_at": "2025-10-17T16:30:00Z"
    }
  ]
}
```

---

## 🔐 Sistema de QR Codes

### Generación de QR

**Librería**: `qrcode` + `image` (Rust)

**Proceso**:
1. Generar código único: `LUMS-XXXX-XXXX-XXXX`
2. Crear token JWT de validación (expira en 1 min)
3. Construir URL: `https://lumis.pa/r/{code}?t={token}`
4. Generar QR base (800x800px)
5. Cargar logo (`assets/logoqr.png`)
6. Redimensionar logo a 15% del QR
7. Overlay logo en centro
8. Subir a storage (S3 o local)
9. Guardar URL en `user_redemptions.qr_image_url`

**Código Rust**:
```rust
use qrcode::QrCode;
use image::{DynamicImage, ImageBuffer, Rgba, imageops::overlay};

async fn generate_redemption_qr(
    redemption_code: &str,
    validation_token: &str,
) -> Result<Vec<u8>> {
    
    // 1. URL del QR
    let qr_url = format!(
        "https://lumis.pa/r/{}?t={}",
        redemption_code,
        validation_token
    );
    
    // 2. Generar QR base
    let qr = QrCode::new(qr_url.as_bytes())?;
    let qr_image = qr.render::<Rgba<u8>>()
        .max_dimensions(800, 800)
        .build();
    
    // 3. Cargar y redimensionar logo
    let logo = image::open("assets/logoqr.png")?;
    let logo_size = (qr_image.width() * 15) / 100; // 15%
    let logo_resized = logo.resize(
        logo_size, 
        logo_size, 
        image::imageops::FilterType::Lanczos3
    );
    
    // 4. Overlay logo en centro
    let mut final_image = qr_image.clone();
    let x_offset = (qr_image.width() - logo_size) / 2;
    let y_offset = (qr_image.height() - logo_size) / 2;
    overlay(&mut final_image, &logo_resized.to_rgba8(), x_offset, y_offset);
    
    // 5. Convertir a bytes
    let mut buffer = Vec::new();
    final_image.write_to(&mut buffer, image::ImageOutputFormat::Png)?;
    
    Ok(buffer)
}
```

### Estructura del Código QR

**Formato del redemption_code**:
```
LUMS-{HEX1}-{HEX2}-{HEX3}

Donde:
- HEX1 = Timestamp truncado (4 hex chars)
- HEX2 = Timestamp shifted (4 hex chars)
- HEX3 = Random (4 hex chars)

Ejemplo: LUMS-A7F2-9K3X-B5Y1
```

**Token JWT de Validación**:
```json
{
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "user_id": 123,
  "exp": 1729180860,  // Expira en 60 segundos
  "jti": "unique-jwt-id"  // Previene replay
}
```

### Seguridad del QR

**Capas de Protección**:

1. **Token de corta duración** (1 min)
   - Fuerza regeneración frecuente
   - Previene screenshots antiguos

2. **Código único por redención**
   - redemption_code es UUID-based
   - No reutilizable

3. **Constraint de base de datos**
   ```sql
   CREATE UNIQUE INDEX idx_unique_active_redemption 
   ON user_redemptions(redemption_code) 
   WHERE redemption_status = 'pending' AND is_used = false;
   ```

4. **Row-level locking en confirmación**
   ```sql
   SELECT FOR UPDATE NOWAIT
   WHERE redemption_code = $1 
     AND redemption_status = 'pending';
   ```

5. **Audit logging completo**
   - Cada validación registrada
   - IP, user-agent, timestamps
   - Detección de patrones de fraude

---

## 🔒 Seguridad

### Autenticación

**Usuarios**:
- JWT tokens (mismo sistema existente)
- Expira en 24 horas
- Claims: `user_id`, `role: "user"`

**Comercios**:
- API Key + Secret (bcrypt hashed)
- JWT token (expira en 1 hora)
- Claims: `merchant_id`, `role: "merchant"`

### Autorización

**Endpoints de Usuario**:
```rust
// Middleware de autenticación
async fn require_user_auth(req: Request, next: Next) -> Response {
    let token = extract_jwt_from_header(&req);
    let claims = validate_jwt(token)?;
    
    if claims.role != "user" {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    req.extensions_mut().insert(claims);
    next.run(req).await
}
```

**Endpoints de Comercio**:
```rust
async fn require_merchant_auth(req: Request, next: Next) -> Response {
    let token = extract_jwt_from_header(&req);
    let claims = validate_jwt(token)?;
    
    if claims.role != "merchant" {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    // Verificar que comercio está activo
    let merchant = db.get_merchant(claims.merchant_id).await?;
    if !merchant.is_active {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    req.extensions_mut().insert(merchant);
    next.run(req).await
}
```

### Rate Limiting

**Por Usuario**:
```rust
#[rate_limit(
    key = "user_redemptions:{user_id}",
    limit = 5,
    period = "1h"
)]
async fn create_redemption(...) { }
```

**Por Comercio**:
```rust
#[rate_limit(
    key = "merchant_validations:{merchant_id}",
    limit = 100,
    period = "1m"
)]
async fn validate_redemption(...) { }
```

**Por IP (DDoS protection)**:
```rust
#[rate_limit(
    key = "ip:{ip_address}",
    limit = 1000,
    period = "1h"
)]
async fn public_endpoints(...) { }
```

### Prevención de Fraude

**Race Condition Protection**:
```sql
BEGIN;

-- Bloqueo pesimista
SELECT * FROM user_redemptions 
WHERE redemption_code = $1 
  AND redemption_status = 'pending'
FOR UPDATE NOWAIT;

-- Si llega aquí, nadie más está procesando
UPDATE user_redemptions 
SET redemption_status = 'confirmed',
    is_used = true
WHERE redemption_code = $1;

COMMIT;
```

**Detección de Patrones Sospechosos**:
```rust
// Alertar si:
// - Mismo IP intenta validar >10 códigos en 1 min
// - Mismo usuario redime >5 ofertas en 10 min
// - Token JWT usado múltiples veces (replay attack)

async fn detect_fraud_patterns(redemption: &Redemption) {
    let recent_attempts = db.count_validation_attempts(
        ip_address,
        Duration::minutes(1)
    ).await?;
    
    if recent_attempts > 10 {
        alert_security_team(
            "Posible ataque de validación masiva",
            ip_address
        ).await;
        
        return Err(RedemptionError::RateLimitExceeded);
    }
}
```

---

## 💡 Ejemplos de Uso

### Ejemplo 1: Usuario Redime Café

```bash
# 1. Listar ofertas disponibles
curl -X GET "https://api.lumis.pa/api/v1/rewards/offers?category=food" \
  -H "Authorization: Bearer eyJ0eXAi..."

# Response:
{
  "offers": [
    {
      "offer_id": "550e8400-...",
      "name_friendly": "Café Americano",
      "lumis_cost": 55,
      "merchant_name": "Starbucks"
    }
  ],
  "user_balance": 450
}

# 2. Redimir oferta
curl -X POST "https://api.lumis.pa/api/v1/rewards/redeem" \
  -H "Authorization: Bearer eyJ0eXAi..." \
  -H "Content-Type: application/json" \
  -d '{
    "offer_id": "550e8400-e29b-41d4-a716-446655440000",
    "redemption_method": "qr_code"
  }'

# Response:
{
  "redemption_id": "770e8400-...",
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "qr_image_url": "https://cdn.lumis.pa/qr/770e8400.png",
  "qr_landing_url": "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1",
  "new_balance": 395
}

# 3. Usuario muestra QR en Starbucks
```

### Ejemplo 2: Comercio Valida y Confirma

```bash
# 1. Autenticación del comercio
curl -X POST "https://api.lumis.pa/api/v1/merchant/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "mk_live_1234567890abcdef",
    "api_secret": "sk_live_fedcba0987654321"
  }'

# Response:
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "expires_in": 3600
}

# 2. Validar código (preview)
curl -X POST "https://api.lumis.pa/api/v1/merchant/validate" \
  -H "Authorization: Bearer eyJ0eXAi..." \
  -H "Content-Type: application/json" \
  -d '{
    "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
    "validation_token": "eyJ0eXAi..."
  }'

# Response:
{
  "valid": true,
  "offer_name": "Café Americano",
  "user_name": "Juan P.",
  "lumis_spent": 55,
  "can_confirm": true
}

# 3. Confirmar redención
curl -X POST "https://api.lumis.pa/api/v1/merchant/confirm" \
  -H "Authorization: Bearer eyJ0eXAi..." \
  -H "Content-Type: application/json" \
  -d '{
    "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
    "notes": "Redimido en sucursal Centro"
  }'

# Response:
{
  "success": true,
  "confirmed_at": "2025-10-17T16:30:00Z",
  "message": "Redención confirmada"
}
```

### Ejemplo 3: Regenerar QR (Token Expirado)

```bash
# Usuario intenta mostrar QR después de 5 minutos
# El comercio ve: "Token de validación expirado"

# Usuario regenera QR:
curl -X GET "https://api.lumis.pa/api/v1/rewards/redemptions/770e8400-.../qr/refresh" \
  -H "Authorization: Bearer eyJ0eXAi..."

# Response:
{
  "redemption_code": "LUMS-A7F2-9K3X-B5Y1",
  "new_validation_token": "eyJ0eXAi...[NEW]",
  "qr_landing_url": "https://lumis.pa/r/LUMS-A7F2-9K3X-B5Y1?t=eyJ0eXAi...[NEW]",
  "token_expires_at": "2025-10-17T16:36:00Z"
}

# QR actualizado, ahora válido por 1 minuto más
```

---

## 📊 Métricas Prometheus

### Métricas Adicionales para Redenciones

```rust
// Total de redenciones por status
redemptions_total{status="pending|confirmed|cancelled|expired"}

// Latencia de validación
redemption_validation_duration_seconds

// Tasa de confirmación
redemption_confirmation_success_rate

// Stock de ofertas
offer_stock_remaining{offer_id}

// Redenciones por comercio
redemptions_by_merchant{merchant_id}

// Intentos de fraude
redemption_fraud_attempts_total{reason}
```

---

## 🚀 Deployment

### Variables de Entorno

```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost/rewards

# Storage
S3_BUCKET=lumis-qr-codes
S3_REGION=us-east-1
S3_ACCESS_KEY=AKIA...
S3_SECRET_KEY=...

# JWT
JWT_SECRET=your-secret-key-here
JWT_EXPIRATION=86400  # 24 horas

# QR Settings
QR_CODE_EXPIRATION_DAYS=7
QR_TOKEN_EXPIRATION_SECONDS=60
QR_LOGO_PATH=assets/logoqr.png

# Merchant
MERCHANT_JWT_EXPIRATION=3600  # 1 hora

# URLs
API_BASE_URL=https://api.lumis.pa
QR_LANDING_BASE_URL=https://lumis.pa
CDN_BASE_URL=https://cdn.lumis.pa
```

---

## 📝 Notas de Implementación

### Tareas Pendientes

- [ ] Implementar job de expiración automática (cron cada hora)
- [ ] Webhook notifications a comercios cuando hay redención
- [ ] Panel de admin para gestionar ofertas
- [ ] Sistema de reportes para comercios (PDF/Excel)
- [ ] Notificaciones push al usuario cuando se confirma
- [ ] Soporte para múltiples métodos (barcode, NFC)
- [ ] A/B testing de ofertas
- [ ] Sistema de recomendaciones personalizadas

### Consideraciones de Escalabilidad

- Cache de ofertas activas (Redis, TTL 5 min)
- CDN para imágenes de QR (CloudFront/CloudFlare)
- Read replicas para queries de dashboard
- Queue para procesamiento de QR (SQS/RabbitMQ) si volumen alto

---

**Última actualización**: 17 de Octubre 2025  
**Versión API**: v1  
**Mantenido por**: Equipo de Desarrollo Lümis
