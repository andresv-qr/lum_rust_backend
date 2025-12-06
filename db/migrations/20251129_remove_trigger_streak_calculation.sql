-- ============================================================================
-- MIGRACIÓN: Remover cálculo de streak del trigger
-- Fecha: 2025-11-29
-- Descripción: 
--   - Modifica el trigger para NO calcular streaks por INSERT
--   - El cálculo de streaks lo hará el batch job cada 12 horas
-- ============================================================================

-- ============================================================================
-- PASO 1: Verificar función actual del trigger
-- ============================================================================

-- NOTA: Este script asume que el trigger actual llama a update_user_streaks()
-- Si la estructura es diferente, ajustar según corresponda.

-- ============================================================================
-- PASO 2: Nueva versión del trigger (sin cálculo de streak)
-- ============================================================================

CREATE OR REPLACE FUNCTION gamification.trigger_refresh_lum_levels()
RETURNS TRIGGER AS $$
BEGIN
    -- ========================================================================
    -- IMPORTANTE: NO calcular streaks aquí
    -- El batch job batch_consistent_month() lo hace cada 12 horas
    -- Esto reduce de ~8-10 queries por INSERT a solo 2-3 queries
    -- ========================================================================
    
    -- Solo actualizar niveles de Lümis (operación ligera)
    IF TG_OP = 'INSERT' THEN
        -- Actualizar el nivel del usuario basado en su balance actual
        -- Esta operación es rápida (solo UPDATE a una fila)
        PERFORM gamification.update_user_level(NEW.user_id);
        
        -- Registrar actividad de factura para auditoría
        INSERT INTO gamification.fact_user_activity_log (
            user_id,
            activity_type,
            activity_data,
            created_at
        ) VALUES (
            NEW.user_id,
            'invoice_upload',
            jsonb_build_object(
                'invoice_id', NEW.id,
                'cufe', NEW.cufe,
                'issuer_name', NEW.issuer_name,
                'total', NEW.total_invoice_amount,
                'source', 'trigger'
            ),
            COALESCE(NEW.reception_date, NOW())
        );
        
        -- NOTA: Ya NO llamamos a update_user_streaks()
        -- PERFORM gamification.update_user_streaks(NEW.user_id);  -- REMOVIDO
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION gamification.trigger_refresh_lum_levels() IS 
'Trigger optimizado para invoice_header INSERT.
NO calcula streaks (lo hace batch_consistent_month cada 12h).
Solo actualiza nivel de usuario y registra actividad.
Reducido de ~10 queries a ~2-3 queries por INSERT.';


-- ============================================================================
-- PASO 3: Función update_user_level (solo actualiza nivel, sin streak)
-- ============================================================================

CREATE OR REPLACE FUNCTION gamification.update_user_level(p_user_id INTEGER)
RETURNS VOID AS $$
DECLARE
    v_total_lumis BIGINT;
    v_new_level INTEGER;
BEGIN
    -- Obtener balance actual
    SELECT COALESCE(current_balance, 0) + COALESCE(lifetime_earned, 0)
    INTO v_total_lumis
    FROM gamification.fact_lumi_balances
    WHERE user_id = p_user_id;
    
    IF v_total_lumis IS NULL THEN
        v_total_lumis := 0;
    END IF;
    
    -- Determinar nivel basado en lumis (simplificado)
    SELECT level_id INTO v_new_level
    FROM gamification.dim_levels
    WHERE v_total_lumis >= min_lumis
    ORDER BY min_lumis DESC
    LIMIT 1;
    
    -- Actualizar nivel del usuario si cambió
    UPDATE gamification.fact_user_levels
    SET 
        current_level_id = COALESCE(v_new_level, 1),
        updated_at = NOW()
    WHERE user_id = p_user_id
    AND current_level_id IS DISTINCT FROM v_new_level;
    
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- PASO 4: Verificar que el trigger está asignado correctamente
-- ============================================================================

-- Recrear el trigger para asegurar que usa la nueva función
DROP TRIGGER IF EXISTS trg_refresh_lum_levels ON public.invoice_header;

CREATE TRIGGER trg_refresh_lum_levels
AFTER INSERT ON public.invoice_header
FOR EACH ROW
EXECUTE FUNCTION gamification.trigger_refresh_lum_levels();


-- ============================================================================
-- PASO 5: Mensaje de confirmación
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '
=====================================================
MIGRACIÓN COMPLETADA: Trigger optimizado
=====================================================

Cambios realizados:
  1. trigger_refresh_lum_levels() ya NO llama a update_user_streaks()
  2. Reducido de ~10 queries por INSERT a ~2-3 queries
  
Antes (por cada INSERT en invoice_header):
  - 4 EXISTS para calcular streak
  - UPDATE a fact_user_streaks  
  - UPDATE a fact_lumi_balances
  - INSERT a fact_user_activity_log
  = ~8-10 queries

Ahora (por cada INSERT en invoice_header):
  - SELECT balance
  - UPDATE nivel (si cambió)
  - INSERT activity_log
  = ~2-3 queries

El cálculo de streaks ahora lo hace:
  batch_consistent_month() cada 12 horas
  
Reducción estimada:
  Antes: 500,000 queries/mes
  Ahora: ~150,000 queries/mes + 180 queries/mes (batch)
  Ahorro: ~70%
=====================================================
';
END $$;
