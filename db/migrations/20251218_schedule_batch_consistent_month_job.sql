-- Migration: Ensure pg_cron job exists for consistent_month batch
-- Date: 2025-12-18
-- Purpose:
--   - The invoice trigger no longer recalculates streaks for performance.
--   - consistent_month must therefore be refreshed by a scheduled job.
--   - This migration schedules (or re-schedules) the pg_cron job in an idempotent way.

DO $$
DECLARE
    v_jobid INTEGER;
BEGIN
    -- Ensure pg_cron extension exists if we have privileges.
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_cron') THEN
        BEGIN
            CREATE EXTENSION pg_cron;
            RAISE NOTICE '✅ pg_cron extension created';
        EXCEPTION WHEN insufficient_privilege THEN
            RAISE NOTICE '⚠️  pg_cron extension is missing and current role cannot CREATE EXTENSION. Skipping scheduling.';
            RETURN;
        END;
    END IF;

    -- If cron schema/tables are unavailable, skip gracefully.
    BEGIN
        SELECT jobid INTO v_jobid
        FROM cron.job
        WHERE jobname = 'batch_consistent_month_job'
        LIMIT 1;

        IF v_jobid IS NOT NULL THEN
            PERFORM cron.unschedule(v_jobid);
            RAISE NOTICE 'ℹ️  Unschedule existing job batch_consistent_month_job (jobid=%)', v_jobid;
        END IF;

        PERFORM cron.schedule(
            'batch_consistent_month_job',
            '0 0,12 * * *',
            'SELECT gamification.run_batch_consistent_month_with_log()'
        );

        RAISE NOTICE '✅ Scheduled batch_consistent_month_job (0 0,12 * * *)';
    EXCEPTION
        WHEN undefined_table OR undefined_schema THEN
            RAISE NOTICE '⚠️  cron.job table/schema not available. Is pg_cron configured (shared_preload_libraries) on this DB? Skipping scheduling.';
        WHEN insufficient_privilege THEN
            RAISE NOTICE '⚠️  Insufficient privilege to schedule pg_cron jobs. Skipping scheduling.';
        WHEN OTHERS THEN
            RAISE NOTICE '⚠️  Could not schedule pg_cron job: [%] %', SQLSTATE, SQLERRM;
    END;
END $$;
