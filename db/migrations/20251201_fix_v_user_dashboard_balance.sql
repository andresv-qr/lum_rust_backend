-- Migration: Update v_user_dashboard to use rewards.fact_balance_points
-- Date: 2025-12-01
-- Description: The current view uses gamification.user_status.current_balance which is not updated by the rewards triggers.
--              We need to join with rewards.fact_balance_points to get the real-time balance.

BEGIN;

DROP VIEW IF EXISTS gamification.v_user_dashboard;

CREATE VIEW gamification.v_user_dashboard AS
SELECT 
    us.user_id,
    u.email,
    us.total_xp as total_invoices,
    COALESCE(fbp.balance, 0) as wallet_balance, -- Use real-time balance from rewards system
    l.level_id as current_level,
    l.level_name,
    l.level_color,
    l.benefits_json as level_benefits,
    COALESCE(nl.min_xp - us.total_xp, 0) as invoices_to_next_level,
    nl.level_name as next_level_name,
    -- Active Streaks
    COALESCE(
        (SELECT jsonb_agg(jsonb_build_object(
            'type', streak_type,
            'current', current_count,
            'max', max_count
        )) FROM gamification.user_streaks 
        WHERE user_id = us.user_id AND is_active = true),
        '[]'::jsonb
    ) as active_streaks,
    -- Active Mechanics
    (SELECT COUNT(*) FROM gamification.user_mechanics 
     WHERE user_id = us.user_id AND status = 'active') as active_mechanics_count
FROM gamification.user_status us
JOIN public.dim_users u ON us.user_id = u.id
JOIN gamification.dim_user_levels l ON us.current_level_id = l.level_id
LEFT JOIN gamification.dim_user_levels nl ON nl.level_number = l.level_number + 1
LEFT JOIN rewards.fact_balance_points fbp ON us.user_id = fbp.user_id;

COMMIT;
