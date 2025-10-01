-- ============================================================================
-- UNIFIED AUTH MIGRATION - FIX FOR EMPTY EMAIL USERS
-- ============================================================================
-- Fix migration for users with empty email strings
-- ============================================================================

-- First, let's fix users with empty emails by giving them proper email addresses
UPDATE public.dim_users 
SET email = CONCAT('user_', id, '@system.local')
WHERE email IS NULL OR email = '' OR TRIM(email) = '';

-- Now run the provider links creation for all users (including the fixed ones)
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
        'migration_date', NOW(),
        'system_generated_email', CASE 
            WHEN email LIKE '%@system.local' THEN true 
            ELSE false 
        END
    ) as metadata
FROM public.dim_users 
WHERE NOT EXISTS (
    SELECT 1 FROM public.auth_provider_links 
    WHERE user_id = dim_users.id AND provider_type = 'email'
)
ON CONFLICT (provider_type, provider_id) DO NOTHING;

-- Fix the log_auth_event function (had parameter order issue)
DROP FUNCTION IF EXISTS public.log_auth_event(INTEGER, VARCHAR, VARCHAR, INET, TEXT, BOOLEAN, VARCHAR, TEXT, JSONB, VARCHAR, VARCHAR);

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

-- Verification query
DO $$
DECLARE
    user_count INTEGER;
    provider_link_count INTEGER;
    users_with_providers INTEGER;
    empty_email_users INTEGER;
BEGIN
    -- Count total users
    SELECT COUNT(*) INTO user_count FROM public.dim_users;
    
    -- Count provider links
    SELECT COUNT(*) INTO provider_link_count FROM public.auth_provider_links;
    
    -- Count users with auth_providers set
    SELECT COUNT(*) INTO users_with_providers 
    FROM public.dim_users 
    WHERE auth_providers IS NOT NULL AND auth_providers != '[]'::jsonb;
    
    -- Count users with system-generated emails
    SELECT COUNT(*) INTO empty_email_users 
    FROM public.dim_users 
    WHERE email LIKE '%@system.local';
    
    RAISE NOTICE '=== MIGRATION FIX SUMMARY ===';
    RAISE NOTICE 'Total users: %', user_count;
    RAISE NOTICE 'Provider links created: %', provider_link_count;
    RAISE NOTICE 'Users with providers configured: %', users_with_providers;
    RAISE NOTICE 'System-generated emails: %', empty_email_users;
    
    IF users_with_providers = user_count AND provider_link_count >= user_count THEN
        RAISE NOTICE '✅ Migration completed successfully!';
    ELSE
        RAISE WARNING '❌ Migration needs attention!';
    END IF;
END $$;