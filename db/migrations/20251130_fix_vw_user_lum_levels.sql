-- Migration: Fix Materialized View vw_user_lum_levels
-- Description: Updates the materialized view to use the new 17-level system and optimized schema tables.
-- Strategy: Drop and recreate the view with correct logic.

BEGIN;

-- 1. Drop the existing materialized view and dependent objects
DROP MATERIALIZED VIEW IF EXISTS gamification.vw_user_lum_levels CASCADE;

-- 2. Recreate the materialized view with updated logic
CREATE MATERIALIZED VIEW gamification.vw_user_lum_levels AS
WITH user_invoice_stats AS (
    -- Calculate base stats from invoice_header
    SELECT 
        h.user_id,
        count(*) AS total_invoices,
        COALESCE(sum(h.tot_amount), 0::double precision) AS total_spent,
        count(DISTINCT h.issuer_name) AS unique_merchants,
        count(DISTINCT date_trunc('month'::text, h.reception_date)) AS active_months,
        min(h.reception_date) AS first_invoice_date,
        max(h.reception_date) AS last_invoice_date
    FROM invoice_header h
    WHERE h.user_id IS NOT NULL
    GROUP BY h.user_id
),
user_level_calc AS (
    -- Determine level based on total_invoices using the new dim_user_levels table
    SELECT 
        uis.*,
        l.level_id,
        l.level_number,
        l.level_name,
        l.level_color,
        l.benefits_json,
        nl.level_name AS next_level_name,
        nl.min_xp AS next_level_min_lumis,
        GREATEST(0, nl.min_xp - uis.total_invoices) AS invoices_to_next_level
    FROM user_invoice_stats uis
    LEFT JOIN gamification.dim_user_levels l 
        ON uis.total_invoices >= l.min_xp AND uis.total_invoices <= l.max_xp
    LEFT JOIN gamification.dim_user_levels nl 
        ON nl.level_number = (l.level_number + 1)
)
SELECT 
    ulc.user_id,
    u.email,
    u.name,
    ulc.total_invoices,
    ulc.total_spent,
    ulc.unique_merchants,
    ulc.active_months,
    ulc.first_invoice_date,
    ulc.last_invoice_date,
    ulc.total_invoices AS current_lumis,
    ulc.level_number AS current_level, -- Using level_number (1-17) instead of ID
    ulc.level_name,
    ulc.level_color,
    ulc.benefits_json AS level_benefits,
    ulc.invoices_to_next_level AS lumis_to_next_level,
    ulc.next_level_name,
    ulc.next_level_min_lumis,
    NOW() AS last_calculated_at,
    
    -- Engagement Score Calculation (Simplified)
    LEAST(100::bigint, 
        ulc.total_invoices * 3 + 
        ulc.unique_merchants * 2 + 
        ulc.active_months * 8 +
        CASE WHEN ulc.last_invoice_date > (NOW() - INTERVAL '7 days') THEN 15 ELSE 0 END
    )::integer AS engagement_score,

    -- Streaks from new table
    COALESCE(s_daily.current_count, 0) AS daily_login_strikes,
    COALESCE(s_month.current_count, 0) AS consistent_month_strikes,
    
    -- Configs from new dim_mechanics (replacing _backup_dim_achievements)
    m_daily.config AS daily_login_config,
    m_month.config AS consistent_month_config

FROM user_level_calc ulc
JOIN public.dim_users u ON ulc.user_id = u.id
LEFT JOIN gamification.user_streaks s_daily 
    ON ulc.user_id = s_daily.user_id AND s_daily.streak_type = 'daily_login'
LEFT JOIN gamification.user_streaks s_month 
    ON ulc.user_id = s_month.user_id AND s_month.streak_type = 'consistent_month'
LEFT JOIN gamification.dim_mechanics m_daily 
    ON m_daily.mechanic_code = 'daily_login'
LEFT JOIN gamification.dim_mechanics m_month 
    ON m_month.mechanic_code = 'consistent_month'
WHERE u.is_active = true;

-- 3. Recreate Indexes
CREATE UNIQUE INDEX idx_vw_user_lum_levels_user_id ON gamification.vw_user_lum_levels(user_id);
CREATE INDEX idx_vw_user_lum_levels_level ON gamification.vw_user_lum_levels(current_level);
CREATE INDEX idx_vw_user_lum_levels_engagement ON gamification.vw_user_lum_levels(engagement_score);

COMMIT;
