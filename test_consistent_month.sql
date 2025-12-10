
-- Test script for consistent_month logic
-- Simulates a user who uploaded invoices last week but not this week yet.

DO $$
DECLARE
    p_user_id INTEGER := 1; -- Test user
    current_week DATE := DATE_TRUNC('week', CURRENT_DATE);
    week_start DATE;
    has_invoice BOOLEAN;
    consecutive_weeks INTEGER := 0;
    found_first_gap BOOLEAN := FALSE;
    
    -- Simulation variables
    simulated_invoices TABLE (reception_date DATE);
BEGIN
    RAISE NOTICE 'Testing consistent_month logic...';
    RAISE NOTICE 'Current week starts: %', current_week;

    -- Scenario: User uploaded invoices last week (week 1), but not this week (week 0)
    -- We expect streak to be 1 (from last week), not 0.
    
    -- Simulate loop 0..3
    FOR i IN 0..3 LOOP
        week_start := current_week - (i * INTERVAL '1 week');
        
        -- Mock data: Invoice exists for week 1 (last week), but not week 0 (current)
        IF i = 1 THEN
            has_invoice := TRUE; -- Last week has invoice
        ELSE
            has_invoice := FALSE; -- This week (0) empty, others empty
        END IF;
        
        RAISE NOTICE 'Week % (start %): has_invoice = %', i, week_start, has_invoice;
        
        -- CURRENT LOGIC (reproduced from SQL)
        IF has_invoice THEN
            IF NOT found_first_gap THEN
                consecutive_weeks := consecutive_weeks + 1;
            END IF;
        ELSE
            -- If no invoice, mark gap
            IF NOT found_first_gap THEN
                found_first_gap := TRUE;
                RAISE NOTICE 'Gap found at week % (Current Logic breaks here)', i;
            END IF;
        END IF;
    END LOOP;
    
    RAISE NOTICE 'Result with CURRENT logic: %', consecutive_weeks;
    
    -- PROPOSED LOGIC
    RAISE NOTICE '--- Testing PROPOSED logic ---';
    consecutive_weeks := 0;
    found_first_gap := FALSE;
    
    FOR i IN 0..3 LOOP
        week_start := current_week - (i * INTERVAL '1 week');
        
        -- Mock data again
        IF i = 1 THEN has_invoice := TRUE; ELSE has_invoice := FALSE; END IF;
        
        -- NEW LOGIC:
        -- If it's current week (i=0) and empty, treat as "pending", don't break streak yet.
        -- Just don't increment count, but don't set found_first_gap either?
        -- No, if week 0 is empty, we just look at week 1.
        -- If week 0 has invoice, count it.
        
        IF has_invoice THEN
            IF NOT found_first_gap THEN
                consecutive_weeks := consecutive_weeks + 1;
            END IF;
        ELSE
            -- If no invoice
            IF i = 0 THEN
                -- Special case: Current week can be empty without breaking streak
                RAISE NOTICE 'Week 0 empty, skipping without breaking streak...';
            ELSE
                -- For past weeks, empty means gap
                IF NOT found_first_gap THEN
                    found_first_gap := TRUE;
                    RAISE NOTICE 'Gap found at week %', i;
                END IF;
            END IF;
        END IF;
    END LOOP;
    
    RAISE NOTICE 'Result with PROPOSED logic: %', consecutive_weeks;
    
END$$;
