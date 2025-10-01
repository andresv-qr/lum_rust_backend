-- ============================================================================
-- GAMIFICATION COMPLETE SETUP WITH MULTI-CHANNEL SUPPORT
-- ============================================================================
-- Date: August 29, 2025
-- Version: 2.0 - Multi-channel & Full Descriptions
-- ============================================================================

-- ============================================================================
-- 1. CLEAR PREVIOUS TEST DATA (Safe for development)
-- ============================================================================
TRUNCATE TABLE gamification.dim_engagement_mechanics CASCADE;
TRUNCATE TABLE gamification.dim_rewards_config CASCADE;
TRUNCATE TABLE gamification.dim_events CASCADE;
TRUNCATE TABLE gamification.dim_user_levels CASCADE;
TRUNCATE TABLE gamification.dim_achievements CASCADE;

-- ============================================================================
-- 2. USER LEVELS WITH FULL DESCRIPTIONS AND BENEFITS
-- ============================================================================
INSERT INTO gamification.dim_user_levels (
    level_number, 
    level_name, 
    min_xp, 
    max_xp, 
    level_color, 
    benefits_json
) VALUES
(1, 'Chispa Lüm', 0, 4, '#FFD700', '{
    "multiplier": 1.0,
    "description": "El primer destello. Comienza tu viaje en el universo Lüm.",
    "benefits": [
        "Acceso básico a misiones",
        "Rachas de ingreso diario",
        "1x multiplicador base"
    ],
    "next_level_hint": "Gana 5 Lümis para alcanzar el siguiente nivel",
    "unlock_features": []
}'),
(2, 'Destello Lunar', 5, 9, '#C0C0C0', '{
    "multiplier": 1.05,
    "description": "Tu luz crece, suave pero constante como la Luna.",
    "benefits": [
        "5% bonus en todas las recompensas",
        "Desbloquea misiones especiales",
        "1 protección de racha gratis al mes"
    ],
    "next_level_hint": "Gana 5 Lümis más para evolucionar",
    "unlock_features": ["streak_freeze", "special_missions"]
}'),
(3, 'Eco de Estrella', 10, 19, '#87CEEB', '{
    "multiplier": 1.1,
    "description": "Ya se sienten los ecos de tu energía en el cosmos.",
    "benefits": [
        "10% bonus en recompensas",
        "Acceso a eventos VIP",
        "2 protecciones de racha al mes",
        "Prioridad en soporte"
    ],
    "next_level_hint": "10 Lümis más para el siguiente nivel",
    "unlock_features": ["vip_events", "priority_support"]
}'),
(4, 'Aurora Viva', 20, 39, '#FF69B4', '{
    "multiplier": 1.15,
    "description": "Comienzas a brillar con colores únicos y vibrantes.",
    "benefits": [
        "15% bonus en recompensas",
        "Misiones exclusivas semanales",
        "3 protecciones de racha al mes",
        "Badge especial en perfil"
    ],
    "next_level_hint": "20 Lümis para continuar ascendiendo",
    "unlock_features": ["weekly_exclusive_missions", "profile_badge"]
}'),
(5, 'Bruma Luminosa', 40, 79, '#9370DB', '{
    "multiplier": 1.2,
    "description": "Te envuelve una energía difusa pero creciente.",
    "benefits": [
        "20% bonus en recompensas",
        "Acceso anticipado a features",
        "4 protecciones de racha al mes",
        "Recompensas dobles en Happy Hour"
    ],
    "next_level_hint": "40 Lümis para el próximo salto",
    "unlock_features": ["early_access", "double_happy_hour"]
}'),
(6, 'Rayo Celeste', 80, 159, '#00CED1', '{
    "multiplier": 1.25,
    "description": "Tu energía se vuelve veloz, brillante y visible.",
    "benefits": [
        "25% bonus en recompensas",
        "Creación de equipos",
        "5 protecciones de racha al mes",
        "Sorteos exclusivos mensuales"
    ],
    "next_level_hint": "80 Lümis para alcanzar el pulso galáctico",
    "unlock_features": ["team_creation", "exclusive_raffles"]
}'),
(7, 'Pulso Galáctico', 160, 319, '#FF4500', '{
    "multiplier": 1.3,
    "description": "Vibra en ti la fuerza de un sistema entero.",
    "benefits": [
        "30% bonus en recompensas",
        "Líder de comunidad",
        "Rachas protegidas ilimitadas",
        "Acceso a beta features"
    ],
    "next_level_hint": "160 Lümis para la corona estelar",
    "unlock_features": ["community_leader", "unlimited_freeze", "beta_access"]
}'),
(8, 'Corona Estelar', 320, 739, '#FFD700', '{
    "multiplier": 1.35,
    "description": "Irradias calor y energía como una estrella activa.",
    "benefits": [
        "35% bonus en recompensas",
        "Mentor de nuevos usuarios",
        "Eventos personalizados",
        "Recompensas triples en eventos especiales"
    ],
    "next_level_hint": "420 Lümis para el núcleo solar",
    "unlock_features": ["mentorship", "personalized_events", "triple_event_rewards"]
}'),
(9, 'Núcleo Solar', 740, 999, '#FF8C00', '{
    "multiplier": 1.4,
    "description": "Eres la fuente de fusión: intensa, estable, poderosa.",
    "benefits": [
        "40% bonus en recompensas",
        "Voz en decisiones de producto",
        "Línea directa con el equipo",
        "NFT exclusivo anual"
    ],
    "next_level_hint": "260 Lümis para la supremacía total",
    "unlock_features": ["product_voice", "direct_line", "exclusive_nft"]
}'),
(10, 'Lüm Supremo', 1000, 999999, '#FF0000', '{
    "multiplier": 1.5,
    "description": "Has alcanzado el dominio total de la energía Lüm.",
    "benefits": [
        "50% bonus permanente",
        "Estatus legendario de por vida",
        "Todas las features premium gratis",
        "Nombre en el Hall of Fame"
    ],
    "next_level_hint": "Has alcanzado la cima",
    "unlock_features": ["legendary_status", "hall_of_fame", "all_premium_free"]
}');

-- ============================================================================
-- 3. ENGAGEMENT MECHANICS WITH DETAILED EXPLANATIONS
-- ============================================================================
INSERT INTO gamification.dim_engagement_mechanics (
    mechanic_code, 
    mechanic_name, 
    mechanic_type, 
    description, 
    config_json
) VALUES
-- Racha de Login Diario
('daily_login_streak', 'Racha de Ingreso Diario', 'streak', 
'Ingresa consecutivamente para ganar Lümis. ¡Mientras más días seguidos, mayores las recompensas!', 
'{
    "display_name": "🔥 Racha Diaria",
    "short_description": "Ingresa todos los días para mantener tu racha",
    "long_description": "Mantén tu racha viva ingresando a la app diariamente. Las recompensas aumentan exponencialmente con cada día consecutivo. Si pierdes un día, la racha se reinicia desde cero.",
    "how_it_works": [
        "Ingresa a la app cada día antes de medianoche (hora Panamá)",
        "Cada día consecutivo aumenta tu contador de racha",
        "Las recompensas crecen según los hitos alcanzados",
        "Si pierdes un día, la racha se reinicia (puedes usar protección)"
    ],
    "rewards": [
        {"day": 1, "lumis": 1, "message": "¡Buen inicio! +1 Lümi"},
        {"day": 2, "lumis": 1, "message": "¡Sigue así! +1 Lümi"},
        {"day": 3, "lumis": 2, "message": "¡3 días! +2 Lümis"},
        {"day": 5, "lumis": 3, "message": "¡Casi una semana! +3 Lümis"},
        {"day": 7, "lumis": 5, "message": "🏆 ¡Semana perfecta! +5 Lümis", "badge": "week_perfect"},
        {"day": 14, "lumis": 10, "message": "🔥 ¡2 semanas en llamas! +10 Lümis", "badge": "two_weeks"},
        {"day": 30, "lumis": 25, "message": "🌟 ¡Mes legendario! +25 Lümis", "badge": "month_complete"}
    ],
    "tips": [
        "Activa notificaciones para no olvidar tu racha",
        "Ingresa aunque sea 1 minuto al día",
        "Los fines de semana también cuentan",
        "Usa protecciones sabiamente para días ocupados"
    ],
    "reset_on_miss": true,
    "freeze_available": true,
    "max_freezes_per_month": 2,
    "source_channels": ["mobile_app", "web_app", "whatsapp_bot"]
}'),

-- Racha de Facturas Semanal
('weekly_invoice_streak', 'Racha de Facturas Semanal', 'streak',
'Sube al menos una factura cada semana durante un mes y gana recompensas crecientes.',
'{
    "display_name": "📄 Racha Semanal de Facturas",
    "short_description": "1 factura por semana durante un mes",
    "long_description": "Mantén el hábito de subir tus facturas. Solo necesitas 1 factura por semana para mantener la racha. Mientras más semanas consecutivas, mayores las recompensas.",
    "how_it_works": [
        "Sube mínimo 1 factura cada semana (Lunes a Domingo)",
        "Completa 4 semanas seguidas para el bonus máximo",
        "Puedes subir por WhatsApp, app móvil o web",
        "Todas las facturas cuentan: restaurantes, supermercados, farmacia, etc."
    ],
    "rewards": [
        {"week": 1, "lumis": 2, "message": "Primera semana completa +2 Lümis"},
        {"week": 2, "lumis": 3, "message": "¡2 semanas! +3 Lümis"},
        {"week": 3, "lumis": 4, "message": "¡Casi un mes! +4 Lümis"},
        {"week": 4, "lumis": 5, "message": "🏆 ¡Mes consistente! +5 Lümis", "badge": "consistent_month"}
    ],
    "tips": [
        "Sube las facturas el mismo día que compras",
        "WhatsApp es más rápido para facturas físicas",
        "Acumula varias y súbelas de una vez",
        "Los recibos de delivery también cuentan"
    ],
    "min_invoices_per_week": 1,
    "reset_on_miss": true,
    "source_channels": ["mobile_app", "whatsapp", "web_app"]
}'),

-- Hitos de Facturas
('invoice_milestones', 'Hitos de Facturas', 'milestone',
'Gana Lümis adicionales al alcanzar cierta cantidad total de facturas.',
'{
    "display_name": "🎯 Hitos de Facturas",
    "short_description": "Recompensas por cantidad total de facturas",
    "long_description": "Cada factura cuenta para alcanzar estos hitos especiales con recompensas únicas. Es acumulativo y permanente, así que cada factura te acerca más al siguiente hito.",
    "how_it_works": [
        "Cada factura suma a tu contador total",
        "Los hitos son acumulativos y permanentes",
        "No importa el canal: WhatsApp, App o Web",
        "Las recompensas se otorgan automáticamente al alcanzar cada hito"
    ],
    "milestones": [
        {"count": 1, "lumis": 1, "message": "🎉 ¡Primera factura!", "description": "El primer paso de muchos"},
        {"count": 10, "lumis": 2, "message": "📊 ¡10 facturas!", "description": "Ya eres un usuario activo"},
        {"count": 25, "lumis": 3, "message": "⭐ ¡25 facturas!", "description": "Cuarto de centenar"},
        {"count": 50, "lumis": 5, "message": "🏆 ¡50 facturas!", "description": "Medio centenar alcanzado"},
        {"count": 100, "lumis": 10, "message": "💎 ¡100 facturas!", "description": "Eres un usuario élite"},
        {"count": 250, "lumis": 25, "message": "🔥 ¡250 facturas!", "description": "Usuario legendario"}
    ],
    "tips": [
        "Todas las facturas cuentan, sin importar el monto",
        "Sube facturas de diferentes categorías",
        "Las facturas viejas también suman al total"
    ],
    "source_channels": ["mobile_app", "whatsapp", "web_app", "email", "api"]
}');

-- ============================================================================
-- 4. MISSIONS WITH DETAILED DESCRIPTIONS
-- ============================================================================
INSERT INTO gamification.dim_rewards_config (
    reward_code, 
    reward_name, 
    reward_type, 
    base_amount, 
    requirements_json
) VALUES
-- Misión Septiembre Restaurantes
('sept_restaurant_mission', 'Misión: Foodie de Septiembre', 'lumis', 5, '{
    "display_name": "🍴 Foodie de Septiembre",
    "mission_type": "monthly_category",
    "category": "restaurant",
    "description": "Explora la escena gastronómica este mes y gana recompensas especiales",
    "long_description": "Sube facturas de tus restaurantes favoritos durante septiembre y gana recompensas especiales. Desde tu cafetería matutina hasta cenas elegantes, cada experiencia gastronómica cuenta.",
    "requirements": {
        "min_count": 2,
        "category": "restaurant",
        "date_range": {
            "start": "2025-09-01",
            "end": "2025-09-30"
        }
    },
    "objectives": [
        {"count": 2, "lumis": 5, "description": "Sube 2 facturas de restaurante", "progress_message": "2 de 2 facturas"},
        {"count": 5, "lumis": 8, "description": "Bonus: 5 facturas = 3 Lümis extra", "progress_message": "5 de 5 facturas"},
        {"count": 10, "lumis": 15, "description": "Super bonus: 10 facturas = 10 Lümis extra", "progress_message": "10 de 10 facturas"}
    ],
    "tips": [
        "Incluye desayunos, almuerzos y cenas",
        "Los servicios de delivery también cuentan",
        "Comparte con amigos y sube más facturas",
        "Explora nuevos lugares para más diversión"
    ],
    "valid_channels": ["mobile_app", "whatsapp"],
    "auto_claim": true
}'),

-- Misión Semanal Supermercado
('supermarket_weekly', 'Misión: Comprador Inteligente', 'lumis', 3, '{
    "display_name": "🛒 Comprador Inteligente",
    "mission_type": "weekly_category",
    "category": "supermarket",
    "description": "Registra tus compras del supermercado semanalmente",
    "long_description": "Ayúdanos a entender los hábitos de compra subiendo tus facturas de supermercado cada semana. Desde productos básicos hasta especialidades, cada compra cuenta.",
    "requirements": {
        "min_count": 1,
        "category": "supermarket",
        "recurring": "weekly"
    },
    "objectives": [
        {"count": 1, "lumis": 3, "description": "1 factura de supermercado esta semana"}
    ],
    "tips": [
        "Sube la factura inmediatamente después de comprar",
        "Incluye todas las cadenas: Rey, Super99, Riba Smith, etc.",
        "Las compras online también cuentan"
    ],
    "valid_channels": ["mobile_app", "whatsapp"]
}');

-- ============================================================================
-- 5. ACHIEVEMENTS WITH DESCRIPTIONS
-- ============================================================================
INSERT INTO gamification.dim_achievements (
    achievement_code, 
    achievement_name, 
    description, 
    category, 
    difficulty, 
    requirements_json, 
    reward_lumis
) VALUES
('first_invoice', 'Primera Factura', 'Sube tu primera factura a la plataforma', 'invoices', 'bronze', 
'{"invoice_count": 1, "description": "El primer paso en tu viaje Lüm"}', 1),

('week_perfect', 'Semana Perfecta', 'Ingresa a la app 7 días consecutivos', 'streaks', 'silver', 
'{"streak_days": 7, "streak_type": "daily_login", "description": "Dedicación durante una semana completa"}', 5),

('two_weeks', 'Dos Semanas Imparable', 'Ingresa a la app 14 días consecutivos', 'streaks', 'gold', 
'{"streak_days": 14, "streak_type": "daily_login", "description": "Dos semanas de constancia absoluta"}', 10),

('month_complete', 'Mes Legendario', 'Ingresa a la app 30 días consecutivos', 'streaks', 'platinum', 
'{"streak_days": 30, "streak_type": "daily_login", "description": "Un mes completo de dedicación"}', 25),

('consistent_month', 'Mes Consistente', 'Sube facturas 4 semanas seguidas', 'invoices', 'gold', 
'{"weeks": 4, "streak_type": "weekly_invoice", "description": "Un mes de hábitos saludables"}', 5),

('restaurant_lover', 'Amante de Restaurantes', 'Sube 10 facturas de restaurante', 'invoices', 'silver', 
'{"category": "restaurant", "count": 10, "description": "Explorador de la escena gastronómica"}', 10),

('early_adopter', 'Pionero Lüm', 'Uno de los primeros 100 usuarios', 'special', 'platinum', 
'{"user_rank": 100, "description": "Pionero en el universo Lüm"}', 50),

('social_butterfly', 'Mariposa Social', 'Comparte 5 logros en redes sociales', 'social', 'silver', 
'{"shares": 5, "description": "Embajador de la comunidad Lüm"}', 15);

-- ============================================================================
-- 6. HAPPY HOUR EVENTS
-- ============================================================================
CREATE OR REPLACE FUNCTION gamification.create_daily_happy_hour(
    p_date DATE DEFAULT CURRENT_DATE
)
RETURNS void AS $$
BEGIN
    INSERT INTO gamification.dim_events (
        event_code,
        event_name,
        event_type,
        start_date,
        end_date,
        multiplier,
        bonus_lumis,
        target_actions,
        config_json
    ) VALUES (
        'happy_hour_' || to_char(p_date, 'YYYY_MM_DD'),
        'Happy Hour 2x - ' || to_char(p_date, 'DD/MM'),
        'daily',
        p_date::timestamp + interval '18 hours',  -- 6 PM Panama
        p_date::timestamp + interval '20 hours',  -- 8 PM Panama
        2.00,
        0,
        '["invoice_upload", "survey_complete", "daily_login"]'::jsonb,
        '{
            "display_name": "🎉 Happy Hour 2x",
            "description": "¡Duplica tus Lümis de 6PM a 8PM!",
            "long_description": "Durante Happy Hour, todas las acciones valen el doble. Sube facturas, completa encuestas o simplemente ingresa para duplicar tus recompensas.",
            "timezone": "America/Panama",
            "notification_minutes_before": 10,
            "notification_message": "⏰ Happy Hour comienza en 10 minutos. ¡Prepárate para duplicar tus Lümis!",
            "max_bonus_per_user": 100,
            "applicable_actions": [
                {"action": "invoice_upload", "description": "Facturas valen 2x"},
                {"action": "survey_complete", "description": "Encuestas valen 2x"},
                {"action": "daily_login", "description": "Login vale 2x"}
            ],
            "valid_channels": ["mobile_app", "whatsapp", "web_app"],
            "tips": [
                "Programa tus actividades para este horario",
                "Las recompensas se duplican automáticamente",
                "Máximo 100 Lümis bonus por día"
            ]
        }'::jsonb
    )
    ON CONFLICT (event_code) DO UPDATE
    SET config_json = EXCLUDED.config_json,
        is_active = true;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- 7. CHANNEL TRACKING TABLE
-- ============================================================================
INSERT INTO gamification.dim_action_channels (channel_code, channel_name, channel_type) VALUES
('mobile_app', 'Aplicación Móvil', 'primary'),
('whatsapp', 'WhatsApp Bot', 'integration'),
('web_app', 'Aplicación Web', 'primary'),
('api', 'API Externa', 'integration'),
('email', 'Email Processing', 'integration'),
('admin', 'Admin Manual', 'internal')
ON CONFLICT (channel_code) DO UPDATE SET
    channel_name = EXCLUDED.channel_name,
    channel_type = EXCLUDED.channel_type;

-- ============================================================================
-- 8. CORE TRACKING FUNCTIONS
-- ============================================================================

-- Master function to track any user action
CREATE OR REPLACE FUNCTION gamification.track_user_action(
    p_user_id INTEGER,
    p_action_type VARCHAR, -- 'daily_login', 'invoice_upload', 'survey_complete'
    p_channel VARCHAR DEFAULT 'mobile_app',
    p_metadata JSONB DEFAULT '{}'::jsonb
)
RETURNS TABLE(
    lumis_earned INTEGER,
    xp_earned INTEGER,
    streak_info JSONB,
    achievements_unlocked JSONB,
    active_events JSONB,
    message TEXT
) AS $$
DECLARE
    v_lumis_earned INTEGER := 0;
    v_xp_earned INTEGER := 0;
    v_message TEXT;
    v_multiplier DECIMAL := 1.0;
    v_streak_info JSONB := '{}'::jsonb;
    v_achievements JSONB := '[]'::jsonb;
    v_events JSONB := '[]'::jsonb;
    v_level_multiplier DECIMAL := 1.0;
BEGIN
    -- Log the action with channel
    INSERT INTO gamification.fact_user_activity_log (
        user_id, activity_type, activity_data, created_at
    ) VALUES (
        p_user_id, p_action_type, 
        jsonb_build_object(
            'channel', p_channel,
            'metadata', p_metadata,
            'timestamp', NOW()
        ),
        NOW()
    );
    
    -- Get user's level multiplier
    SELECT COALESCE((l.benefits_json->>'multiplier')::decimal, 1.0)
    INTO v_level_multiplier
    FROM gamification.fact_user_progression p
    JOIN gamification.dim_user_levels l ON p.current_level = l.level_id
    WHERE p.user_id = p_user_id;
    
    v_level_multiplier := COALESCE(v_level_multiplier, 1.0);
    
    -- Process based on action type
    CASE p_action_type
        WHEN 'daily_login' THEN
            SELECT * INTO v_lumis_earned, v_streak_info
            FROM gamification.process_daily_login(p_user_id, p_channel);
            
        WHEN 'invoice_upload' THEN
            SELECT * INTO v_lumis_earned, v_message
            FROM gamification.process_invoice_upload(
                p_user_id, 
                p_channel,
                p_metadata->>'category'
            );
            
        WHEN 'survey_complete' THEN
            v_lumis_earned := COALESCE((p_metadata->>'lumis_reward')::integer, 2);
            v_message := 'Encuesta completada +' || v_lumis_earned || ' Lümis';
    END CASE;
    
    -- Check for active events (Happy Hour, etc)
    SELECT 
        COALESCE(MAX(e.multiplier), 1.0),
        jsonb_agg(
            jsonb_build_object(
                'event_name', e.event_name,
                'multiplier', e.multiplier,
                'bonus', e.bonus_lumis,
                'description', e.config_json->>'description'
            )
        ) FILTER (WHERE e.event_id IS NOT NULL)
    INTO v_multiplier, v_events
    FROM gamification.dim_events e
    WHERE e.is_active = true
    AND NOW() BETWEEN e.start_date AND e.end_date
    AND e.target_actions::jsonb ? p_action_type;
    
    -- Apply multipliers
    v_lumis_earned := (v_lumis_earned * v_multiplier * v_level_multiplier)::integer;
    v_xp_earned := v_lumis_earned; -- 1:1 ratio for now
    
    -- Update user's total lumis and XP (skip for daily_login as it only affects streaks)
    IF p_action_type != 'daily_login' THEN
        INSERT INTO gamification.fact_user_progression (
            user_id, current_xp, total_xp, current_level
        ) VALUES (
            p_user_id, v_xp_earned, v_xp_earned, 1
        )
        ON CONFLICT (user_id) DO UPDATE
        SET current_xp = fact_user_progression.current_xp + v_xp_earned,
            total_xp = fact_user_progression.total_xp + v_xp_earned,
            updated_at = NOW();
    END IF;
    
    -- Check for level up (skip for daily_login)
    IF p_action_type != 'daily_login' THEN
        UPDATE gamification.fact_user_progression
        SET current_level = (
            SELECT level_id FROM gamification.dim_user_levels
            WHERE total_xp >= min_xp AND total_xp <= max_xp
            ORDER BY level_id DESC LIMIT 1
        )
        WHERE user_id = p_user_id;
    END IF;
    
    -- Record transaction
    INSERT INTO gamification.fact_engagement_transactions (
        user_id, 
        source_type, 
        action_type, 
        lumis_amount, 
        xp_amount,
        multiplier_applied,
        event_context
    ) VALUES (
        p_user_id, 
        p_action_type,
        p_action_type,
        v_lumis_earned,
        v_xp_earned,
        v_multiplier * v_level_multiplier,
        jsonb_build_object(
            'channel', p_channel,
            'metadata', p_metadata,
            'events', v_events,
            'level_multiplier', v_level_multiplier
        )
    );
    
    -- Build final message
    IF v_message IS NULL THEN
        v_message := format('Ganaste %s Lümis', v_lumis_earned);
    END IF;
    
    IF COALESCE(v_multiplier, 1.0) > 1 THEN
        v_message := v_message || format(' (x%.1f Happy Hour!)', COALESCE(v_multiplier, 1.0));
    END IF;
    
    IF COALESCE(v_level_multiplier, 1.0) > 1 THEN
        v_message := v_message || format(' (x%.1f Nivel)', COALESCE(v_level_multiplier, 1.0));
    END IF;
    
    RETURN QUERY SELECT 
        v_lumis_earned,
        v_xp_earned,
        COALESCE(v_streak_info, '{}'::jsonb),
        COALESCE(v_achievements, '[]'::jsonb),
        COALESCE(v_events, '[]'::jsonb),
        v_message;
END;
$$ LANGUAGE plpgsql;

-- Function to process daily login
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
            'next_reward_day', CASE
                WHEN v_current_streak < 3 THEN 3
                WHEN v_current_streak < 7 THEN 7
                WHEN v_current_streak < 14 THEN 14
                WHEN v_current_streak < 30 THEN 30
                ELSE NULL
            END
        );
        RETURN QUERY SELECT 0, v_streak_info;
        RETURN;
    END IF;
    
    -- Update streak
    IF v_last_login = CURRENT_DATE - 1 THEN
        v_current_streak := v_current_streak + 1;
    ELSE
        v_current_streak := 1;
    END IF;
    
    -- Calculate lumis based on milestones
    v_lumis_earned := CASE 
        WHEN v_current_streak = 1 THEN 1
        WHEN v_current_streak = 2 THEN 1
        WHEN v_current_streak = 3 THEN 2
        WHEN v_current_streak IN (4) THEN 2
        WHEN v_current_streak = 5 THEN 3
        WHEN v_current_streak IN (6) THEN 3
        WHEN v_current_streak = 7 THEN 5
        WHEN v_current_streak = 14 THEN 10
        WHEN v_current_streak = 30 THEN 25
        ELSE 1
    END;
    
    -- Check achievements
    v_achievement := CASE
        WHEN v_current_streak = 7 THEN 'week_perfect'
        WHEN v_current_streak = 14 THEN 'two_weeks'
        WHEN v_current_streak = 30 THEN 'month_complete'
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
    
    -- Unlock achievement if applicable
    IF v_achievement IS NOT NULL THEN
        INSERT INTO gamification.fact_user_achievements (user_id, achievement_id, unlocked_at)
        SELECT p_user_id, achievement_id, NOW()
        FROM gamification.dim_achievements
        WHERE achievement_code = v_achievement
        ON CONFLICT (user_id, achievement_id) DO NOTHING;
    END IF;
    
    -- Build streak info
    v_streak_info := jsonb_build_object(
        'current_streak', v_current_streak,
        'lumis_earned', v_lumis_earned,
        'max_streak', (SELECT max_count FROM gamification.fact_user_streaks 
                       WHERE user_id = p_user_id AND streak_type = 'daily_login'),
        'next_milestone', CASE
            WHEN v_current_streak < 3 THEN 3
            WHEN v_current_streak < 7 THEN 7
            WHEN v_current_streak < 14 THEN 14
            WHEN v_current_streak < 30 THEN 30
            ELSE NULL
        END,
        'achievement_unlocked', v_achievement
    );
    
    RETURN QUERY SELECT v_lumis_earned, v_streak_info;
END;
$$ LANGUAGE plpgsql;

-- Function to process invoice upload
CREATE OR REPLACE FUNCTION gamification.process_invoice_upload(
    p_user_id INTEGER,
    p_channel VARCHAR,
    p_category VARCHAR DEFAULT NULL
)
RETURNS TABLE(lumis_earned INTEGER, message TEXT) AS $$
DECLARE
    v_total_invoices INTEGER;
    v_lumis_earned INTEGER := 0;
    v_message TEXT := '';
    v_week_invoices INTEGER;
    v_week_start DATE;
BEGIN
    -- Get total invoice count (assuming we have access to invoices table)
    -- This would need to be adjusted based on your actual invoice table structure
    v_total_invoices := COALESCE((
        SELECT COUNT(*) 
        FROM public.invoices 
        WHERE user_id = p_user_id
    ), 0);
    
    -- If no access to invoices table, track via metadata
    IF v_total_invoices = 0 THEN
        v_total_invoices := COALESCE((
            SELECT COUNT(*)
            FROM gamification.fact_engagement_transactions
            WHERE user_id = p_user_id AND action_type = 'invoice_upload'
        ), 0) + 1;
    END IF;
    
    -- Check milestones
    v_lumis_earned := CASE v_total_invoices
        WHEN 1 THEN 1
        WHEN 10 THEN 2
        WHEN 25 THEN 3
        WHEN 50 THEN 5
        WHEN 100 THEN 10
        WHEN 250 THEN 25
        ELSE 0
    END;
    
    IF v_lumis_earned > 0 THEN
        v_message := format('¡Hito alcanzado! Factura #%s = +%s Lümis', 
                           v_total_invoices, v_lumis_earned);
        
        -- Unlock milestone achievement
        INSERT INTO gamification.fact_user_achievements (user_id, achievement_id, unlocked_at)
        SELECT p_user_id, achievement_id, NOW()
        FROM gamification.dim_achievements
        WHERE achievement_code = CASE v_total_invoices
            WHEN 1 THEN 'first_invoice'
            WHEN 10 THEN 'restaurant_lover'
            ELSE NULL
        END
        AND achievement_code IS NOT NULL
        ON CONFLICT (user_id, achievement_id) DO NOTHING;
    END IF;
    
    -- Update weekly invoice streak
    v_week_start := date_trunc('week', CURRENT_DATE)::date;
    
    SELECT COUNT(*) INTO v_week_invoices
    FROM gamification.fact_engagement_transactions
    WHERE user_id = p_user_id
    AND action_type = 'invoice_upload'
    AND created_at >= v_week_start;
    
    IF v_week_invoices = 1 THEN -- First invoice of the week
        -- Update or create weekly streak
        INSERT INTO gamification.fact_user_streaks (
            user_id, streak_type, current_count, last_activity_date, streak_start_date
        ) VALUES (
            p_user_id, 'weekly_invoice', 1, v_week_start, v_week_start
        )
        ON CONFLICT (user_id, streak_type) DO UPDATE
        SET current_count = CASE
                WHEN fact_user_streaks.last_activity_date = v_week_start - interval '1 week' 
                THEN fact_user_streaks.current_count + 1
                ELSE 1
            END,
            last_activity_date = v_week_start,
            updated_at = NOW();
    END IF;
    
    -- Check for mission progress (September restaurants)
    IF p_category = 'restaurant' AND CURRENT_DATE BETWEEN '2025-09-01' AND '2025-09-30' THEN
        UPDATE gamification.fact_user_missions
        SET current_progress = current_progress + 1,
            updated_at = NOW()
        WHERE user_id = p_user_id
        AND mission_code = 'sept_restaurant_mission'
        AND status = 'active';
    END IF;
    
    RETURN QUERY SELECT v_lumis_earned, v_message;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- 9. DATABASE TRIGGERS FOR AUTOMATIC TRACKING
-- ============================================================================

-- Trigger function for invoice uploads
CREATE OR REPLACE FUNCTION gamification.trigger_invoice_gamification()
RETURNS TRIGGER AS $$
BEGIN
    -- Call tracking function automatically when invoice is inserted
    PERFORM gamification.track_user_action(
        NEW.user_id,
        'invoice_upload',
        COALESCE(NEW.source_channel, 'unknown'),
        jsonb_build_object(
            'invoice_id', NEW.id,
            'category', COALESCE(NEW.category, 'general'),
            'amount', COALESCE(NEW.total_amount, 0)
        )
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Note: The actual trigger would be created on your invoice table
-- Example (adjust table name as needed):
-- DROP TRIGGER IF EXISTS trg_gamification_invoice ON public.invoices;
-- CREATE TRIGGER trg_gamification_invoice
-- AFTER INSERT ON public.invoices
-- FOR EACH ROW
-- EXECUTE FUNCTION gamification.trigger_invoice_gamification();

-- ============================================================================
-- 10. API VIEWS FOR EASY ACCESS
-- ============================================================================

-- View for user gamification dashboard
CREATE OR REPLACE VIEW gamification.v_user_dashboard AS
SELECT 
    u.id as user_id,
    u.email,
    COALESCE(p.total_xp, 0) as total_lumis,
    COALESCE(p.current_level, 1) as current_level,
    COALESCE(l.level_name, 'Chispa Lüm') as level_name,
    l.benefits_json->>'description' as level_description,
    l.level_color,
    l.benefits_json->'benefits' as level_benefits,
    l.benefits_json->>'next_level_hint' as next_level_hint,
    
    -- Progress to next level
    COALESCE(nl.min_xp - p.total_xp, 0) as lumis_to_next_level,
    COALESCE(nl.level_name, 'Máximo Nivel') as next_level_name,
    
    -- Active streaks
    (SELECT jsonb_object_agg(
        streak_type,
        jsonb_build_object(
            'current', current_count,
            'max', max_count,
            'last_activity', last_activity_date,
            'total_lumis', total_lumis_earned
        )
    )
    FROM gamification.fact_user_streaks
    WHERE user_id = u.id AND is_active = true
    ) as active_streaks,
    
    -- Mission counts
    (SELECT COUNT(*) 
     FROM gamification.fact_user_missions
     WHERE user_id = u.id AND status = 'active'
    ) as active_missions_count,
    
    (SELECT COUNT(*) 
     FROM gamification.fact_user_missions
     WHERE user_id = u.id AND status = 'completed'
    ) as completed_missions_count,
    
    -- Achievement counts
    (SELECT COUNT(*) 
     FROM gamification.fact_user_achievements
     WHERE user_id = u.id
    ) as total_achievements,
    
    -- Recent activity
    (SELECT jsonb_agg(
        jsonb_build_object(
            'action', action_type,
            'lumis', lumis_amount,
            'created_at', created_at
        ) ORDER BY created_at DESC
    )
    FROM gamification.fact_engagement_transactions
    WHERE user_id = u.id
    LIMIT 10
    ) as recent_activity
    
FROM public.dim_users u
LEFT JOIN gamification.fact_user_progression p ON u.id = p.user_id
LEFT JOIN gamification.dim_user_levels l ON p.current_level = l.level_id
LEFT JOIN gamification.dim_user_levels nl ON nl.level_number = l.level_number + 1;

-- View for mechanics explanations
CREATE OR REPLACE VIEW gamification.v_mechanics_info AS
SELECT 
    mechanic_code,
    mechanic_name,
    mechanic_type,
    description,
    config_json->>'display_name' as display_name,
    config_json->>'short_description' as short_description,
    config_json->>'long_description' as long_description,
    config_json->'how_it_works' as how_it_works,
    config_json->'rewards' as rewards,
    config_json->'tips' as tips,
    is_active
FROM gamification.dim_engagement_mechanics
WHERE is_active = true
ORDER BY 
    CASE mechanic_type 
        WHEN 'streak' THEN 1 
        WHEN 'milestone' THEN 2 
        WHEN 'mission' THEN 3 
        ELSE 4 
    END,
    mechanic_name;

-- ============================================================================
-- 11. INITIAL DATA SETUP
-- ============================================================================

-- Create happy hour events for next 30 days
DO $$
BEGIN
    FOR i IN 0..29 LOOP
        PERFORM gamification.create_daily_happy_hour(CURRENT_DATE + i);
    END LOOP;
END;
$$;

-- Create initial missions for all active users
INSERT INTO gamification.fact_user_missions (
    user_id, mission_code, mission_name, mission_type,
    target_count, reward_lumis, assigned_date, due_date, status
)
SELECT 
    id,
    'sept_restaurant_mission',
    'Foodie de Septiembre',
    'monthly',
    2,
    5,
    '2025-09-01'::date,
    '2025-09-30'::date,
    'active'
FROM public.dim_users
WHERE is_active = true
ON CONFLICT DO NOTHING;

-- ============================================================================
-- VALIDATION AND SETUP SUMMARY
-- ============================================================================

-- Check setup completion
DO $$
DECLARE
    mechanics_count INTEGER;
    levels_count INTEGER;
    achievements_count INTEGER;
    events_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO mechanics_count FROM gamification.dim_engagement_mechanics;
    SELECT COUNT(*) INTO levels_count FROM gamification.dim_user_levels;
    SELECT COUNT(*) INTO achievements_count FROM gamification.dim_achievements;
    SELECT COUNT(*) INTO events_count FROM gamification.dim_events WHERE is_active = true;
    
    RAISE NOTICE '==========================================';
    RAISE NOTICE 'GAMIFICATION SETUP COMPLETE';
    RAISE NOTICE '==========================================';
    RAISE NOTICE 'Mechanics configured: %', mechanics_count;
    RAISE NOTICE 'Levels configured: %', levels_count;
    RAISE NOTICE 'Achievements available: %', achievements_count;
    RAISE NOTICE 'Active events: %', events_count;
    RAISE NOTICE '==========================================';
    
    IF mechanics_count >= 3 AND levels_count >= 10 AND achievements_count >= 5 THEN
        RAISE NOTICE '✅ Setup completed successfully!';
    ELSE
        RAISE NOTICE '❌ Setup incomplete - check data insertion';
    END IF;
END;
$$;
