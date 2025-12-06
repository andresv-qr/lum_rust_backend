-- Update track_user_action to use new schema
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

    -- 2. Return default values (Logic handled by triggers/batch for now)
    -- In a full implementation, we would calculate rewards here based on dim_mechanics
    
    RETURN QUERY SELECT 
        v_lumis_earned, 
        v_xp_earned, 
        v_streak_info, 
        v_achievements, 
        v_events, 
        v_message;
END;
$function$;
