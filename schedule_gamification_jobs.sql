-- 1. Definir la función wrapper con logging
CREATE OR REPLACE FUNCTION gamification.run_batch_consistent_month_with_log()
 RETURNS text
 LANGUAGE plpgsql
AS $function$
DECLARE
    v_result RECORD;
    v_message TEXT;
BEGIN
    SELECT * INTO v_result FROM gamification.batch_consistent_month();
    
    v_message := format(
        'Batch consistent_month completado:
        - Usuarios procesados: %s
        - Registros eliminados: %s
        - Registros insertados: %s
        - Streaks actualizados: %s
        - Tiempo de ejecución: %s ms',
        v_result.users_processed,
        v_result.records_deleted,
        v_result.records_inserted,
        v_result.streaks_updated,
        v_result.execution_time_ms
    );
    
    -- Log en tabla de auditoría si existe
    BEGIN
        INSERT INTO gamification.fact_audit_log (
            action_type,
            action_details,
            performed_by,
            created_at
        ) VALUES (
            'batch_consistent_month',
            jsonb_build_object(
                'users_processed', v_result.users_processed,
                'records_deleted', v_result.records_deleted,
                'records_inserted', v_result.records_inserted,
                'streaks_updated', v_result.streaks_updated,
                'execution_time_ms', v_result.execution_time_ms
            ),
            'system',
            NOW()
        );
    EXCEPTION WHEN OTHERS THEN
        -- Ignorar si no existe la tabla de auditoría
        NULL;
    END;

    RETURN v_message;
END;
$function$;

-- 2. Limpiar jobs existentes para evitar duplicados
-- Nota: cron.unschedule puede requerir permisos de superusuario o ser el dueño del job.
-- Intentamos borrar por nombre si es posible, o por comando.
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT jobid FROM cron.job WHERE command LIKE '%gamification.run_batch_consistent_month_with_log%' LOOP
        PERFORM cron.unschedule(r.jobid);
    END LOOP;
END$$;

-- 3. Programar Job 1: 02:00 AM Panama (07:00 UTC)
SELECT cron.schedule(
    'gamification_consistent_month_am',
    '0 7 * * *',
    'SELECT gamification.run_batch_consistent_month_with_log()'
);

-- 4. Programar Job 2: 11:40 AM Panama (16:40 UTC)
SELECT cron.schedule(
    'gamification_consistent_month_pm',
    '40 16 * * *',
    'SELECT gamification.run_batch_consistent_month_with_log()'
);
