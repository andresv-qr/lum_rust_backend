
-- Diagnostic script for user invoice count discrepancy
-- User ID: 1

DO $$
DECLARE
    v_user_id INTEGER := 1;
    v_real_count INTEGER;
    v_stored_count INTEGER;
    v_stored_level INTEGER;
BEGIN
    RAISE NOTICE '--- DIAGNOSTIC START ---';
    
    -- 1. Get Real Count from invoice_header
    SELECT COUNT(*) INTO v_real_count
    FROM public.invoice_header
    WHERE user_id = v_user_id;
    
    RAISE NOTICE 'Real Invoice Count (invoice_header): %', v_real_count;
    
    -- 2. Get Stored Count from user_status
    SELECT total_xp, current_level_id INTO v_stored_count, v_stored_level
    FROM gamification.user_status
    WHERE user_id = v_user_id;
    
    RAISE NOTICE 'Stored XP (user_status): %', v_stored_count;
    RAISE NOTICE 'Stored Level ID: %', v_stored_level;
    
    -- 3. Compare and Fix
    IF v_real_count != v_stored_count THEN
        RAISE NOTICE 'MISMATCH DETECTED! Updating user status...';
        
        -- Call the update function
        PERFORM gamification.update_user_level(v_user_id);
        
        -- Verify update
        SELECT total_xp INTO v_stored_count
        FROM gamification.user_status
        WHERE user_id = v_user_id;
        
        RAISE NOTICE 'New Stored XP after update: %', v_stored_count;
    ELSE
        RAISE NOTICE 'Counts match. No update needed.';
    END IF;
    
    RAISE NOTICE '--- DIAGNOSTIC END ---';
END$$;
