
-- Migration: Fix missing trigger for user level update
-- Date: 2025-12-09
-- Description: Ensures that invoice_header has a trigger to call update_user_level
--              whenever a new invoice is inserted.

-- 1. Create the trigger function if it doesn't exist or update it
CREATE OR REPLACE FUNCTION gamification.trigger_update_user_level()
RETURNS TRIGGER AS $$
BEGIN
    -- Call the main update function for the user
    PERFORM gamification.update_user_level(NEW.user_id);
    
    -- Also update streaks (daily login, etc) if needed, though usually handled by API
    -- PERFORM gamification.update_user_streaks(NEW.user_id);
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. Drop old triggers that might be conflicting or named differently
DROP TRIGGER IF EXISTS trg_update_user_level ON public.invoice_header;
DROP TRIGGER IF EXISTS trg_refresh_lum_levels ON public.invoice_header; -- Old name

-- 3. Create the definitive trigger
CREATE TRIGGER trg_update_user_level
    AFTER INSERT OR DELETE ON public.invoice_header
    FOR EACH ROW
    EXECUTE FUNCTION gamification.trigger_update_user_level();

-- 4. Verify by forcing an update for all users (optional, but good for consistency)
-- DO $$
-- DECLARE
--     r RECORD;
-- BEGIN
--     FOR r IN SELECT DISTINCT user_id FROM public.invoice_header LOOP
--         PERFORM gamification.update_user_level(r.user_id);
--     END LOOP;
-- END$$;
