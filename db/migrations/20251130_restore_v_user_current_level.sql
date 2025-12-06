-- Recreate the dependent view v_user_current_level
-- This view was dropped by CASCADE in the previous migration
CREATE OR REPLACE VIEW gamification.v_user_current_level AS
SELECT 
    user_id,
    current_level,
    level_name,
    level_color,
    current_lumis,
    lumis_to_next_level,
    next_level_name,
    engagement_score
FROM gamification.vw_user_lum_levels;
