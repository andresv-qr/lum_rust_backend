-- Migration: Fix batch_consistent_month function (v3 - FINAL)
-- Date: 2025-12-01
-- Issue: CTE scope issue in previous version
-- Fix: Use temp table or restructure queries

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
    v_rewards_given INTEGER := 0;
    v_current_week_start DATE;
    v_user_record RECORD;
BEGIN
    -- Calcular el lunes de la semana actual
    v_current_week_start := DATE_TRUNC('week', CURRENT_DATE)::DATE;
    
    RAISE NOTICE 'batch_consistent_month: Semana actual: %', v_current_week_start;
    
    -- Crear tabla temporal para almacenar los resultados del cálculo
    CREATE TEMP TABLE IF NOT EXISTS tmp_streak_calc (
        user_id INTEGER PRIMARY KEY,
        consecutive_weeks INTEGER
    ) ON COMMIT DROP;
    
    TRUNCATE tmp_streak_calc;
    
    -- PASO 1: Calcular semanas consecutivas directamente desde invoice_header
    INSERT INTO tmp_streak_calc (user_id, consecutive_weeks)
    WITH user_weekly_invoices AS (
        SELECT 
            ih.user_id,
            DATE_TRUNC('week', ih.reception_date)::DATE as week_start
        FROM public.invoice_header ih
        WHERE ih.user_id IS NOT NULL
        AND ih.reception_date >= v_current_week_start - INTERVAL '3 weeks'
        GROUP BY ih.user_id, DATE_TRUNC('week', ih.reception_date)
    ),
    consecutive_weeks_calc AS (
        SELECT 
            uwi.user_id,
            SUM(CASE WHEN uwi.week_start = v_current_week_start THEN 1 ELSE 0 END) as has_week_0,
            SUM(CASE WHEN uwi.week_start = v_current_week_start - INTERVAL '1 week' THEN 1 ELSE 0 END) as has_week_1,
            SUM(CASE WHEN uwi.week_start = v_current_week_start - INTERVAL '2 weeks' THEN 1 ELSE 0 END) as has_week_2,
            SUM(CASE WHEN uwi.week_start = v_current_week_start - INTERVAL '3 weeks' THEN 1 ELSE 0 END) as has_week_3
        FROM user_weekly_invoices uwi
        GROUP BY uwi.user_id
    )
    SELECT 
        cwc.user_id,
        CASE 
            WHEN cwc.has_week_0 = 0 THEN 0
            WHEN cwc.has_week_1 = 0 THEN 1
            WHEN cwc.has_week_2 = 0 THEN 2
            WHEN cwc.has_week_3 = 0 THEN 3
            ELSE 4
        END as consecutive_weeks
    FROM consecutive_weeks_calc cwc;
    
    GET DIAGNOSTICS v_users_processed = ROW_COUNT;
    RAISE NOTICE 'batch_consistent_month: Calculados % usuarios', v_users_processed;
    
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
        tsc.user_id,
        'consistent_month',
        CASE WHEN tsc.consecutive_weeks >= 4 THEN 0 ELSE tsc.consecutive_weeks END,
        4,
        CURRENT_DATE,
        v_current_week_start - ((GREATEST(tsc.consecutive_weeks, 1) - 1) * INTERVAL '1 week'),
        true,
        NOW()
    FROM tmp_streak_calc tsc
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
    
    -- PASO 3: Otorgar recompensa a usuarios que completaron 4 semanas
    -- Usar la función centralizada gamification.grant_achievement_reward para insertar
    -- en rewards.fact_accumulations (y crear entries en dim_accumulations si es necesario)
    v_rewards_given := 0;
    FOR v_user_record IN SELECT user_id FROM tmp_streak_calc WHERE consecutive_weeks >= 4 LOOP
        -- Verificar si ya existe acumulación para esta racha (evitar duplicados)
        IF NOT EXISTS (
            SELECT 1 FROM rewards.fact_accumulations fa
            JOIN rewards.dim_accumulations da ON fa.accum_id = da.id
            WHERE fa.user_id = v_user_record.user_id
            AND (da.name = 'gamification_consistent_month' OR da.name = 'consistent_month')
            AND fa.date >= v_current_week_start - INTERVAL '4 weeks'
        ) THEN
            PERFORM gamification.grant_achievement_reward(v_user_record.user_id, 'consistent_month');
            v_rewards_given := v_rewards_given + 1;
        END IF;
    END LOOP;

    RAISE NOTICE 'batch_consistent_month: Otorgadas % recompensas', v_rewards_given;
    
    RETURN QUERY SELECT 
        v_users_processed,
        v_records_deleted,
        v_records_inserted,
        v_streaks_updated,
        EXTRACT(MILLISECONDS FROM (clock_timestamp() - v_start_time))::INTEGER;
END;
$function$;

COMMIT;
