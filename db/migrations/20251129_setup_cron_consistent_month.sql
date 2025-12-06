-- ============================================================================
-- CONFIGURACIÓN pg_cron: Batch consistent_month cada 12 horas
-- Fecha: 2025-11-29
-- ============================================================================

-- ============================================================================
-- OPCIÓN 1: Usando pg_cron (extensión PostgreSQL)
-- ============================================================================

-- Verificar si pg_cron está instalado
-- SELECT * FROM pg_extension WHERE extname = 'pg_cron';

-- Si no está instalado:
-- CREATE EXTENSION pg_cron;

-- Programar el batch para ejecutarse a las 00:00 y 12:00 cada día
SELECT cron.schedule(
    'batch_consistent_month_job',
    '0 0,12 * * *',  -- A las 00:00 y 12:00
    $$SELECT gamification.run_batch_consistent_month_with_log()$$
);

-- Para verificar jobs programados:
-- SELECT * FROM cron.job;

-- Para ver historial de ejecuciones:
-- SELECT * FROM cron.job_run_details ORDER BY start_time DESC LIMIT 20;

-- Para eliminar el job si necesitas reprogramarlo:
-- SELECT cron.unschedule('batch_consistent_month_job');


-- ============================================================================
-- OPCIÓN 2: Script bash para cron del sistema operativo
-- ============================================================================

/*
Si no tienes pg_cron, usa crontab del sistema:

1. Crear script /opt/scripts/batch_consistent_month.sh:

#!/bin/bash
PGPASSWORD="Jacobo23" psql -h localhost -U avalencia -d tfactu -c \
    "SELECT gamification.run_batch_consistent_month_with_log();" \
    >> /var/log/batch_consistent_month.log 2>&1

2. Agregar a crontab (crontab -e):

# Batch consistent_month cada 12 horas (00:00 y 12:00)
0 0,12 * * * /opt/scripts/batch_consistent_month.sh

*/


-- ============================================================================
-- OPCIÓN 3: Desde código Rust/Python (llamada manual)
-- ============================================================================

/*
En Rust con sqlx:

let result = sqlx::query!(
    "SELECT * FROM gamification.batch_consistent_month()"
)
.fetch_one(&pool)
.await?;

println!("Batch completado: {} usuarios, {} inserciones", 
    result.users_processed, result.records_inserted);


En Python con psycopg2:

cursor.execute("SELECT * FROM gamification.batch_consistent_month()")
result = cursor.fetchone()
print(f"Batch completado: {result[0]} usuarios, {result[2]} inserciones")
*/


-- ============================================================================
-- Verificación final
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '
=====================================================
CONFIGURACIÓN pg_cron COMPLETADA
=====================================================

Job programado:
  Nombre: batch_consistent_month_job
  Schedule: 0 0,12 * * * (cada 12h: 00:00 y 12:00)
  Comando: SELECT gamification.run_batch_consistent_month_with_log()

Comandos útiles:
  -- Ver jobs activos
  SELECT * FROM cron.job;
  
  -- Ver últimas ejecuciones
  SELECT * FROM cron.job_run_details 
  WHERE jobname = ''batch_consistent_month_job''
  ORDER BY start_time DESC LIMIT 10;
  
  -- Ejecutar manualmente
  SELECT * FROM gamification.batch_consistent_month();
  
  -- Eliminar job
  SELECT cron.unschedule(''batch_consistent_month_job'');
=====================================================
';
END $$;
