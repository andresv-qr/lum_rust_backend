-- ============================================================================
-- SISTEMA DE RACHAS SEMANALES - IMPLEMENTACIÓN CORRECTA
-- ============================================================================
-- Fecha: 2025-10-30
-- Descripción: Implementa sistema de racha semanal donde:
--   - Días 1-6: Sin recompensa (solo progreso)
--   - Día 7: 1 Lümi + Achievement + Reseteo automático a 1
--   - Ciclo se repite infinitamente
-- ============================================================================

-- 1. BACKUP YA CREADO: gamification.fact_user_streaks_backup_20251030

-- 2. ACTUALIZAR FUNCIÓN DE PROCESAMIENTO DE LOGIN DIARIO
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
BEGIN
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
            'lumis_at_day_7', 1,
            'message', 'Ya registraste tu ingreso hoy'
        );
        RETURN QUERY SELECT 0, v_streak_info;
        RETURN;
    END IF;
    
    -- Update streak with weekly reset logic
    IF v_last_login = CURRENT_DATE - 1 THEN
        -- Login consecutivo
        IF v_current_streak = 7 THEN
            -- ✅ RESETEO SEMANAL: Al completar día 7, vuelve a 1
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
    
    -- ✅ RECOMPENSAS: Solo día 7 da 1 Lümi
    v_lumis_earned := CASE 
        WHEN v_current_streak = 7 THEN 1
        ELSE 0  -- Días 1-6: sin Lümis
    END;
    
    -- ✅ ACHIEVEMENT: Solo al completar día 7
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
            WHEN v_current_streak = 7 THEN '🏆 ¡Semana perfecta! +1 Lümi. Contador resetea para nueva semana'
            WHEN v_current_streak = 1 THEN format('Día 1 de 7 - ¡Comienza tu racha!')
            ELSE format('Día %s de 7 - ¡Sigue así!', v_current_streak)
        END,
        'days_until_reward', 7 - v_current_streak,
        'weekly_cycle', true
    );
    
    RETURN QUERY SELECT v_lumis_earned, v_streak_info;
END;
$$ LANGUAGE plpgsql;

-- 3. RESETEAR TODOS LOS STREAKS ACTUALES A 1 (según tu solicitud)
UPDATE gamification.fact_user_streaks
SET current_count = 1,
    updated_at = NOW()
WHERE streak_type = 'daily_login';

-- 4. ACTUALIZAR CONFIGURACIÓN DE QUEST (opcional, para UI)
UPDATE gamification.dim_quests
SET config_json = jsonb_set(
    config_json,
    '{rewards}',
    '[
        {"day": 1, "lumis": 0, "message": "Día 1 de 7 - ¡Mantén tu racha!"},
        {"day": 2, "lumis": 0, "message": "Día 2 de 7 - ¡Vas bien!"},
        {"day": 3, "lumis": 0, "message": "Día 3 de 7 - ¡Sigue así!"},
        {"day": 4, "lumis": 0, "message": "Día 4 de 7 - ¡Ya casi!"},
        {"day": 5, "lumis": 0, "message": "Día 5 de 7 - ¡Un poco más!"},
        {"day": 6, "lumis": 0, "message": "Día 6 de 7 - ¡Último esfuerzo!"},
        {"day": 7, "lumis": 1, "message": "🏆 ¡Semana perfecta! +1 Lümi (resetea)", "badge": "week_perfect"}
    ]'::jsonb
)
WHERE quest_code = 'daily_login_streak';

-- 5. VERIFICACIÓN: Ver distribución actual de streaks
SELECT 
    current_count as dias_racha,
    COUNT(*) as usuarios,
    ARRAY_AGG(user_id ORDER BY user_id LIMIT 5) as ejemplo_usuarios
FROM gamification.fact_user_streaks
WHERE streak_type = 'daily_login'
GROUP BY current_count
ORDER BY current_count;

-- 6. LOG DE CAMBIOS
DO $$
BEGIN
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'SISTEMA DE RACHAS SEMANALES IMPLEMENTADO';
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'Cambios aplicados:';
    RAISE NOTICE '1. Función process_daily_login() actualizada';
    RAISE NOTICE '2. Días 1-6: 0 Lümis (solo progreso)';
    RAISE NOTICE '3. Día 7: 1 Lümi + Achievement';
    RAISE NOTICE '4. Día 8: Contador resetea automáticamente a 1';
    RAISE NOTICE '5. Todos los streaks actuales reseteados a 1';
    RAISE NOTICE '6. Sistema de ciclo semanal infinito activo';
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'Backup guardado en: fact_user_streaks_backup_20251030';
    RAISE NOTICE '==============================================';
END $$;
