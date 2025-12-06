-- Migration: Optimize Gamification Schema to 14 Tables
-- Date: 2025-11-30

BEGIN;

-- 1. CONFIGURATION TABLES
-- ============================================================================

-- 1.1 dim_mechanics (Unifies achievements, events, missions, combos)
CREATE TABLE gamification.dim_mechanics (
    mechanic_id SERIAL PRIMARY KEY,
    mechanic_type VARCHAR(20) NOT NULL CHECK (mechanic_type IN ('streak', 'mission', 'achievement', 'event', 'combo')),
    mechanic_code VARCHAR(50) UNIQUE NOT NULL,
    mechanic_name VARCHAR(100) NOT NULL,
    description TEXT,
    config JSONB NOT NULL DEFAULT '{}',
    icon_url VARCHAR(200),
    difficulty VARCHAR(20),
    reward_lumis INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_mechanics_active_type ON gamification.dim_mechanics(mechanic_type) WHERE is_active = true;

-- 1.2 dim_rewards (Renamed from dim_rewards_config)
CREATE TABLE gamification.dim_rewards (
    reward_id SERIAL PRIMARY KEY,
    reward_code VARCHAR(50) UNIQUE NOT NULL,
    reward_name VARCHAR(100) NOT NULL,
    lumis_amount INTEGER NOT NULL,
    reward_type VARCHAR(30) DEFAULT 'lumis',
    conditions JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 1.3 teams (Renamed from dim_teams)
CREATE TABLE gamification.teams (
    team_id SERIAL PRIMARY KEY,
    team_code VARCHAR(50) UNIQUE NOT NULL,
    team_name VARCHAR(100) NOT NULL,
    captain_user_id INTEGER, -- FK to users
    team_config JSONB DEFAULT '{}',
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 1.4 dim_user_levels (Ensure naming consistency)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'gamification' AND tablename = 'dim_levels') THEN
        ALTER TABLE gamification.dim_levels RENAME TO dim_user_levels;
    END IF;
    
    -- Ensure columns exist (min_xp vs min_lumis)
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'gamification' AND table_name = 'dim_user_levels' AND column_name = 'min_lumis') THEN
        ALTER TABLE gamification.dim_user_levels RENAME COLUMN min_lumis TO min_xp;
    END IF;
    
    -- Ensure max_xp exists
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'gamification' AND table_name = 'dim_user_levels' AND column_name = 'max_xp') THEN
        ALTER TABLE gamification.dim_user_levels ADD COLUMN max_xp INTEGER DEFAULT 999999;
    END IF;
END $$;

-- 2. USER STATE TABLES
-- ============================================================================

-- 2.1 user_status (Enhanced user_progression)
CREATE TABLE gamification.user_status (
    user_id INTEGER PRIMARY KEY, -- FK to users
    current_level_id INTEGER NOT NULL, -- FK to dim_user_levels
    current_balance INTEGER DEFAULT 0, -- Wallet balance
    lifetime_earned INTEGER DEFAULT 0, -- Total accumulated
    current_xp INTEGER DEFAULT 0,
    total_xp INTEGER DEFAULT 0,
    last_level_up TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2.2 user_mechanics (Unifies user progress)
CREATE TABLE gamification.user_mechanics (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL, -- FK to users
    mechanic_id INTEGER NOT NULL REFERENCES gamification.dim_mechanics(mechanic_id),
    status VARCHAR(20) DEFAULT 'active' CHECK (status IN ('active', 'completed', 'expired', 'claimed')),
    progress JSONB DEFAULT '{}',
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    claimed_at TIMESTAMPTZ,
    UNIQUE(user_id, mechanic_id)
);

CREATE INDEX idx_user_mechanics_status ON gamification.user_mechanics(user_id, status);

-- 2.3 user_settings (Consolidated preferences)
CREATE TABLE gamification.user_settings (
    user_id INTEGER PRIMARY KEY, -- FK to users
    notification_prefs JSONB DEFAULT '{}',
    timezone VARCHAR(50) DEFAULT 'UTC',
    language VARCHAR(10) DEFAULT 'es',
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2.4 user_streaks (Renamed/Kept from fact_user_streaks)
-- We will rename the existing table later.

-- 2.5 team_members (Renamed from fact_team_members)
CREATE TABLE gamification.team_members (
    membership_id SERIAL PRIMARY KEY,
    team_id INTEGER NOT NULL REFERENCES gamification.teams(team_id),
    user_id INTEGER NOT NULL, -- FK to users
    role VARCHAR(20) DEFAULT 'member',
    contribution_score INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'active',
    joined_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. TRANSACTIONAL TABLES
-- ============================================================================

-- 3.1 point_ledger (Renamed from fact_engagement_transactions)
CREATE TABLE gamification.point_ledger (
    tx_id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL, -- FK to users
    amount INTEGER NOT NULL,
    source_type VARCHAR(30) NOT NULL,
    source_id INTEGER,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3.2 activity_log (Renamed from fact_user_activity_log)
-- We will rename the existing table later.

-- 3.3 leaderboard_cache (Renamed from cache_leaderboards)
-- We will rename the existing table later.

-- 4. DATA MIGRATION
-- ============================================================================

-- Migrate Achievements -> dim_mechanics
INSERT INTO gamification.dim_mechanics (
    mechanic_type, mechanic_code, mechanic_name, description,
    config, icon_url, difficulty, reward_lumis, is_active, created_at
)
SELECT 
    'achievement', achievement_code, achievement_name, description,
    jsonb_build_object(
        'category', category,
        'requirements', requirements_json,
        'is_hidden', is_hidden
    ),
    icon_url, difficulty, reward_lumis, is_active, created_at
FROM gamification.dim_achievements;

-- Migrate Events -> dim_mechanics
INSERT INTO gamification.dim_mechanics (
    mechanic_type, mechanic_code, mechanic_name, description,
    config, reward_lumis, is_active, start_date, end_date, created_at
)
SELECT 
    'event', event_code, event_name, COALESCE(config_json->>'description', event_name),
    jsonb_build_object(
        'event_type', event_type,
        'multiplier', multiplier,
        'target_actions', target_actions
    ) || config_json,
    bonus_lumis, is_active, start_date, end_date, created_at
FROM gamification.dim_events;

-- Migrate Combo Chains -> dim_mechanics
INSERT INTO gamification.dim_mechanics (
    mechanic_type, mechanic_code, mechanic_name, 
    config, reward_lumis, is_active, created_at
)
SELECT 
    'combo', combo_code, combo_name,
    jsonb_build_object(
        'combo_type', combo_type,
        'steps_required', steps_required,
        'time_window_hours', time_window_hours,
        'reward_multiplier', reward_multiplier
    ) || combo_config,
    bonus_lumis, is_active, created_at
FROM gamification.dim_combo_chains;

-- Migrate Rewards Config -> dim_rewards
INSERT INTO gamification.dim_rewards (
    reward_code, reward_name, lumis_amount, reward_type, conditions, is_active, created_at
)
SELECT 
    reward_code, reward_name, base_amount, reward_type, requirements_json, is_active, created_at
FROM gamification.dim_rewards_config;

-- Migrate User Progression -> user_status
INSERT INTO gamification.user_status (
    user_id, current_level_id, current_xp, total_xp, last_level_up, updated_at, current_balance, lifetime_earned
)
SELECT 
    user_id, current_level, current_xp, total_xp, last_level_up, updated_at, 0, 0
FROM gamification.fact_user_progression;

-- Try to update balance from rewards.fact_balance_points
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'rewards' AND tablename = 'fact_balance_points') THEN
        EXECUTE '
            UPDATE gamification.user_status us
            SET 
                current_balance = fbp.balance,
                -- We assume lifetime_earned is at least the current balance if not tracked elsewhere
                lifetime_earned = GREATEST(us.lifetime_earned, fbp.balance)
            FROM rewards.fact_balance_points fbp
            WHERE us.user_id = fbp.user_id
        ';
    END IF;
END $$;

-- Migrate User Achievements -> user_mechanics
INSERT INTO gamification.user_mechanics (
    user_id, mechanic_id, status, progress, started_at, completed_at, claimed_at
)
SELECT 
    ua.user_id,
    dm.mechanic_id,
    CASE WHEN ua.is_claimed THEN 'claimed' ELSE 'completed' END,
    jsonb_build_object(
        'unlocked_at', ua.unlocked_at,
        'progress_data', ua.progress_data
    ),
    ua.unlocked_at, ua.unlocked_at, ua.claimed_at
FROM gamification.fact_user_achievements ua
JOIN gamification.dim_achievements da ON ua.achievement_id = da.achievement_id
JOIN gamification.dim_mechanics dm ON dm.mechanic_code = da.achievement_code;

-- Migrate Engagement Transactions -> point_ledger
INSERT INTO gamification.point_ledger (
    user_id, amount, source_type, source_id, created_at
)
SELECT 
    user_id, lumis_amount, source_type, source_id, created_at
FROM gamification.fact_engagement_transactions;

-- Migrate Notification Prefs -> user_settings
INSERT INTO gamification.user_settings (
    user_id, notification_prefs, timezone, language, updated_at
)
SELECT 
    user_id,
    jsonb_build_object(
        'push_enabled', push_enabled,
        'email_enabled', email_enabled,
        'sms_enabled', sms_enabled,
        'quiet_hours_start', quiet_hours_start,
        'quiet_hours_end', quiet_hours_end,
        'frequency', frequency_preference
    ),
    timezone, language_preference, updated_at
FROM gamification.fact_user_notification_preferences;

-- 5. RENAME / CLEANUP
-- ============================================================================

-- Rename tables to keep (removing fact_ prefix where applicable)
ALTER TABLE gamification.fact_user_streaks RENAME TO user_streaks;
ALTER TABLE gamification.fact_user_activity_log RENAME TO activity_log;
ALTER TABLE gamification.cache_leaderboards RENAME TO leaderboard_cache;

-- Rename old tables to _backup_
ALTER TABLE gamification.dim_achievements RENAME TO _backup_dim_achievements;
ALTER TABLE gamification.dim_events RENAME TO _backup_dim_events;
ALTER TABLE gamification.dim_combo_chains RENAME TO _backup_dim_combo_chains;
ALTER TABLE gamification.dim_engagement_mechanics RENAME TO _backup_dim_engagement_mechanics;
ALTER TABLE gamification.dim_rewards_config RENAME TO _backup_dim_rewards_config;
ALTER TABLE gamification.dim_dynamic_rewards RENAME TO _backup_dim_dynamic_rewards;
ALTER TABLE gamification.dim_teams RENAME TO _backup_dim_teams;
ALTER TABLE gamification.dim_action_channels RENAME TO _backup_dim_action_channels;
ALTER TABLE gamification.dim_targeting_criteria RENAME TO _backup_dim_targeting_criteria;
ALTER TABLE gamification.dim_localized_events RENAME TO _backup_dim_localized_events;
ALTER TABLE gamification.dim_fraud_rules RENAME TO _backup_dim_fraud_rules;
ALTER TABLE gamification.dim_notification_templates RENAME TO _backup_dim_notification_templates;

ALTER TABLE gamification.fact_user_achievements RENAME TO _backup_fact_user_achievements;
ALTER TABLE gamification.fact_user_missions RENAME TO _backup_fact_user_missions;
ALTER TABLE gamification.fact_user_events RENAME TO _backup_fact_user_events;
ALTER TABLE gamification.fact_user_combo_progress RENAME TO _backup_fact_user_combo_progress;
ALTER TABLE gamification.fact_user_progression RENAME TO _backup_fact_user_progression;
ALTER TABLE gamification.fact_engagement_transactions RENAME TO _backup_fact_engagement_transactions;
ALTER TABLE gamification.fact_user_notification_preferences RENAME TO _backup_fact_user_notification_preferences;
ALTER TABLE gamification.fact_team_members RENAME TO _backup_fact_team_members;
ALTER TABLE gamification.fact_event_targeting RENAME TO _backup_fact_event_targeting;
ALTER TABLE gamification.fact_fraud_signals RENAME TO _backup_fact_fraud_signals;
ALTER TABLE gamification.fact_notification_queue RENAME TO _backup_fact_notification_queue;
ALTER TABLE gamification.fact_user_social RENAME TO _backup_fact_user_social;
ALTER TABLE gamification.fact_leaderboards RENAME TO _backup_fact_leaderboards;
ALTER TABLE gamification.fact_team_competitions RENAME TO _backup_fact_team_competitions;

-- 6. UPDATE FUNCTIONS
-- ============================================================================

-- Update update_user_level to use user_status and dim_user_levels
CREATE OR REPLACE FUNCTION gamification.update_user_level(p_user_id INTEGER)
RETURNS VOID AS $$
DECLARE
    v_total_invoices INTEGER;
    v_new_level INTEGER;
    v_current_level INTEGER;
BEGIN
    -- 1. Count invoices (Source of Truth)
    SELECT COUNT(*) INTO v_total_invoices
    FROM public.invoice_header
    WHERE user_id = p_user_id;
    
    -- 2. Determine level based on invoice count (XP)
    SELECT level_id INTO v_new_level
    FROM gamification.dim_user_levels
    WHERE v_total_invoices >= min_xp
    AND v_total_invoices <= max_xp
    ORDER BY min_xp DESC
    LIMIT 1;
    
    -- Default to level 1 if no match
    IF v_new_level IS NULL THEN
        SELECT level_id INTO v_new_level FROM gamification.dim_user_levels ORDER BY level_number ASC LIMIT 1;
    END IF;

    -- 3. Update user_status
    INSERT INTO gamification.user_status (user_id, current_level_id, current_xp, total_xp, updated_at)
    VALUES (p_user_id, v_new_level, v_total_invoices, v_total_invoices, NOW())
    ON CONFLICT (user_id) DO UPDATE
    SET 
        current_level_id = EXCLUDED.current_level_id,
        current_xp = EXCLUDED.current_xp,
        total_xp = EXCLUDED.total_xp,
        last_level_up = CASE 
            WHEN gamification.user_status.current_level_id != EXCLUDED.current_level_id THEN NOW() 
            ELSE gamification.user_status.last_level_up 
        END,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- 7. VIEWS
-- ============================================================================

-- Recreate v_user_dashboard
DROP MATERIALIZED VIEW IF EXISTS gamification.vw_user_dashboard;
DROP VIEW IF EXISTS gamification.v_user_dashboard; -- Drop if exists as view

CREATE VIEW gamification.v_user_dashboard AS
SELECT 
    us.user_id,
    u.email,
    us.total_xp as total_invoices,
    us.current_balance as wallet_balance,
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
LEFT JOIN gamification.dim_user_levels nl ON nl.level_number = l.level_number + 1;

COMMIT;
