# 03 - Modelo de Datos

## Esquema de Base de Datos

Schema: `rewards`

### Tablas Principales

#### 1. `redemption_offers`
Catálogo de productos/servicios disponibles para redención.

```sql
CREATE TABLE rewards.redemption_offers (
    offer_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id UUID REFERENCES rewards.merchants(merchant_id),
    name TEXT NOT NULL,
    name_friendly TEXT,
    description TEXT,
    lumis_cost INTEGER NOT NULL,
    points INTEGER NOT NULL,
    is_active BOOLEAN DEFAULT true,
    stock_quantity INTEGER,
    terms_and_conditions TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

#### 2. `user_redemptions`
Instancias de redenciones creadas por usuarios.

```sql
CREATE TABLE rewards.user_redemptions (
    redemption_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INTEGER NOT NULL,
    offer_id UUID REFERENCES rewards.redemption_offers(offer_id),
    merchant_id UUID REFERENCES rewards.merchants(merchant_id),
    lumis_spent INTEGER NOT NULL,
    redemption_code TEXT UNIQUE NOT NULL,
    redemption_status TEXT DEFAULT 'pending',
    code_expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    validated_at TIMESTAMP WITH TIME ZONE,
    expiration_alert_sent BOOLEAN DEFAULT false
);
```

#### 3. `fact_accumulations`
Registro de todas las transacciones de Lümis.

```sql
CREATE TABLE rewards.fact_accumulations (
    acum_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    accum_type TEXT CHECK (accum_type IN ('earn', 'spend')),
    dtype TEXT DEFAULT 'points',
    quantity INTEGER NOT NULL,
    balance INTEGER,
    date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    redemption_id UUID REFERENCES rewards.user_redemptions(redemption_id)
);
```

#### 4. `fact_balance_points`
Balance actual de Lümis por usuario.

```sql
CREATE TABLE rewards.fact_balance_points (
    user_id INTEGER PRIMARY KEY,
    balance INTEGER DEFAULT 0,
    latest_update TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

#### 5. `merchants`
Información de comercios aliados.

```sql
CREATE TABLE rewards.merchants (
    merchant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_name TEXT NOT NULL,
    api_key_hash TEXT NOT NULL,
    webhook_url TEXT,
    webhook_secret TEXT,
    webhook_events TEXT[],
    webhook_enabled BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

## Diagrama ER

```
┌─────────────────────┐
│   merchants         │
│                     │
│  merchant_id (PK)   │
│  merchant_name      │
│  api_key_hash       │
│  webhook_url        │
└──────┬──────────────┘
       │
       │ 1:N
       ↓
┌─────────────────────┐
│ redemption_offers   │
│                     │
│  offer_id (PK)      │
│  merchant_id (FK)   │
│  name               │
│  lumis_cost         │
│  is_active          │
└──────┬──────────────┘
       │
       │ 1:N
       ↓
┌─────────────────────┐       ┌──────────────────────┐
│ user_redemptions    │ 1:N   │  fact_accumulations  │
│                     │◄──────┤                      │
│  redemption_id (PK) │       │  acum_id (PK)        │
│  user_id            │       │  user_id             │
│  offer_id (FK)      │       │  redemption_id (FK)  │
│  redemption_code    │       │  quantity            │
│  redemption_status  │       │  accum_type          │
└─────────────────────┘       └──────────────────────┘
         │
         │ Updates via trigger
         ↓
┌─────────────────────┐
│ fact_balance_points │
│                     │
│  user_id (PK)       │
│  balance            │
└─────────────────────┘
```

## Triggers Importantes

### 1. `fun_update_balance_points()`
Recalcula balance cuando cambia `fact_accumulations`.

### 2. `update_balance_on_redemption()`  
Actualiza balance cuando se crea redención.

### 3. `update_merchant_stats()`
Actualiza estadísticas del merchant.

**Siguiente**: [04-api-usuarios.md](./04-api-usuarios.md)
