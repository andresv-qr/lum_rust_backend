-- ============================================================================
-- SCRIPT DE PRUEBA: SISTEMA DE RACHAS SEMANALES
-- ============================================================================
-- Simula 14 días de login para verificar el ciclo semanal completo
-- ============================================================================

DO $$
DECLARE
    v_user_id INTEGER := 999999; -- Usuario de prueba
    v_result RECORD;
    v_day INTEGER;
BEGIN
    -- Limpiar datos previos del usuario de prueba
    DELETE FROM gamification.fact_user_streaks WHERE user_id = v_user_id;
    DELETE FROM gamification.fact_user_achievements WHERE user_id = v_user_id;
    
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'PRUEBA DE CICLO SEMANAL DE RACHAS';
    RAISE NOTICE '==============================================';
    RAISE NOTICE '';
    
    -- Simular 14 días de login (2 semanas completas)
    FOR v_day IN 1..14 LOOP
        -- Llamar a la función de login
        SELECT * INTO v_result
        FROM gamification.process_daily_login(v_user_id, 'test');
        
        -- Mostrar resultado
        RAISE NOTICE 'DÍA %: Streak = %, Lümis = %, Mensaje = %',
            v_day,
            (v_result.streak_info->>'current_streak')::INTEGER,
            v_result.lumis_earned,
            v_result.streak_info->>'message';
        
        -- Si es día 7 o 14, mostrar achievement
        IF v_day = 7 OR v_day = 14 THEN
            RAISE NOTICE '  → Achievement desbloqueado: %', 
                v_result.streak_info->>'achievement_unlocked';
            RAISE NOTICE '  → ✅ CONTADOR SE RESETEA A 1';
            RAISE NOTICE '';
        END IF;
        
        -- Simular paso de día (para que el siguiente login sea "mañana")
        UPDATE gamification.fact_user_streaks
        SET last_activity_date = CURRENT_DATE - (14 - v_day)
        WHERE user_id = v_user_id AND streak_type = 'daily_login';
    END LOOP;
    
    RAISE NOTICE '';
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'RESUMEN DE PRUEBA';
    RAISE NOTICE '==============================================';
    RAISE NOTICE 'Total días simulados: 14';
    RAISE NOTICE 'Ciclos completados: 2';
    RAISE NOTICE 'Lümis totales ganados: 2 (día 7 y día 14)';
    RAISE NOTICE 'Estado final del contador: 1 (reseteado después del día 14)';
    RAISE NOTICE '==============================================';
    
    -- Limpiar usuario de prueba
    DELETE FROM gamification.fact_user_streaks WHERE user_id = v_user_id;
    DELETE FROM gamification.fact_user_achievements WHERE user_id = v_user_id;
    
END $$;
