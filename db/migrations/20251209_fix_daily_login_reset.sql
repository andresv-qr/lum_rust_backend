-- Migration: Fix daily_login streak reset
-- Date: 2025-12-09
-- Description: Resets the daily_login streak to 1 if the user logs in the day after completing a 7-day cycle.

DROP FUNCTION IF EXISTS gamification.update_daily_login_streak(integer);

CREATE OR REPLACE FUNCTION gamification.update_daily_login_streak(p_user_id INTEGER)
RETURNS void AS $$
DECLARE
    old_streak INTEGER := 0;
    new_streak INTEGER;
    last_login DATE;
    current_date_only DATE := CURRENT_DATE;
    old_streak_start_date DATE;
    calculated_start_date DATE;
    reward_already_granted BOOLEAN := FALSE;
    STREAK_CYCLE_LIMIT INTEGER := 7; -- Cycle length
BEGIN
    -- Obtener el streak anterior, última fecha de login y fecha de inicio de racha
    SELECT COALESCE(fus.current_count, 0), fus.last_activity_date, fus.streak_start_date
    INTO old_streak, last_login, old_streak_start_date
    FROM gamification.user_streaks fus
    WHERE fus.user_id = p_user_id AND fus.streak_type = 'daily_login';
    
    -- Calcular nuevo streak basado en login consecutivos
    IF last_login IS NULL THEN
        -- Primer login
        new_streak := 1;
        calculated_start_date := current_date_only;
    ELSIF last_login = current_date_only THEN
        -- Ya hizo login hoy, mantener streak
        new_streak := old_streak;
        calculated_start_date := old_streak_start_date; -- Mantener fecha de inicio original
        RETURN; -- No hacer nada si ya hizo login hoy
    ELSIF last_login = current_date_only - INTERVAL '1 day' THEN
        -- Login ayer, continuar streak O reiniciar si completó ciclo
        IF old_streak >= STREAK_CYCLE_LIMIT THEN
             -- Ciclo completado ayer. Iniciar nuevo ciclo hoy.
             new_streak := 1;
             calculated_start_date := current_date_only;
             RAISE NOTICE 'Usuario % completó ciclo de % días ayer. Iniciando nuevo ciclo.', p_user_id, STREAK_CYCLE_LIMIT;
        ELSE
             -- Continuar ciclo
             new_streak := old_streak + 1;
             calculated_start_date := COALESCE(old_streak_start_date, current_date_only - (old_streak * INTERVAL '1 day'));
        END IF;
    ELSE
        -- Gap en logins (más de 1 día), reiniciar streak
        new_streak := 1;
        calculated_start_date := current_date_only;
    END IF;
    
    -- Actualizar o insertar el streak
    INSERT INTO gamification.user_streaks (
        user_id, 
        streak_type, 
        current_count,
        last_activity_date,
        streak_start_date
    ) VALUES (
        p_user_id, 
        'daily_login', 
        new_streak,
        current_date_only,
        calculated_start_date
    )
    ON CONFLICT (user_id, streak_type) 
    DO UPDATE SET 
        current_count = EXCLUDED.current_count,
        last_activity_date = EXCLUDED.last_activity_date,
        streak_start_date = EXCLUDED.streak_start_date;
    
    RAISE NOTICE 'Usuario % daily login streak actualizado: % días (inicio: %)', p_user_id, new_streak, calculated_start_date;
    
    -- Verificar achievements de daily login (week_perfect = 7 días)
    -- Solo otorgar si llegamos EXACTAMENTE a 7 (o al límite)
    IF new_streak = STREAK_CYCLE_LIMIT THEN
        -- Verificar si ya se otorgó recompensa para esta racha específica
        SELECT EXISTS(
            SELECT 1 FROM rewards.fact_accumulations fa
            JOIN rewards.dim_accumulations da ON fa.accum_id = da.id
            WHERE fa.user_id = p_user_id 
            AND da.name LIKE '%week_perfect%'
            AND fa.date >= calculated_start_date
        ) INTO reward_already_granted;
        
        IF NOT reward_already_granted THEN
            RAISE NOTICE 'Usuario % completó week_perfect achievement! Otorgando recompensa (racha iniciada: %)...', p_user_id, calculated_start_date;
            PERFORM gamification.grant_achievement_reward(p_user_id, 'week_perfect');
        ELSE
            RAISE NOTICE 'Usuario % ya recibió recompensa week_perfect para esta racha (iniciada: %)', p_user_id, calculated_start_date;
        END IF;
    END IF;
    
END;
$$ LANGUAGE plpgsql;
