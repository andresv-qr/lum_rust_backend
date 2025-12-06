-- ======================================================================
-- MIGRACIÓN: SISTEMA DE REDENCIÓN DE LÜMIS
-- Fecha: 2025-10-17
-- Descripción: Adapta schema existente para soportar QR codes y
--              validación por comercios
-- ======================================================================

BEGIN;

-- ======================================================================
-- 1. EXTENSIÓN DE TABLAS EXISTENTES
-- ======================================================================

-- 1.1 Extender fact_accumulations para vincular redenciones
ALTER TABLE fact_accumulations 
ADD COLUMN IF NOT EXISTS redemption_id UUID;

CREATE INDEX IF NOT EXISTS idx_fact_accumulations_redemption 
ON fact_accumulations(redemption_id) 
WHERE redemption_id IS NOT NULL;

COMMENT ON COLUMN fact_accumulations.redemption_id IS 
'FK a user_redemptions. Vincula transacciones negativas con redenciones específicas';

-- ======================================================================
-- 2. RENOMBRAR Y ADAPTAR dim_redemptions → redemption_offers
-- ======================================================================

-- 2.1 Renombrar tabla
ALTER TABLE dim_redemptions RENAME TO redemption_offers;

-- 2.2 Agregar columnas nuevas
ALTER TABLE redemption_offers 
ADD COLUMN IF NOT EXISTS offer_id UUID DEFAULT gen_random_uuid() UNIQUE,
ADD COLUMN IF NOT EXISTS offer_category VARCHAR(50),
ADD COLUMN IF NOT EXISTS merchant_id UUID,
ADD COLUMN IF NOT EXISTS merchant_name VARCHAR(255),
ADD COLUMN IF NOT EXISTS stock_quantity INTEGER,
ADD COLUMN IF NOT EXISTS max_redemptions_per_user INTEGER DEFAULT 5,
ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true,
ADD COLUMN IF NOT EXISTS lumis_cost INTEGER;

-- 2.3 Migrar datos legacy (points → lumis_cost)
UPDATE redemption_offers 
SET lumis_cost = points 
WHERE lumis_cost IS NULL;

-- 2.4 Índices optimizados
CREATE INDEX IF NOT EXISTS idx_offers_active_valid 
ON redemption_offers(is_active, valid_to) 
WHERE is_active = true AND valid_to > NOW();

CREATE INDEX IF NOT EXISTS idx_offers_category 
ON redemption_offers(offer_category);

CREATE INDEX IF NOT EXISTS idx_offers_cost 
ON redemption_offers(lumis_cost);

CREATE INDEX IF NOT EXISTS idx_offers_merchant 
ON redemption_offers(merchant_id) 
WHERE merchant_id IS NOT NULL;

COMMENT ON TABLE redemption_offers IS 
'Catálogo de ofertas para redimir con Lümis. Antes: dim_redemptions';

-- ======================================================================
-- 3. BACKUP Y REEMPLAZO DE fact_redemptions
-- ======================================================================

-- 3.1 Respaldar tabla antigua
CREATE TABLE IF NOT EXISTS fact_redemptions_legacy AS 
SELECT * FROM fact_redemptions;

COMMENT ON TABLE fact_redemptions_legacy IS 
'Backup de fact_redemptions antes de migración 2025-10-17';

-- 3.2 Eliminar tabla vieja (CASCADE elimina FKs)
DROP TABLE IF EXISTS fact_redemptions CASCADE;

-- 3.3 Crear nueva tabla con campos completos para QR
CREATE TABLE user_redemptions (
    -- Identificación
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
    
    -- Metadata adicional
    metadata JSONB,
    
    -- Constraints
    CONSTRAINT valid_status CHECK (
        redemption_status IN ('pending', 'confirmed', 'cancelled', 'expired')
    ),
    CONSTRAINT valid_method CHECK (
        redemption_method IN ('qr_code', 'barcode', 'nfc', 'manual')
    )
);

-- 3.4 Índices críticos para rendimiento
CREATE INDEX idx_redemptions_user 
ON user_redemptions(user_id, created_at DESC);

CREATE INDEX idx_redemptions_code 
ON user_redemptions(redemption_code) 
WHERE redemption_status = 'pending';

CREATE INDEX idx_redemptions_status 
ON user_redemptions(user_id, redemption_status);

CREATE UNIQUE INDEX idx_unique_active_redemption 
ON user_redemptions(redemption_code) 
WHERE redemption_status = 'pending' AND is_used = false;

CREATE INDEX idx_redemptions_merchant 
ON user_redemptions(validated_by_merchant_id, validated_at) 
WHERE validated_by_merchant_id IS NOT NULL;

COMMENT ON TABLE user_redemptions IS 
'Redenciones de Lümis por usuarios con soporte para QR codes y validación por comercios';

-- ======================================================================
-- 4. TABLA DE AUDITORÍA
-- ======================================================================

CREATE TABLE IF NOT EXISTS redemption_audit_log (
    log_id BIGSERIAL PRIMARY KEY,
    redemption_id UUID NOT NULL REFERENCES user_redemptions(redemption_id),
    
    -- Acción
    action_type VARCHAR(50) NOT NULL,  -- 'created', 'validated', 'confirmed', 'cancelled'
    
    -- Actor
    performed_by VARCHAR(50),          -- 'user', 'merchant', 'system'
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

CREATE INDEX idx_audit_redemption 
ON redemption_audit_log(redemption_id, created_at DESC);

CREATE INDEX idx_audit_action 
ON redemption_audit_log(action_type, created_at DESC);

CREATE INDEX idx_audit_merchant 
ON redemption_audit_log(merchant_id, created_at DESC) 
WHERE merchant_id IS NOT NULL;

COMMENT ON TABLE redemption_audit_log IS 
'Log de auditoría para todas las acciones sobre redenciones';

-- ======================================================================
-- 5. TABLA DE COMERCIOS
-- ======================================================================

CREATE TABLE IF NOT EXISTS merchants (
    -- Identificación
    merchant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_name VARCHAR(255) NOT NULL UNIQUE,
    merchant_type VARCHAR(50),         -- 'restaurant', 'cinema', 'bookstore', etc.
    
    -- Contacto
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),
    
    -- API Credentials
    api_key_hash VARCHAR(255) NOT NULL,  -- bcrypt hash
    webhook_url TEXT,
    
    -- Estado
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Estadísticas (denormalized para performance)
    total_redemptions INTEGER DEFAULT 0,
    total_lumis_redeemed BIGINT DEFAULT 0
);

CREATE INDEX idx_merchants_active 
ON merchants(is_active) 
WHERE is_active = true;

COMMENT ON TABLE merchants IS 
'Comercios aliados autorizados para validar y confirmar redenciones';

-- ======================================================================
-- 6. ACTUALIZAR VISTA vw_hist_accum_redem
-- ======================================================================

DROP VIEW IF EXISTS vw_hist_accum_redem;

CREATE OR REPLACE VIEW vw_hist_accum_redem AS
SELECT 
    fa.user_id,
    fa.accum_type,
    fa.dtype,
    fa.quantity,
    fa.balance,
    fa.date,
    
    -- Información de acumulación (ganar)
    da.name AS accumulation_name,
    da.points AS accumulation_points,
    
    -- Información de redención (gastar)
    ro.name_friendly AS offer_name,
    ro.lumis_cost AS redemption_cost,
    ro.merchant_name,
    
    -- Vinculación con redención específica
    ur.redemption_id,
    ur.redemption_code,
    ur.redemption_status,
    ur.validated_at
    
FROM fact_accumulations fa
LEFT JOIN dim_accumulations da ON fa.accum_id = da.id
LEFT JOIN user_redemptions ur ON fa.redemption_id = ur.redemption_id
LEFT JOIN redemption_offers ro ON ur.offer_id = ro.offer_id;

COMMENT ON VIEW vw_hist_accum_redem IS 
'Vista unificada de historial de acumulaciones y redenciones con detalles completos';

-- ======================================================================
-- 7. TRIGGERS PARA ACTUALIZACIÓN AUTOMÁTICA
-- ======================================================================

-- 7.1 Trigger para actualizar balance al confirmar redención
CREATE OR REPLACE FUNCTION update_balance_on_redemption()
RETURNS TRIGGER AS $$
BEGIN
    -- Solo procesar si cambia a 'confirmed'
    IF NEW.redemption_status = 'confirmed' AND OLD.redemption_status != 'confirmed' THEN
        
        -- Insertar transacción negativa en fact_accumulations
        INSERT INTO fact_accumulations (
            user_id,
            accum_type,
            dtype,
            quantity,
            balance,
            date,
            redemption_id
        )
        SELECT 
            NEW.user_id,
            'spend',
            'redemption',
            -NEW.lumis_spent,
            COALESCE(fbp.balance, 0) - NEW.lumis_spent,
            NOW(),
            NEW.redemption_id
        FROM fact_balance_points fbp
        WHERE fbp.user_id = NEW.user_id;
        
        -- Actualizar balance actual
        UPDATE fact_balance_points
        SET balance = balance - NEW.lumis_spent,
            latest_update = NOW()
        WHERE user_id = NEW.user_id;
        
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_balance_on_redemption ON user_redemptions;

CREATE TRIGGER trigger_update_balance_on_redemption
AFTER UPDATE ON user_redemptions
FOR EACH ROW
EXECUTE FUNCTION update_balance_on_redemption();

-- 7.2 Trigger para devolver Lümis al cancelar
CREATE OR REPLACE FUNCTION refund_lumis_on_cancel()
RETURNS TRIGGER AS $$
BEGIN
    -- Solo procesar si cambia a 'cancelled' y estaba 'pending'
    IF NEW.redemption_status = 'cancelled' AND OLD.redemption_status = 'pending' THEN
        
        -- Insertar transacción positiva (devolución)
        INSERT INTO fact_accumulations (
            user_id,
            accum_type,
            dtype,
            quantity,
            balance,
            date,
            redemption_id
        )
        SELECT 
            NEW.user_id,
            'earn',
            'refund',
            NEW.lumis_spent,
            COALESCE(fbp.balance, 0) + NEW.lumis_spent,
            NOW(),
            NEW.redemption_id
        FROM fact_balance_points fbp
        WHERE fbp.user_id = NEW.user_id;
        
        -- Actualizar balance actual
        UPDATE fact_balance_points
        SET balance = balance + NEW.lumis_spent,
            latest_update = NOW()
        WHERE user_id = NEW.user_id;
        
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_refund_lumis_on_cancel ON user_redemptions;

CREATE TRIGGER trigger_refund_lumis_on_cancel
AFTER UPDATE ON user_redemptions
FOR EACH ROW
EXECUTE FUNCTION refund_lumis_on_cancel();

-- 7.3 Trigger para actualizar estadísticas de comercio
CREATE OR REPLACE FUNCTION update_merchant_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.redemption_status = 'confirmed' AND OLD.redemption_status != 'confirmed' THEN
        
        UPDATE merchants
        SET total_redemptions = total_redemptions + 1,
            total_lumis_redeemed = total_lumis_redeemed + NEW.lumis_spent,
            updated_at = NOW()
        WHERE merchant_id = NEW.validated_by_merchant_id;
        
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_merchant_stats ON user_redemptions;

CREATE TRIGGER trigger_update_merchant_stats
AFTER UPDATE ON user_redemptions
FOR EACH ROW
EXECUTE FUNCTION update_merchant_stats();

-- ======================================================================
-- 8. DATOS DE EJEMPLO
-- ======================================================================

-- 8.1 Insertar ofertas de ejemplo
INSERT INTO redemption_offers (
    id, offer_id, name, name_friendly, description_friendly, 
    lumis_cost, offer_category, merchant_name, 
    valid_from, valid_to, is_active, img
) VALUES
    (
        100,
        '550e8400-e29b-41d4-a716-446655440000',
        'cafe_americano_starbucks',
        'Café Americano',
        'Disfruta de un delicioso café americano en cualquier sucursal de Starbucks',
        55,
        'food',
        'Starbucks Panamá',
        '2025-01-01 00:00:00+00',
        '2026-12-31 23:59:59+00',
        true,
        'https://cdn.lumis.pa/offers/starbucks-cafe.jpg'
    ),
    (
        101,
        '660e8400-e29b-41d4-a716-446655440001',
        'entrada_cine_2d',
        'Entrada a Cine 2D',
        '1 entrada para película 2D en cualquier sala',
        180,
        'entertainment',
        'Cinépolis',
        '2025-01-01 00:00:00+00',
        '2026-06-30 23:59:59+00',
        true,
        'https://cdn.lumis.pa/offers/cine.jpg'
    ),
    (
        102,
        '770e8400-e29b-41d4-a716-446655440002',
        'libro_bestseller',
        'Libro Bestseller',
        'Cualquier libro de la sección bestsellers',
        120,
        'books',
        'Librería Argosy',
        '2025-01-01 00:00:00+00',
        '2026-12-31 23:59:59+00',
        true,
        'https://cdn.lumis.pa/offers/libro.jpg'
    ),
    (
        103,
        '880e8400-e29b-41d4-a716-446655440003',
        'cena_2_personas',
        'Cena para 2 Personas',
        'Menú especial para 2 personas en restaurante aliado',
        450,
        'food',
        'Restaurantes Aliados',
        '2025-01-01 00:00:00+00',
        '2026-12-31 23:59:59+00',
        true,
        'https://cdn.lumis.pa/offers/cena.jpg'
    )
ON CONFLICT (id) DO UPDATE SET
    offer_id = EXCLUDED.offer_id,
    name_friendly = EXCLUDED.name_friendly,
    lumis_cost = EXCLUDED.lumis_cost,
    offer_category = EXCLUDED.offer_category;

-- 8.2 Insertar comercio de ejemplo
INSERT INTO merchants (
    merchant_id,
    merchant_name,
    merchant_type,
    contact_email,
    api_key_hash,
    is_active
) VALUES (
    '990e8400-e29b-41d4-a716-446655440004',
    'Starbucks Centro Comercial',
    'restaurant',
    'partner@starbucks.pa',
    '$2a$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewnYNM7rvMVCGNQa', -- 'password123' en bcrypt
    true
) ON CONFLICT (merchant_name) DO NOTHING;

-- ======================================================================
-- 9. FUNCIONES ÚTILES
-- ======================================================================

-- 9.1 Función para marcar códigos expirados (ejecutar en cron)
CREATE OR REPLACE FUNCTION expire_old_redemptions()
RETURNS INTEGER AS $$
DECLARE
    expired_count INTEGER;
BEGIN
    UPDATE user_redemptions
    SET redemption_status = 'expired'
    WHERE redemption_status = 'pending'
      AND code_expires_at < NOW()
      AND is_used = false;
    
    GET DIAGNOSTICS expired_count = ROW_COUNT;
    
    RETURN expired_count;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION expire_old_redemptions() IS 
'Marca como expirados los códigos pendientes que pasaron su fecha límite. Ejecutar cada hora via cron.';

-- 9.2 Función para obtener balance de usuario
CREATE OR REPLACE FUNCTION get_user_balance(p_user_id INTEGER)
RETURNS INTEGER AS $$
DECLARE
    user_balance INTEGER;
BEGIN
    SELECT COALESCE(balance, 0)
    INTO user_balance
    FROM fact_balance_points
    WHERE user_id = p_user_id;
    
    RETURN COALESCE(user_balance, 0);
END;
$$ LANGUAGE plpgsql;

-- 9.3 Función para validar si usuario puede redimir oferta
CREATE OR REPLACE FUNCTION can_user_redeem_offer(
    p_user_id INTEGER,
    p_offer_id UUID
)
RETURNS TABLE(
    can_redeem BOOLEAN,
    reason TEXT,
    user_balance INTEGER,
    offer_cost INTEGER,
    user_redemptions_count INTEGER,
    max_allowed INTEGER
) AS $$
DECLARE
    v_balance INTEGER;
    v_cost INTEGER;
    v_count INTEGER;
    v_max INTEGER;
    v_stock INTEGER;
BEGIN
    -- Obtener datos
    SELECT get_user_balance(p_user_id) INTO v_balance;
    
    SELECT lumis_cost, max_redemptions_per_user, stock_quantity
    INTO v_cost, v_max, v_stock
    FROM redemption_offers
    WHERE offer_id = p_offer_id AND is_active = true;
    
    SELECT COUNT(*)
    INTO v_count
    FROM user_redemptions
    WHERE user_id = p_user_id 
      AND offer_id = p_offer_id
      AND redemption_status != 'cancelled';
    
    -- Validaciones
    IF v_cost IS NULL THEN
        RETURN QUERY SELECT false, 'Oferta no encontrada o inactiva'::TEXT, v_balance, v_cost, v_count, v_max;
        RETURN;
    END IF;
    
    IF v_balance < v_cost THEN
        RETURN QUERY SELECT false, 'Saldo insuficiente'::TEXT, v_balance, v_cost, v_count, v_max;
        RETURN;
    END IF;
    
    IF v_count >= v_max THEN
        RETURN QUERY SELECT false, 'Límite de redenciones alcanzado'::TEXT, v_balance, v_cost, v_count, v_max;
        RETURN;
    END IF;
    
    IF v_stock IS NOT NULL AND v_stock <= 0 THEN
        RETURN QUERY SELECT false, 'Sin stock disponible'::TEXT, v_balance, v_cost, v_count, v_max;
        RETURN;
    END IF;
    
    -- Todo OK
    RETURN QUERY SELECT true, 'Puede redimir'::TEXT, v_balance, v_cost, v_count, v_max;
END;
$$ LANGUAGE plpgsql;

-- ======================================================================
-- 10. PERMISOS (Ajustar según usuarios de BD)
-- ======================================================================

-- GRANT SELECT, INSERT, UPDATE ON user_redemptions TO lum_api_user;
-- GRANT SELECT ON redemption_offers TO lum_api_user;
-- GRANT INSERT ON redemption_audit_log TO lum_api_user;
-- GRANT SELECT ON merchants TO lum_api_user;

-- ======================================================================
-- FIN DE MIGRACIÓN
-- ======================================================================

COMMIT;

-- Verificación post-migración
DO $$
BEGIN
    RAISE NOTICE '✅ MIGRACIÓN COMPLETADA EXITOSAMENTE';
    RAISE NOTICE '✅ Tabla redemption_offers: % registros', (SELECT COUNT(*) FROM redemption_offers);
    RAISE NOTICE '✅ Tabla user_redemptions: creada';
    RAISE NOTICE '✅ Tabla redemption_audit_log: creada';
    RAISE NOTICE '✅ Tabla merchants: % registros', (SELECT COUNT(*) FROM merchants);
    RAISE NOTICE '✅ Vista vw_hist_accum_redem: actualizada';
    RAISE NOTICE '✅ Triggers: 3 instalados';
    RAISE NOTICE '✅ Funciones útiles: 3 creadas';
END $$;
