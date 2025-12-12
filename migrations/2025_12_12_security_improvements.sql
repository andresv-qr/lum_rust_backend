-- ============================================================================
-- MEJORAS DE SEGURIDAD E INTEGRIDAD - Sistema de Redenciones
-- Fecha: 2025-12-12
-- Descripción: Correcciones identificadas en auditoría de código
-- ============================================================================

-- 1. FK CONSTRAINT: user_redemptions.user_id → users.user_id
-- Garantiza integridad referencial
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'fk_redemptions_user' 
        AND table_schema = 'rewards'
    ) THEN
        ALTER TABLE rewards.user_redemptions 
        ADD CONSTRAINT fk_redemptions_user 
        FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE;
        
        RAISE NOTICE 'FK constraint fk_redemptions_user created';
    ELSE
        RAISE NOTICE 'FK constraint fk_redemptions_user already exists';
    END IF;
END $$;

-- 2. FIX: Índice con nombre de columna correcto (code_expires_at, no expires_at)
-- Eliminar índice incorrecto si existe
DROP INDEX IF EXISTS rewards.idx_redemptions_expiring;

-- Crear índice correcto para consultas de expiración
CREATE INDEX IF NOT EXISTS idx_redemptions_expiring_v2
ON rewards.user_redemptions(user_id, redemption_status, code_expires_at)
WHERE redemption_status = 'pending';

-- 3. Índice adicional para búsqueda exacta de códigos (seguridad)
CREATE INDEX IF NOT EXISTS idx_redemptions_code_exact
ON rewards.user_redemptions(redemption_code)
WHERE redemption_status = 'pending';

-- 4. Índice para offer_id (FK lookup optimization)
CREATE INDEX IF NOT EXISTS idx_redemptions_offer_id
ON rewards.user_redemptions(offer_id);

-- 5. Agregar columna contact_email a merchants si no existe
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_schema = 'rewards' 
        AND table_name = 'merchants' 
        AND column_name = 'contact_email'
    ) THEN
        ALTER TABLE rewards.merchants 
        ADD COLUMN contact_email VARCHAR(255);
        
        RAISE NOTICE 'Column contact_email added to merchants';
    END IF;
END $$;

-- 6. Trigger para restaurar stock automáticamente en cancelaciones
-- (Backup del código Rust en caso de que el trigger no se ejecute)
CREATE OR REPLACE FUNCTION rewards.restore_stock_on_cancel()
RETURNS TRIGGER AS $$
BEGIN
    -- Solo actuar cuando el estado cambia a 'cancelled'
    IF NEW.redemption_status = 'cancelled' AND OLD.redemption_status = 'pending' THEN
        UPDATE rewards.redemption_offers
        SET stock_quantity = COALESCE(stock_quantity, 0) + 1
        WHERE offer_id = OLD.offer_id
          AND stock_quantity IS NOT NULL;
        
        RAISE NOTICE 'Stock restored for offer % after cancellation of redemption %', OLD.offer_id, OLD.redemption_id;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Eliminar trigger anterior si existe
DROP TRIGGER IF EXISTS trigger_restore_stock_on_cancel ON rewards.user_redemptions;

-- Crear nuevo trigger
CREATE TRIGGER trigger_restore_stock_on_cancel
AFTER UPDATE OF redemption_status ON rewards.user_redemptions
FOR EACH ROW
EXECUTE FUNCTION rewards.restore_stock_on_cancel();

-- 7. Crear tabla para caché de QR codes si no existe (para cleanup job)
CREATE TABLE IF NOT EXISTS rewards.qr_code_cache (
    cache_id SERIAL PRIMARY KEY,
    redemption_id UUID REFERENCES rewards.user_redemptions(redemption_id) ON DELETE CASCADE,
    qr_image_data BYTEA,
    file_path VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_qr_cache_created 
ON rewards.qr_code_cache(created_at);

-- ============================================================================
-- VERIFICACIONES POST-MIGRACIÓN
-- ============================================================================
DO $$
DECLARE
    v_fk_count INT;
    v_idx_count INT;
BEGIN
    -- Verificar FK creado
    SELECT COUNT(*) INTO v_fk_count 
    FROM information_schema.table_constraints 
    WHERE constraint_name = 'fk_redemptions_user';
    
    -- Verificar índices creados
    SELECT COUNT(*) INTO v_idx_count 
    FROM pg_indexes 
    WHERE schemaname = 'rewards' 
    AND indexname IN ('idx_redemptions_expiring_v2', 'idx_redemptions_code_exact', 'idx_redemptions_offer_id');
    
    RAISE NOTICE '=== MIGRATION SUMMARY ===';
    RAISE NOTICE 'FK constraints: %', v_fk_count;
    RAISE NOTICE 'New indexes: %', v_idx_count;
    RAISE NOTICE '========================';
END $$;

-- Actualizar email de Demo Store para testing
UPDATE rewards.merchants
SET contact_email = 'demo@lumapp.org'
WHERE merchant_name = 'Demo Store'
  AND (contact_email IS NULL OR contact_email = '');

COMMIT;
