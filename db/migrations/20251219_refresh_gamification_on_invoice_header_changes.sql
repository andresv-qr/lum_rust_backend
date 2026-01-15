-- Migration: Refresh gamification (level + streaks) on invoice_header changes
-- Date: 2025-12-19
-- Purpose:
--   - Ensure consistent_month updates automatically when invoices are inserted,
--     soft-deleted (is_deleted), or restored.
--   - Ensure user level stays consistent with invoice source-of-truth.
-- Notes:
--   - We update on INSERT/UPDATE/DELETE.
--   - For UPDATE, we only do work when is_deleted or user_id changes.

BEGIN;

CREATE OR REPLACE FUNCTION gamification.trigger_update_user_level()
RETURNS TRIGGER AS $$
DECLARE
    v_user_id_new BIGINT;
    v_user_id_old BIGINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        v_user_id_new := NEW.user_id;
    ELSIF TG_OP = 'DELETE' THEN
        v_user_id_old := OLD.user_id;
    ELSE
        -- UPDATE
        v_user_id_new := NEW.user_id;
        v_user_id_old := OLD.user_id;

        -- If nothing relevant changed, skip.
        IF NEW.is_deleted IS NOT DISTINCT FROM OLD.is_deleted
           AND NEW.user_id IS NOT DISTINCT FROM OLD.user_id THEN
            RETURN NEW;
        END IF;
    END IF;

    -- Update for NEW user_id (INSERT/UPDATE)
    IF v_user_id_new IS NOT NULL THEN
        PERFORM gamification.update_user_level(v_user_id_new::int);
        PERFORM gamification.update_user_streaks(v_user_id_new::int);
    END IF;

    -- If user_id changed on UPDATE or DELETE, also update for OLD
    IF v_user_id_old IS NOT NULL AND v_user_id_old IS DISTINCT FROM v_user_id_new THEN
        PERFORM gamification.update_user_level(v_user_id_old::int);
        PERFORM gamification.update_user_streaks(v_user_id_old::int);
    END IF;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Replace trigger to include UPDATE (soft-delete / restore) as well.
DROP TRIGGER IF EXISTS trg_update_user_level ON public.invoice_header;

CREATE TRIGGER trg_update_user_level
    AFTER INSERT OR UPDATE OR DELETE ON public.invoice_header
    FOR EACH ROW
    EXECUTE FUNCTION gamification.trigger_update_user_level();

COMMIT;
