-- Migration: Fix consistent_month streak logic
-- Date: 2025-12-09
-- Description: Modified calculate_consistent_month_streak to not break the streak
--              if the current week has no invoices yet. It only breaks if past weeks are empty.

DROP FUNCTION IF EXISTS gamification.calculate_consistent_month_streak(integer);

CREATE OR REPLACE FUNCTION gamification.calculate_consistent_month_streak(p_user_id INTEGER)
RETURNS INTEGER AS $$
DECLARE
    consecutive_weeks INTEGER := 0;
    week_start DATE;
    current_week DATE;
    has_invoice BOOLEAN;
    found_first_gap BOOLEAN := FALSE;
BEGIN
    -- Siempre empezar desde la semana actual
    current_week := DATE_TRUNC('week', CURRENT_DATE);
    
    -- Verificar hasta 4 semanas hacia atrás (incluyendo semana actual)
    FOR i IN 0..3 LOOP
        week_start := current_week - (i * INTERVAL '1 week');
        
        -- Verificar si hay al menos 1 factura en esta semana
        SELECT EXISTS(
            SELECT 1 FROM public.invoice_header 
            WHERE user_id = p_user_id 
            AND reception_date >= week_start 
            AND reception_date < week_start + INTERVAL '1 week'
        ) INTO has_invoice;
        
        -- Si esta semana tiene factura, incrementar contador
        IF has_invoice THEN
            -- Solo contar si no hemos encontrado un gap previamente
            IF NOT found_first_gap THEN
                consecutive_weeks := consecutive_weeks + 1;
            END IF;
        ELSE
            -- Si no hay factura
            IF i = 0 THEN
                -- CASO ESPECIAL: Si es la semana actual y está vacía, NO es un gap todavía.
                -- Simplemente no sumamos a la racha, pero permitimos que continúe evaluando
                -- las semanas anteriores.
                -- Ejemplo: Lunes sin facturas, pero semana anterior completa -> Racha 1 (no 0).
                NULL;
            ELSE
                -- Si una semana PASADA está vacía, entonces sí se rompió la racha.
                IF NOT found_first_gap THEN
                    found_first_gap := TRUE;
                END IF;
            END IF;
        END IF;
    END LOOP;
    
    RETURN consecutive_weeks;
END;
$$ LANGUAGE plpgsql;

-- Recalculate streaks for all users to apply the fix immediately
SELECT gamification.update_user_streaks(user_id) FROM gamification.user_status;
