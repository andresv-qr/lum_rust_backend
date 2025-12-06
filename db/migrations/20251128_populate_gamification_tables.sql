-- =====================================================
-- SCRIPT: Poblar tablas vacías de gamificación
-- Fecha: 2024-11-28
-- Base de datos: tfactu
-- Esquema: gamification
-- =====================================================

BEGIN;

-- =====================================================
-- 1. DIM_USER_LEVELS - 10 niveles de usuario
-- Columnas: level_number, level_name, min_xp, max_xp, level_color, icon_url, benefits_json
-- =====================================================
INSERT INTO gamification.dim_user_levels 
    (level_number, level_name, min_xp, max_xp, level_color, icon_url, benefits_json)
VALUES
    (1, 'Novato', 0, 99, '#9E9E9E', '/icons/levels/novato.png', 
        '{"lumi_multiplier": 1.0, "description": "Bienvenido al programa"}'),
    (2, 'Explorador', 100, 299, '#8BC34A', '/icons/levels/explorador.png', 
        '{"lumi_multiplier": 1.05, "description": "5% extra en lumis"}'),
    (3, 'Aventurero', 300, 599, '#4CAF50', '/icons/levels/aventurero.png', 
        '{"lumi_multiplier": 1.10, "description": "10% extra en lumis"}'),
    (4, 'Veterano', 600, 999, '#2196F3', '/icons/levels/veterano.png', 
        '{"lumi_multiplier": 1.15, "description": "15% extra en lumis", "priority_support": true}'),
    (5, 'Experto', 1000, 1499, '#3F51B5', '/icons/levels/experto.png', 
        '{"lumi_multiplier": 1.20, "description": "20% extra en lumis", "priority_support": true}'),
    (6, 'Maestro', 1500, 2199, '#9C27B0', '/icons/levels/maestro.png', 
        '{"lumi_multiplier": 1.25, "description": "25% extra en lumis", "exclusive_offers": true}'),
    (7, 'Campeón', 2200, 2999, '#E91E63', '/icons/levels/campeon.png', 
        '{"lumi_multiplier": 1.30, "description": "30% extra en lumis", "exclusive_offers": true}'),
    (8, 'Leyenda', 3000, 3999, '#FF9800', '/icons/levels/leyenda.png', 
        '{"lumi_multiplier": 1.40, "description": "40% extra en lumis", "vip_access": true}'),
    (9, 'Élite', 4000, 5499, '#FF5722', '/icons/levels/elite.png', 
        '{"lumi_multiplier": 1.50, "description": "50% extra en lumis", "vip_access": true, "cashback_bonus": 0.02}'),
    (10, 'Diamante', 5500, 999999, '#FFD700', '/icons/levels/diamante.png', 
        '{"lumi_multiplier": 2.0, "description": "100% extra en lumis", "vip_access": true, "cashback_bonus": 0.05, "personal_manager": true}')
ON CONFLICT (level_number) DO UPDATE SET
    level_name = EXCLUDED.level_name,
    min_xp = EXCLUDED.min_xp,
    max_xp = EXCLUDED.max_xp,
    level_color = EXCLUDED.level_color,
    benefits_json = EXCLUDED.benefits_json;

-- =====================================================
-- 2. DIM_ACTION_CHANNELS - Canales de acción
-- Columnas: channel_code, channel_name, description, is_active
-- =====================================================
INSERT INTO gamification.dim_action_channels 
    (channel_code, channel_name, description, is_active)
VALUES
    ('MOBILE', 'Aplicación Móvil', 'Acciones desde la app iOS/Android', true),
    ('WHATSAPP', 'WhatsApp Bot', 'Acciones desde el bot de WhatsApp', true),
    ('WEB', 'Portal Web', 'Acciones desde el portal web', true),
    ('POS', 'Punto de Venta', 'Acciones desde terminal POS', true),
    ('API', 'Integración API', 'Acciones desde integraciones externas', true)
ON CONFLICT (channel_code) DO UPDATE SET
    channel_name = EXCLUDED.channel_name,
    description = EXCLUDED.description,
    is_active = EXCLUDED.is_active;

-- =====================================================
-- 3. DIM_ACHIEVEMENTS - Logros del sistema
-- Columnas: achievement_code, achievement_name, description, category, 
--           difficulty, icon_url, requirements_json, reward_lumis, 
--           is_hidden, sort_order, is_active
-- =====================================================
INSERT INTO gamification.dim_achievements 
    (achievement_code, achievement_name, description, category, difficulty,
     icon_url, requirements_json, reward_lumis, is_hidden, sort_order, is_active)
VALUES
    -- Logros de primera vez (onboarding)
    ('FIRST_INVOICE', 'Primera Factura', 'Registra tu primera factura', 'onboarding', 'bronze',
     '/icons/achievements/first_invoice.png', 
     '{"type": "invoice_count", "count": 1}', 10, false, 1, true),
    
    ('FIRST_REDEMPTION', 'Primera Redención', 'Canjea tus primeros lumis', 'onboarding', 'bronze',
     '/icons/achievements/first_redemption.png',
     '{"type": "redemption_count", "count": 1}', 5, false, 2, true),
    
    -- Logros de volumen
    ('INVOICE_10', 'Coleccionista', 'Registra 10 facturas', 'volume', 'bronze',
     '/icons/achievements/invoice_10.png',
     '{"type": "invoice_count", "count": 10}', 25, false, 10, true),
    
    ('INVOICE_50', 'Acumulador', 'Registra 50 facturas', 'volume', 'silver',
     '/icons/achievements/invoice_50.png',
     '{"type": "invoice_count", "count": 50}', 75, false, 11, true),
    
    ('INVOICE_100', 'Facturero Pro', 'Registra 100 facturas', 'volume', 'gold',
     '/icons/achievements/invoice_100.png',
     '{"type": "invoice_count", "count": 100}', 150, false, 12, true),
    
    ('INVOICE_500', 'Maestro Facturero', 'Registra 500 facturas', 'volume', 'platinum',
     '/icons/achievements/invoice_500.png',
     '{"type": "invoice_count", "count": 500}', 500, false, 13, true),
    
    -- Logros de rachas
    ('STREAK_7', 'Semana Perfecta', 'Mantén una racha de 7 días', 'streak', 'bronze',
     '/icons/achievements/streak_7.png',
     '{"type": "streak_days", "days": 7}', 50, false, 20, true),
    
    ('STREAK_30', 'Mes Imparable', 'Mantén una racha de 30 días', 'streak', 'silver',
     '/icons/achievements/streak_30.png',
     '{"type": "streak_days", "days": 30}', 200, false, 21, true),
    
    ('STREAK_90', 'Leyenda Constante', 'Mantén una racha de 90 días', 'streak', 'gold',
     '/icons/achievements/streak_90.png',
     '{"type": "streak_days", "days": 90}', 600, true, 22, true),
    
    ('STREAK_365', 'El Inquebrantable', 'Mantén una racha de 365 días', 'streak', 'platinum',
     '/icons/achievements/streak_365.png',
     '{"type": "streak_days", "days": 365}', 2000, true, 23, true),
    
    -- Logros de monto gastado
    ('SPEND_100', 'Comprador Activo', 'Acumula $100 en facturas', 'spending', 'bronze',
     '/icons/achievements/spend_100.png',
     '{"type": "total_spent", "amount": 100}', 20, false, 30, true),
    
    ('SPEND_500', 'Gran Consumidor', 'Acumula $500 en facturas', 'spending', 'silver',
     '/icons/achievements/spend_500.png',
     '{"type": "total_spent", "amount": 500}', 60, false, 31, true),
    
    ('SPEND_1000', 'VIP Shopper', 'Acumula $1000 en facturas', 'spending', 'gold',
     '/icons/achievements/spend_1000.png',
     '{"type": "total_spent", "amount": 1000}', 150, false, 32, true),
    
    ('SPEND_5000', 'Elite Buyer', 'Acumula $5000 en facturas', 'spending', 'platinum',
     '/icons/achievements/spend_5000.png',
     '{"type": "total_spent", "amount": 5000}', 500, false, 33, true),
    
    -- Logros especiales
    ('HAPPY_HOUR_HUNTER', 'Cazador de Happy Hours', 'Usa 5 Happy Hours', 'special', 'silver',
     '/icons/achievements/happy_hour.png',
     '{"type": "happy_hour_count", "count": 5}', 75, false, 40, true),
    
    ('WEEKEND_WARRIOR', 'Guerrero del Fin de Semana', 'Registra facturas 4 fines de semana seguidos', 'special', 'silver',
     '/icons/achievements/weekend.png',
     '{"type": "weekend_streak", "weeks": 4}', 50, false, 41, true),
    
    ('EARLY_BIRD', 'Madrugador', 'Registra 10 facturas antes de las 8am', 'special', 'bronze',
     '/icons/achievements/early_bird.png',
     '{"type": "time_based", "hour_before": 8, "count": 10}', 30, false, 42, true),
    
    ('NIGHT_OWL', 'Noctámbulo', 'Registra 10 facturas después de las 10pm', 'special', 'bronze',
     '/icons/achievements/night_owl.png',
     '{"type": "time_based", "hour_after": 22, "count": 10}', 30, false, 43, true),
    
    -- Logros sociales
    ('REFERRAL_1', 'Amigo Reclutador', 'Refiere a tu primer amigo', 'social', 'bronze',
     '/icons/achievements/referral_1.png',
     '{"type": "referral_count", "count": 1}', 50, false, 50, true),
    
    ('REFERRAL_5', 'Influencer', 'Refiere a 5 amigos', 'social', 'silver',
     '/icons/achievements/referral_5.png',
     '{"type": "referral_count", "count": 5}', 200, false, 51, true),
    
    ('REFERRAL_25', 'Maestro de Referidos', 'Refiere a 25 amigos', 'social', 'gold',
     '/icons/achievements/referral_25.png',
     '{"type": "referral_count", "count": 25}', 1000, true, 52, true),
    
    -- Logros de engagement
    ('SURVEY_5', 'Opinador', 'Completa 5 encuestas', 'engagement', 'bronze',
     '/icons/achievements/survey_5.png',
     '{"type": "survey_count", "count": 5}', 40, false, 60, true),
    
    ('SURVEY_25', 'Campeón de Encuestas', 'Completa 25 encuestas', 'engagement', 'silver',
     '/icons/achievements/survey_25.png',
     '{"type": "survey_count", "count": 25}', 150, false, 61, true),
    
    -- Logros de nivel
    ('LEVEL_5', 'En Camino', 'Alcanza el nivel 5', 'progression', 'bronze',
     '/icons/achievements/level_5.png',
     '{"type": "level_reached", "level": 5}', 100, false, 70, true),
    
    ('LEVEL_10', 'Diamante', 'Alcanza el nivel máximo', 'progression', 'platinum',
     '/icons/achievements/level_10.png',
     '{"type": "level_reached", "level": 10}', 1000, false, 71, true)
ON CONFLICT (achievement_code) DO UPDATE SET
    achievement_name = EXCLUDED.achievement_name,
    description = EXCLUDED.description,
    difficulty = EXCLUDED.difficulty,
    requirements_json = EXCLUDED.requirements_json,
    reward_lumis = EXCLUDED.reward_lumis,
    is_hidden = EXCLUDED.is_hidden,
    sort_order = EXCLUDED.sort_order;

-- =====================================================
-- 4. DIM_EVENTS - Eventos y multiplicadores
-- Columnas: event_code, event_name, event_type, start_date, end_date, 
--           multiplier, bonus_lumis, target_actions, config_json, is_active
-- =====================================================
INSERT INTO gamification.dim_events 
    (event_code, event_name, event_type, start_date, end_date, 
     multiplier, bonus_lumis, target_actions, config_json, is_active)
VALUES
    -- Happy Hours diarios (12:00-14:00)
    ('HAPPY_HOUR_LUNCH', 'Happy Hour Almuerzo', 'happy_hour',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     2.00, 0,
     '["invoice_scan", "invoice_submit"]',
     '{"recurring": true, "schedule": {"type": "daily", "start_time": "12:00", "end_time": "14:00"}, "min_amount": 5.00}',
     true),
    
    -- Happy Hours noche (18:00-20:00)
    ('HAPPY_HOUR_EVENING', 'Happy Hour Noche', 'happy_hour',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     2.00, 0,
     '["invoice_scan", "invoice_submit"]',
     '{"recurring": true, "schedule": {"type": "daily", "start_time": "18:00", "end_time": "20:00"}, "min_amount": 5.00}',
     true),
    
    -- Viernes Feliz (todo el día)
    ('HAPPY_DAY_FRIDAY', 'Viernes Feliz', 'happy_day',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     1.50, 0,
     '["invoice_scan", "invoice_submit"]',
     '{"recurring": true, "schedule": {"type": "weekly", "day_of_week": 5}}',
     true),
    
    -- Fin de semana especial
    ('HAPPY_WEEKEND', 'Fin de Semana Especial', 'happy_day',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     1.25, 0,
     '["invoice_scan", "invoice_submit"]',
     '{"recurring": true, "schedule": {"type": "weekly", "days_of_week": [6, 0]}}',
     true),
    
    -- Primera semana del mes
    ('FIRST_WEEK_BONUS', 'Bonus Primera Semana', 'monthly',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     1.30, 0,
     '["invoice_scan", "invoice_submit"]',
     '{"recurring": true, "schedule": {"type": "monthly", "days": [1,2,3,4,5,6,7]}}',
     true),
    
    -- Día de pago (15 y 30)
    ('PAYDAY_BONUS', 'Bonus Día de Pago', 'monthly',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     2.50, 0,
     '["invoice_scan", "invoice_submit"]',
     '{"recurring": true, "schedule": {"type": "monthly", "days": [15, 30]}, "max_bonus": 500}',
     true),
    
    -- Black Friday 2025
    ('BLACK_FRIDAY_2025', 'Black Friday 2025', 'seasonal',
     '2025-11-28 00:00:00'::timestamptz, '2025-11-28 23:59:59'::timestamptz,
     3.00, 50,
     '["invoice_scan", "invoice_submit", "redemption"]',
     '{"one_time": true, "special_badge": "black_friday_2025"}',
     true),
    
    -- Cyber Monday 2025
    ('CYBER_MONDAY_2025', 'Cyber Monday 2025', 'seasonal',
     '2025-12-01 00:00:00'::timestamptz, '2025-12-01 23:59:59'::timestamptz,
     2.50, 25,
     '["invoice_scan", "invoice_submit"]',
     '{"one_time": true}',
     true),
    
    -- Navidad 2025
    ('NAVIDAD_2025', 'Navidad 2025', 'seasonal',
     '2025-12-20 00:00:00'::timestamptz, '2025-12-25 23:59:59'::timestamptz,
     2.00, 0,
     '["invoice_scan", "invoice_submit"]',
     '{"one_time": false}',
     true),
    
    -- Año Nuevo 2026
    ('NEW_YEAR_2026', 'Año Nuevo 2026', 'seasonal',
     '2025-12-31 00:00:00'::timestamptz, '2026-01-02 23:59:59'::timestamptz,
     2.00, 100,
     '["invoice_scan", "invoice_submit"]',
     '{"one_time": false, "special_badge": "new_year_2026"}',
     true),
    
    -- Bonus por usar app móvil
    ('MOBILE_BONUS', 'Bonus App Móvil', 'channel',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     1.10, 0,
     '["invoice_scan"]',
     '{"channel": "MOBILE", "stackable": true}',
     true),
    
    -- Primer escaneo del día
    ('FIRST_SCAN_DAY', 'Primer Escaneo del Día', 'daily',
     '2024-01-01 00:00:00'::timestamptz, '2026-12-31 23:59:59'::timestamptz,
     1.20, 5,
     '["invoice_scan"]',
     '{"limit_per_day": 1, "first_only": true}',
     true)
ON CONFLICT (event_code) DO UPDATE SET
    event_name = EXCLUDED.event_name,
    event_type = EXCLUDED.event_type,
    start_date = EXCLUDED.start_date,
    end_date = EXCLUDED.end_date,
    multiplier = EXCLUDED.multiplier,
    bonus_lumis = EXCLUDED.bonus_lumis,
    config_json = EXCLUDED.config_json,
    is_active = EXCLUDED.is_active;

-- =====================================================
-- 5. DIM_ENGAGEMENT_MECHANICS - Mecánicas de engagement
-- Columnas: mechanic_code, mechanic_name, mechanic_type, description, 
--           is_active, config_json
-- =====================================================
INSERT INTO gamification.dim_engagement_mechanics
    (mechanic_code, mechanic_name, mechanic_type, description, is_active, config_json)
VALUES
    ('DAILY_LOGIN', 'Login Diario', 'daily_reward',
     'Bonus por entrar a la app cada día',
     true, '{"base_xp": 5, "base_lumis": 1, "streak_multiplier": true}'),
    
    ('STREAK_BONUS', 'Bonus de Racha', 'streak',
     'Multiplicador por días consecutivos de actividad',
     true, '{"day_3": 1.1, "day_7": 1.25, "day_14": 1.5, "day_30": 2.0, "day_60": 2.5, "day_90": 3.0}'),
    
    ('REFERRAL_PROGRAM', 'Programa de Referidos', 'referral',
     'Gana lumis por cada amigo que se registre con tu código',
     true, '{"referrer_reward": 50, "referee_reward": 25, "max_referrals_per_month": 20}'),
    
    ('SURVEY_REWARDS', 'Encuestas Remuneradas', 'survey',
     'Completa encuestas para ganar lumis extra',
     true, '{"min_reward": 10, "max_reward": 100, "cooldown_hours": 24, "max_per_week": 5}'),
    
    ('DAILY_GAME', 'Juego Diario', 'game',
     'Juega una vez al día para ganar premios',
     true, '{"type": "spin_wheel", "prizes": [5, 10, 25, 50, 100, 500], "probabilities": [0.40, 0.30, 0.15, 0.10, 0.04, 0.01], "free_spins_per_day": 1}'),
    
    ('SCRATCH_CARD', 'Raspadita', 'game',
     'Raspa y descubre premios sorpresa',
     false, '{"cost_lumis": 5, "prizes": [0, 5, 10, 25, 100], "probabilities": [0.30, 0.35, 0.20, 0.12, 0.03]}'),
    
    ('DAILY_MISSIONS', 'Misiones Diarias', 'mission',
     'Completa misiones diarias para ganar rewards',
     true, '{"missions_per_day": 3, "reset_hour": 0, "completion_bonus": 20}'),
    
    ('WEEKLY_MISSIONS', 'Misiones Semanales', 'mission',
     'Completa misiones semanales para mayores rewards',
     true, '{"missions_per_week": 5, "reset_day": 1, "completion_bonus": 100}'),
    
    ('LEVEL_UP_BONUS', 'Bonus de Nivel', 'progression',
     'Rewards especiales al subir de nivel',
     true, '{"lumi_bonus_per_level": 50, "xp_multiplier_next_day": 1.5}'),
    
    ('COMEBACK_BONUS', 'Bonus de Regreso', 'retention',
     'Bonus especial para usuarios que vuelven después de inactividad',
     true, '{"days_inactive": 7, "bonus_lumis": 25, "multiplier_duration_hours": 24, "multiplier": 1.5}')
ON CONFLICT (mechanic_code) DO UPDATE SET
    mechanic_name = EXCLUDED.mechanic_name,
    description = EXCLUDED.description,
    config_json = EXCLUDED.config_json,
    is_active = EXCLUDED.is_active;

-- =====================================================
-- 6. FACT_USER_PROGRESSION - Crear registros para usuarios existentes
-- Columnas: user_id, current_level, current_xp, total_xp, prestige_count, last_level_up
-- FK: current_level -> dim_user_levels(level_id), user_id -> dim_users(id)
-- =====================================================
DO $$
DECLARE
    v_level_1_id INTEGER;
    v_count INTEGER;
BEGIN
    -- Obtener el level_id del nivel 1
    SELECT level_id INTO v_level_1_id 
    FROM gamification.dim_user_levels 
    WHERE level_number = 1;
    
    IF v_level_1_id IS NULL THEN
        RAISE EXCEPTION 'No se encontró el nivel 1 en dim_user_levels. Ejecute primero el INSERT de niveles.';
    END IF;
    
    -- Insertar progression para usuarios que no tienen registro
    INSERT INTO gamification.fact_user_progression 
        (user_id, current_level, current_xp, total_xp, prestige_count, last_level_up, created_at, updated_at)
    SELECT 
        u.id as user_id,
        v_level_1_id as current_level,
        0 as current_xp,
        0 as total_xp,
        0 as prestige_count,
        NULL as last_level_up,
        NOW() as created_at,
        NOW() as updated_at
    FROM dim_users u
    WHERE u.is_active = true
      AND NOT EXISTS (
        SELECT 1 FROM gamification.fact_user_progression p 
        WHERE p.user_id = u.id
    );
    
    GET DIAGNOSTICS v_count = ROW_COUNT;
    RAISE NOTICE 'Registros de progression creados: %', v_count;
END $$;

-- =====================================================
-- 7. Verificación final
-- =====================================================
DO $$
DECLARE
    v_levels INTEGER;
    v_achievements INTEGER;
    v_events INTEGER;
    v_channels INTEGER;
    v_mechanics INTEGER;
    v_progressions INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_levels FROM gamification.dim_user_levels;
    SELECT COUNT(*) INTO v_achievements FROM gamification.dim_achievements;
    SELECT COUNT(*) INTO v_events FROM gamification.dim_events;
    SELECT COUNT(*) INTO v_channels FROM gamification.dim_action_channels;
    SELECT COUNT(*) INTO v_mechanics FROM gamification.dim_engagement_mechanics;
    SELECT COUNT(*) INTO v_progressions FROM gamification.fact_user_progression;
    
    RAISE NOTICE '========================================';
    RAISE NOTICE 'RESUMEN DE DATOS INSERTADOS:';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'dim_user_levels: % registros', v_levels;
    RAISE NOTICE 'dim_achievements: % registros', v_achievements;
    RAISE NOTICE 'dim_events: % registros', v_events;
    RAISE NOTICE 'dim_action_channels: % registros', v_channels;
    RAISE NOTICE 'dim_engagement_mechanics: % registros', v_mechanics;
    RAISE NOTICE 'fact_user_progression: % registros', v_progressions;
    RAISE NOTICE '========================================';
END $$;

COMMIT;
