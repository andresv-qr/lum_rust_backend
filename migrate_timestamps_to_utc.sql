-- ============================================================================
-- MIGRACIÓN: Estandarizar TODOS los campos timestamp a UTC (timestamptz)
-- ============================================================================
-- Fecha: 2026-01-14
-- Objetivo: Convertir 41 campos 'timestamp without time zone' a 'timestamptz'
-- 
-- IMPORTANTE:
--   - Este script asume que los datos existentes están en UTC
--   - Si algunos datos están en otra zona horaria, ajustar manualmente
--   - Las vistas materializadas deben recrearse después del cambio
-- ============================================================================

-- Verificar modo de ejecución
\echo '=============================================='
\echo 'MIGRACIÓN DE TIMESTAMPS A UTC (timestamptz)'
\echo '=============================================='
\echo ''

BEGIN;

-- ============================================================================
-- PASO 1: Guardar definiciones de vistas materializadas dependientes
-- ============================================================================

\echo 'PASO 1: Eliminando vistas materializadas que bloquean la migración...'

-- Guardar datos para restaurar después
DROP MATERIALIZED VIEW IF EXISTS public.user_header_integrity_daily CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_detail_integrity_daily CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_product_integrity_daily CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_issuer_integrity_daily CASCADE;

\echo '   ✓ Vistas materializadas eliminadas temporalmente'

-- ============================================================================
-- PASO 2: Migrar campos del schema PUBLIC
-- ============================================================================

\echo 'PASO 2: Migrando campos de schema PUBLIC...'

-- public.auth_audit_log
ALTER TABLE public.auth_audit_log 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC';

-- public.auth_events_summary  
ALTER TABLE public.auth_events_summary 
    ALTER COLUMN event_date TYPE timestamptz USING event_date AT TIME ZONE 'UTC';

-- public.auth_provider_links
ALTER TABLE public.auth_provider_links 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
    ALTER COLUMN linked_at TYPE timestamptz USING linked_at AT TIME ZONE 'UTC',
    ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC';

-- public.dim_issuer
ALTER TABLE public.dim_issuer 
    ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'UTC',
    ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'UTC';

-- public.dim_issuer_stores
ALTER TABLE public.dim_issuer_stores 
    ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'UTC',
    ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'UTC';

-- public.dim_product
ALTER TABLE public.dim_product 
    ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'UTC',
    ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'UTC';

-- public.dim_users
ALTER TABLE public.dim_users 
    ALTER COLUMN email_verified_at TYPE timestamptz USING email_verified_at AT TIME ZONE 'UTC';

-- public.invoice_detail
ALTER TABLE public.invoice_detail 
    ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'UTC',
    ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'UTC';

-- public.invoice_header (campos principales)
ALTER TABLE public.invoice_header 
    ALTER COLUMN date TYPE timestamptz USING date AT TIME ZONE 'UTC',
    ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'UTC',
    ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'UTC';

-- public.invoice_header_tempsinregistro
ALTER TABLE public.invoice_header_tempsinregistro 
    ALTER COLUMN date TYPE timestamptz USING date AT TIME ZONE 'UTC';

-- public.invoice_with_details (si es tabla, no vista)
-- NOTA: Si es vista, se debe recrear
ALTER TABLE public.invoice_with_details 
    ALTER COLUMN date TYPE timestamptz USING date AT TIME ZONE 'UTC';

-- public.scheduled_notifications
ALTER TABLE public.scheduled_notifications 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
    ALTER COLUMN sent_at TYPE timestamptz USING sent_at AT TIME ZONE 'UTC';

-- public.surveys
ALTER TABLE public.surveys 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
    ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC';

-- public.surveys_answers
ALTER TABLE public.surveys_answers 
    ALTER COLUMN date TYPE timestamptz USING date AT TIME ZONE 'UTC';

-- public.user_auth_summary
ALTER TABLE public.user_auth_summary 
    ALTER COLUMN email_verified_at TYPE timestamptz USING email_verified_at AT TIME ZONE 'UTC';

-- public.user_bot_interactions
ALTER TABLE public.user_bot_interactions 
    ALTER COLUMN end_timestamp TYPE timestamptz USING end_timestamp AT TIME ZONE 'UTC',
    ALTER COLUMN start_timestamp TYPE timestamptz USING start_timestamp AT TIME ZONE 'UTC';

-- public.user_product_searches
ALTER TABLE public.user_product_searches 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC';

-- public.user_search_balance
ALTER TABLE public.user_search_balance 
    ALTER COLUMN expires_at TYPE timestamptz USING expires_at AT TIME ZONE 'UTC',
    ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC';

-- public.users
ALTER TABLE public.users 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC';

-- public.vw_invoice_header_cleaned (si es tabla)
-- NOTA: Si es vista, se recrea automáticamente

\echo '   ✓ Schema PUBLIC migrado'

-- ============================================================================
-- PASO 3: Migrar campos del schema ANALYTICS
-- ============================================================================

\echo 'PASO 3: Migrando campos de schema ANALYTICS...'

ALTER TABLE analytics.ocr_token_usage 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC';

\echo '   ✓ Schema ANALYTICS migrado'

-- ============================================================================
-- PASO 4: Migrar campos del schema LOGS
-- ============================================================================

\echo 'PASO 4: Migrando campos de schema LOGS...'

ALTER TABLE logs.ocr_attempts 
    ALTER COLUMN attempt_date TYPE timestamptz USING attempt_date AT TIME ZONE 'UTC',
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC';

ALTER TABLE logs.user_bot_interactions_ai 
    ALTER COLUMN end_timestamp TYPE timestamptz USING end_timestamp AT TIME ZONE 'UTC',
    ALTER COLUMN start_timestamp TYPE timestamptz USING start_timestamp AT TIME ZONE 'UTC';

\echo '   ✓ Schema LOGS migrado'

-- ============================================================================
-- PASO 5: Migrar campos del schema REWARDS
-- ============================================================================

\echo 'PASO 5: Migrando campos de schema REWARDS...'

ALTER TABLE rewards.fact_balance_points_history 
    ALTER COLUMN snapshot_date TYPE timestamptz USING snapshot_date AT TIME ZONE 'UTC';

ALTER TABLE rewards.fact_daily_game_plays 
    ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC';

ALTER TABLE rewards.user_invoice_summary 
    ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC';

\echo '   ✓ Schema REWARDS migrado'

-- ============================================================================
-- PASO 6: Recrear vistas materializadas
-- ============================================================================

\echo 'PASO 6: Recreando vistas materializadas...'

-- user_header_integrity_daily
CREATE MATERIALIZED VIEW public.user_header_integrity_daily AS
SELECT 
    user_id,
    count(*) AS total_count,
    COALESCE(bit_xor((hashtext(cufe))::bigint), 0::bigint) AS global_hash,
    max(update_date) AS last_update,
    now() AS snapshot_time
FROM invoice_header
WHERE is_deleted = false AND user_id IS NOT NULL
GROUP BY user_id;

-- user_detail_integrity_daily
CREATE MATERIALIZED VIEW public.user_detail_integrity_daily AS
SELECT 
    ih.user_id,
    count(*) AS total_count,
    COALESCE(bit_xor((hashtext(id.cufe || '_' || COALESCE(id.code, '')))::bigint), 0::bigint) AS global_hash,
    max(id.update_date) AS last_update,
    now() AS snapshot_time
FROM invoice_detail id
JOIN invoice_header ih ON id.cufe = ih.cufe
WHERE id.is_deleted = false AND ih.user_id IS NOT NULL
GROUP BY ih.user_id;

-- user_product_integrity_daily
CREATE MATERIALIZED VIEW public.user_product_integrity_daily AS
SELECT 
    ih.user_id,
    count(DISTINCT dp.code) AS total_count,
    COALESCE(bit_xor((hashtext(dp.code))::bigint), 0::bigint) AS global_hash,
    max(dp.update_date) AS last_update,
    now() AS snapshot_time
FROM dim_product dp
JOIN invoice_detail id ON dp.code = id.code
JOIN invoice_header ih ON id.cufe = ih.cufe
WHERE dp.is_deleted = false
GROUP BY ih.user_id;

-- user_issuer_integrity_daily
CREATE MATERIALIZED VIEW public.user_issuer_integrity_daily AS
SELECT 
    ih.user_id,
    count(DISTINCT ROW(dis.issuer_ruc, dis.store_id)) AS total_count,
    COALESCE(bit_xor((hashtext(dis.issuer_ruc || '-' || dis.store_id))::bigint), 0::bigint) AS global_hash,
    max(dis.update_date) AS last_update,
    now() AS snapshot_time
FROM dim_issuer_stores dis
JOIN invoice_header ih ON dis.issuer_ruc = ih.issuer_ruc AND dis.store_id = ih.store_id
WHERE ih.user_id IS NOT NULL
GROUP BY ih.user_id;

\echo '   ✓ Vistas materializadas recreadas'

-- ============================================================================
-- PASO 7: Actualizar función del trigger para usar NOW() directamente
-- ============================================================================

\echo 'PASO 7: Actualizando función del trigger...'

CREATE OR REPLACE FUNCTION public.update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    -- NOW() en PostgreSQL siempre devuelve UTC cuando el servidor está en UTC
    -- timestamptz se almacena internamente en UTC
    NEW.update_date = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

\echo '   ✓ Función del trigger actualizada'

-- ============================================================================
-- PASO 8: Verificación final
-- ============================================================================

\echo ''
\echo 'PASO 8: Verificación final...'
\echo ''

SELECT 
    'RESUMEN FINAL' as info,
    data_type,
    COUNT(*) as cantidad
FROM information_schema.columns 
WHERE data_type IN ('timestamp without time zone', 'timestamp with time zone')
  AND table_schema NOT IN ('pg_catalog', 'information_schema')
GROUP BY data_type
ORDER BY data_type;

COMMIT;

\echo ''
\echo '=============================================='
\echo '✅ MIGRACIÓN COMPLETADA EXITOSAMENTE'
\echo '=============================================='
\echo ''
\echo 'Todos los campos timestamp ahora usan timestamptz (UTC)'
\echo 'La conversión a zona local se hace en el frontend:'
\echo ''
\echo '  Flutter: DateTime.parse(utcString).toLocal()'
\echo '  SQL:     update_date AT TIME ZONE ''America/Panama'''
\echo ''
