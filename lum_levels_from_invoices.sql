-- ============================================================================
-- VISTA MATERIALIZADA PARA NIVELES LUM BASADA EN FACTURAS REALES
-- ============================================================================
-- Esta vista calcula los niveles Lum basándose directamente en public.invoice_header
-- Se actualiza automáticamente cuando se insertan nuevas facturas
-- ============================================================================

-- 1. ELIMINAR VISTA ANTERIOR SI EXISTE
DROP MATERIALIZED VIEW IF EXISTS gamification.vw_user_lum_levels CASCADE;

-- 2. CREAR VISTA MATERIALIZADA OPTIMIZADA
CREATE MATERIALIZED VIEW gamification.vw_user_lum_levels AS
WITH user_invoice_stats AS (
    -- Estadísticas base por usuario desde facturas reales
    SELECT 
        h.user_id as user_id,
        COUNT(*) as total_invoices,
        COALESCE(SUM(h.tot_amount), 0) as total_spent,
        COUNT(DISTINCT h.issuer_name) as unique_merchants,
        COUNT(DISTINCT DATE_TRUNC('month', h.reception_date)) as active_months,
        MIN(h.reception_date) as first_invoice_date,
        MAX(h.reception_date) as last_invoice_date,
        
        -- Lümis NO se calculan aquí (esos vienen del balance real del usuario)
        -- Pero mantenemos calculated_lumis para compatibilidad con API legacy
        COUNT(*) as calculated_lumis
        
    FROM public.invoice_header h
    WHERE h.user_id IS NOT NULL
    --AND h.reception_date >= '2024-01-01'::date  -- Solo facturas del año actual hacia adelante
    GROUP BY h.user_id
),
user_with_levels AS (
    -- Asignar nivel basado en NÚMERO DE FACTURAS (más simple y directo)
    SELECT 
        uis.*,
        CASE 
            WHEN uis.total_invoices >= 500 THEN 8   -- Platinum Master - 500+ facturas
            WHEN uis.total_invoices >= 250 THEN 7   -- Gold Legend - 250+ facturas
            WHEN uis.total_invoices >= 100 THEN 6   -- Gold Hunter - 100+ facturas
            WHEN uis.total_invoices >= 50 THEN 5    -- Silver Expert - 50+ facturas
            WHEN uis.total_invoices >= 25 THEN 4    -- Silver Hunter - 25+ facturas
            WHEN uis.total_invoices >= 10 THEN 3    -- Bronze Master - 10+ facturas
            WHEN uis.total_invoices >= 5 THEN 2     -- Bronze Explorer - 5+ facturas
            ELSE 1                                   -- Chispa Lüm - 0-4 facturas
        END as calculated_level,
        
        -- Progreso hacia siguiente nivel (basado en facturas)
        CASE 
            WHEN uis.total_invoices >= 500 THEN 0                    -- Ya en máximo nivel
            WHEN uis.total_invoices >= 250 THEN 500 - uis.total_invoices
            WHEN uis.total_invoices >= 100 THEN 250 - uis.total_invoices  
            WHEN uis.total_invoices >= 50 THEN 100 - uis.total_invoices
            WHEN uis.total_invoices >= 25 THEN 50 - uis.total_invoices
            WHEN uis.total_invoices >= 10 THEN 25 - uis.total_invoices
            WHEN uis.total_invoices >= 5 THEN 10 - uis.total_invoices
            ELSE 5 - uis.total_invoices
        END as invoices_to_next_level
        
    FROM user_invoice_stats uis
)
SELECT 
    uwl.user_id,
    u.email,
    u.name,
    
    -- Estadísticas de facturas
    uwl.total_invoices,
    uwl.total_spent,
    uwl.unique_merchants,
    uwl.active_months,
    uwl.first_invoice_date,
    uwl.last_invoice_date,
    
    -- Sistema Lüm (niveles basados en facturas, NO en balance de Lümis)
    uwl.total_invoices as current_lumis,                        -- Para compatibilidad API, pero representa facturas
    uwl.calculated_level as current_level,
    ul.level_name,
    ul.level_color,
    ul.benefits_json as level_benefits,
    uwl.invoices_to_next_level as lumis_to_next_level,          -- Facturas restantes para siguiente nivel
    
    -- Siguiente nivel
    next_ul.level_name as next_level_name,
    next_ul.min_xp as next_level_min_lumis,
    
    -- Metadatos
    NOW() as last_calculated_at,
    
    -- Score de engagement (0-100) basado en actividad real
    LEAST(100, 
        (uwl.total_invoices * 3) +                    -- 3 puntos por factura
        (uwl.unique_merchants * 2) +                  -- 2 puntos por merchant
        (uwl.active_months * 8) +                     -- 8 puntos por mes activo  
        (CASE WHEN uwl.last_invoice_date > NOW() - INTERVAL '7 days' 
              THEN 15 ELSE 0 END)                     -- 15 puntos si activo última semana
    )::integer as engagement_score,

    -- Racha de login diario
    COALESCE(fs.current_count, 0) as daily_login_strikes,
    
    -- Racha de semanas consecutivas con facturas
    COALESCE(fs_month.current_count, 0) as consistent_month_strikes,
    
    -- Fechas de inicio de rachas (para estabilidad)
    fs.streak_start_date as daily_login_start_date,
    fs_month.streak_start_date as consistent_month_start_date,
    
    -- Status de redención de achievements (basado en recompensas existentes, más estable)
    CASE 
        WHEN COALESCE(fs.current_count, 0) >= 7 THEN
            -- Verificar si existe alguna acumulación week_perfect para este usuario
            EXISTS(
                SELECT 1 FROM rewards.fact_accumulations fa_rewards
                JOIN rewards.dim_accumulations da_rewards ON fa_rewards.accum_id = da_rewards.id
                WHERE fa_rewards.user_id = uwl.user_id 
                AND da_rewards.name LIKE '%week_perfect%'
                AND fa_rewards.date >= COALESCE(fs.streak_start_date, CURRENT_DATE - INTERVAL '30 days')
            )
        ELSE false
    END as week_perfect_redeemed,
    
    CASE 
        WHEN COALESCE(fs_month.current_count, 0) >= 4 THEN
            -- Verificar si existe alguna acumulación consistent_month para este usuario
            EXISTS(
                SELECT 1 FROM rewards.fact_accumulations fa_rewards
                JOIN rewards.dim_accumulations da_rewards ON fa_rewards.accum_id = da_rewards.id
                WHERE fa_rewards.user_id = uwl.user_id 
                AND da_rewards.name LIKE '%consistent_month%'
                AND fa_rewards.date >= COALESCE(fs_month.streak_start_date, CURRENT_DATE - INTERVAL '60 days')
            )
        ELSE false
    END as consistent_month_redeemed,
    
    -- Configuraciones de achievements en JSONB
    to_jsonb(da_week.*) as daily_login_config,
    to_jsonb(da_month.*) as consistent_month_config

FROM user_with_levels uwl
LEFT JOIN public.dim_users u ON uwl.user_id = u.id
LEFT JOIN gamification.dim_user_levels ul ON uwl.calculated_level = ul.level_number
LEFT JOIN gamification.dim_user_levels next_ul ON (uwl.calculated_level + 1) = next_ul.level_number
LEFT JOIN gamification.user_streaks fs ON uwl.user_id = fs.user_id AND fs.streak_type = 'daily_login'
LEFT JOIN gamification.user_streaks fs_month ON uwl.user_id = fs_month.user_id AND fs_month.streak_type = 'consistent_month'
LEFT JOIN gamification.dim_mechanics da_week ON da_week.mechanic_code = 'week_perfect'
LEFT JOIN gamification.dim_mechanics da_month ON da_month.mechanic_code = 'consistent_month'
WHERE u.is_active = true;

-- 3. CREAR ÍNDICES PARA PERFORMANCE
CREATE UNIQUE INDEX idx_user_lum_levels_user_id ON gamification.vw_user_lum_levels(user_id);
CREATE INDEX idx_user_lum_levels_level ON gamification.vw_user_lum_levels(current_level);
CREATE INDEX idx_user_lum_levels_lumis ON gamification.vw_user_lum_levels(current_lumis);
CREATE INDEX idx_user_lum_levels_engagement ON gamification.vw_user_lum_levels(engagement_score);

-- 4. FUNCIÓN PARA REFRESCAR LA VISTA AUTOMÁTICAMENTE
CREATE OR REPLACE FUNCTION gamification.refresh_user_lum_levels()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY gamification.vw_user_lum_levels;
    
    -- Log eliminado para evitar problema de foreign key con user_id=0
    -- El refresh se puede monitorear desde logs de PostgreSQL o monitoring externo
END;
$$ LANGUAGE plpgsql;

-- 5. TRIGGER PARA ACTUALIZACIÓN OPTIMIZADA
-- Refresh selectivo en lugar de completo
CREATE OR REPLACE FUNCTION gamification.trigger_refresh_lum_levels()
RETURNS TRIGGER AS $$
BEGIN
    -- Actualizar streaks para el usuario específico PRIMERO
    IF TG_OP = 'INSERT' THEN
        PERFORM gamification.update_user_streaks(NEW.user_id);
    END IF;
    
    -- Refresh SOLO si han pasado más de 5 minutos desde el último refresh
    -- (Para evitar refreshes excesivos en alta concurrencia)
    IF NOT EXISTS (
        SELECT 1 FROM pg_stat_user_tables 
        WHERE schemaname = 'gamification' 
        AND relname = 'vw_user_lum_levels'
        AND last_vacuum > NOW() - INTERVAL '5 minutes'
    ) THEN
        PERFORM gamification.refresh_user_lum_levels();
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 6. CREAR TRIGGER EN TABLA DE FACTURAS
DROP TRIGGER IF EXISTS trg_refresh_lum_levels ON public.invoice_header;
CREATE TRIGGER trg_refresh_lum_levels
    AFTER INSERT ON public.invoice_header
    FOR EACH ROW  -- Cambiado para acceder a NEW.user_id
    EXECUTE FUNCTION gamification.trigger_refresh_lum_levels();

-- 6.1. FUNCIÓN PARA CALCULAR CONSISTENT_MONTH AUTOMÁTICAMENTE
CREATE OR REPLACE FUNCTION gamification.calculate_consistent_month_streak(p_user_id INTEGER)
RETURNS INTEGER AS $$
DECLARE
    consecutive_weeks INTEGER := 0;
    week_start DATE;
    current_week DATE;
    has_invoice BOOLEAN;
    found_first_gap BOOLEAN := FALSE;
BEGIN
    -- Siempre empezar desde la semana actual
    current_week := DATE_TRUNC('week', CURRENT_DATE);
    
    -- Debug: mostrar desde dónde empezamos
    RAISE NOTICE 'Usuario %, evaluando desde semana actual: %', p_user_id, current_week;
    
    -- Verificar hasta 4 semanas hacia atrás (incluyendo semana actual)
    FOR i IN 0..3 LOOP
        week_start := current_week - (i * INTERVAL '1 week');
        
        -- Verificar si hay al menos 1 factura en esta semana
        SELECT EXISTS(
            SELECT 1 FROM public.invoice_header 
            WHERE user_id = p_user_id 
            AND reception_date >= week_start 
            AND reception_date < week_start + INTERVAL '1 week'
        ) INTO has_invoice;
        
        -- Debug: mostrar resultado de cada semana
        RAISE NOTICE 'Semana % (desde % hasta %): facturas = %', 
            i, week_start, week_start + INTERVAL '1 week' - INTERVAL '1 day', has_invoice;
        
        -- Si esta semana tiene factura, incrementar contador
        IF has_invoice THEN
            -- Solo contar si no hemos encontrado un gap previamente
            IF NOT found_first_gap THEN
                consecutive_weeks := consecutive_weeks + 1;
            ELSE
                -- Si ya encontramos un gap, esta factura no cuenta para la secuencia actual
                RAISE NOTICE 'Factura encontrada después de gap, no cuenta para secuencia actual';
            END IF;
        ELSE
            -- Si no hay factura, marcar que encontramos el primer gap
            IF NOT found_first_gap THEN
                found_first_gap := TRUE;
                RAISE NOTICE 'Primer gap encontrado en semana %', i;
            END IF;
        END IF;
    END LOOP;
    
    RAISE NOTICE 'Resultado final para usuario %: % semanas consecutivas', p_user_id, consecutive_weeks;
    RETURN consecutive_weeks;
END;
$$ LANGUAGE plpgsql;

-- 6.2. FUNCIÓN PARA ACTUALIZAR STREAKS AUTOMÁTICAMENTE
CREATE OR REPLACE FUNCTION gamification.update_user_streaks(p_user_id INTEGER)
RETURNS void AS $$
DECLARE
    month_streak INTEGER;
    old_streak INTEGER := 0;
    old_streak_start_date DATE;
    current_start_date DATE;
    reward_already_granted BOOLEAN := FALSE;
BEGIN
    -- Obtener el streak anterior y fecha de inicio para detectar completions
    SELECT COALESCE(fus.current_count, 0), fus.streak_start_date
    INTO old_streak, old_streak_start_date
    FROM gamification.user_streaks fus
    WHERE fus.user_id = p_user_id AND fus.streak_type = 'consistent_month';
    
    -- Calcular racha de semanas consecutivas
    month_streak := gamification.calculate_consistent_month_streak(p_user_id);
    
    -- Calcular fecha de inicio de la racha actual
    IF month_streak > old_streak OR old_streak = 0 THEN
        -- Racha nueva o creciendo, calcular fecha de inicio
        current_start_date := DATE_TRUNC('week', CURRENT_DATE) - ((month_streak - 1) * INTERVAL '1 week');
    ELSIF month_streak < old_streak THEN
        -- Racha se rompió, nueva fecha de inicio
        current_start_date := DATE_TRUNC('week', CURRENT_DATE) - ((month_streak - 1) * INTERVAL '1 week');
    ELSE
        -- Si month_streak = old_streak, mantener la fecha de inicio actual
        current_start_date := old_streak_start_date;
    END IF;
    
    -- Actualizar o insertar en user_streaks (incluir campos requeridos)
    INSERT INTO gamification.user_streaks (
        user_id, 
        streak_type, 
        current_count,
        last_activity_date,
        streak_start_date
    ) VALUES (
        p_user_id, 
        'consistent_month', 
        month_streak,
        CURRENT_DATE,
        COALESCE(current_start_date, CURRENT_DATE)
    )
    ON CONFLICT (user_id, streak_type) 
    DO UPDATE SET 
        current_count = EXCLUDED.current_count,
        last_activity_date = EXCLUDED.last_activity_date,
        streak_start_date = EXCLUDED.streak_start_date;
    
    -- Verificar si se completó el achievement y otorgar recompensa
    IF month_streak >= 4 AND old_streak < 4 THEN
        -- Verificar si ya se otorgó recompensa para esta racha específica
        SELECT EXISTS(
            SELECT 1 FROM rewards.fact_accumulations fa
            JOIN rewards.dim_accumulations da ON fa.accum_id = da.id
            WHERE fa.user_id = p_user_id 
            AND da.name LIKE '%consistent_month%'
            AND fa.date >= COALESCE(current_start_date, CURRENT_DATE - INTERVAL '4 weeks')
        ) INTO reward_already_granted;
        
        IF NOT reward_already_granted THEN
            RAISE NOTICE 'Usuario % completó consistent_month achievement! Otorgando recompensa (racha iniciada: %)...', p_user_id, current_start_date;
            PERFORM gamification.grant_achievement_reward(p_user_id, 'consistent_month');
        ELSE
            RAISE NOTICE 'Usuario % ya recibió recompensa consistent_month para esta racha (iniciada: %)', p_user_id, current_start_date;
        END IF;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- 6.2.1. FUNCIÓN PARA OTORGAR RECOMPENSAS DE ACHIEVEMENTS
CREATE OR REPLACE FUNCTION gamification.grant_achievement_reward(p_user_id INTEGER, p_achievement_code TEXT)
RETURNS void AS $$
DECLARE
    achievement_record RECORD;
    accumulation_def_id INTEGER;
    reward_amount INTEGER;
BEGIN
    -- Buscar la configuración del achievement en gamification.dim_mechanics
    SELECT * INTO achievement_record
    FROM gamification.dim_mechanics 
    WHERE mechanic_code = p_achievement_code;
    
    -- Si no existe el achievement, crear uno por defecto con 1 lumi (evita bloqueo de flujo)
    IF NOT FOUND THEN
        INSERT INTO gamification.dim_mechanics (
            mechanic_code,
            mechanic_name,
            mechanic_type,
            description,
            reward_lumis,
            created_at
        ) VALUES (
            p_achievement_code,
            p_achievement_code,
            'achievement',
            format('Auto-created achievement for %', p_achievement_code),
            1,
            NOW()
        ) RETURNING * INTO achievement_record;

        RAISE NOTICE 'Achievement % no encontrado — creado por defecto con 1 Lümis', p_achievement_code;
    END IF;
    
    -- Extraer la cantidad de recompensa
    reward_amount := achievement_record.reward_lumis;
    
    -- Si no hay recompensa, salir
    IF reward_amount IS NULL OR reward_amount <= 0 THEN
        RAISE NOTICE 'Achievement % no tiene recompensa configurada', p_achievement_code;
        RETURN;
    END IF;
    
    -- Buscar o crear la definición de acumulación en rewards.dim_accumulations
    SELECT id INTO accumulation_def_id
    FROM rewards.dim_accumulations 
    WHERE name = p_achievement_code;
    
    -- Si no existe la definición, crearla
    IF NOT FOUND THEN
        INSERT INTO rewards.dim_accumulations (
            name,
            name_friendly,
            description_friendly,
            points,
            created_at
        ) VALUES (
            p_achievement_code,
            achievement_record.mechanic_name,
            format('Logro %s: %s', p_achievement_code, achievement_record.mechanic_name),
            reward_amount,
            NOW()
        ) RETURNING id INTO accumulation_def_id;
        
        RAISE NOTICE 'Creada nueva definición de acumulación para achievement %', p_achievement_code;
    END IF;
    
    -- Registrar la transacción real en rewards.fact_accumulations
    INSERT INTO rewards.fact_accumulations (
        user_id,
        accum_id,
        accum_type,
        quantity,
        date
    ) VALUES (
        p_user_id,
        accumulation_def_id,
        'achievement',
        reward_amount,
        NOW()
    );
    
    RAISE NOTICE 'Recompensa otorgada: % Lümis para usuario % por achievement % (fecha: %)', 
        reward_amount, p_user_id, p_achievement_code, CURRENT_DATE;
    
EXCEPTION WHEN OTHERS THEN
    RAISE WARNING 'Error otorgando recompensa para achievement %: %', p_achievement_code, SQLERRM;
END;
$$ LANGUAGE plpgsql;

-- 6.2.2. FUNCIÓN PARA ACTUALIZAR DAILY LOGIN STREAKS Y OTORGAR RECOMPENSAS
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
        -- Login ayer, continuar streak
        new_streak := old_streak + 1;
        calculated_start_date := COALESCE(old_streak_start_date, current_date_only - (old_streak * INTERVAL '1 day'));
    ELSE
        -- Gap en logins, reiniciar streak
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
    IF new_streak >= 7 AND old_streak < 7 THEN
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

-- 6.2.3. FUNCIÓN PARA VERIFICAR STATUS DE ACHIEVEMENTS DE MANERA ESTABLE
DROP FUNCTION IF EXISTS gamification.get_achievement_status(INTEGER, TEXT);
CREATE OR REPLACE FUNCTION gamification.get_achievement_status(p_user_id INTEGER, p_achievement_code TEXT)
RETURNS BOOLEAN AS $$
DECLARE
    user_streak INTEGER := 0;
    user_streak_start DATE;
    required_count INTEGER;
    has_reward BOOLEAN := FALSE;
BEGIN
    -- Determinar el tipo de streak y requisitos según el achievement
    IF p_achievement_code = 'week_perfect' THEN
        SELECT COALESCE(fus.current_count, 0), fus.streak_start_date
        INTO user_streak, user_streak_start
        FROM gamification.user_streaks fus
        WHERE fus.user_id = p_user_id AND fus.streak_type = 'daily_login';
        
        required_count := 7;
        
    ELSIF p_achievement_code = 'consistent_month' THEN
        SELECT COALESCE(fus.current_count, 0), fus.streak_start_date
        INTO user_streak, user_streak_start
        FROM gamification.user_streaks fus
        WHERE fus.user_id = p_user_id AND fus.streak_type = 'consistent_month';
        
        required_count := 4;
    ELSE
        -- Achievement code desconocido
        RETURN FALSE;
    END IF;
    
    -- Si no se alcanzó el requisito mínimo, retornar false
    IF user_streak < required_count THEN
        RETURN FALSE;
    END IF;
    
    -- Verificar si existe recompensa para esta racha
    -- Usar una ventana de tiempo razonable desde el inicio de la racha
    SELECT EXISTS(
        SELECT 1 FROM rewards.fact_accumulations fa
        JOIN rewards.dim_accumulations da ON fa.accum_id = da.id
        WHERE fa.user_id = p_user_id 
        AND da.name LIKE '%' || p_achievement_code || '%'
        AND fa.date >= COALESCE(user_streak_start, CURRENT_DATE - INTERVAL '90 days')
    ) INTO has_reward;
    
    RETURN has_reward;
END;
$$ LANGUAGE plpgsql;

-- 6.3. TRIGGER OPTIMIZADO QUE ACTUALIZA STREAKS Y VISTA
CREATE OR REPLACE FUNCTION gamification.trigger_refresh_lum_levels()
RETURNS TRIGGER AS $$
BEGIN
    -- Actualizar streaks para el usuario específico PRIMERO
    IF TG_OP = 'INSERT' THEN
        PERFORM gamification.update_user_streaks(NEW.user_id);
    END IF;
    
    -- Refresh SOLO si han pasado más de 5 minutos desde el último refresh
    -- (Para evitar refreshes excesivos en alta concurrencia)
    IF NOT EXISTS (
        SELECT 1 FROM pg_stat_user_tables 
        WHERE schemaname = 'gamification' 
        AND relname = 'vw_user_lum_levels'
        AND last_vacuum > NOW() - INTERVAL '5 minutes'
    ) THEN
        PERFORM gamification.refresh_user_lum_levels();
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 7. VISTA SIMPLIFICADA PARA LA API
CREATE OR REPLACE VIEW gamification.v_user_current_level AS
SELECT 
    user_id,
    email,
    total_invoices as total_lumis,           -- CORREGIDO: total_lumis muestra el CONTEO de facturas
    current_level,
    level_name,
    level_color,
    level_benefits,
    lumis_to_next_level,                     -- En realidad = facturas_to_next_level
    next_level_name,
    engagement_score,
    total_invoices,                          -- Campo real para mostrar facturas subidas
    daily_login_strikes,                     -- Racha de login diario actual
    consistent_month_strikes,                -- Racha de semanas consecutivas con facturas
    daily_login_start_date,                  -- Fecha de inicio de racha diaria (para estabilidad)
    consistent_month_start_date,             -- Fecha de inicio de racha mensual (para estabilidad)
    week_perfect_redeemed,                   -- Status: si ya se redimió premio de semana perfecta
    consistent_month_redeemed,               -- Status: si ya se redimió premio de mes consistente
    daily_login_config,                      -- Config de achievement week_perfect
    consistent_month_config,                 -- Config de achievement consistent_month
    last_invoice_date,
    last_calculated_at
FROM gamification.vw_user_lum_levels;

-- 8. REFRESH INICIAL
SELECT gamification.refresh_user_lum_levels();

-- ============================================================================
-- COMENTARIOS Y USO
-- ============================================================================

/*
VENTAJAS DE ESTA IMPLEMENTACIÓN:

✅ **Basada en datos reales:** Usa public.invoice_header como fuente de verdad
✅ **Eficiente:** Vista materializada con refreshes inteligentes  
✅ **Actualizada:** Trigger que refresca automáticamente con nuevas facturas
✅ **Flexible:** Fácil modificar reglas de cálculo de Lümis
✅ **Escalable:** Índices optimizados para queries rápidos
✅ **Auditable:** Log de refreshes en fact_engagement_transactions

CÓMO USAR EN LA API:
- Cambiar query actual por: SELECT * FROM gamification.v_user_current_level WHERE user_id = $1
- total_lumis representa facturas subidas (NO balance de Lümis disponibles)
- Para balance de Lümis usar tabla de wallet/balance separada
- current_level se basa en total_invoices, NO en balance de Lümis

REGLAS DE NEGOCIO IMPLEMENTADAS:
- Niveles basados ÚNICAMENTE en número total de facturas subidas
- 8 niveles: desde Chispa Lüm (0-4 facturas) hasta Platinum Master (500+ facturas) 
- Los Lümis disponibles para redimir NO afectan el nivel del usuario
- Engagement score basado en actividad real del usuario

SEPARACIÓN DE CONCEPTOS:
- 📊 NIVEL = Total de facturas subidas (nunca baja, histórico)
- 💰 LÜMIS = Balance disponible para redimir (puede bajar por redenciones)
- 🎯 PROGRESO = Facturas restantes para siguiente nivel

MANTENIMIENTO:
- Vista se actualiza automáticamente con cada factura nueva
- Refresh manual: SELECT gamification.refresh_user_lum_levels();
- Monitoring: Verificar en logs de PostgreSQL o herramientas de monitoring externas
- Log interno eliminado para evitar problemas de foreign key
*/
