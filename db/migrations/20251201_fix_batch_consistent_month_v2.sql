-- Migration: Fix batch_consistent_month function (v2)
-- Date: 2025-12-01
-- Issue: fact_user_activity_log is partitioned, and we should calculate directly from invoice_header
-- Fix: Simplified approach - calculate streaks directly from invoice_header

BEGIN;

CREATE OR REPLACE FUNCTION gamification.batch_consistent_month()
RETURNS TABLE(users_processed integer, records_deleted integer, records_inserted integer, streaks_updated integer, execution_time_ms integer)
LANGUAGE plpgsql
AS $function$
DECLARE
    v_start_time TIMESTAMPTZ := clock_timestamp();
    v_users_processed INTEGER := 0;
    v_records_deleted INTEGER := 0;
    v_records_inserted INTEGER := 0;
    v_streaks_updated INTEGER := 0;
    v_current_week_start DATE;
BEGIN
    -- Calcular el lunes de la semana actual
    v_current_week_start := DATE_TRUNC('week', CURRENT_DATE)::DATE;
    
    RAISE NOTICE 'batch_consistent_month: Semana actual: %', v_current_week_start;
    
    -- PASO 1: Calcular semanas consecutivas directamente desde invoice_header
    -- Miramos las últimas 4 semanas (incluyendo la actual)
    WITH user_weekly_invoices AS (
        -- Obtener semanas únicas con al menos 1 factura por usuario
        SELECT 
            ih.user_id,
            DATE_TRUNC('week', ih.reception_date)::DATE as week_start
        FROM public.invoice_header ih
        WHERE ih.user_id IS NOT NULL
        AND ih.reception_date >= v_current_week_start - INTERVAL '3 weeks'
        GROUP BY ih.user_id, DATE_TRUNC('week', ih.reception_date)
    ),
    consecutive_weeks_calc AS (
        -- Contar semanas consecutivas hacia atrás
        SELECT 
            uwi.user_id,
            COUNT(*) as total_weeks_in_range,
            -- Verificar si tiene las 4 semanas consecutivas
            SUM(CASE WHEN uwi.week_start = v_current_week_start THEN 1 ELSE 0 END) as has_week_0,
            SUM(CASE WHEN uwi.week_start = v_current_week_start - INTERVAL '1 week' THEN 1 ELSE 0 END) as has_week_1,
            SUM(CASE WHEN uwi.week_start = v_current_week_start - INTERVAL '2 weeks' THEN 1 ELSE 0 END) as has_week_2,
            SUM(CASE WHEN uwi.week_start = v_current_week_start - INTERVAL '3 weeks' THEN 1 ELSE 0 END) as has_week_3
        FROM user_weekly_invoices uwi
        GROUP BY uwi.user_id
    ),
    user_streak_result AS (
        SELECT 
            cwc.user_id,
            -- Calcular racha consecutiva desde la semana más reciente
            CASE 
                WHEN cwc.has_week_0 = 0 THEN 0  -- No tiene factura esta semana
                WHEN cwc.has_week_1 = 0 THEN 1  -- Solo esta semana
                WHEN cwc.has_week_2 = 0 THEN 2  -- Esta y la anterior
                WHEN cwc.has_week_3 = 0 THEN 3  -- 3 semanas
                ELSE 4                           -- 4 semanas completas!
            END as consecutive_weeks
        FROM consecutive_weeks_calc cwc
    )
    -- PASO 2: Actualizar user_streaks
    INSERT INTO gamification.user_streaks (
        user_id,
        streak_type,
        current_count,
        max_count,
        last_activity_date,
        streak_start_date,
        is_active,
        updated_at
    )
    SELECT 
        usr.user_id,
        'consistent_month',
        -- Si llegó a 4, resetear a 0 (ciclo completado, se otorgó recompensa)
        CASE WHEN usr.consecutive_weeks >= 4 THEN 0 ELSE usr.consecutive_weeks END,
        4, -- max_count siempre es 4
        CURRENT_DATE,
        v_current_week_start - ((GREATEST(usr.consecutive_weeks, 1) - 1) * INTERVAL '1 week'),
        true,
        NOW()
    FROM user_streak_result usr
    ON CONFLICT (user_id, streak_type) 
    DO UPDATE SET 
        current_count = CASE 
            WHEN EXCLUDED.current_count >= 4 THEN 0 
            ELSE EXCLUDED.current_count 
        END,
        max_count = 4,
        last_activity_date = EXCLUDED.last_activity_date,
        updated_at = NOW();
    
    GET DIAGNOSTICS v_streaks_updated = ROW_COUNT;
    
    -- Contar usuarios procesados
    SELECT COUNT(DISTINCT user_id) INTO v_users_processed
    FROM gamification.user_streaks
    WHERE streak_type = 'consistent_month'
    AND updated_at >= v_start_time;
    
    RAISE NOTICE 'batch_consistent_month: Actualizados % streaks para % usuarios', v_streaks_updated, v_users_processed;
    
    -- PASO 3: Otorgar recompensa (50 Lumis) a usuarios que completaron 4 semanas
    WITH completed_users AS (
        SELECT user_id
        FROM user_streak_result
        WHERE consecutive_weeks >= 4
    )
    INSERT INTO gamification.point_ledger (
        user_id,
        amount,
        source_type,
        source_id,
        description,
        created_at
    )
    SELECT 
        cu.user_id,
        50, -- Recompensa Perfect Month
        'achievement',
        'consistent_month_' || TO_CHAR(v_current_week_start, 'YYYYMMDD'),
        'Perfect Month - 4 semanas consecutivas de facturas',
        NOW()
    FROM completed_users cu
    -- Evitar duplicados: no otorgar si ya se dio esta semana
    WHERE NOT EXISTS (
        SELECT 1 FROM gamification.point_ledger pl
        WHERE pl.user_id = cu.user_id
        AND pl.source_type = 'achievement'
        AND pl.source_id = 'consistent_month_' || TO_CHAR(v_current_week_start, 'YYYYMMDD')
    );
    
    RETURN QUERY SELECT 
        v_users_processed,
        v_records_deleted,
        v_records_inserted,
        v_streaks_updated,
        EXTRACT(MILLISECONDS FROM (clock_timestamp() - v_start_time))::INTEGER;
END;
$function$;

COMMIT;
