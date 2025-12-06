-- ======================================================================
-- MIGRACIÓN: MOVER TABLAS AL SCHEMA REWARDS
-- Fecha: 2025-10-18
-- Descripción: Mueve/crea tablas de redención en el schema rewards
--              para que coincida con las queries del código Rust
-- ======================================================================

BEGIN;

-- ======================================================================
-- 1. ASEGURAR QUE EL SCHEMA REWARDS EXISTE
-- ======================================================================

CREATE SCHEMA IF NOT EXISTS rewards;

SET search_path = rewards, public;

-- ======================================================================
-- 2. MOVER/CREAR TABLA MERCHANTS EN REWARDS
-- ======================================================================

-- Si existe en public, moverla a rewards
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'public' AND tablename = 'merchants') THEN
        ALTER TABLE public.merchants SET SCHEMA rewards;
        RAISE NOTICE 'Tabla merchants movida de public a rewards';
    END IF;
END $$;

-- Crear tabla merchants en rewards si no existe
CREATE TABLE IF NOT EXISTS rewards.merchants (
    merchant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_name VARCHAR(255) NOT NULL UNIQUE,
    merchant_type VARCHAR(50),
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),
    api_key_hash VARCHAR(255) NOT NULL,
    webhook_url TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    total_redemptions INTEGER DEFAULT 0,
    total_lumis_redeemed BIGINT DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_merchants_active 
ON rewards.merchants(is_active) 
WHERE is_active = true;

-- ======================================================================
-- 3. CREAR/ADAPTAR REDEMPTION_OFFERS EN REWARDS
-- ======================================================================

-- Verificar si dim_redemptions existe en public
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'public' AND tablename = 'dim_redemptions') THEN
        -- Renombrar y mover a rewards
        ALTER TABLE public.dim_redemptions RENAME TO redemption_offers;
        ALTER TABLE public.redemption_offers SET SCHEMA rewards;
        RAISE NOTICE 'Tabla dim_redemptions renombrada y movida a rewards.redemption_offers';
    ELSIF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'public' AND tablename = 'redemption_offers') THEN
        -- Solo mover a rewards
        ALTER TABLE public.redemption_offers SET SCHEMA rewards;
        RAISE NOTICE 'Tabla redemption_offers movida a rewards';
    END IF;
END $$;

-- Crear tabla si no existe
CREATE TABLE IF NOT EXISTS rewards.redemption_offers (
    id SERIAL PRIMARY KEY,
    offer_id UUID DEFAULT gen_random_uuid() UNIQUE NOT NULL,
    name VARCHAR(255),
    name_friendly VARCHAR(255) NOT NULL,
    description TEXT,
    description_friendly TEXT,
    points INTEGER,
    lumis_cost INTEGER,
    offer_category VARCHAR(50),
    merchant_id UUID REFERENCES rewards.merchants(merchant_id),
    merchant_name VARCHAR(255),
    stock_quantity INTEGER,
    max_redemptions_per_user INTEGER DEFAULT 5,
    is_active BOOLEAN DEFAULT true,
    valid_from TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    valid_to TIMESTAMP WITH TIME ZONE,
    img TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Agregar columnas faltantes si la tabla ya existía
ALTER TABLE rewards.redemption_offers 
ADD COLUMN IF NOT EXISTS offer_id UUID DEFAULT gen_random_uuid() UNIQUE,
ADD COLUMN IF NOT EXISTS offer_category VARCHAR(50),
ADD COLUMN IF NOT EXISTS merchant_id UUID,
ADD COLUMN IF NOT EXISTS merchant_name VARCHAR(255),
ADD COLUMN IF NOT EXISTS stock_quantity INTEGER,
ADD COLUMN IF NOT EXISTS max_redemptions_per_user INTEGER DEFAULT 5,
ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true,
ADD COLUMN IF NOT EXISTS lumis_cost INTEGER;

-- Migrar points a lumis_cost si es necesario
UPDATE rewards.redemption_offers 
SET lumis_cost = points 
WHERE lumis_cost IS NULL AND points IS NOT NULL;

-- Índices
CREATE INDEX IF NOT EXISTS idx_offers_active_valid 
ON rewards.redemption_offers(is_active, valid_to) 
WHERE is_active = true AND valid_to > NOW();

CREATE INDEX IF NOT EXISTS idx_offers_category 
ON rewards.redemption_offers(offer_category);

CREATE INDEX IF NOT EXISTS idx_offers_cost 
ON rewards.redemption_offers(lumis_cost);

CREATE INDEX IF NOT EXISTS idx_offers_merchant 
ON rewards.redemption_offers(merchant_id) 
WHERE merchant_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_offers_offer_id
ON rewards.redemption_offers(offer_id);

-- ======================================================================
-- 4. CREAR USER_REDEMPTIONS EN REWARDS
-- ======================================================================

CREATE TABLE IF NOT EXISTS rewards.user_redemptions (
    redemption_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INTEGER NOT NULL,
    offer_id UUID NOT NULL REFERENCES rewards.redemption_offers(offer_id),
    lumis_spent INTEGER NOT NULL,
    redemption_method VARCHAR(50) NOT NULL DEFAULT 'qr_code',
    redemption_status VARCHAR(50) NOT NULL DEFAULT 'pending',
    redemption_code VARCHAR(100) UNIQUE NOT NULL,
    qr_image_url TEXT,
    qr_landing_url TEXT,
    validated_by_merchant_id UUID REFERENCES rewards.merchants(merchant_id),
    validated_at TIMESTAMP WITH TIME ZONE,
    validation_ip_address INET,
    code_expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    cancelled_at TIMESTAMP WITH TIME ZONE,
    cancellation_reason TEXT,
    validation_token_hash VARCHAR(255),
    is_used BOOLEAN DEFAULT false,
    metadata JSONB,
    CONSTRAINT valid_status CHECK (
        redemption_status IN ('pending', 'confirmed', 'cancelled', 'expired')
    ),
    CONSTRAINT valid_method CHECK (
        redemption_method IN ('qr_code', 'barcode', 'nfc', 'manual')
    )
);

-- Índices para user_redemptions
CREATE INDEX IF NOT EXISTS idx_redemptions_user 
ON rewards.user_redemptions(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_redemptions_code 
ON rewards.user_redemptions(redemption_code) 
WHERE redemption_status = 'pending';

CREATE INDEX IF NOT EXISTS idx_redemptions_status 
ON rewards.user_redemptions(user_id, redemption_status);

CREATE INDEX IF NOT EXISTS idx_redemptions_offer
ON rewards.user_redemptions(offer_id);

CREATE INDEX IF NOT EXISTS idx_redemptions_merchant 
ON rewards.user_redemptions(validated_by_merchant_id, validated_at) 
WHERE validated_by_merchant_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_redemptions_created
ON rewards.user_redemptions(created_at DESC);

-- ======================================================================
-- 5. CREAR REDEMPTION_AUDIT_LOG EN REWARDS
-- ======================================================================

CREATE TABLE IF NOT EXISTS rewards.redemption_audit_log (
    log_id BIGSERIAL PRIMARY KEY,
    redemption_id UUID NOT NULL REFERENCES rewards.user_redemptions(redemption_id),
    action_type VARCHAR(50) NOT NULL,
    performed_by VARCHAR(50),
    merchant_id UUID,
    ip_address INET,
    user_agent TEXT,
    request_id UUID,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_redemption 
ON rewards.redemption_audit_log(redemption_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_action 
ON rewards.redemption_audit_log(action_type, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_merchant 
ON rewards.redemption_audit_log(merchant_id, created_at DESC) 
WHERE merchant_id IS NOT NULL;

-- ======================================================================
-- 6. INSERTAR DATOS DE EJEMPLO (si las tablas están vacías)
-- ======================================================================

-- Insertar ofertas de ejemplo si no hay ninguna
INSERT INTO rewards.redemption_offers (
    id, offer_id, name, name_friendly, description_friendly, 
    lumis_cost, offer_category, merchant_name, 
    valid_from, valid_to, is_active, img
)
SELECT * FROM (VALUES
    (100, '550e8400-e29b-41d4-a716-446655440000'::uuid, 'cafe_americano_starbucks', 'Café Americano', 'Disfruta de un delicioso café americano en cualquier sucursal de Starbucks', 55, 'food', 'Starbucks Panamá', '2025-01-01 00:00:00+00'::timestamptz, '2026-12-31 23:59:59+00'::timestamptz, true, 'https://cdn.lumis.pa/offers/starbucks-cafe.jpg'),
    (101, '660e8400-e29b-41d4-a716-446655440001'::uuid, 'entrada_cine_2d', 'Entrada a Cine 2D', '1 entrada para película 2D en cualquier sala', 180, 'entertainment', 'Cinépolis', '2025-01-01 00:00:00+00'::timestamptz, '2026-06-30 23:59:59+00'::timestamptz, true, 'https://cdn.lumis.pa/offers/cine.jpg'),
    (102, '770e8400-e29b-41d4-a716-446655440002'::uuid, 'libro_bestseller', 'Libro Bestseller', 'Cualquier libro de la sección bestsellers', 120, 'books', 'Librería Argosy', '2025-01-01 00:00:00+00'::timestamptz, '2026-12-31 23:59:59+00'::timestamptz, true, 'https://cdn.lumis.pa/offers/libro.jpg'),
    (103, '880e8400-e29b-41d4-a716-446655440003'::uuid, 'cena_2_personas', 'Cena para 2 Personas', 'Menú especial para 2 personas en restaurante aliado', 450, 'food', 'Restaurantes Aliados', '2025-01-01 00:00:00+00'::timestamptz, '2026-12-31 23:59:59+00'::timestamptz, true, 'https://cdn.lumis.pa/offers/cena.jpg')
) AS v(id, offer_id, name, name_friendly, description_friendly, lumis_cost, offer_category, merchant_name, valid_from, valid_to, is_active, img)
WHERE NOT EXISTS (SELECT 1 FROM rewards.redemption_offers LIMIT 1)
ON CONFLICT (id) DO UPDATE SET
    offer_id = EXCLUDED.offer_id,
    name_friendly = EXCLUDED.name_friendly,
    lumis_cost = EXCLUDED.lumis_cost,
    offer_category = EXCLUDED.offer_category,
    is_active = EXCLUDED.is_active;

-- Insertar comercio de ejemplo si no hay ninguno
INSERT INTO rewards.merchants (
    merchant_id,
    merchant_name,
    merchant_type,
    contact_email,
    api_key_hash,
    is_active
) 
SELECT 
    '990e8400-e29b-41d4-a716-446655440004'::uuid,
    'Starbucks Centro Comercial',
    'restaurant',
    'partner@starbucks.pa',
    '$2a$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewnYNM7rvMVCGNQa',
    true
WHERE NOT EXISTS (SELECT 1 FROM rewards.merchants WHERE merchant_name = 'Starbucks Centro Comercial')
ON CONFLICT (merchant_name) DO NOTHING;

-- ======================================================================
-- 7. PERMISOS
-- ======================================================================

GRANT USAGE ON SCHEMA rewards TO avalencia;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA rewards TO avalencia;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA rewards TO avalencia;

-- ======================================================================
-- FIN DE MIGRACIÓN
-- ======================================================================

COMMIT;

-- Verificación
DO $$
DECLARE
    offers_count INTEGER;
    merchants_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO offers_count FROM rewards.redemption_offers;
    SELECT COUNT(*) INTO merchants_count FROM rewards.merchants;
    
    RAISE NOTICE '';
    RAISE NOTICE '✅ ========================================';
    RAISE NOTICE '✅ MIGRACIÓN COMPLETADA EXITOSAMENTE';
    RAISE NOTICE '✅ ========================================';
    RAISE NOTICE '✅ Schema: rewards';
    RAISE NOTICE '✅ Tabla rewards.redemption_offers: % registros', offers_count;
    RAISE NOTICE '✅ Tabla rewards.user_redemptions: creada';
    RAISE NOTICE '✅ Tabla rewards.redemption_audit_log: creada';
    RAISE NOTICE '✅ Tabla rewards.merchants: % registros', merchants_count;
    RAISE NOTICE '✅ ========================================';
END $$;
