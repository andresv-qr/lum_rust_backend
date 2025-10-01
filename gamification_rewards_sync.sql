-- Función para sincronizar puntos de gamificación con el sistema de recompensas
-- Esta función debe ser llamada cada vez que se otorguen puntos por gamificación

CREATE OR REPLACE FUNCTION sync_gamification_to_rewards(
    p_user_id INTEGER,
    p_action_type TEXT,
    p_points INTEGER,
    p_metadata JSONB DEFAULT '{}'::jsonb
)
RETURNS BOOLEAN
LANGUAGE plpgsql
AS $$
DECLARE
    v_accumulation_id INTEGER;
    v_existing_count INTEGER;
    v_daily_limit INTEGER;
    v_today_count INTEGER;
BEGIN
    -- Buscar la definición de acumulación correspondiente
    SELECT id INTO v_accumulation_id
    FROM rewards.dim_accumulations
    WHERE name = 'gamification_' || p_action_type
    AND (valid_to IS NULL OR valid_to > NOW())
    AND cond_1_key = 'action_type'
    AND cond_1_value = p_action_type;
    
    -- Si no existe la definición, salir
    IF v_accumulation_id IS NULL THEN
        RAISE NOTICE 'No se encontró definición de acumulación para action_type: %', p_action_type;
        RETURN FALSE;
    END IF;
    
    -- Verificar límites diarios si aplican
    SELECT duration_value INTO v_daily_limit
    FROM rewards.dim_accumulations
    WHERE id = v_accumulation_id
    AND duration_unit = 'day';
    
    IF v_daily_limit IS NOT NULL THEN
        -- Contar transacciones del día actual para este usuario y tipo
        SELECT COUNT(*) INTO v_today_count
        FROM rewards.fact_accumulations
        WHERE user_id = p_user_id
        AND accum_id = v_accumulation_id
        AND DATE(date) = CURRENT_DATE;
        
        -- Si ya alcanzó el límite diario, no procesar
        IF v_today_count >= v_daily_limit THEN
            RAISE NOTICE 'Usuario % ya alcanzó el límite diario (%) para %', p_user_id, v_daily_limit, p_action_type;
            RETURN FALSE;
        END IF;
    END IF;
    
    -- Insertar en fact_accumulations
    INSERT INTO rewards.fact_accumulations (
        user_id,
        accum_id,
        accum_type,
        accum_key,
        dtype,
        quantity,
        date,
        balance
    ) VALUES (
        p_user_id,
        v_accumulation_id,
        'gamification',
        p_action_type,
        'points',
        p_points,
        NOW(),
        p_points  -- Por ahora balance = cantidad ganada
    );
    
    RAISE NOTICE 'Sincronizado: Usuario % ganó % puntos por % (accumulation_id: %)', 
                 p_user_id, p_points, p_action_type, v_accumulation_id;
    
    RETURN TRUE;
    
EXCEPTION
    WHEN OTHERS THEN
        RAISE NOTICE 'Error al sincronizar gamificación con rewards: %', SQLERRM;
        RETURN FALSE;
END;
$$;

-- Función helper para obtener puntos totales de un usuario desde rewards
CREATE OR REPLACE FUNCTION get_user_total_reward_points(p_user_id INTEGER)
RETURNS INTEGER
LANGUAGE plpgsql
AS $$
DECLARE
    v_total_points INTEGER := 0;
BEGIN
    SELECT COALESCE(SUM(quantity), 0) INTO v_total_points
    FROM rewards.fact_accumulations
    WHERE user_id = p_user_id
    AND dtype = 'points';
    
    RETURN v_total_points;
END;
$$;

-- Trigger para sincronizar automáticamente cuando se registren transacciones de engagement
CREATE OR REPLACE FUNCTION trigger_sync_engagement_to_rewards()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    -- Solo sincronizar si la transacción otorga lumis
    IF NEW.lumis_amount > 0 THEN
        PERFORM sync_gamification_to_rewards(
            NEW.user_id,
            NEW.action_type,
            NEW.lumis_amount,
            jsonb_build_object(
                'engagement_transaction_id', NEW.transaction_id,
                'source_type', NEW.source_type,
                'auto_sync', true
            )
        );
    END IF;
    
    RETURN NEW;
END;
$$;

-- Crear el trigger en fact_engagement_transactions
DROP TRIGGER IF EXISTS sync_to_rewards_trigger ON gamification.fact_engagement_transactions;
CREATE TRIGGER sync_to_rewards_trigger
    AFTER INSERT ON gamification.fact_engagement_transactions
    FOR EACH ROW
    EXECUTE FUNCTION trigger_sync_engagement_to_rewards();

COMMENT ON FUNCTION sync_gamification_to_rewards IS 'Sincroniza puntos de gamificación con el sistema de recompensas';
COMMENT ON FUNCTION get_user_total_reward_points IS 'Obtiene el total de puntos de recompensas de un usuario';
COMMENT ON FUNCTION trigger_sync_engagement_to_rewards IS 'Trigger para sincronización automática de engagement a rewards';
