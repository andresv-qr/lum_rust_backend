-- =====================================================================
-- SCRIPT DE MIGRACIÓN UTC COMPLETO V2
-- Convierte todos los campos timestamp a timestamptz (UTC)
-- Maneja tanto vistas regulares como materializadas
-- =====================================================================
-- IMPORTANTE: Ejecutar con superusuario PostgreSQL
-- Hacer BACKUP antes de ejecutar
-- =====================================================================

BEGIN;

-- =====================================================================
-- PASO 1: ELIMINAR VISTAS REGULARES QUE DEPENDEN DE CAMPOS TIMESTAMP
-- =====================================================================

DROP VIEW IF EXISTS gamification.v_user_dashboard CASCADE;
DROP VIEW IF EXISTS public.auth_events_summary CASCADE;
DROP VIEW IF EXISTS public.invoice_with_details CASCADE;
DROP VIEW IF EXISTS public.surveys_summary CASCADE;
DROP VIEW IF EXISTS public.user_auth_summary CASCADE;
DROP VIEW IF EXISTS public.vw_invoice_header_cleaned CASCADE;
DROP VIEW IF EXISTS public.vw_usr_compras_p6m CASCADE;
DROP VIEW IF EXISTS public.vw_usr_compras_p6m_det CASCADE;
DROP VIEW IF EXISTS public.vw_usr_last_invoice CASCADE;
DROP VIEW IF EXISTS public.vw_usr_top_issuers CASCADE;
DROP VIEW IF EXISTS public.vw_usr_top_products CASCADE;
DROP VIEW IF EXISTS public.vw_usr_total_invoices CASCADE;

-- =====================================================================
-- PASO 2: ELIMINAR VISTAS MATERIALIZADAS QUE BLOQUEAN LA MIGRACIÓN
-- =====================================================================

DROP MATERIALIZED VIEW IF EXISTS analytics.mv_monthly_summary CASCADE;
DROP MATERIALIZED VIEW IF EXISTS analytics.mv_user_segments CASCADE;
DROP MATERIALIZED VIEW IF EXISTS rewards.mv_balance_summary CASCADE;
DROP MATERIALIZED VIEW IF EXISTS gamification.mv_user_stats CASCADE;
DROP MATERIALIZED VIEW IF EXISTS gamification.mv_leaderboard CASCADE;

-- =====================================================================
-- PASO 3: MIGRAR CAMPOS DE SCHEMA PUBLIC
-- =====================================================================

-- auth_audit_log
ALTER TABLE public.auth_audit_log ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';

-- auth_provider_links
ALTER TABLE public.auth_provider_links ALTER COLUMN linked_at TYPE timestamptz USING linked_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.auth_provider_links ALTER COLUMN unlinked_at TYPE timestamptz USING unlinked_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.auth_provider_links ALTER COLUMN last_used_at TYPE timestamptz USING last_used_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.auth_provider_links ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.auth_provider_links ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';

-- dim_users
ALTER TABLE public.dim_users ALTER COLUMN email_verified_at TYPE timestamptz USING email_verified_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_users ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_users ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';

-- email_verification_tokens
ALTER TABLE public.email_verification_tokens ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.email_verification_tokens ALTER COLUMN expires_at TYPE timestamptz USING expires_at AT TIME ZONE 'America/Panama';

-- invoice_header
ALTER TABLE public.invoice_header ALTER COLUMN reception_date TYPE timestamptz USING reception_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_header ALTER COLUMN process_date TYPE timestamptz USING process_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_header ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'America/Panama';

-- password_reset_tokens
ALTER TABLE public.password_reset_tokens ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.password_reset_tokens ALTER COLUMN expires_at TYPE timestamptz USING expires_at AT TIME ZONE 'America/Panama';

-- surveys
ALTER TABLE public.surveys ALTER COLUMN start_date TYPE timestamptz USING start_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.surveys ALTER COLUMN end_date TYPE timestamptz USING end_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.surveys ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';

-- surveys_answers
ALTER TABLE public.surveys_answers ALTER COLUMN answered_at TYPE timestamptz USING answered_at AT TIME ZONE 'America/Panama';

-- telegram_groups
ALTER TABLE public.telegram_groups ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';

-- users
ALTER TABLE public.users ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';

-- =====================================================================
-- PASO 4: MIGRAR CAMPOS DE SCHEMA GAMIFICATION
-- =====================================================================

ALTER TABLE gamification.dim_achievements ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.dim_mechanics ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.dim_rewards ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.fact_progress_log ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_achievements ALTER COLUMN completed_at TYPE timestamptz USING completed_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_achievements ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_mechanics ALTER COLUMN started_at TYPE timestamptz USING started_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_mechanics ALTER COLUMN completed_at TYPE timestamptz USING completed_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_mechanics ALTER COLUMN expires_at TYPE timestamptz USING expires_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_mechanics ALTER COLUMN last_progress_at TYPE timestamptz USING last_progress_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_status ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_status ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_streaks ALTER COLUMN last_activity_at TYPE timestamptz USING last_activity_at AT TIME ZONE 'America/Panama';
ALTER TABLE gamification.user_streaks ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';

-- =====================================================================
-- PASO 5: MIGRAR CAMPOS DE SCHEMA REWARDS
-- =====================================================================

ALTER TABLE rewards.dim_offers ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.dim_offers ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.fact_balance_points ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.fact_balance_points ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.fact_transactions ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.merchants ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.merchants ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.redemptions ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.redemptions ALTER COLUMN verified_at TYPE timestamptz USING verified_at AT TIME ZONE 'America/Panama';

-- =====================================================================
-- PASO 6: MIGRAR CAMPOS DE SCHEMA LOGS
-- =====================================================================

ALTER TABLE logs.ocr_logs ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';

-- =====================================================================
-- PASO 7: RECREAR VISTAS REGULARES
-- =====================================================================

-- auth_events_summary
CREATE OR REPLACE VIEW public.auth_events_summary AS
SELECT 
    date_trunc('day', created_at) AS event_date,
    event_type,
    provider,
    success,
    count(*) AS event_count,
    count(DISTINCT user_id) AS unique_users,
    count(DISTINCT ip_address) AS unique_ips
FROM auth_audit_log
GROUP BY date_trunc('day', created_at), event_type, provider, success
ORDER BY date_trunc('day', created_at) DESC, count(*) DESC;

-- surveys_summary
CREATE OR REPLACE VIEW public.surveys_summary AS
SELECT 
    surveys.survey_id,
    username,
    CASE
        WHEN sa.answers IS NULL THEN 0
        ELSE 1
    END AS completed
FROM surveys
CROSS JOIN users
LEFT JOIN surveys_answers sa USING (survey_id, username);

-- user_auth_summary
CREATE OR REPLACE VIEW public.user_auth_summary AS
SELECT 
    u.id,
    u.email,
    u.name,
    u.auth_providers,
    u.google_id,
    u.email_verified_at,
    u.last_login_provider,
    u.account_status,
    u.created_at,
    u.updated_at,
    COALESCE(
        json_agg(
            json_build_object(
                'provider_type', apl.provider_type,
                'provider_id', apl.provider_id,
                'provider_email', apl.provider_email,
                'is_primary', apl.is_primary,
                'linked_at', apl.linked_at,
                'link_method', apl.link_method
            ) ORDER BY apl.is_primary DESC, apl.linked_at
        ) FILTER (WHERE apl.id IS NOT NULL),
        '[]'::json
    ) AS provider_links
FROM dim_users u
LEFT JOIN auth_provider_links apl ON u.id = apl.user_id
GROUP BY u.id, u.email, u.name, u.auth_providers, u.google_id, 
         u.email_verified_at, u.last_login_provider, u.account_status, 
         u.created_at, u.updated_at;

-- invoice_with_details
CREATE OR REPLACE VIEW public.invoice_with_details AS
SELECT 
    a.cufe,
    a.date,
    a.user_email,
    a.user_id,
    a.issuer_name,
    a.user_phone_number,
    a.user_telegram_id,
    a.process_date,
    a.reception_date,
    a.type,
    b.partkey,
    b.code,
    b.description,
    b.quantity,
    b.unit_price,
    b.unit_discount,
    b.amount,
    b.total,
    b.itbms
FROM (
    SELECT 
        invoice_header.date,
        invoice_header.user_email,
        invoice_header.cufe,
        invoice_header.issuer_name,
        invoice_header.user_phone_number,
        invoice_header.user_id,
        invoice_header.user_telegram_id,
        invoice_header.process_date,
        invoice_header.reception_date,
        invoice_header.type
    FROM invoice_header
) a
JOIN (
    SELECT 
        invoice_detail.cufe,
        invoice_detail.partkey,
        ltrim(invoice_detail.code, '0') AS code,
        invoice_detail.description,
        invoice_detail.quantity::double precision AS quantity,
        invoice_detail.unit_price::double precision AS unit_price,
        invoice_detail.unit_discount,
        invoice_detail.amount::double precision AS amount,
        invoice_detail.total::double precision AS total,
        invoice_detail.itbms::double precision AS itbms
    FROM invoice_detail
) b USING (cufe);

-- vw_invoice_header_cleaned
CREATE OR REPLACE VIEW public.vw_invoice_header_cleaned AS
SELECT 
    invoice_header.cufe,
    invoice_header.issuer_name,
    invoice_header.date,
    invoice_header.process_date,
    CASE
        WHEN invoice_header.user_telegram_id IS NULL AND vw_telegram_users_full.telegram_id IS NULL THEN '-1'
        WHEN invoice_header.user_telegram_id IS NULL THEN vw_telegram_users_full.telegram_id::text
        ELSE invoice_header.user_telegram_id
    END AS user_telegram_id,
    CASE
        WHEN invoice_header.user_email IS NULL AND vw_telegram_users_full.email IS NULL THEN '-1'
        WHEN invoice_header.user_email IS NULL THEN vw_telegram_users_full.email::text
        ELSE invoice_header.user_email
    END AS user_email,
    invoice_header.tot_itbms,
    invoice_header.tot_amount
FROM invoice_header
LEFT JOIN vw_telegram_users_full ON (
    invoice_header.user_telegram_id = vw_telegram_users_full.telegram_id::text 
    OR invoice_header.user_email = vw_telegram_users_full.email::text
);

-- vw_usr_compras_p6m
CREATE OR REPLACE VIEW public.vw_usr_compras_p6m AS
WITH ranked_data AS (
    SELECT 
        a.user_email,
        (to_char(a.date, 'YYYYMM01'))::date AS mes,
        sum(a.tot_amount) AS monto,
        count(DISTINCT a.cufe) AS num_facturas,
        count(DISTINCT a.issuer_name) AS comercios
    FROM invoice_header a
    WHERE a.date > (CURRENT_DATE - '6 mons'::interval)
    GROUP BY a.user_email, (to_char(a.date, 'YYYYMM01'))::date
)
SELECT 
    user_email,
    jsonb_agg(jsonb_build_object('mes', mes, 'comercios', comercios, 'num_facturas', num_facturas, 'monto', monto)) AS summ_mes
FROM ranked_data
GROUP BY user_email;

-- vw_usr_compras_p6m_det
CREATE OR REPLACE VIEW public.vw_usr_compras_p6m_det AS
WITH ranked_data AS (
    SELECT 
        a.user_email,
        (to_char(a.date, 'YYYYMM01'))::date AS mes,
        sum(d.quantity::double precision) AS qty,
        sum(
            CASE
                WHEN d.unit_discount = '' THEN NULL
                ELSE d.unit_discount::double precision
            END * d.quantity::double precision
        ) AS descuentos,
        count(DISTINCT d.description) AS articulos
    FROM invoice_header a
    JOIN invoice_detail d USING (cufe)
    WHERE a.date > (CURRENT_DATE - '6 mons'::interval)
    GROUP BY a.user_email, (to_char(a.date, 'YYYYMM01'))::date
    ORDER BY a.user_email, (to_char(a.date, 'YYYYMM01'))::date DESC
)
SELECT 
    user_email,
    jsonb_agg(jsonb_build_object('mes', mes, 'qty', qty, 'descuento', descuentos, 'monto', articulos)) AS summ_mes_det
FROM ranked_data
GROUP BY user_email;

-- vw_usr_last_invoice
CREATE OR REPLACE VIEW public.vw_usr_last_invoice AS
WITH t AS (
    SELECT 
        a.cufe,
        a.issuer_name,
        a.date,
        a.process_date,
        a.user_telegram_id,
        a.user_email,
        a.tot_itbms,
        a.tot_amount
    FROM invoice_header a
    JOIN (
        SELECT 
            invoice_header.user_email,
            max(invoice_header.process_date) AS process_date
        FROM invoice_header
        GROUP BY invoice_header.user_email
    ) u USING (user_email, process_date)
)
SELECT 
    user_email,
    jsonb_agg(jsonb_build_object('issuer_name', issuer_name, 'date', date, 'tot_amount', tot_amount)) AS latest_invoice
FROM t
GROUP BY user_email;

-- vw_usr_top_issuers
CREATE OR REPLACE VIEW public.vw_usr_top_issuers AS
WITH ranked_data AS (
    SELECT 
        invoice_header.user_email,
        invoice_header.issuer_name,
        count(*) AS visitas,
        sum(invoice_header.tot_amount) AS monto,
        row_number() OVER (PARTITION BY invoice_header.user_email ORDER BY count(*) DESC) AS rank
    FROM invoice_header
    WHERE invoice_header.date > (CURRENT_DATE - '6 mons'::interval)
    GROUP BY invoice_header.user_email, invoice_header.issuer_name
),
rd2 AS (
    SELECT 
        ranked_data.user_email,
        ranked_data.issuer_name,
        ranked_data.visitas,
        ranked_data.monto
    FROM ranked_data
    WHERE ranked_data.rank <= 5
    ORDER BY ranked_data.user_email, ranked_data.rank
)
SELECT 
    user_email,
    jsonb_agg(jsonb_build_object('issuer_name', issuer_name, 'visitas', visitas, 'monto', monto)) AS top_issuers
FROM rd2
GROUP BY user_email;

-- vw_usr_top_products
CREATE OR REPLACE VIEW public.vw_usr_top_products AS
WITH ranked_data AS (
    SELECT 
        a.user_email,
        d.description,
        sum(d.quantity::double precision) AS qty,
        sum(d.amount::double precision) AS amt,
        row_number() OVER (PARTITION BY a.user_email ORDER BY sum(d.quantity::double precision) DESC) AS rank
    FROM invoice_header a
    JOIN invoice_detail d USING (cufe)
    WHERE a.date > (CURRENT_DATE - '6 mons'::interval)
    GROUP BY a.user_email, d.description
),
rd2 AS (
    SELECT 
        ranked_data.user_email,
        ranked_data.description,
        ranked_data.qty,
        ranked_data.amt
    FROM ranked_data
    WHERE ranked_data.rank <= 10
    ORDER BY ranked_data.user_email, ranked_data.rank
)
SELECT 
    user_email,
    jsonb_agg(jsonb_build_object('description', description, 'qty', qty, 'amt', amt)) AS top_products
FROM rd2
GROUP BY user_email;

-- vw_usr_total_invoices
CREATE OR REPLACE VIEW public.vw_usr_total_invoices AS
SELECT 
    user_email,
    jsonb_agg(jsonb_build_object('facturas', num_facturas, 'comercios', num_comercios, 'itbms', tot_itbms)) AS tot_invoices
FROM (
    SELECT 
        ih.user_email,
        count(DISTINCT ih.cufe) AS num_facturas,
        count(DISTINCT ih.issuer_name) AS num_comercios,
        sum(ih.tot_itbms) AS tot_itbms
    FROM invoice_header ih
    GROUP BY ih.user_email
) unnamed_subquery
GROUP BY user_email;

-- gamification.v_user_dashboard
CREATE OR REPLACE VIEW gamification.v_user_dashboard AS
SELECT 
    us.user_id,
    u.email,
    us.total_xp AS total_invoices,
    COALESCE(fbp.balance, 0::numeric) AS wallet_balance,
    l.level_id AS current_level,
    l.level_name,
    l.level_color,
    l.benefits_json AS level_benefits,
    l.min_xp AS level_min_points,
    l.max_xp AS level_max_points,
    COALESCE(nl.min_xp - us.total_xp, 0) AS invoices_to_next_level,
    nl.level_name AS next_level_name,
    COALESCE(
        (SELECT jsonb_object_agg(
            user_streaks.streak_type,
            jsonb_build_object('current', user_streaks.current_count, 'max', user_streaks.max_count)
        )
        FROM gamification.user_streaks
        WHERE user_streaks.user_id = us.user_id AND user_streaks.is_active = true),
        '{}'::jsonb
    ) AS active_streaks,
    (SELECT count(*)
     FROM gamification.user_mechanics
     WHERE user_mechanics.user_id = us.user_id AND user_mechanics.status::text = 'active') AS active_mechanics_count
FROM gamification.user_status us
JOIN dim_users u ON us.user_id = u.id
JOIN gamification.dim_user_levels l ON us.current_level_id = l.level_id
LEFT JOIN gamification.dim_user_levels nl ON nl.level_number = l.level_number + 1
LEFT JOIN rewards.fact_balance_points fbp ON us.user_id = fbp.user_id;

-- =====================================================================
-- PASO 8: RECREAR VISTAS MATERIALIZADAS (SI EXISTÍAN)
-- =====================================================================

-- analytics.mv_monthly_summary
CREATE MATERIALIZED VIEW IF NOT EXISTS analytics.mv_monthly_summary AS
SELECT 
    date_trunc('month', ih.date) as month,
    COUNT(DISTINCT ih.cufe) as total_invoices,
    COUNT(DISTINCT ih.user_email) as unique_users,
    SUM(ih.tot_amount) as total_amount,
    AVG(ih.tot_amount) as avg_ticket
FROM invoice_header ih
WHERE ih.date >= NOW() - INTERVAL '12 months'
GROUP BY date_trunc('month', ih.date)
ORDER BY month DESC;

-- rewards.mv_balance_summary
CREATE MATERIALIZED VIEW IF NOT EXISTS rewards.mv_balance_summary AS
SELECT 
    COUNT(*) as total_users,
    SUM(balance) as total_balance,
    AVG(balance) as avg_balance,
    MAX(balance) as max_balance
FROM rewards.fact_balance_points;

-- =====================================================================
-- PASO 9: ACTUALIZAR FUNCIÓN DEL TRIGGER PARA UTC
-- =====================================================================

CREATE OR REPLACE FUNCTION public.update_invoice_header_update_date()
RETURNS TRIGGER AS $$
BEGIN
    NEW.update_date = NOW() AT TIME ZONE 'UTC';
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMIT;

-- Verificación post-migración
SELECT 'VERIFICACIÓN CAMPOS TIMESTAMP' as info;
SELECT table_schema, table_name, column_name, data_type
FROM information_schema.columns
WHERE data_type = 'timestamp without time zone'
  AND table_schema IN ('public', 'gamification', 'rewards', 'logs')
  AND column_name IN ('created_at', 'updated_at', 'process_date', 'reception_date', 
                      'update_date', 'completed_at', 'started_at', 'expires_at',
                      'linked_at', 'email_verified_at', 'verified_at')
ORDER BY table_schema, table_name;
