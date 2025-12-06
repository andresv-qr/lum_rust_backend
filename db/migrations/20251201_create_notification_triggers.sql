-- Migration: Create notification triggers
-- Date: 2025-12-01
-- Purpose: Automatic notifications for invoices, achievements, and streak alerts
-- Features: Deduplication via idempotency_key, push queue integration

BEGIN;

-- ============================================================================
-- 1. TRIGGER: Notify Invoice Processed
-- ============================================================================

CREATE OR REPLACE FUNCTION public.notify_invoice_processed()
RETURNS TRIGGER AS $$
BEGIN
    -- Only for new invoices with associated user
    IF TG_OP = 'INSERT' AND NEW.user_id IS NOT NULL THEN
        -- Create notification with idempotency_key based on CUFE
        -- Note: The actual lumis earned are calculated by the rewards system
        PERFORM public.create_notification(
            p_user_id := NEW.user_id,
            p_title := '¡Factura procesada!',
            p_body := FORMAT('Tu factura de %s fue procesada exitosamente', COALESCE(NEW.issuer_name, 'comercio')),
            p_type := 'invoice',
            p_priority := 'normal',
            p_action_url := FORMAT('/invoices/%s', NEW.cufe),
            p_payload := jsonb_build_object(
                'cufe', NEW.cufe,
                'merchant_name', NEW.issuer_name,
                'amount', NEW.tot_amount,
                'date', NEW.date
            ),
            p_idempotency_key := FORMAT('invoice_%s', NEW.cufe),  -- CUFE is unique
            p_send_push := TRUE
        );
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.notify_invoice_processed() IS 
'Trigger function to notify users when their invoice is processed.
Creates in-app notification and queues push notification.
Uses CUFE as idempotency key to prevent duplicates.';

-- Drop existing trigger if exists and create new one
DROP TRIGGER IF EXISTS trg_notify_invoice_processed ON public.invoice_header;

CREATE TRIGGER trg_notify_invoice_processed
    AFTER INSERT ON public.invoice_header
    FOR EACH ROW
    EXECUTE FUNCTION public.notify_invoice_processed();

-- ============================================================================
-- 2. FUNCTION: Notify Achievement Unlocked (Callable, not a trigger)
-- ============================================================================

-- This was already created in the schema migration, verify it exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_proc 
        WHERE proname = 'notify_achievement_unlocked' 
        AND pronamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'public')
    ) THEN
        CREATE OR REPLACE FUNCTION public.notify_achievement_unlocked(
            p_user_id BIGINT,
            p_achievement_code VARCHAR(50),
            p_achievement_name VARCHAR(200),
            p_lumis_reward INTEGER
        )
        RETURNS BIGINT AS $func$
        BEGIN
            RETURN public.create_notification(
                p_user_id := p_user_id,
                p_title := '🏆 ¡Logro desbloqueado!',
                p_body := FORMAT('Completaste "%s" y ganaste %s Lümis', p_achievement_name, p_lumis_reward),
                p_type := 'achievement',
                p_priority := 'high',
                p_action_url := '/achievements',
                p_payload := jsonb_build_object(
                    'achievement_code', p_achievement_code,
                    'achievement_name', p_achievement_name,
                    'lumis_reward', p_lumis_reward
                ),
                p_idempotency_key := FORMAT('achievement_%s_%s_%s', p_user_id, p_achievement_code, CURRENT_DATE),
                p_send_push := TRUE
            );
        END;
        $func$ LANGUAGE plpgsql;

        COMMENT ON FUNCTION public.notify_achievement_unlocked IS 
        'Callable function to notify achievement unlocks.
        Call from gamification.grant_achievement_reward() after inserting into fact_accumulations.
        NOT an automatic trigger to avoid coupling with rewards schema.';
    END IF;
END $$;

-- ============================================================================
-- 3. FUNCTION: Notify Streak at Risk (Scheduled Job)
-- ============================================================================

CREATE OR REPLACE FUNCTION public.notify_streak_at_risk()
RETURNS TABLE(notifications_sent INTEGER) AS $$
DECLARE
    v_sent INTEGER := 0;
    v_user RECORD;
    v_today DATE := CURRENT_DATE;
BEGIN
    -- Find users with active streak who haven't logged in today
    FOR v_user IN 
        SELECT 
            us.user_id,
            us.current_count as streak_days,
            us.last_activity_date
        FROM gamification.user_streaks us
        WHERE us.streak_type = 'daily_login'
        AND us.current_count >= 3  -- Only alert if they have 3+ days
        AND us.last_activity_date < v_today  -- Haven't logged in today
        AND us.last_activity_date >= v_today - INTERVAL '1 day'  -- But did yesterday (streak not broken yet)
        AND NOT EXISTS (
            -- Don't send if we already sent today
            SELECT 1 FROM public.notifications n
            WHERE n.user_id = us.user_id
            AND n.type = 'streak'
            AND n.idempotency_key = FORMAT('streak_risk_%s_%s', us.user_id, v_today)
        )
    LOOP
        PERFORM public.create_notification(
            p_user_id := v_user.user_id,
            p_title := '🔥 ¡Tu racha está en peligro!',
            p_body := FORMAT('Llevas %s días consecutivos. ¡No pierdas tu racha!', v_user.streak_days),
            p_type := 'streak',
            p_priority := 'high',
            p_action_url := '/earn',
            p_payload := jsonb_build_object(
                'streak_days', v_user.streak_days,
                'risk_level', 'high'
            ),
            p_idempotency_key := FORMAT('streak_risk_%s_%s', v_user.user_id, v_today),
            p_send_push := TRUE
        );
        
        v_sent := v_sent + 1;
    END LOOP;
    
    RETURN QUERY SELECT v_sent;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.notify_streak_at_risk() IS 
'Scheduled job function to alert users whose streak is at risk.
Run hourly via pg_cron between 10am and 10pm Panama time.
Uses idempotency_key to prevent duplicate notifications per day.';

-- Schedule with pg_cron if available (hourly between 10am and 10pm Panama = 15:00-03:00 UTC)
-- SELECT cron.schedule('streak-risk-notify', '0 15-23,0-3 * * *', 'SELECT * FROM public.notify_streak_at_risk()');

-- ============================================================================
-- 4. FUNCTION: Cleanup Expired Notifications (Scheduled Job)
-- ============================================================================

CREATE OR REPLACE FUNCTION public.cleanup_expired_notifications()
RETURNS TABLE(deleted_count INTEGER) AS $$
DECLARE
    v_deleted INTEGER := 0;
    v_temp INTEGER;
BEGIN
    -- Delete expired notifications
    DELETE FROM public.notifications 
    WHERE expires_at IS NOT NULL 
    AND expires_at < NOW();
    
    GET DIAGNOSTICS v_temp = ROW_COUNT;
    v_deleted := v_deleted + v_temp;
    
    -- Clean old read notifications (>90 days)
    DELETE FROM public.notifications 
    WHERE is_read = TRUE 
    AND created_at < NOW() - INTERVAL '90 days';
    
    GET DIAGNOSTICS v_temp = ROW_COUNT;
    v_deleted := v_deleted + v_temp;
    
    -- Clean inactive tokens (>60 days without use)
    DELETE FROM public.device_tokens 
    WHERE is_active = FALSE 
    AND updated_at < NOW() - INTERVAL '60 days';
    
    -- Clean processed push queue (>7 days)
    DELETE FROM public.notification_push_queue
    WHERE status IN ('sent', 'skipped')
    AND created_at < NOW() - INTERVAL '7 days';
    
    -- Clean failed queue items (>30 days)
    DELETE FROM public.notification_push_queue
    WHERE status = 'failed'
    AND created_at < NOW() - INTERVAL '30 days';
    
    RETURN QUERY SELECT v_deleted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.cleanup_expired_notifications() IS 
'Daily maintenance job to clean up old notifications and tokens.
Run at 3am Panama time (8am UTC) via pg_cron.';

-- Schedule with pg_cron if available
-- SELECT cron.schedule('cleanup-notifications', '0 8 * * *', 'SELECT * FROM public.cleanup_expired_notifications()');

-- ============================================================================
-- 5. FUNCTION: Notify Level Up (Callable)
-- ============================================================================

CREATE OR REPLACE FUNCTION public.notify_level_up(
    p_user_id BIGINT,
    p_new_level INTEGER,
    p_level_name VARCHAR(100),
    p_lumis_bonus INTEGER DEFAULT 0
)
RETURNS BIGINT AS $$
BEGIN
    RETURN public.create_notification(
        p_user_id := p_user_id,
        p_title := '🎉 ¡Subiste de nivel!',
        p_body := FORMAT('Ahora eres nivel %s: %s', p_new_level, p_level_name),
        p_type := 'level_up',
        p_priority := 'high',
        p_action_url := '/profile',
        p_payload := jsonb_build_object(
            'new_level', p_new_level,
            'level_name', p_level_name,
            'lumis_bonus', p_lumis_bonus
        ),
        p_idempotency_key := FORMAT('level_up_%s_%s', p_user_id, p_new_level),
        p_send_push := TRUE
    );
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.notify_level_up IS 
'Callable function to notify users when they level up.
Call from the level calculation logic after determining level change.';

-- ============================================================================
-- 6. FUNCTION: Notify Promo (for marketing campaigns)
-- ============================================================================

CREATE OR REPLACE FUNCTION public.notify_promo(
    p_user_id BIGINT,
    p_title VARCHAR(200),
    p_body TEXT,
    p_action_url VARCHAR(255) DEFAULT NULL,
    p_image_url TEXT DEFAULT NULL,
    p_campaign_id VARCHAR(50) DEFAULT NULL,
    p_expires_at TIMESTAMPTZ DEFAULT NULL
)
RETURNS BIGINT AS $$
BEGIN
    RETURN public.create_notification(
        p_user_id := p_user_id,
        p_title := p_title,
        p_body := p_body,
        p_type := 'promo',
        p_priority := 'low',
        p_action_url := p_action_url,
        p_image_url := p_image_url,
        p_payload := jsonb_build_object(
            'campaign_id', p_campaign_id
        ),
        p_idempotency_key := FORMAT('promo_%s_%s', p_user_id, COALESCE(p_campaign_id, 'general')),
        p_expires_at := p_expires_at,
        p_send_push := TRUE
    );
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.notify_promo IS 
'Callable function to send promotional notifications.
Use for marketing campaigns with optional expiration.';

-- ============================================================================
-- VERIFICATION
-- ============================================================================

DO $$
DECLARE
    v_trigger_count INTEGER;
    v_function_count INTEGER;
BEGIN
    -- Count triggers
    SELECT COUNT(*) INTO v_trigger_count
    FROM pg_trigger t
    JOIN pg_class c ON t.tgrelid = c.oid
    WHERE t.tgname LIKE 'trg_notify_%';
    
    -- Count notification functions
    SELECT COUNT(*) INTO v_function_count
    FROM pg_proc p
    JOIN pg_namespace n ON p.pronamespace = n.oid
    WHERE n.nspname = 'public'
    AND p.proname IN (
        'create_notification',
        'notify_invoice_processed',
        'notify_achievement_unlocked',
        'notify_streak_at_risk',
        'cleanup_expired_notifications',
        'notify_level_up',
        'notify_promo'
    );
    
    RAISE NOTICE '';
    RAISE NOTICE '╔══════════════════════════════════════════════════════════════╗';
    RAISE NOTICE '║          NOTIFICATION TRIGGERS MIGRATION COMPLETE            ║';
    RAISE NOTICE '╠══════════════════════════════════════════════════════════════╣';
    RAISE NOTICE '║  ✅ Triggers created: %                                      ║', v_trigger_count;
    RAISE NOTICE '║  ✅ Functions created: %                                     ║', v_function_count;
    RAISE NOTICE '╠══════════════════════════════════════════════════════════════╣';
    RAISE NOTICE '║  Available triggers:                                         ║';
    RAISE NOTICE '║    • trg_notify_invoice_processed (invoice_header)           ║';
    RAISE NOTICE '║                                                              ║';
    RAISE NOTICE '║  Callable functions:                                         ║';
    RAISE NOTICE '║    • notify_achievement_unlocked(user_id, code, name, lumis) ║';
    RAISE NOTICE '║    • notify_level_up(user_id, level, name, bonus)            ║';
    RAISE NOTICE '║    • notify_promo(user_id, title, body, ...)                 ║';
    RAISE NOTICE '║                                                              ║';
    RAISE NOTICE '║  Scheduled jobs (configure with pg_cron):                    ║';
    RAISE NOTICE '║    • notify_streak_at_risk() - hourly 10am-10pm              ║';
    RAISE NOTICE '║    • cleanup_expired_notifications() - daily 3am             ║';
    RAISE NOTICE '╚══════════════════════════════════════════════════════════════╝';
END $$;

COMMIT;
