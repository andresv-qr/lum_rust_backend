-- Migration: Fix batch_consistent_month function
-- Date: 2025-12-01
-- Issue: Function was writing to non-existent table fact_user_streaks
-- Fix: Update to use user_streaks and add proper reset logic

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
    v_week_start DATE;
    v_current_week_start DATE;
BEGIN
    -- Calcular inicio del rango (5 semanas hacia atrás desde el lunes de esta semana)
    v_current_week_start := DATE_TRUNC('week', CURRENT_DATE)::DATE;
    v_week_start := v_current_week_start - INTERVAL '4 weeks';
    
    RAISE NOTICE 'batch_consistent_month: Iniciando desde semana % hasta %', v_week_start, v_current_week_start;
    
    -- PASO 1: Eliminar registros consistent_month de las últimas 5 semanas (log de actividad)
    DELETE FROM gamification.fact_user_activity_log
    WHERE activity_type = 'consistent_month'
    AND created_at >= v_week_start;
    
    GET DIAGNOSTICS v_records_deleted = ROW_COUNT;
    RAISE NOTICE 'batch_consistent_month: Eliminados % registros antiguos', v_records_deleted;
    
    -- PASO 2: Insertar primera factura de cada semana por usuario
    WITH weekly_first_invoices AS (
        SELECT DISTINCT ON (ih.user_id, DATE_TRUNC('week', ih.reception_date))
            ih.user_id,
            ih.cufe as invoice_id,
            ih.reception_date,
            ih.issuer_name,
            ih.tot_amount as total_invoice_amount,
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
        wfi.user_id::INTEGER,
        'consistent_month',
        jsonb_build_object(
            'cufe', wfi.invoice_id,
            'issuer_name', wfi.issuer_name,
            'amount', wfi.total_invoice_amount,
            'week_start', wfi.week_start,
            'week_number', wfi.week_number,
            'year', wfi.year,
            'source', 'batch'
        ),
        wfi.reception_date
    FROM weekly_first_invoices wfi;
    
    GET DIAGNOSTICS v_records_inserted = ROW_COUNT;
    RAISE NOTICE 'batch_consistent_month: Insertados % registros nuevos', v_records_inserted;
    
    -- Contar usuarios únicos procesados
    SELECT COUNT(DISTINCT user_id) INTO v_users_processed
    FROM gamification.fact_user_activity_log
    WHERE activity_type = 'consistent_month'
    AND created_at >= v_week_start;

    -- PASO 3: Calcular semanas consecutivas y actualizar user_streaks
    WITH user_weekly_activity AS (
        -- Obtener semanas únicas con actividad por usuario
        SELECT 
            user_id,
            DATE_TRUNC('week', created_at)::DATE as week_start
        FROM gamification.fact_user_activity_log
        WHERE activity_type = 'consistent_month'
        AND created_at >= v_week_start
        GROUP BY user_id, DATE_TRUNC('week', created_at)
    ),
    user_streak_calc AS (
        -- Calcular semanas consecutivas hacia atrás desde la semana actual
        SELECT 
            uwa.user_id,
            COUNT(*) FILTER (WHERE uwa.week_start >= v_current_week_start - INTERVAL '3 weeks') as consecutive_weeks
        FROM user_weekly_activity uwa
        WHERE uwa.week_start IN (
            v_current_week_start,
            v_current_week_start - INTERVAL '1 week',
            v_current_week_start - INTERVAL '2 weeks',
            v_current_week_start - INTERVAL '3 weeks'
        )
        GROUP BY uwa.user_id
    )
    -- Actualizar user_streaks (tabla correcta)
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
        usc.user_id,
        'consistent_month',
        -- Si llegó a 4, resetear a 0 (el ciclo se completó)
        CASE WHEN usc.consecutive_weeks >= 4 THEN 0 ELSE usc.consecutive_weeks END,
        4, -- max_count siempre es 4 para consistent_month
        CURRENT_DATE,
        CURRENT_DATE - ((usc.consecutive_weeks - 1) * 7),
        true,
        NOW()
    FROM user_streak_calc usc
    WHERE usc.consecutive_weeks > 0
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
    RAISE NOTICE 'batch_consistent_month: Actualizados % streaks', v_streaks_updated;
    
    -- PASO 4: Otorgar recompensa a usuarios que completaron el ciclo (4 semanas)
    -- Insertar en point_ledger para los que llegaron a 4
    INSERT INTO gamification.point_ledger (
        user_id,
        amount,
        source_type,
        source_id,
        description,
        created_at
    )
    SELECT 
        usc.user_id,
        50, -- Recompensa por Perfect Month
        'achievement',
        'consistent_month_' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD'),
        'Perfect Month - 4 semanas consecutivas',
        NOW()
    FROM user_streak_calc usc
    WHERE usc.consecutive_weeks >= 4
    -- Evitar duplicados: verificar que no se haya otorgado esta semana
    AND NOT EXISTS (
        SELECT 1 FROM gamification.point_ledger pl
        WHERE pl.user_id = usc.user_id
        AND pl.source_type = 'achievement'
        AND pl.source_id LIKE 'consistent_month_%'
        AND pl.created_at >= DATE_TRUNC('week', CURRENT_DATE)
    );
    
    RETURN QUERY SELECT 
        v_users_processed,
        v_records_deleted,
        v_records_inserted,
        v_streaks_updated,
        EXTRACT(MILLISECONDS FROM (clock_timestamp() - v_start_time))::INTEGER;
END;
$function$;

-- Asegurar que todos los user_streaks de consistent_month tengan max_count = 4
UPDATE gamification.user_streaks 
SET max_count = 4 
WHERE streak_type = 'consistent_month' AND (max_count IS NULL OR max_count != 4);

-- Asegurar que todos los user_streaks de daily_login tengan max_count = 7
UPDATE gamification.user_streaks 
SET max_count = 7 
WHERE streak_type = 'daily_login' AND (max_count IS NULL OR max_count != 7);

COMMIT;
