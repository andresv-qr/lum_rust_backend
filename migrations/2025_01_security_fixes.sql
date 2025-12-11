-- ============================================================================
-- MIGRATION: Security Fixes - Race condition, JWT jti, indexes
-- Date: 2025-01
-- ============================================================================

-- 1. Tabla para tracking de tokens JWT usados (previene replay attacks)
CREATE TABLE IF NOT EXISTS rewards.used_validation_tokens (
    jti VARCHAR(64) PRIMARY KEY,  -- UUID del JWT
    redemption_id UUID NOT NULL REFERENCES rewards.user_redemptions(redemption_id),
    used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_by_merchant_id UUID REFERENCES rewards.merchants(merchant_id)
);

-- Índice para limpieza automática de tokens viejos
CREATE INDEX IF NOT EXISTS idx_used_tokens_used_at 
ON rewards.used_validation_tokens(used_at);

-- 2. Índices compuestos para optimización de queries frecuentes
-- Índice para listar redenciones pendientes por usuario
CREATE INDEX IF NOT EXISTS idx_user_redemptions_user_pending 
ON rewards.user_redemptions(user_id, redemption_status, expires_at)
WHERE redemption_status = 'pending';

-- Índice para búsqueda por código con status
CREATE INDEX IF NOT EXISTS idx_user_redemptions_code_status 
ON rewards.user_redemptions(redemption_code, redemption_status);

-- Índice para ofertas activas ordenadas por costo
CREATE INDEX IF NOT EXISTS idx_offers_active_cost 
ON rewards.redemption_offers(is_active, lumis_cost)
WHERE is_active = true;

-- Índice para ofertas por merchant
CREATE INDEX IF NOT EXISTS idx_offers_merchant_active 
ON rewards.redemption_offers(merchant_id, is_active)
WHERE is_active = true;

-- 3. Tabla de auditoría para redenciones
CREATE TABLE IF NOT EXISTS rewards.redemption_audit (
    audit_id SERIAL PRIMARY KEY,
    redemption_id UUID NOT NULL REFERENCES rewards.user_redemptions(redemption_id),
    action VARCHAR(50) NOT NULL,  -- 'created', 'validated', 'confirmed', 'cancelled', 'expired'
    actor_type VARCHAR(20) NOT NULL,  -- 'user', 'merchant', 'system'
    actor_id VARCHAR(50),  -- user_id o merchant_id (UUID) según actor_type
    old_status VARCHAR(20),
    new_status VARCHAR(20),
    metadata JSONB DEFAULT '{}',  -- Datos adicionales (IP, user agent, etc)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Índice para búsquedas por redemption
CREATE INDEX IF NOT EXISTS idx_audit_redemption_id 
ON rewards.redemption_audit(redemption_id);

-- Índice para búsquedas por fecha
CREATE INDEX IF NOT EXISTS idx_audit_created_at 
ON rewards.redemption_audit(created_at DESC);

-- 4. Agregar columna validated_by_merchant_id a user_redemptions si no existe
-- (Ya existe como validated_by_merchant_id UUID en la migración original)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_schema = 'rewards' 
        AND table_name = 'user_redemptions' 
        AND column_name = 'validated_by_merchant_id'
    ) THEN
        ALTER TABLE rewards.user_redemptions 
        ADD COLUMN validated_by_merchant_id UUID REFERENCES rewards.merchants(merchant_id);
    END IF;
END $$;

-- 5. Función para limpiar tokens viejos (ejecutar periódicamente)
CREATE OR REPLACE FUNCTION rewards.cleanup_expired_tokens()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM rewards.used_validation_tokens
    WHERE used_at < NOW() - INTERVAL '24 hours';
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- 6. Trigger para auditar cambios automáticamente
CREATE OR REPLACE FUNCTION rewards.audit_redemption_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO rewards.redemption_audit (
            redemption_id, action, actor_type, actor_id, 
            new_status, metadata
        ) VALUES (
            NEW.redemption_id, 'created', 'user', NEW.user_id::text,
            NEW.redemption_status, 
            jsonb_build_object('offer_id', NEW.offer_id, 'lumis_spent', NEW.lumis_spent)
        );
    ELSIF TG_OP = 'UPDATE' AND OLD.redemption_status != NEW.redemption_status THEN
        INSERT INTO rewards.redemption_audit (
            redemption_id, action, actor_type, actor_id,
            old_status, new_status, metadata
        ) VALUES (
            NEW.redemption_id,
            CASE NEW.redemption_status
                WHEN 'confirmed' THEN 'confirmed'
                WHEN 'cancelled' THEN 'cancelled'
                WHEN 'expired' THEN 'expired'
                ELSE 'status_changed'
            END,
            CASE 
                WHEN NEW.redemption_status = 'confirmed' THEN 'merchant'
                WHEN NEW.redemption_status = 'cancelled' THEN 'user'
                ELSE 'system'
            END,
            COALESCE(NEW.validated_by_merchant_id::text, NEW.user_id::text),
            OLD.redemption_status,
            NEW.redemption_status,
            jsonb_build_object('validated_at', NEW.validated_at)
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Crear trigger si no existe
DROP TRIGGER IF EXISTS trg_audit_redemption ON rewards.user_redemptions;
CREATE TRIGGER trg_audit_redemption
    AFTER INSERT OR UPDATE ON rewards.user_redemptions
    FOR EACH ROW EXECUTE FUNCTION rewards.audit_redemption_changes();

-- 7. Comentarios para documentación
COMMENT ON TABLE rewards.used_validation_tokens IS 'Tokens JWT usados para prevenir replay attacks';
COMMENT ON TABLE rewards.redemption_audit IS 'Auditoría completa de todas las acciones sobre redenciones';
COMMENT ON COLUMN rewards.user_redemptions.validated_by_merchant_id IS 'ID del merchant que confirmó la redención';
