-- ============================================================================
-- CORRECCIÓN: LEER REWARD_LUMIS DINÁMICAMENTE DESDE DIM_ACHIEVEMENTS
-- ============================================================================
-- Fecha: 2025-10-30
-- Descripción: Modificar process_daily_login() para que lea reward_lumis
--              desde gamification.dim_achievements (achievement_code = 'week_perfect')
--              en lugar de tener el valor hardcodeado en 1
-- ============================================================================

DROP FUNCTION IF EXISTS gamification.process_daily_login(INTEGER, VARCHAR);
CREATE OR REPLACE FUNCTION gamification.process_daily_login(
    p_user_id INTEGER,
    p_channel VARCHAR
)
RETURNS TABLE(lumis_earned INTEGER, streak_info JSONB) AS $$
DECLARE
    v_last_login DATE;
    v_current_streak INTEGER;
    v_lumis_earned INTEGER := 0;
    v_streak_info JSONB;
    v_achievement VARCHAR;
    v_total_weeks_completed INTEGER := 0;
    v_reward_lumis INTEGER; -- ✅ NUEVO: Variable para almacenar reward_lumis de dim_achievements
BEGIN
    -- ✅ OBTENER REWARD_LUMIS DESDE DIM_ACHIEVEMENTS
    SELECT reward_lumis INTO v_reward_lumis
    FROM gamification.dim_achievements
    WHERE achievement_code = 'week_perfect';
    
    -- Validación: Si no existe el achievement, usar 1 como fallback
    IF v_reward_lumis IS NULL THEN
        v_reward_lumis := 1;
        RAISE WARNING 'Achievement week_perfect no encontrado. Usando valor por defecto: 1 Lümi';
    END IF;

    -- Get or create streak record
    INSERT INTO gamification.fact_user_streaks (
        user_id, streak_type, current_count, last_activity_date, streak_start_date
    ) VALUES (
        p_user_id, 'daily_login', 0, CURRENT_DATE, CURRENT_DATE
    )
    ON CONFLICT (user_id, streak_type) DO NOTHING;
    
    -- Get current streak with lock
    SELECT last_activity_date, current_count 
    INTO v_last_login, v_current_streak
    FROM gamification.fact_user_streaks 
    WHERE user_id = p_user_id AND streak_type = 'daily_login'
    FOR UPDATE;
    
    -- Check if already logged in today
    IF v_last_login = CURRENT_DATE THEN
        v_streak_info := jsonb_build_object(
            'current_streak', v_current_streak,
            'already_claimed', true,
            'next_reward_day', 7,
            'lumis_at_day_7', v_reward_lumis, -- ✅ DINÁMICO
            'message', 'Ya registraste tu ingreso hoy'
        );
        RETURN QUERY SELECT 0, v_streak_info;
        RETURN;
    END IF;
    
    -- Update streak with weekly reset logic
    IF v_last_login = CURRENT_DATE - 1 THEN
        -- Login consecutivo
        IF v_current_streak = 7 THEN
            -- RESETEO SEMANAL: Al completar día 7, vuelve a 1
            v_current_streak := 1;
            RAISE NOTICE 'Usuario % completó semana perfecta. Contador reseteado a 1 para nuevo ciclo', p_user_id;
        ELSE
            -- Incrementar normalmente (días 1-6)
            v_current_streak := v_current_streak + 1;
        END IF;
    ELSE
        -- Gap en logins, reiniciar streak
        v_current_streak := 1;
        RAISE NOTICE 'Usuario % rompió racha. Reseteando a 1', p_user_id;
    END IF;
    
    -- ✅ RECOMPENSAS: Leer dinámicamente desde dim_achievements
    v_lumis_earned := CASE 
        WHEN v_current_streak = 7 THEN v_reward_lumis -- ✅ AHORA ES DINÁMICO
        ELSE 0  -- Días 1-6: sin Lümis
    END;
    
    -- ACHIEVEMENT: Solo al completar día 7
    v_achievement := CASE
        WHEN v_current_streak = 7 THEN 'week_perfect'
        ELSE NULL
    END;
    
    -- Update streak record
    UPDATE gamification.fact_user_streaks 
    SET current_count = v_current_streak,
        last_activity_date = CURRENT_DATE,
        max_count = GREATEST(max_count, v_current_streak),
        total_lumis_earned = total_lumis_earned + v_lumis_earned,
        updated_at = NOW()
    WHERE user_id = p_user_id AND streak_type = 'daily_login';
    
    -- Unlock achievement if applicable (solo día 7)
    IF v_achievement IS NOT NULL THEN
        INSERT INTO gamification.fact_user_achievements (user_id, achievement_id, unlocked_at)
        SELECT p_user_id, achievement_id, NOW()
        FROM gamification.dim_achievements
        WHERE achievement_code = v_achievement
        ON CONFLICT (user_id, achievement_id) DO NOTHING;
        
        RAISE NOTICE 'Usuario % desbloqueó achievement: %', p_user_id, v_achievement;
    END IF;
    
    -- Build streak info
    v_streak_info := jsonb_build_object(
        'current_streak', v_current_streak,
        'lumis_earned', v_lumis_earned,
        'max_streak', (SELECT max_count FROM gamification.fact_user_streaks 
                       WHERE user_id = p_user_id AND streak_type = 'daily_login'),
        'next_milestone', 7,
        'achievement_unlocked', v_achievement,
        'message', CASE
            WHEN v_current_streak = 7 THEN format('🏆 ¡Semana perfecta! +%s Lümi%s. Contador resetea para nueva semana', 
                                                    v_reward_lumis, 
                                                    CASE WHEN v_reward_lumis > 1 THEN 's' ELSE '' END)
            WHEN v_current_streak = 1 THEN format('Día 1 de 7 - ¡Comienza tu racha!')
            ELSE format('Día %s de 7 - ¡Sigue así!', v_current_streak)
        END,
        'days_until_reward', 7 - v_current_streak,
        'weekly_cycle', true,
        'reward_lumis', v_reward_lumis -- ✅ Incluir en respuesta para transparencia
    );
    
    RETURN QUERY SELECT v_lumis_earned, v_streak_info;
END;
$$ LANGUAGE plpgsql;

-- VERIFICACIÓN: Probar que la función lee correctamente desde dim_achievements
DO $$
DECLARE
    v_reward_value INTEGER;
BEGIN
    SELECT reward_lumis INTO v_reward_value
    FROM gamification.dim_achievements
    WHERE achievement_code = 'week_perfect';
    
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'VERIFICACIÓN DE CONFIGURACIÓN';
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'Achievement: week_perfect';
    RAISE NOTICE 'Reward Lumis configurado: %', COALESCE(v_reward_value::TEXT, 'NO ENCONTRADO');
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'La función process_daily_login() ahora lee';
    RAISE NOTICE 'dinámicamente este valor en cada ejecución.';
    RAISE NOTICE '==============================================';
END $$;
