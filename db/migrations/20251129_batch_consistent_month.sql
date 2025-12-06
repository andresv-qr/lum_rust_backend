-- ============================================================================
-- MIGRACIÓN: Batch Job para consistent_month en fact_activity_log
-- Fecha: 2025-11-29
-- Descripción: 
--   - Función batch que inserta primera factura semanal por usuario
--   - Se ejecuta cada 12 horas
--   - DELETE + INSERT para últimas 5 semanas
--   - Actualiza fact_user_streaks basado en fact_activity_log
-- ============================================================================

-- ============================================================================
-- PARTE 1: Función principal del batch
-- ============================================================================

CREATE OR REPLACE FUNCTION gamification.batch_consistent_month()
RETURNS TABLE (
    users_processed INTEGER,
    records_deleted INTEGER,
    records_inserted INTEGER,
    streaks_updated INTEGER,
    execution_time_ms INTEGER
) AS $$
DECLARE
    v_start_time TIMESTAMPTZ := clock_timestamp();
    v_users_processed INTEGER := 0;
    v_records_deleted INTEGER := 0;
    v_records_inserted INTEGER := 0;
    v_streaks_updated INTEGER := 0;
    v_week_start DATE;
BEGIN
    -- Calcular inicio del rango (5 semanas hacia atrás desde el lunes de esta semana)
    v_week_start := DATE_TRUNC('week', CURRENT_DATE)::DATE - INTERVAL '4 weeks';
    
    RAISE NOTICE 'batch_consistent_month: Iniciando desde semana %', v_week_start;
    
    -- ========================================================================
    -- PASO 1: Eliminar registros consistent_month de las últimas 5 semanas
    -- ========================================================================
    DELETE FROM gamification.fact_user_activity_log
    WHERE activity_type = 'consistent_month'
    AND created_at >= v_week_start;
    
    GET DIAGNOSTICS v_records_deleted = ROW_COUNT;
    RAISE NOTICE 'batch_consistent_month: Eliminados % registros antiguos', v_records_deleted;
    
    -- ========================================================================
    -- PASO 2: Insertar primera factura de cada semana por usuario
    -- ========================================================================
    WITH weekly_first_invoices AS (
        -- Encontrar la primera factura de cada usuario en cada semana
        SELECT DISTINCT ON (ih.user_id, DATE_TRUNC('week', ih.reception_date))
            ih.user_id,
            ih.id as invoice_id,
            ih.reception_date,
            ih.cufe,
            ih.issuer_name,
            ih.total_invoice_amount,
            DATE_TRUNC('week', ih.reception_date)::DATE as week_start,
            EXTRACT(WEEK FROM ih.reception_date)::INTEGER as week_number,
            EXTRACT(YEAR FROM ih.reception_date)::INTEGER as year
        FROM public.invoice_header ih
        WHERE ih.reception_date >= v_week_start
        AND ih.user_id IS NOT NULL
        ORDER BY ih.user_id, DATE_TRUNC('week', ih.reception_date), ih.reception_date ASC
    )
    INSERT INTO gamification.fact_user_activity_log (
        user_id,
        activity_type,
        activity_data,
        created_at
    )
    SELECT 
        wfi.user_id,
        'consistent_month',
        jsonb_build_object(
            'invoice_id', wfi.invoice_id,
            'cufe', wfi.cufe,
            'issuer_name', wfi.issuer_name,
            'amount', wfi.total_invoice_amount,
            'week_start', wfi.week_start,
            'week_number', wfi.week_number,
            'year', wfi.year,
            'source', 'batch'
        ),
        wfi.reception_date  -- created_at = fecha real de la primera factura
    FROM weekly_first_invoices wfi;
    
    GET DIAGNOSTICS v_records_inserted = ROW_COUNT;
    RAISE NOTICE 'batch_consistent_month: Insertados % registros nuevos', v_records_inserted;
    
    -- Contar usuarios únicos procesados
    SELECT COUNT(DISTINCT user_id) INTO v_users_processed
    FROM gamification.fact_user_activity_log
    WHERE activity_type = 'consistent_month'
    AND created_at >= v_week_start;
    
    -- ========================================================================
    -- PASO 3: Actualizar fact_user_streaks para todos los usuarios afectados
    -- ========================================================================
    WITH user_streaks AS (
        -- Calcular streak desde fact_activity_log
        SELECT 
            user_id,
            gamification.calculate_streak_from_activity_log(user_id) as streak_count
        FROM (
            SELECT DISTINCT user_id 
            FROM gamification.fact_user_activity_log
            WHERE activity_type = 'consistent_month'
            AND created_at >= v_week_start
        ) users
    )
    INSERT INTO gamification.fact_user_streaks (
        user_id,
        streak_type,
        current_count,
        last_activity_date,
        streak_start_date,
        updated_at
    )
    SELECT 
        us.user_id,
        'consistent_month',
        us.streak_count,
        CURRENT_DATE,
        CURRENT_DATE - ((us.streak_count - 1) * 7),  -- Aproximar fecha inicio
        NOW()
    FROM user_streaks us
    WHERE us.streak_count > 0
    ON CONFLICT (user_id, streak_type) 
    DO UPDATE SET 
        current_count = EXCLUDED.current_count,
        last_activity_date = EXCLUDED.last_activity_date,
        updated_at = NOW();
    
    GET DIAGNOSTICS v_streaks_updated = ROW_COUNT;
    RAISE NOTICE 'batch_consistent_month: Actualizados % streaks', v_streaks_updated;
    
    -- ========================================================================
    -- PASO 4: Retornar resultados
    -- ========================================================================
    RETURN QUERY SELECT 
        v_users_processed,
        v_records_deleted,
        v_records_inserted,
        v_streaks_updated,
        EXTRACT(MILLISECONDS FROM (clock_timestamp() - v_start_time))::INTEGER;
        
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION gamification.batch_consistent_month() IS 
'Batch job que sincroniza primera factura semanal de invoice_header a fact_activity_log.
Ejecutar cada 12 horas. Idempotente (DELETE + INSERT).';


-- ============================================================================
-- PARTE 2: Función auxiliar para calcular streak desde activity_log
-- ============================================================================

CREATE OR REPLACE FUNCTION gamification.calculate_streak_from_activity_log(p_user_id INTEGER)
RETURNS INTEGER AS $$
DECLARE
    v_consecutive_weeks INTEGER := 0;
    v_current_week DATE;
    v_week_cursor DATE;
    v_has_activity BOOLEAN;
    v_gap_found BOOLEAN := FALSE;
BEGIN
    -- Empezar desde la semana actual
    v_current_week := DATE_TRUNC('week', CURRENT_DATE)::DATE;
    
    -- Verificar hasta 5 semanas hacia atrás (incluyendo semana actual)
    FOR i IN 0..4 LOOP
        v_week_cursor := v_current_week - (i * INTERVAL '1 week');
        
        -- Verificar si hay registro de consistent_month para esta semana
        SELECT EXISTS(
            SELECT 1 
            FROM gamification.fact_user_activity_log
            WHERE user_id = p_user_id
            AND activity_type = 'consistent_month'
            AND DATE_TRUNC('week', created_at)::DATE = v_week_cursor
        ) INTO v_has_activity;
        
        IF v_has_activity THEN
            -- Solo contar si no hemos encontrado un gap
            IF NOT v_gap_found THEN
                v_consecutive_weeks := v_consecutive_weeks + 1;
            END IF;
        ELSE
            -- Encontramos un gap, marcar y dejar de contar
            v_gap_found := TRUE;
        END IF;
    END LOOP;
    
    RETURN v_consecutive_weeks;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION gamification.calculate_streak_from_activity_log(INTEGER) IS 
'Calcula semanas consecutivas con actividad consistent_month desde fact_activity_log.
Retorna 0-5 (máximo 5 semanas hacia atrás).';


-- ============================================================================
-- PARTE 3: Función para ejecutar manualmente y ver resultados
-- ============================================================================

CREATE OR REPLACE FUNCTION gamification.run_batch_consistent_month_with_log()
RETURNS TEXT AS $$
DECLARE
    v_result RECORD;
    v_message TEXT;
BEGIN
    SELECT * INTO v_result FROM gamification.batch_consistent_month();
    
    v_message := format(
        'Batch consistent_month completado:
        - Usuarios procesados: %s
        - Registros eliminados: %s
        - Registros insertados: %s
        - Streaks actualizados: %s
        - Tiempo de ejecución: %s ms',
        v_result.users_processed,
        v_result.records_deleted,
        v_result.records_inserted,
        v_result.streaks_updated,
        v_result.execution_time_ms
    );
    
    -- Log en tabla de auditoría si existe
    BEGIN
        INSERT INTO gamification.fact_audit_log (
            action_type,
            action_details,
            performed_by,
            created_at
        ) VALUES (
            'batch_consistent_month',
            jsonb_build_object(
                'users_processed', v_result.users_processed,
                'records_deleted', v_result.records_deleted,
                'records_inserted', v_result.records_inserted,
                'streaks_updated', v_result.streaks_updated,
                'execution_time_ms', v_result.execution_time_ms
            ),
            'system',
            NOW()
        );
    EXCEPTION WHEN OTHERS THEN
        -- Ignorar si no existe la tabla de auditoría
        NULL;
    END;
    
    RETURN v_message;
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- PARTE 4: Crear índice para optimizar consultas de activity_log
-- ============================================================================

-- Índice para buscar consistent_month por usuario y fecha
CREATE INDEX IF NOT EXISTS idx_activity_log_consistent_month
ON gamification.fact_user_activity_log (user_id, created_at)
WHERE activity_type = 'consistent_month';

-- Índice para el DELETE del batch
CREATE INDEX IF NOT EXISTS idx_activity_log_type_date
ON gamification.fact_user_activity_log (activity_type, created_at);


-- ============================================================================
-- PARTE 5: Verificación (solo muestra, no ejecuta el batch)
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '
=====================================================
MIGRACIÓN COMPLETADA: batch_consistent_month
=====================================================

Funciones creadas:
  1. gamification.batch_consistent_month()
     - Ejecutar cada 12 horas
     - DELETE + INSERT para últimas 5 semanas
     
  2. gamification.calculate_streak_from_activity_log(user_id)
     - Calcula streak desde fact_activity_log
     
  3. gamification.run_batch_consistent_month_with_log()
     - Wrapper con logging

Para ejecutar manualmente:
  SELECT * FROM gamification.batch_consistent_month();
  
O con log:
  SELECT gamification.run_batch_consistent_month_with_log();

Para configurar pg_cron (cada 12 horas):
  SELECT cron.schedule(
    ''batch_consistent_month'',
    ''0 0,12 * * *'',
    ''SELECT gamification.run_batch_consistent_month_with_log()''
  );
=====================================================
';
END $$;
