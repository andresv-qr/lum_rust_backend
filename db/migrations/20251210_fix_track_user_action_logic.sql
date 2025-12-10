-- Migration: Fix track_user_action to actually update streaks
-- Date: 2025-12-10
-- Description: The previous version of track_user_action only logged the activity but didn't trigger
--              any game logic (streaks, rewards, etc). This update connects the dots.

CREATE OR REPLACE FUNCTION gamification.track_user_action(
    p_user_id integer, 
    p_action_type character varying, 
    p_channel character varying DEFAULT 'mobile_app'::character varying, 
    p_metadata jsonb DEFAULT '{}'::jsonb
)
RETURNS TABLE(lumis_earned integer, xp_earned integer, streak_info jsonb, achievements_unlocked jsonb, active_events jsonb, message text)
LANGUAGE plpgsql
AS $function$
DECLARE
    v_lumis_earned INTEGER := 0;
    v_xp_earned INTEGER := 0;
    v_message TEXT;
    v_streak_info JSONB := '{}'::jsonb;
    v_achievements JSONB := '[]'::jsonb;
    v_events JSONB := '[]'::jsonb;
BEGIN
    -- 1. Log activity (Renamed table: activity_log)
    INSERT INTO gamification.activity_log (
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

    -- 2. GAME LOGIC DISPATCHER
    -- Based on action type, trigger specific game mechanics
    
    IF p_action_type = 'daily_login' THEN
        -- Update daily login streak (handles increment and reset logic)
        PERFORM gamification.update_daily_login_streak(p_user_id);
        v_message := 'Daily login recorded';
    END IF;

    -- 3. Fetch updated streak info to return to frontend
    SELECT jsonb_build_object(
        'current', current_count,
        'max', 7, -- Hardcoded for now based on week_perfect
        'type', streak_type
    ) INTO v_streak_info
    FROM gamification.user_streaks
    WHERE user_id = p_user_id AND streak_type = 'daily_login';

    -- 4. Return results
    RETURN QUERY SELECT 
        v_lumis_earned, 
        v_xp_earned, 
        v_streak_info, 
        v_achievements, 
        v_events, 
        v_message;
END;
$function$;
