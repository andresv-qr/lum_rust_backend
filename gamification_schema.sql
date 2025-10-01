-- ============================================================================
-- GAMIFICATION SYSTEM DATABASE SCHEMA
-- ============================================================================
-- Comprehensive gamification system for engagement and user retention
-- Author: System Design Team
-- Date: August 27, 2025
-- ============================================================================

-- Create gamification schema
CREATE SCHEMA IF NOT EXISTS gamification;

-- ============================================================================
-- 1. CORE CONFIGURATION TABLES
-- ============================================================================

-- Table: Engagement mechanics configuration
CREATE TABLE gamification.dim_engagement_mechanics (
    mechanic_id SERIAL PRIMARY KEY,
    mechanic_code VARCHAR(50) UNIQUE NOT NULL, -- 'daily_streak', 'survey_bonus', 'happy_hour'
    mechanic_name VARCHAR(100) NOT NULL,
    mechanic_type VARCHAR(30) NOT NULL, -- 'streak', 'mission', 'event', 'multiplier'
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    config_json JSONB, -- Flexible configuration
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Rewards configuration
CREATE TABLE gamification.dim_rewards_config (
    reward_id SERIAL PRIMARY KEY,
    reward_code VARCHAR(50) UNIQUE NOT NULL, -- 'daily_login_bonus', 'streak_7_days'
    reward_name VARCHAR(100) NOT NULL,
    reward_type VARCHAR(30) NOT NULL, -- 'lumis', 'multiplier', 'unlock', 'badge'
    base_amount INTEGER NOT NULL, -- Base Lümis amount
    multiplier DECIMAL(3,2) DEFAULT 1.00,
    rarity VARCHAR(20) DEFAULT 'common', -- 'common', 'rare', 'epic', 'legendary'
    requirements_json JSONB, -- Conditions to obtain reward
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Temporal events
CREATE TABLE gamification.dim_events (
    event_id SERIAL PRIMARY KEY,
    event_code VARCHAR(50) UNIQUE NOT NULL, -- 'christmas_2025', 'happy_hour_evening'
    event_name VARCHAR(100) NOT NULL,
    event_type VARCHAR(30) NOT NULL, -- 'seasonal', 'daily', 'flash', 'tournament'
    start_date TIMESTAMP WITH TIME ZONE NOT NULL,
    end_date TIMESTAMP WITH TIME ZONE NOT NULL,
    multiplier DECIMAL(3,2) DEFAULT 1.00,
    bonus_lumis INTEGER DEFAULT 0,
    target_actions JSONB, -- Actions that qualify for the event
    config_json JSONB, -- Event-specific configuration
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Targeting criteria for personalized events
CREATE TABLE gamification.dim_targeting_criteria (
    criteria_id SERIAL PRIMARY KEY,
    criteria_code VARCHAR(50) UNIQUE NOT NULL, -- 'age_25_35', 'premium_users', 'inactive_7days'
    criteria_name VARCHAR(100) NOT NULL,
    criteria_type VARCHAR(30) NOT NULL, -- 'demographic', 'behavioral', 'segment', 'location'
    evaluation_query TEXT NOT NULL, -- SQL query to evaluate the criteria
    cache_ttl_seconds INTEGER DEFAULT 3600,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Event targeting relationships
CREATE TABLE gamification.fact_event_targeting (
    event_id INTEGER REFERENCES gamification.dim_events(event_id),
    criteria_id INTEGER REFERENCES gamification.dim_targeting_criteria(criteria_id),
    is_required BOOLEAN DEFAULT TRUE, -- AND vs OR logic
    weight DECIMAL(3,2) DEFAULT 1.00, -- For targeting score
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY(event_id, criteria_id)
);

-- Table: Dynamic rewards with variable states
CREATE TABLE gamification.dim_dynamic_rewards (
    dynamic_reward_id SERIAL PRIMARY KEY,
    reward_code VARCHAR(50) UNIQUE NOT NULL,
    reward_name VARCHAR(100) NOT NULL,
    reward_type VARCHAR(30) NOT NULL, -- 'probability_pool', 'progressive', 'tiered', 'conditional'
    base_config JSONB NOT NULL, -- Base configuration
    current_state JSONB, -- Current state (jackpot amount, probability changes, etc.)
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    reset_schedule VARCHAR(50), -- 'daily', 'weekly', 'monthly', 'never', cron expression
    next_reset TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Combo chains configuration
CREATE TABLE gamification.dim_combo_chains (
    combo_id SERIAL PRIMARY KEY,
    combo_code VARCHAR(50) UNIQUE NOT NULL,
    combo_name VARCHAR(100) NOT NULL,
    combo_type VARCHAR(30) NOT NULL, -- 'sequential', 'simultaneous', 'time_window'
    steps_required INTEGER NOT NULL,
    time_window_hours INTEGER, -- NULL = no time limit
    reward_multiplier DECIMAL(3,2) DEFAULT 1.00,
    bonus_lumis INTEGER DEFAULT 0,
    unlock_achievement VARCHAR(50), -- achievement_code to unlock
    combo_config JSONB NOT NULL, -- Step configuration
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================================
-- 2. USER TRACKING TABLES
-- ============================================================================

-- Table: User streaks tracking
CREATE TABLE gamification.fact_user_streaks (
    streak_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    streak_type VARCHAR(30) NOT NULL, -- 'daily_login', 'invoice_upload', 'survey_completion'
    current_count INTEGER DEFAULT 0,
    max_count INTEGER DEFAULT 0,
    last_activity_date DATE NOT NULL,
    streak_start_date DATE NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    freeze_count INTEGER DEFAULT 0, -- Times used "freeze"
    total_lumis_earned INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(user_id, streak_type)
);

-- Table: User missions/challenges
CREATE TABLE gamification.fact_user_missions (
    mission_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    mission_code VARCHAR(50) NOT NULL,
    mission_name VARCHAR(100) NOT NULL,
    mission_type VARCHAR(30) NOT NULL, -- 'daily', 'weekly', 'monthly', 'special'
    target_count INTEGER NOT NULL,
    current_progress INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'active', -- 'active', 'completed', 'expired', 'claimed'
    assigned_date DATE NOT NULL,
    due_date DATE,
    completed_at TIMESTAMP WITH TIME ZONE,
    reward_lumis INTEGER NOT NULL,
    bonus_multiplier DECIMAL(3,2) DEFAULT 1.00,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: User event participation
CREATE TABLE gamification.fact_user_events (
    participation_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    event_id INTEGER NOT NULL REFERENCES gamification.dim_events(event_id),
    participation_date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    actions_completed INTEGER DEFAULT 0,
    lumis_earned INTEGER DEFAULT 0,
    rank_position INTEGER, -- For leaderboards
    is_winner BOOLEAN DEFAULT FALSE,
    bonus_earned INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: User combo progress
CREATE TABLE gamification.fact_user_combo_progress (
    progress_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    combo_id INTEGER NOT NULL REFERENCES gamification.dim_combo_chains(combo_id),
    current_step INTEGER DEFAULT 0,
    steps_completed JSONB, -- Array of completed steps with timestamps
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) DEFAULT 'active', -- 'active', 'completed', 'expired'
    
    UNIQUE(user_id, combo_id, started_at) -- Allows multiple attempts
);

-- ============================================================================
-- 3. ACHIEVEMENTS SYSTEM
-- ============================================================================

-- Table: Available achievements/badges
CREATE TABLE gamification.dim_achievements (
    achievement_id SERIAL PRIMARY KEY,
    achievement_code VARCHAR(50) UNIQUE NOT NULL, -- 'first_invoice', 'survey_master'
    achievement_name VARCHAR(100) NOT NULL,
    description TEXT,
    category VARCHAR(30) NOT NULL, -- 'invoices', 'surveys', 'social', 'streaks'
    difficulty VARCHAR(20) DEFAULT 'bronze', -- 'bronze', 'silver', 'gold', 'platinum'
    icon_url VARCHAR(200),
    requirements_json JSONB, -- Conditions to unlock
    reward_lumis INTEGER DEFAULT 0,
    is_hidden BOOLEAN DEFAULT FALSE, -- For secret achievements
    sort_order INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: User unlocked achievements
CREATE TABLE gamification.fact_user_achievements (
    user_achievement_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    achievement_id INTEGER NOT NULL REFERENCES gamification.dim_achievements(achievement_id),
    unlocked_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    progress_data JSONB, -- Progress data when unlocked
    is_claimed BOOLEAN DEFAULT FALSE,
    claimed_at TIMESTAMP WITH TIME ZONE,
    
    UNIQUE(user_id, achievement_id)
);

-- ============================================================================
-- 4. LEVELS AND PROGRESSION
-- ============================================================================

-- Table: User levels configuration
CREATE TABLE gamification.dim_user_levels (
    level_id SERIAL PRIMARY KEY,
    level_number INTEGER UNIQUE NOT NULL,
    level_name VARCHAR(50) NOT NULL, -- 'Bronze Explorer', 'Silver Hunter'
    min_xp INTEGER NOT NULL,
    max_xp INTEGER NOT NULL,
    level_color VARCHAR(7), -- Hex color
    icon_url VARCHAR(200),
    benefits_json JSONB, -- Level benefits
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: User experience and level
CREATE TABLE gamification.fact_user_progression (
    user_id INTEGER PRIMARY KEY REFERENCES public.dim_users(id),
    current_level INTEGER NOT NULL REFERENCES gamification.dim_user_levels(level_id),
    current_xp INTEGER DEFAULT 0,
    total_xp INTEGER DEFAULT 0,
    prestige_count INTEGER DEFAULT 0, -- Times done "prestige"
    last_level_up TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================================
-- 5. SOCIAL AND COMPETITION
-- ============================================================================

-- Table: Leaderboards
CREATE TABLE gamification.fact_leaderboards (
    leaderboard_id SERIAL PRIMARY KEY,
    leaderboard_type VARCHAR(30) NOT NULL, -- 'weekly_invoices', 'monthly_surveys'
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    score INTEGER NOT NULL,
    rank_position INTEGER NOT NULL,
    reward_lumis INTEGER DEFAULT 0,
    is_final BOOLEAN DEFAULT FALSE, -- If period is finalized
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(leaderboard_type, period_start, user_id)
);

-- Table: Social connections (friends/referrals)
CREATE TABLE gamification.fact_user_social (
    connection_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    connected_user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    connection_type VARCHAR(20) NOT NULL, -- 'friend', 'referral', 'team'
    status VARCHAR(20) DEFAULT 'active', -- 'active', 'pending', 'blocked'
    established_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(user_id, connected_user_id, connection_type)
);

-- Table: Teams configuration
CREATE TABLE gamification.dim_teams (
    team_id SERIAL PRIMARY KEY,
    team_code VARCHAR(50) UNIQUE NOT NULL,
    team_name VARCHAR(100) NOT NULL,
    team_type VARCHAR(30) DEFAULT 'casual', -- 'casual', 'competitive', 'corporate'
    max_members INTEGER DEFAULT 10,
    current_members INTEGER DEFAULT 0,
    team_level INTEGER DEFAULT 1,
    team_xp INTEGER DEFAULT 0,
    team_config JSONB, -- Team configuration
    captain_user_id INTEGER REFERENCES public.dim_users(id),
    status VARCHAR(20) DEFAULT 'active', -- 'active', 'disbanded', 'suspended'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Team memberships
CREATE TABLE gamification.fact_team_members (
    membership_id SERIAL PRIMARY KEY,
    team_id INTEGER NOT NULL REFERENCES gamification.dim_teams(team_id),
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    role VARCHAR(20) DEFAULT 'member', -- 'captain', 'co_captain', 'member'
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    contribution_score INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'active', -- 'active', 'inactive', 'kicked'
    
    UNIQUE(team_id, user_id)
);

-- Table: Team competitions
CREATE TABLE gamification.fact_team_competitions (
    competition_id SERIAL PRIMARY KEY,
    competition_code VARCHAR(50) UNIQUE NOT NULL,
    competition_name VARCHAR(100) NOT NULL,
    competition_type VARCHAR(30) NOT NULL, -- 'tournament', 'league', 'event'
    start_date TIMESTAMP WITH TIME ZONE NOT NULL,
    end_date TIMESTAMP WITH TIME ZONE NOT NULL,
    max_teams INTEGER,
    entry_requirements JSONB, -- Requirements to participate
    reward_structure JSONB, -- Prize structure
    status VARCHAR(20) DEFAULT 'upcoming', -- 'upcoming', 'active', 'completed', 'cancelled'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================================
-- 6. FRAUD DETECTION AND ANTI-GAMING
-- ============================================================================

-- Table: Fraud detection rules
CREATE TABLE gamification.dim_fraud_rules (
    rule_id SERIAL PRIMARY KEY,
    rule_code VARCHAR(50) UNIQUE NOT NULL,
    rule_name VARCHAR(100) NOT NULL,
    rule_type VARCHAR(30) NOT NULL, -- 'velocity', 'pattern', 'duplicate', 'anomaly'
    detection_query TEXT NOT NULL, -- Query to detect the pattern
    threshold_config JSONB NOT NULL, -- Threshold configuration
    penalty_action VARCHAR(30) DEFAULT 'warning', -- 'warning', 'reduce_rewards', 'suspend'
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Fraud signals
CREATE TABLE gamification.fact_fraud_signals (
    signal_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    signal_type VARCHAR(30) NOT NULL, -- 'duplicate_action', 'suspicious_pattern', 'velocity_anomaly'
    signal_severity VARCHAR(20) DEFAULT 'low', -- 'low', 'medium', 'high', 'critical'
    signal_data JSONB NOT NULL, -- Signal details
    action_context JSONB, -- Context of the action that generated the signal
    auto_resolved BOOLEAN DEFAULT FALSE,
    resolution_action VARCHAR(50), -- 'ignored', 'warning', 'penalty', 'suspension'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolved_by INTEGER REFERENCES public.dim_users(id)
);

-- ============================================================================
-- 7. LOCALIZATION AND TIMEZONE SUPPORT
-- ============================================================================

-- Table: Localized events
CREATE TABLE gamification.dim_localized_events (
    localized_event_id SERIAL PRIMARY KEY,
    base_event_id INTEGER REFERENCES gamification.dim_events(event_id),
    region_code VARCHAR(10) NOT NULL, -- 'PA', 'US', 'MX'
    timezone VARCHAR(50) NOT NULL, -- 'America/Panama', 'America/New_York'
    local_start_time TIME NOT NULL, -- Local time of the event
    local_end_time TIME NOT NULL,
    local_config_override JSONB, -- Region-specific configuration override
    holiday_calendar JSONB, -- Holidays that affect the event
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(base_event_id, region_code)
);

-- ============================================================================
-- 8. NOTIFICATIONS SYSTEM
-- ============================================================================

-- Table: Notification templates
CREATE TABLE gamification.dim_notification_templates (
    template_id SERIAL PRIMARY KEY,
    template_code VARCHAR(50) UNIQUE NOT NULL,
    template_name VARCHAR(100) NOT NULL,
    notification_type VARCHAR(30) NOT NULL, -- 'push', 'email', 'sms', 'in_app'
    template_content JSONB NOT NULL, -- Templates by language
    template_config JSONB, -- Specific configuration (deep links, etc.)
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: User notification preferences
CREATE TABLE gamification.fact_user_notification_preferences (
    user_id INTEGER PRIMARY KEY REFERENCES public.dim_users(id),
    push_enabled BOOLEAN DEFAULT TRUE,
    email_enabled BOOLEAN DEFAULT TRUE,
    sms_enabled BOOLEAN DEFAULT FALSE,
    quiet_hours_start TIME, -- "Do not disturb" from
    quiet_hours_end TIME, -- "Do not disturb" until
    frequency_preference VARCHAR(20) DEFAULT 'normal', -- 'minimal', 'normal', 'all'
    timezone VARCHAR(50) DEFAULT 'America/Panama',
    language_preference VARCHAR(10) DEFAULT 'es',
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: Notification queue
CREATE TABLE gamification.fact_notification_queue (
    queue_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    template_id INTEGER NOT NULL REFERENCES gamification.dim_notification_templates(template_id),
    notification_type VARCHAR(30) NOT NULL,
    scheduled_for TIMESTAMP WITH TIME ZONE NOT NULL,
    message_data JSONB NOT NULL, -- Variables for the template
    priority VARCHAR(20) DEFAULT 'normal', -- 'low', 'normal', 'high', 'urgent'
    status VARCHAR(20) DEFAULT 'pending', -- 'pending', 'sent', 'failed', 'cancelled'
    attempts INTEGER DEFAULT 0,
    max_attempts INTEGER DEFAULT 3,
    sent_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================================
-- 9. TRANSACTIONS AND LOGS
-- ============================================================================

-- Table: Gamification transactions
CREATE TABLE gamification.fact_engagement_transactions (
    transaction_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    source_type VARCHAR(30) NOT NULL, -- 'streak_bonus', 'mission_complete', 'event_participation'
    source_id INTEGER, -- ID of mission, event, etc.
    action_type VARCHAR(30) NOT NULL, -- 'daily_login', 'invoice_upload', 'survey_complete'
    lumis_amount INTEGER NOT NULL,
    xp_amount INTEGER DEFAULT 0,
    multiplier_applied DECIMAL(3,2) DEFAULT 1.00,
    event_context JSONB, -- Additional event context
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: User activity log (partitioned for performance)
CREATE TABLE gamification.fact_user_activity_log (
    activity_id BIGSERIAL,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id),
    activity_type VARCHAR(30) NOT NULL, -- 'login', 'invoice_upload', 'survey_start'
    activity_data JSONB, -- Activity-specific data
    session_id VARCHAR(100), -- For session tracking
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (activity_id, created_at)
) PARTITION BY RANGE (created_at);

-- ============================================================================
-- 10. PERFORMANCE AND CACHING
-- ============================================================================

-- Table: Cached leaderboards
CREATE TABLE gamification.cache_leaderboards (
    cache_id SERIAL PRIMARY KEY,
    leaderboard_type VARCHAR(30) NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    rankings_data JSONB NOT NULL, -- Top 1000 pre-computed
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(leaderboard_type, period_start)
);

-- ============================================================================
-- 11. INDEXES FOR PERFORMANCE
-- ============================================================================

-- Core performance indexes
-- CREATE INDEX idx_user_activity_log_user_date ON gamification.fact_user_activity_log(user_id, created_at DESC); -- Created after partitions
CREATE INDEX idx_user_streaks_user_type ON gamification.fact_user_streaks(user_id, streak_type);
CREATE INDEX idx_engagement_transactions_user_date ON gamification.fact_engagement_transactions(user_id, created_at DESC);
CREATE INDEX idx_leaderboards_type_period ON gamification.fact_leaderboards(leaderboard_type, period_start, rank_position);
CREATE INDEX idx_events_active_dates ON gamification.dim_events(is_active, start_date, end_date);
CREATE INDEX idx_user_missions_status ON gamification.fact_user_missions(user_id, status);
CREATE INDEX idx_user_events_participation ON gamification.fact_user_events(user_id, event_id);
CREATE INDEX idx_fraud_signals_user ON gamification.fact_fraud_signals(user_id, created_at DESC);
CREATE INDEX idx_notification_queue_scheduled ON gamification.fact_notification_queue(scheduled_for, status);
CREATE INDEX idx_targeting_criteria_type ON gamification.dim_targeting_criteria(criteria_type, is_active);

-- Composite indexes for complex queries
CREATE INDEX idx_user_combo_active ON gamification.fact_user_combo_progress(user_id, status, expires_at);
CREATE INDEX idx_team_members_active ON gamification.fact_team_members(team_id, status);
CREATE INDEX idx_event_targeting_required ON gamification.fact_event_targeting(event_id, is_required);

-- ============================================================================
-- 12. MATERIALIZED VIEWS
-- ============================================================================

-- Materialized view for user dashboard (updated hourly)
CREATE MATERIALIZED VIEW gamification.vw_user_dashboard AS
SELECT 
    u.id as user_id,
    COALESCE(up.current_level, 1) as current_level,
    COALESCE(up.current_xp, 0) as current_xp,
    COALESCE(ul.level_name, 'Bronze Explorer') as level_name,
    
    -- Active streaks
    COALESCE(MAX(CASE WHEN us.streak_type = 'daily_login' THEN us.current_count END), 0) as daily_login_streak,
    COALESCE(MAX(CASE WHEN us.streak_type = 'invoice_upload' THEN us.current_count END), 0) as invoice_streak,
    
    -- Active missions
    COUNT(CASE WHEN um.status = 'active' THEN 1 END) as active_missions,
    COUNT(CASE WHEN um.status = 'completed' AND um.created_at >= CURRENT_DATE THEN 1 END) as missions_completed_today,
    
    -- Achievements
    COUNT(ua.achievement_id) as total_achievements,
    COUNT(CASE WHEN ua.unlocked_at >= CURRENT_DATE - INTERVAL '7 days' THEN 1 END) as recent_achievements,
    
    -- Leaderboard position
    MIN(fl.rank_position) as best_rank_this_week
    
FROM public.dim_users u
LEFT JOIN gamification.fact_user_progression up ON u.id = up.user_id
LEFT JOIN gamification.dim_user_levels ul ON up.current_level = ul.level_id
LEFT JOIN gamification.fact_user_streaks us ON u.id = us.user_id AND us.is_active = TRUE
LEFT JOIN gamification.fact_user_missions um ON u.id = um.user_id
LEFT JOIN gamification.fact_user_achievements ua ON u.id = ua.user_id
LEFT JOIN gamification.fact_leaderboards fl ON u.id = fl.user_id 
    AND fl.period_start >= CURRENT_DATE - INTERVAL '7 days'
GROUP BY u.id, up.current_level, up.current_xp, ul.level_name;

-- Create unique index on the materialized view
CREATE UNIQUE INDEX idx_user_dashboard_user_id ON gamification.vw_user_dashboard(user_id);

-- ============================================================================
-- 13. PARTITIONS (Create monthly partitions for large tables)
-- ============================================================================

-- Create first partition for activity log
CREATE TABLE gamification.fact_user_activity_log_2025_08 PARTITION OF 
gamification.fact_user_activity_log 
FOR VALUES FROM ('2025-08-01') TO ('2025-09-01');

CREATE TABLE gamification.fact_user_activity_log_2025_09 PARTITION OF 
gamification.fact_user_activity_log 
FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');

-- Create indexes on partitions
CREATE INDEX idx_user_activity_log_2025_08_user_date ON gamification.fact_user_activity_log_2025_08(user_id, created_at DESC);
CREATE INDEX idx_user_activity_log_2025_09_user_date ON gamification.fact_user_activity_log_2025_09(user_id, created_at DESC);

-- ============================================================================
-- 14. FUNCTIONS FOR COMMON OPERATIONS
-- ============================================================================

-- Function to get active events for user
CREATE OR REPLACE FUNCTION get_active_events_for_user(
    p_user_id integer,
    p_action_type text,
    p_timestamp timestamp with time zone DEFAULT now()
) 
RETURNS TABLE(
    event_id integer,
    event_code varchar,
    multiplier decimal,
    bonus_lumis integer,
    config_json jsonb
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        e.event_id,
        e.event_code,
        e.multiplier,
        e.bonus_lumis,
        e.config_json
    FROM gamification.dim_events e
    LEFT JOIN gamification.fact_user_progression up ON up.user_id = p_user_id
    LEFT JOIN gamification.dim_user_levels ul ON up.current_level = ul.level_id
    WHERE 
        e.is_active = true
        AND p_timestamp BETWEEN e.start_date AND e.end_date
        AND e.target_actions::jsonb ? p_action_type
        AND (
            -- No user restrictions
            e.config_json->'user_restrictions' IS NULL
            OR
            -- User in specific list
            e.config_json->'user_restrictions'->'specific_user_ids' @> p_user_id::text::jsonb
            OR
            -- User level qualifies
            e.config_json->'user_restrictions'->'user_levels' @> ('"' || LOWER(ul.level_name) || '"')::jsonb
        )
    ORDER BY e.multiplier DESC; -- Apply best multiplier first
END;
$$ LANGUAGE plpgsql;

-- Function to create daily happy hour events
CREATE OR REPLACE FUNCTION create_daily_happy_hour() 
RETURNS void AS $$
DECLARE
    current_date date := CURRENT_DATE;
    start_time time := '18:00:00';
    end_time time := '20:00:00';
BEGIN
    INSERT INTO gamification.dim_events (
        event_code,
        event_name,
        event_type,
        start_date,
        end_date,
        multiplier,
        target_actions
    ) VALUES (
        'happy_hour_' || to_char(current_date, 'YYYY_MM_DD'),
        'Happy Hour ' || to_char(current_date, 'DD/MM/YYYY'),
        'daily',
        (current_date + start_time) AT TIME ZONE 'America/Panama',
        (current_date + end_time) AT TIME ZONE 'America/Panama',
        2.00,
        '["invoice_upload", "survey_complete", "daily_login"]'
    );
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- END OF SCHEMA
-- ============================================================================
