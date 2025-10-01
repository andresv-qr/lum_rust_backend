-- ============================================================================
-- UNIFIED AUTHENTICATION SYSTEM - DATABASE MIGRATION
-- ============================================================================
-- Date: September 18, 2025
-- Purpose: Extend existing schema for unified auth with Google OAuth2 support
-- Features: Multiple providers, account linking, audit logging
-- ============================================================================

-- 1. EXTEND dim_users TABLE FOR UNIFIED AUTH
-- ============================================================================

-- Add new columns for unified authentication
ALTER TABLE public.dim_users 
ADD COLUMN IF NOT EXISTS auth_providers JSONB DEFAULT '["email"]'::jsonb,
ADD COLUMN IF NOT EXISTS google_id VARCHAR(255) UNIQUE,
ADD COLUMN IF NOT EXISTS auth_metadata JSONB DEFAULT '{}'::jsonb,
ADD COLUMN IF NOT EXISTS email_verified_at TIMESTAMP,
ADD COLUMN IF NOT EXISTS last_login_provider VARCHAR(50) DEFAULT 'email',
ADD COLUMN IF NOT EXISTS account_status VARCHAR(20) DEFAULT 'active';

-- Create indexes for performance optimization
CREATE INDEX IF NOT EXISTS idx_users_google_id ON public.dim_users(google_id);
CREATE INDEX IF NOT EXISTS idx_users_auth_providers ON public.dim_users USING GIN(auth_providers);
CREATE INDEX IF NOT EXISTS idx_users_email_verified ON public.dim_users(email_verified_at);
CREATE INDEX IF NOT EXISTS idx_users_account_status ON public.dim_users(account_status);
CREATE INDEX IF NOT EXISTS idx_users_last_login_provider ON public.dim_users(last_login_provider);

-- Add constraint for account_status values
ALTER TABLE public.dim_users 
ADD CONSTRAINT check_account_status 
CHECK (account_status IN ('active', 'suspended', 'pending_verification', 'locked'));

-- Add constraint for last_login_provider values
ALTER TABLE public.dim_users 
ADD CONSTRAINT check_last_login_provider 
CHECK (last_login_provider IN ('email', 'google', 'facebook', 'apple'));

-- 2. CREATE AUTH PROVIDER LINKS TABLE
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.auth_provider_links (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.dim_users(id) ON DELETE CASCADE,
    provider_type VARCHAR(50) NOT NULL, -- 'google', 'email', 'facebook', 'apple'
    provider_id VARCHAR(255) NOT NULL,  -- Google ID, email, Facebook ID, etc.
    provider_email VARCHAR(255),        -- Email from the provider
    linked_at TIMESTAMP DEFAULT NOW(),
    link_method VARCHAR(50) NOT NULL DEFAULT 'automatic', -- 'automatic', 'manual', 'admin'
    is_primary BOOLEAN DEFAULT FALSE,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    
    -- Ensure unique provider per user
    UNIQUE(provider_type, provider_id),
    -- Ensure only one primary provider per type per user
    UNIQUE(user_id, provider_type, is_primary) DEFERRABLE INITIALLY DEFERRED
);

-- Create indexes for auth_provider_links
CREATE INDEX idx_auth_provider_links_user_id ON public.auth_provider_links(user_id);
CREATE INDEX idx_auth_provider_links_provider ON public.auth_provider_links(provider_type, provider_id);
CREATE INDEX idx_auth_provider_links_email ON public.auth_provider_links(provider_email);
CREATE INDEX idx_auth_provider_links_primary ON public.auth_provider_links(user_id, is_primary) WHERE is_primary = true;

-- Add constraints for provider_type and link_method
ALTER TABLE public.auth_provider_links 
ADD CONSTRAINT check_provider_type 
CHECK (provider_type IN ('email', 'google', 'facebook', 'apple'));

ALTER TABLE public.auth_provider_links 
ADD CONSTRAINT check_link_method 
CHECK (link_method IN ('automatic', 'manual', 'admin'));

-- 3. CREATE AUTH AUDIT LOG TABLE
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.auth_audit_log (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES public.dim_users(id) ON DELETE SET NULL,
    event_type VARCHAR(50) NOT NULL,     -- 'login', 'register', 'link_account', 'verify_email', etc.
    provider VARCHAR(50),                -- Provider used for the event
    ip_address INET,
    user_agent TEXT,
    success BOOLEAN NOT NULL,
    error_code VARCHAR(50),
    error_message TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    session_id VARCHAR(100),             -- Track sessions
    request_id VARCHAR(100),             -- Track requests
    created_at TIMESTAMP DEFAULT NOW()
);

-- Create indexes for auth_audit_log
CREATE INDEX idx_auth_audit_user_id ON public.auth_audit_log(user_id);
CREATE INDEX idx_auth_audit_event_type ON public.auth_audit_log(event_type);
CREATE INDEX idx_auth_audit_created_at ON public.auth_audit_log(created_at);
CREATE INDEX idx_auth_audit_success ON public.auth_audit_log(success);
CREATE INDEX idx_auth_audit_provider ON public.auth_audit_log(provider);
CREATE INDEX idx_auth_audit_ip_address ON public.auth_audit_log(ip_address);

-- Add constraint for event_type values
ALTER TABLE public.auth_audit_log 
ADD CONSTRAINT check_event_type 
CHECK (event_type IN (
    'login_attempt', 'login_success', 'login_failure',
    'register_attempt', 'register_success', 'register_failure',
    'google_auth', 'email_auth', 'account_linking',
    'email_verification', 'password_reset', 'password_change',
    'account_locked', 'account_unlocked', 'provider_added', 'provider_removed'
));

-- 4. AUTOMATIC MIGRATION OF EXISTING USERS
-- ============================================================================

-- Migrate existing users to the new system
UPDATE public.dim_users 
SET 
    auth_providers = '["email"]'::jsonb,
    email_verified_at = CASE 
        WHEN password_hash IS NOT NULL THEN created_at 
        ELSE NULL 
    END,
    last_login_provider = 'email',
    account_status = 'active'
WHERE auth_providers IS NULL OR auth_providers = '[]'::jsonb;

-- Create auth_provider_links entries for existing users
INSERT INTO public.auth_provider_links (user_id, provider_type, provider_id, provider_email, link_method, is_primary, metadata)
SELECT 
    id as user_id,
    'email' as provider_type,
    email as provider_id,
    email as provider_email,
    'automatic' as link_method,
    true as is_primary,
    jsonb_build_object(
        'migrated_from_legacy', true,
        'has_password', password_hash IS NOT NULL,
        'migration_date', NOW()
    ) as metadata
FROM public.dim_users 
WHERE NOT EXISTS (
    SELECT 1 FROM public.auth_provider_links 
    WHERE user_id = dim_users.id AND provider_type = 'email'
);

-- 5. CREATE HELPFUL VIEWS FOR UNIFIED AUTH
-- ============================================================================

-- View to get user with all their providers
CREATE OR REPLACE VIEW public.user_auth_summary AS
SELECT 
    u.id,
    u.email,
    u.name,
    u.auth_providers,
    u.google_id,
    u.email_verified_at,
    u.last_login_provider,
    u.account_status,
    u.created_at,
    u.updated_at,
    COALESCE(
        json_agg(
            json_build_object(
                'provider_type', apl.provider_type,
                'provider_id', apl.provider_id,
                'provider_email', apl.provider_email,
                'is_primary', apl.is_primary,
                'linked_at', apl.linked_at,
                'link_method', apl.link_method
            ) ORDER BY apl.is_primary DESC, apl.linked_at
        ) FILTER (WHERE apl.id IS NOT NULL),
        '[]'::json
    ) as provider_links
FROM public.dim_users u
LEFT JOIN public.auth_provider_links apl ON u.id = apl.user_id
GROUP BY u.id, u.email, u.name, u.auth_providers, u.google_id, 
         u.email_verified_at, u.last_login_provider, u.account_status, 
         u.created_at, u.updated_at;

-- View for audit log summary
CREATE OR REPLACE VIEW public.auth_events_summary AS
SELECT 
    DATE_TRUNC('day', created_at) as event_date,
    event_type,
    provider,
    success,
    COUNT(*) as event_count,
    COUNT(DISTINCT user_id) as unique_users,
    COUNT(DISTINCT ip_address) as unique_ips
FROM public.auth_audit_log 
GROUP BY DATE_TRUNC('day', created_at), event_type, provider, success
ORDER BY event_date DESC, event_count DESC;

-- 6. CREATE FUNCTIONS FOR UNIFIED AUTH OPERATIONS
-- ============================================================================

-- Function to safely add provider to user
CREATE OR REPLACE FUNCTION public.add_user_provider(
    p_user_id INTEGER,
    p_provider_type VARCHAR(50),
    p_provider_id VARCHAR(255),
    p_provider_email VARCHAR(255) DEFAULT NULL,
    p_is_primary BOOLEAN DEFAULT FALSE,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS BOOLEAN AS $$
DECLARE
    provider_exists BOOLEAN;
BEGIN
    -- Check if provider already exists
    SELECT EXISTS(
        SELECT 1 FROM public.auth_provider_links 
        WHERE user_id = p_user_id AND provider_type = p_provider_type
    ) INTO provider_exists;
    
    IF provider_exists THEN
        RETURN FALSE; -- Provider already exists
    END IF;
    
    -- Insert new provider link
    INSERT INTO public.auth_provider_links (
        user_id, provider_type, provider_id, provider_email, 
        is_primary, link_method, metadata
    ) VALUES (
        p_user_id, p_provider_type, p_provider_id, p_provider_email,
        p_is_primary, 'manual', p_metadata
    );
    
    -- Update user's auth_providers array
    UPDATE public.dim_users 
    SET auth_providers = auth_providers || to_jsonb(p_provider_type::text)
    WHERE id = p_user_id 
    AND NOT (auth_providers ? p_provider_type);
    
    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- Function to log auth events
CREATE OR REPLACE FUNCTION public.log_auth_event(
    p_user_id INTEGER DEFAULT NULL,
    p_event_type VARCHAR(50),
    p_provider VARCHAR(50) DEFAULT NULL,
    p_ip_address INET DEFAULT NULL,
    p_user_agent TEXT DEFAULT NULL,
    p_success BOOLEAN,
    p_error_code VARCHAR(50) DEFAULT NULL,
    p_error_message TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb,
    p_session_id VARCHAR(100) DEFAULT NULL,
    p_request_id VARCHAR(100) DEFAULT NULL
) RETURNS VOID AS $$
BEGIN
    INSERT INTO public.auth_audit_log (
        user_id, event_type, provider, ip_address, user_agent,
        success, error_code, error_message, metadata, session_id, request_id
    ) VALUES (
        p_user_id, p_event_type, p_provider, p_ip_address, p_user_agent,
        p_success, p_error_code, p_error_message, p_metadata, p_session_id, p_request_id
    );
END;
$$ LANGUAGE plpgsql;

-- 7. UPDATE TRIGGERS FOR MAINTENANCE
-- ============================================================================

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION public.update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add trigger to auth_provider_links
DROP TRIGGER IF EXISTS update_auth_provider_links_updated_at ON public.auth_provider_links;
CREATE TRIGGER update_auth_provider_links_updated_at
    BEFORE UPDATE ON public.auth_provider_links
    FOR EACH ROW
    EXECUTE FUNCTION public.update_updated_at_column();

-- ============================================================================
-- MIGRATION VERIFICATION QUERIES
-- ============================================================================

-- Verify migration success
DO $$
DECLARE
    user_count INTEGER;
    provider_link_count INTEGER;
    users_with_providers INTEGER;
BEGIN
    -- Count total users
    SELECT COUNT(*) INTO user_count FROM public.dim_users;
    
    -- Count provider links
    SELECT COUNT(*) INTO provider_link_count FROM public.auth_provider_links;
    
    -- Count users with auth_providers set
    SELECT COUNT(*) INTO users_with_providers 
    FROM public.dim_users 
    WHERE auth_providers IS NOT NULL AND auth_providers != '[]'::jsonb;
    
    RAISE NOTICE 'Migration Summary:';
    RAISE NOTICE '- Total users: %', user_count;
    RAISE NOTICE '- Provider links created: %', provider_link_count;
    RAISE NOTICE '- Users with providers configured: %', users_with_providers;
    
    IF users_with_providers != user_count THEN
        RAISE WARNING 'Not all users have auth_providers configured!';
    ELSE
        RAISE NOTICE 'Migration completed successfully!';
    END IF;
END $$;

-- ============================================================================
-- SAMPLE QUERIES FOR TESTING
-- ============================================================================

-- Test query: Get user auth summary
-- SELECT * FROM public.user_auth_summary WHERE email = 'test@example.com';

-- Test query: Get recent auth events
-- SELECT * FROM public.auth_audit_log ORDER BY created_at DESC LIMIT 10;

-- Test query: Add Google provider to user
-- SELECT public.add_user_provider(1, 'google', 'google123', 'user@gmail.com', false, '{"verified": true}'::jsonb);

-- Test query: Log auth event
-- SELECT public.log_auth_event(1, 'login_success', 'google', '127.0.0.1'::inet, 'Mozilla/5.0...', true, NULL, NULL, '{"method": "oauth2"}'::jsonb);

-- ============================================================================
-- END OF MIGRATION SCRIPT
-- ============================================================================