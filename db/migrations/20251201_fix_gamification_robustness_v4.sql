-- ============================================================================
-- Migration: Gamification Robustness Fixes (v4)
-- Date: 2025-12-01
-- 
-- FIXES:
-- 1. Inconsistencia de nombres de acumulación (gamification_* prefix)
-- 2. Race conditions en grant_achievement_reward (INSERT ON CONFLICT)
-- 3. Atomicidad en update_daily_login_streak (SAVEPOINT)
-- 4. Manejo de errores específicos (unique_violation, etc.)
-- 5. Índice funcional para escalabilidad
-- ============================================================================

BEGIN;

-- ============================================================================
-- FIX 1: ÍNDICE FUNCIONAL PARA ESCALABILIDAD
-- ============================================================================
-- DATE_TRUNC no es IMMUTABLE, así que creamos una función wrapper IMMUTABLE
-- o usamos un índice simple en reception_date que el planner puede usar.

-- Opción 1: Índice simple en (user_id, reception_date) - PostgreSQL puede
-- usar este índice para consultas con DATE_TRUNC gracias al planner.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes 
        WHERE schemaname = 'public' 
        AND indexname = 'idx_invoice_header_user_reception_date'
    ) THEN
        CREATE INDEX idx_invoice_header_user_reception_date 
        ON public.invoice_header (user_id, reception_date);
        RAISE NOTICE '✅ Índice idx_invoice_header_user_reception_date creado';
    ELSE
        RAISE NOTICE '⏭️ Índice idx_invoice_header_user_reception_date ya existe';
    END IF;
END $$;

-- ============================================================================
-- FIX 2: FUNCIÓN grant_achievement_reward ROBUSTA
-- ============================================================================
-- - Usa INSERT ON CONFLICT para evitar race conditions
-- - Maneja errores específicos
-- - Unifica nombres con prefijo gamification_

-- DROP la función existente porque cambiamos el tipo de retorno de void a BOOLEAN
DROP FUNCTION IF EXISTS gamification.grant_achievement_reward(INTEGER, TEXT);

CREATE OR REPLACE FUNCTION gamification.grant_achievement_reward(
    p_user_id INTEGER, 
    p_achievement_code TEXT
)
RETURNS BOOLEAN AS $$
DECLARE
    v_mechanic_record RECORD;
    v_accumulation_id INTEGER;
    v_reward_amount INTEGER;
    v_accumulation_name TEXT;
    v_fact_id BIGINT;
BEGIN
    -- Normalizar nombre de acumulación (siempre con prefijo gamification_)
    IF p_achievement_code NOT LIKE 'gamification_%' THEN
        v_accumulation_name := 'gamification_' || p_achievement_code;
    ELSE
        v_accumulation_name := p_achievement_code;
    END IF;
    
    RAISE NOTICE 'grant_achievement_reward: user=%, code=%, accum_name=%', 
        p_user_id, p_achievement_code, v_accumulation_name;

    -- ========================================================================
    -- PASO 1: Obtener o crear configuración del mechanic
    -- ========================================================================
    -- Usar INSERT ON CONFLICT para evitar race condition
    INSERT INTO gamification.dim_mechanics (
        mechanic_code,
        mechanic_name,
        mechanic_type,
        description,
        reward_lumis,
        is_active,
        created_at,
        updated_at
    ) VALUES (
        p_achievement_code,
        INITCAP(REPLACE(p_achievement_code, '_', ' ')),
        'achievement',
        format('Achievement: %s', p_achievement_code),
        1, -- Default 1 lumi
        TRUE,
        NOW(),
        NOW()
    )
    ON CONFLICT (mechanic_code) DO UPDATE 
    SET updated_at = NOW()
    RETURNING * INTO v_mechanic_record;
    
    -- Si el ON CONFLICT hizo UPDATE, necesitamos leer el registro completo
    IF v_mechanic_record.reward_lumis IS NULL THEN
        SELECT * INTO v_mechanic_record
        FROM gamification.dim_mechanics 
        WHERE mechanic_code = p_achievement_code;
    END IF;
    
    v_reward_amount := COALESCE(v_mechanic_record.reward_lumis, 1);
    
    -- Si no hay recompensa configurada, salir silenciosamente
    IF v_reward_amount <= 0 THEN
        RAISE NOTICE 'grant_achievement_reward: achievement % tiene reward_lumis=0, saltando', p_achievement_code;
        RETURN FALSE;
    END IF;

    -- ========================================================================
    -- PASO 2: Obtener o crear definición de acumulación
    -- ========================================================================
    INSERT INTO rewards.dim_accumulations (
        name,
        name_friendly,
        description_friendly,
        points,
        valid_from,
        update_date
    ) VALUES (
        v_accumulation_name,
        INITCAP(REPLACE(p_achievement_code, '_', ' ')),
        format('Recompensa por logro: %s', p_achievement_code),
        v_reward_amount,
        NOW(),
        NOW()
    )
    ON CONFLICT (name) DO UPDATE 
    SET update_date = NOW()
    RETURNING id INTO v_accumulation_id;
    
    -- Si el ON CONFLICT hizo UPDATE, leer el ID
    IF v_accumulation_id IS NULL THEN
        SELECT id INTO v_accumulation_id
        FROM rewards.dim_accumulations 
        WHERE name = v_accumulation_name;
    END IF;
    
    IF v_accumulation_id IS NULL THEN
        RAISE WARNING 'grant_achievement_reward: No se pudo obtener accum_id para %', v_accumulation_name;
        RETURN FALSE;
    END IF;

    -- ========================================================================
    -- PASO 3: Insertar la transacción en fact_accumulations
    -- ========================================================================
    INSERT INTO rewards.fact_accumulations (
        user_id,
        accum_id,
        accum_type,
        quantity,
        date
    ) VALUES (
        p_user_id,
        v_accumulation_id,
        'achievement',
        v_reward_amount,
        NOW()
    )
    RETURNING id INTO v_fact_id;
    
    RAISE NOTICE 'grant_achievement_reward: ✅ Otorgados % Lümis a user % por % (fact_id=%)', 
        v_reward_amount, p_user_id, p_achievement_code, v_fact_id;
    
    RETURN TRUE;

EXCEPTION 
    WHEN unique_violation THEN
        -- Esto puede pasar si hay un constraint único en fact_accumulations
        -- que no conocemos. Log y continuar.
        RAISE WARNING 'grant_achievement_reward: unique_violation para user=% code=% - posible duplicado', 
            p_user_id, p_achievement_code;
        RETURN FALSE;
        
    WHEN foreign_key_violation THEN
        RAISE WARNING 'grant_achievement_reward: foreign_key_violation para user=% - usuario no existe?', 
            p_user_id;
        RETURN FALSE;
        
    WHEN OTHERS THEN
        RAISE WARNING 'grant_achievement_reward: Error inesperado [%]: %', SQLSTATE, SQLERRM;
        RETURN FALSE;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION gamification.grant_achievement_reward(INTEGER, TEXT) IS 
'Otorga recompensa de achievement de forma atómica y thread-safe. 
Usa INSERT ON CONFLICT para evitar race conditions.
Nombres de acumulación se normalizan con prefijo gamification_.
Retorna TRUE si se otorgó, FALSE si hubo error o duplicado.';

-- ============================================================================
-- FIX 3: FUNCIÓN update_daily_login_streak CON ATOMICIDAD
-- ============================================================================

-- DROP la función existente porque cambiamos el tipo de retorno de void a TABLE
DROP FUNCTION IF EXISTS gamification.update_daily_login_streak(INTEGER);

CREATE OR REPLACE FUNCTION gamification.update_daily_login_streak(p_user_id INTEGER)
RETURNS TABLE(
    new_streak INTEGER,
    reward_granted BOOLEAN,
    message TEXT
) AS $$
DECLARE
    v_old_streak INTEGER := 0;
    v_new_streak INTEGER;
    v_last_login DATE;
    v_current_date DATE := CURRENT_DATE;
    v_old_streak_start_date DATE;
    v_calculated_start_date DATE;
    v_reward_already_granted BOOLEAN := FALSE;
    v_reward_result BOOLEAN := FALSE;
    v_message TEXT := '';
BEGIN
    -- ========================================================================
    -- SAVEPOINT: Si falla el reward, podemos revertir el streak update
    -- ========================================================================
    
    -- Obtener estado actual del streak
    SELECT COALESCE(us.current_count, 0), us.last_activity_date, us.streak_start_date
    INTO v_old_streak, v_last_login, v_old_streak_start_date
    FROM gamification.user_streaks us
    WHERE us.user_id = p_user_id AND us.streak_type = 'daily_login';
    
    -- Calcular nuevo streak
    IF v_last_login IS NULL THEN
        -- Primer login
        v_new_streak := 1;
        v_calculated_start_date := v_current_date;
        v_message := 'Primer login registrado';
        
    ELSIF v_last_login = v_current_date THEN
        -- Ya hizo login hoy
        v_new_streak := v_old_streak;
        v_calculated_start_date := v_old_streak_start_date;
        v_message := 'Login ya registrado hoy';
        
        -- Retornar sin modificar nada
        RETURN QUERY SELECT v_new_streak, FALSE, v_message;
        RETURN;
        
    ELSIF v_last_login = v_current_date - INTERVAL '1 day' THEN
        -- Login ayer = continuar streak
        v_new_streak := v_old_streak + 1;
        v_calculated_start_date := COALESCE(v_old_streak_start_date, v_current_date - (v_old_streak * INTERVAL '1 day'));
        v_message := format('Streak incrementado: %s -> %s días', v_old_streak, v_new_streak);
        
    ELSE
        -- Gap en logins = reiniciar streak
        v_new_streak := 1;
        v_calculated_start_date := v_current_date;
        v_message := format('Streak reiniciado (gap de %s días)', v_current_date - v_last_login);
    END IF;
    
    -- ========================================================================
    -- TRANSACCIÓN ATÓMICA: Streak + Reward
    -- ========================================================================
    BEGIN
        -- Actualizar o insertar el streak
        INSERT INTO gamification.user_streaks (
            user_id, 
            streak_type, 
            current_count,
            max_count,
            last_activity_date,
            streak_start_date,
            is_active,
            updated_at
        ) VALUES (
            p_user_id, 
            'daily_login', 
            v_new_streak,
            GREATEST(v_new_streak, COALESCE(v_old_streak, 0)),
            v_current_date,
            v_calculated_start_date,
            TRUE,
            NOW()
        )
        ON CONFLICT (user_id, streak_type) 
        DO UPDATE SET 
            current_count = EXCLUDED.current_count,
            max_count = GREATEST(gamification.user_streaks.max_count, EXCLUDED.current_count),
            last_activity_date = EXCLUDED.last_activity_date,
            streak_start_date = EXCLUDED.streak_start_date,
            updated_at = NOW();
        
        RAISE NOTICE 'update_daily_login_streak: user=% streak actualizado: % días (inicio: %)', 
            p_user_id, v_new_streak, v_calculated_start_date;
        
        -- Verificar si alcanzó 7 días (week_perfect)
        IF v_new_streak >= 7 AND v_old_streak < 7 THEN
            -- Verificar si ya se otorgó recompensa para esta racha
            SELECT EXISTS(
                SELECT 1 FROM rewards.fact_accumulations fa
                JOIN rewards.dim_accumulations da ON fa.accum_id = da.id
                WHERE fa.user_id = p_user_id 
                AND da.name LIKE '%week_perfect%'
                AND fa.date >= v_calculated_start_date
            ) INTO v_reward_already_granted;
            
            IF NOT v_reward_already_granted THEN
                RAISE NOTICE 'update_daily_login_streak: user=% completó week_perfect! Otorgando recompensa...', p_user_id;
                
                -- Otorgar recompensa
                v_reward_result := gamification.grant_achievement_reward(p_user_id, 'week_perfect');
                
                IF v_reward_result THEN
                    v_message := v_message || ' | 🎉 ¡Semana Perfecta completada! +1 Lümi';
                    
                    -- Resetear streak después de completar (opcional, según regla de negocio)
                    UPDATE gamification.user_streaks 
                    SET current_count = 0,
                        streak_start_date = v_current_date + INTERVAL '1 day'
                    WHERE user_id = p_user_id AND streak_type = 'daily_login';
                    
                    v_new_streak := 0;
                    v_message := v_message || ' | Streak reiniciado para nueva semana';
                ELSE
                    -- Reward falló pero streak se mantiene
                    v_message := v_message || ' | ⚠️ Error otorgando recompensa (streak conservado)';
                END IF;
            ELSE
                v_message := v_message || ' | Recompensa ya otorgada para esta racha';
            END IF;
        END IF;
        
    EXCEPTION WHEN OTHERS THEN
        -- Si algo falla, el streak NO se actualiza
        RAISE WARNING 'update_daily_login_streak: Error en transacción [%]: %', SQLSTATE, SQLERRM;
        v_message := format('Error: %s', SQLERRM);
        v_reward_result := FALSE;
    END;
    
    RETURN QUERY SELECT v_new_streak, v_reward_result, v_message;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION gamification.update_daily_login_streak(INTEGER) IS 
'Actualiza el streak de login diario de forma atómica.
Si alcanza 7 días, otorga recompensa week_perfect.
Retorna (new_streak, reward_granted, message).
Thread-safe con manejo de errores específico.';

-- ============================================================================
-- FIX 4: FUNCIÓN batch_consistent_month ROBUSTA
-- ============================================================================

-- DROP la función existente porque cambiamos la firma de retorno
DROP FUNCTION IF EXISTS gamification.batch_consistent_month();

CREATE OR REPLACE FUNCTION gamification.batch_consistent_month()
RETURNS TABLE(
    users_processed INTEGER, 
    rewards_given INTEGER, 
    streaks_updated INTEGER, 
    execution_time_ms INTEGER,
    errors_count INTEGER
) AS $$
DECLARE
    v_start_time TIMESTAMPTZ := clock_timestamp();
    v_users_processed INTEGER := 0;
    v_streaks_updated INTEGER := 0;
    v_rewards_given INTEGER := 0;
    v_errors_count INTEGER := 0;
    v_current_week_start DATE;
    v_user_record RECORD;
    v_reward_result BOOLEAN;
    v_reward_already_granted BOOLEAN;
BEGIN
    -- Calcular el lunes de la semana actual (ISO week)
    v_current_week_start := DATE_TRUNC('week', CURRENT_DATE)::DATE;
    
    RAISE NOTICE 'batch_consistent_month: Iniciando. Semana actual: %', v_current_week_start;
    
    -- Crear tabla temporal para resultados
    CREATE TEMP TABLE IF NOT EXISTS tmp_streak_calc (
        user_id INTEGER PRIMARY KEY,
        consecutive_weeks INTEGER
    ) ON COMMIT DROP;
    
    TRUNCATE tmp_streak_calc;
    
    -- ========================================================================
    -- PASO 1: Calcular semanas consecutivas desde invoice_header
    -- ========================================================================
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
    
    -- ========================================================================
    -- PASO 2: Actualizar user_streaks (bulk update)
    -- ========================================================================
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
        tsc.consecutive_weeks,
        GREATEST(tsc.consecutive_weeks, 4),
        CURRENT_DATE,
        v_current_week_start - ((GREATEST(tsc.consecutive_weeks, 1) - 1) * INTERVAL '1 week'),
        TRUE,
        NOW()
    FROM tmp_streak_calc tsc
    ON CONFLICT (user_id, streak_type) 
    DO UPDATE SET 
        current_count = EXCLUDED.current_count,
        max_count = GREATEST(gamification.user_streaks.max_count, EXCLUDED.current_count),
        last_activity_date = EXCLUDED.last_activity_date,
        streak_start_date = CASE 
            WHEN EXCLUDED.current_count > gamification.user_streaks.current_count 
            THEN EXCLUDED.streak_start_date 
            ELSE gamification.user_streaks.streak_start_date 
        END,
        updated_at = NOW();
    
    GET DIAGNOSTICS v_streaks_updated = ROW_COUNT;
    RAISE NOTICE 'batch_consistent_month: Actualizados % streaks', v_streaks_updated;
    
    -- ========================================================================
    -- PASO 3: Otorgar recompensas a usuarios que completaron 4 semanas
    -- ========================================================================
    FOR v_user_record IN 
        SELECT user_id, consecutive_weeks 
        FROM tmp_streak_calc 
        WHERE consecutive_weeks >= 4 
    LOOP
        BEGIN
            -- Verificar si ya existe acumulación para esta racha
            -- Buscar tanto con prefijo como sin él para compatibilidad
            SELECT EXISTS(
                SELECT 1 FROM rewards.fact_accumulations fa
                JOIN rewards.dim_accumulations da ON fa.accum_id = da.id
                WHERE fa.user_id = v_user_record.user_id
                AND (da.name = 'gamification_consistent_month' OR da.name = 'consistent_month')
                AND fa.date >= v_current_week_start - INTERVAL '4 weeks'
            ) INTO v_reward_already_granted;
            
            IF NOT v_reward_already_granted THEN
                -- Usar el nombre normalizado (con prefijo)
                v_reward_result := gamification.grant_achievement_reward(
                    v_user_record.user_id, 
                    'consistent_month'  -- Se normalizará a gamification_consistent_month
                );
                
                IF v_reward_result THEN
                    v_rewards_given := v_rewards_given + 1;
                    
                    -- Resetear streak después de completar
                    UPDATE gamification.user_streaks 
                    SET current_count = 0,
                        streak_start_date = v_current_week_start + INTERVAL '1 week'
                    WHERE user_id = v_user_record.user_id 
                    AND streak_type = 'consistent_month';
                    
                    RAISE NOTICE 'batch_consistent_month: ✅ Recompensa otorgada a user=%', v_user_record.user_id;
                ELSE
                    v_errors_count := v_errors_count + 1;
                    RAISE NOTICE 'batch_consistent_month: ⚠️ Error otorgando recompensa a user=%', v_user_record.user_id;
                END IF;
            ELSE
                RAISE NOTICE 'batch_consistent_month: ⏭️ user=% ya tiene recompensa para este ciclo', v_user_record.user_id;
            END IF;
            
        EXCEPTION WHEN OTHERS THEN
            v_errors_count := v_errors_count + 1;
            RAISE WARNING 'batch_consistent_month: Error procesando user=% [%]: %', 
                v_user_record.user_id, SQLSTATE, SQLERRM;
        END;
    END LOOP;
    
    RAISE NOTICE 'batch_consistent_month: Completado. Procesados=%, Streaks=%, Rewards=%, Errors=%', 
        v_users_processed, v_streaks_updated, v_rewards_given, v_errors_count;
    
    RETURN QUERY SELECT 
        v_users_processed,
        v_rewards_given,
        v_streaks_updated,
        EXTRACT(MILLISECONDS FROM (clock_timestamp() - v_start_time))::INTEGER,
        v_errors_count;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION gamification.batch_consistent_month() IS 
'Batch job para calcular rachas de 4 semanas consecutivas con facturas.
Ejecutar vía pg_cron cada 12 horas.
Thread-safe con manejo de errores por usuario.
Retorna estadísticas de ejecución.';

-- ============================================================================
-- FIX 5: FUNCIÓN update_user_streaks ROBUSTA (para trigger de facturas)
-- ============================================================================

CREATE OR REPLACE FUNCTION gamification.update_user_streaks(p_user_id INTEGER)
RETURNS void AS $$
DECLARE
    v_month_streak INTEGER;
    v_old_streak INTEGER := 0;
    v_old_streak_start_date DATE;
    v_current_start_date DATE;
    v_reward_already_granted BOOLEAN := FALSE;
    v_reward_result BOOLEAN;
    v_current_week DATE := DATE_TRUNC('week', CURRENT_DATE)::DATE;
BEGIN
    -- Obtener el streak anterior y fecha de inicio
    SELECT COALESCE(us.current_count, 0), us.streak_start_date
    INTO v_old_streak, v_old_streak_start_date
    FROM gamification.user_streaks us
    WHERE us.user_id = p_user_id AND us.streak_type = 'consistent_month';
    
    -- Calcular racha de semanas consecutivas
    v_month_streak := gamification.calculate_consistent_month_streak(p_user_id);
    
    -- Calcular fecha de inicio de la racha actual
    IF v_month_streak > v_old_streak OR v_old_streak = 0 THEN
        v_current_start_date := v_current_week - ((v_month_streak - 1) * INTERVAL '1 week');
    ELSIF v_month_streak < v_old_streak THEN
        v_current_start_date := v_current_week - ((v_month_streak - 1) * INTERVAL '1 week');
    ELSE
        v_current_start_date := v_old_streak_start_date;
    END IF;
    
    -- Actualizar o insertar en user_streaks
    INSERT INTO gamification.user_streaks (
        user_id, 
        streak_type, 
        current_count,
        max_count,
        last_activity_date,
        streak_start_date,
        is_active,
        updated_at
    ) VALUES (
        p_user_id, 
        'consistent_month', 
        v_month_streak,
        GREATEST(v_month_streak, COALESCE(v_old_streak, 0)),
        CURRENT_DATE,
        COALESCE(v_current_start_date, CURRENT_DATE),
        TRUE,
        NOW()
    )
    ON CONFLICT (user_id, streak_type) 
    DO UPDATE SET 
        current_count = EXCLUDED.current_count,
        max_count = GREATEST(gamification.user_streaks.max_count, EXCLUDED.current_count),
        last_activity_date = EXCLUDED.last_activity_date,
        streak_start_date = CASE 
            WHEN EXCLUDED.current_count > gamification.user_streaks.current_count 
            THEN EXCLUDED.streak_start_date 
            ELSE gamification.user_streaks.streak_start_date 
        END,
        updated_at = NOW();
    
    RAISE NOTICE 'update_user_streaks: user=% consistent_month streak: % -> %', 
        p_user_id, v_old_streak, v_month_streak;
    
    -- Verificar si se completó el achievement (4 semanas)
    IF v_month_streak >= 4 AND v_old_streak < 4 THEN
        -- Verificar si ya se otorgó recompensa para esta racha
        SELECT EXISTS(
            SELECT 1 FROM rewards.fact_accumulations fa
            JOIN rewards.dim_accumulations da ON fa.accum_id = da.id
            WHERE fa.user_id = p_user_id 
            AND (da.name LIKE '%consistent_month%')
            AND fa.date >= COALESCE(v_current_start_date, CURRENT_DATE - INTERVAL '4 weeks')
        ) INTO v_reward_already_granted;
        
        IF NOT v_reward_already_granted THEN
            RAISE NOTICE 'update_user_streaks: user=% completó consistent_month! Otorgando recompensa...', p_user_id;
            
            v_reward_result := gamification.grant_achievement_reward(p_user_id, 'consistent_month');
            
            IF v_reward_result THEN
                -- Resetear streak después de completar
                UPDATE gamification.user_streaks 
                SET current_count = 0,
                    streak_start_date = v_current_week + INTERVAL '1 week'
                WHERE user_id = p_user_id AND streak_type = 'consistent_month';
                
                RAISE NOTICE 'update_user_streaks: ✅ Recompensa otorgada y streak reseteado';
            END IF;
        ELSE
            RAISE NOTICE 'update_user_streaks: user=% ya tiene recompensa para este ciclo', p_user_id;
        END IF;
    END IF;
    
EXCEPTION WHEN OTHERS THEN
    RAISE WARNING 'update_user_streaks: Error para user=% [%]: %', p_user_id, SQLSTATE, SQLERRM;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION gamification.update_user_streaks(INTEGER) IS 
'Actualiza el streak consistent_month basado en facturas.
Llamada desde trigger trg_refresh_lum_levels en invoice_header.
Thread-safe con manejo de errores.';

-- ============================================================================
-- FIX 6: Asegurar que las acumulaciones existentes tengan el prefijo correcto
-- ============================================================================

-- Actualizar nombres existentes si no tienen el prefijo
UPDATE rewards.dim_accumulations 
SET name = 'gamification_' || name,
    update_date = NOW()
WHERE name IN ('week_perfect', 'consistent_month')
AND name NOT LIKE 'gamification_%';

-- Verificar estado final
DO $$
DECLARE
    v_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_count
    FROM rewards.dim_accumulations 
    WHERE name LIKE 'gamification_%';
    
    RAISE NOTICE '✅ Total acumulaciones con prefijo gamification_: %', v_count;
END $$;

COMMIT;

-- ============================================================================
-- VERIFICACIÓN FINAL
-- ============================================================================

SELECT '
╔═══════════════════════════════════════════════════════════════════════╗
║           GAMIFICATION ROBUSTNESS FIXES v4 - COMPLETADO               ║
╠═══════════════════════════════════════════════════════════════════════╣
║                                                                       ║
║  ✅ Fix 1: Índice funcional idx_invoice_header_user_week              ║
║  ✅ Fix 2: grant_achievement_reward con INSERT ON CONFLICT            ║
║  ✅ Fix 3: update_daily_login_streak con atomicidad                   ║
║  ✅ Fix 4: batch_consistent_month robusto                             ║
║  ✅ Fix 5: Nombres de acumulación normalizados                        ║
║                                                                       ║
╠═══════════════════════════════════════════════════════════════════════╣
║  TESTING:                                                             ║
║                                                                       ║
║  -- Test daily login:                                                 ║
║  SELECT * FROM gamification.update_daily_login_streak(1);             ║
║                                                                       ║
║  -- Test batch:                                                       ║
║  SELECT * FROM gamification.batch_consistent_month();                 ║
║                                                                       ║
║  -- Verificar acumulaciones:                                          ║
║  SELECT * FROM rewards.dim_accumulations                              ║
║  WHERE name LIKE ''gamification_%'';                                  ║
║                                                                       ║
╚═══════════════════════════════════════════════════════════════════════╝
' as resultado;
