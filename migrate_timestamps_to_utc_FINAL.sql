-- =====================================================================
-- SCRIPT DE MIGRACIÓN UTC FINAL
-- Convierte todos los campos timestamp a timestamptz (UTC)
-- =====================================================================

BEGIN;

-- =====================================================================
-- PASO 1: ELIMINAR VISTAS MATERIALIZADAS
-- =====================================================================

DROP MATERIALIZED VIEW IF EXISTS public.user_detail_integrity_daily CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_header_integrity_daily CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_issuer_integrity_daily CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_product_integrity_daily CASCADE;

-- =====================================================================
-- PASO 2: ELIMINAR VISTAS REGULARES
-- =====================================================================

DROP VIEW IF EXISTS public.vw_usr_general_metrics CASCADE;
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
-- PASO 3: MIGRAR CAMPOS DE LOGS SCHEMA
-- =====================================================================

ALTER TABLE logs.ocr_attempts ALTER COLUMN attempt_date TYPE timestamptz USING attempt_date AT TIME ZONE 'America/Panama';
ALTER TABLE logs.ocr_attempts ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE logs.user_bot_interactions_ai ALTER COLUMN start_timestamp TYPE timestamptz USING start_timestamp AT TIME ZONE 'America/Panama';
ALTER TABLE logs.user_bot_interactions_ai ALTER COLUMN end_timestamp TYPE timestamptz USING end_timestamp AT TIME ZONE 'America/Panama';

-- =====================================================================
-- PASO 4: MIGRAR CAMPOS DE PUBLIC SCHEMA
-- =====================================================================

ALTER TABLE public.auth_audit_log ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.auth_provider_links ALTER COLUMN linked_at TYPE timestamptz USING linked_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.auth_provider_links ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.auth_provider_links ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_issuer ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_issuer ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_issuer_stores ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_issuer_stores ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_product ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_product ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.dim_users ALTER COLUMN email_verified_at TYPE timestamptz USING email_verified_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_detail ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_detail ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_header ALTER COLUMN date TYPE timestamptz USING date AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_header ALTER COLUMN update_date TYPE timestamptz USING update_date AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_header ALTER COLUMN deleted_at TYPE timestamptz USING deleted_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.invoice_header_tempsinregistro ALTER COLUMN date TYPE timestamptz USING date AT TIME ZONE 'America/Panama';
ALTER TABLE public.scheduled_notifications ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.scheduled_notifications ALTER COLUMN sent_at TYPE timestamptz USING sent_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.surveys ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.surveys ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.surveys_answers ALTER COLUMN date TYPE timestamptz USING date AT TIME ZONE 'America/Panama';
ALTER TABLE public.user_bot_interactions ALTER COLUMN start_timestamp TYPE timestamptz USING start_timestamp AT TIME ZONE 'America/Panama';
ALTER TABLE public.user_bot_interactions ALTER COLUMN end_timestamp TYPE timestamptz USING end_timestamp AT TIME ZONE 'America/Panama';
ALTER TABLE public.user_product_searches ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.user_search_balance ALTER COLUMN expires_at TYPE timestamptz USING expires_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.user_search_balance ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';
ALTER TABLE public.users ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';

-- =====================================================================
-- PASO 5: MIGRAR CAMPOS DE REWARDS SCHEMA
-- =====================================================================

ALTER TABLE rewards.fact_balance_points_history ALTER COLUMN snapshot_date TYPE timestamptz USING snapshot_date AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.fact_daily_game_plays ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'America/Panama';
ALTER TABLE rewards.user_invoice_summary ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'America/Panama';

-- =====================================================================
-- PASO 6: RECREAR VISTAS REGULARES
-- =====================================================================

CREATE OR REPLACE VIEW public.auth_events_summary AS
SELECT date_trunc('day', created_at) AS event_date, event_type, provider, success,
    count(*) AS event_count, count(DISTINCT user_id) AS unique_users, count(DISTINCT ip_address) AS unique_ips
FROM auth_audit_log
GROUP BY date_trunc('day', created_at), event_type, provider, success
ORDER BY date_trunc('day', created_at) DESC, count(*) DESC;

CREATE OR REPLACE VIEW public.surveys_summary AS
SELECT surveys.survey_id, username, CASE WHEN sa.answers IS NULL THEN 0 ELSE 1 END AS completed
FROM surveys CROSS JOIN users LEFT JOIN surveys_answers sa USING (survey_id, username);

CREATE OR REPLACE VIEW public.user_auth_summary AS
SELECT u.id, u.email, u.name, u.auth_providers, u.google_id, u.email_verified_at, u.last_login_provider, u.account_status, u.created_at, u.updated_at,
    COALESCE(json_agg(json_build_object('provider_type', apl.provider_type, 'provider_id', apl.provider_id, 'provider_email', apl.provider_email, 'is_primary', apl.is_primary, 'linked_at', apl.linked_at, 'link_method', apl.link_method) ORDER BY apl.is_primary DESC, apl.linked_at) FILTER (WHERE apl.id IS NOT NULL), '[]'::json) AS provider_links
FROM dim_users u LEFT JOIN auth_provider_links apl ON u.id = apl.user_id
GROUP BY u.id, u.email, u.name, u.auth_providers, u.google_id, u.email_verified_at, u.last_login_provider, u.account_status, u.created_at, u.updated_at;

CREATE OR REPLACE VIEW public.invoice_with_details AS
SELECT a.cufe, a.date, a.user_email, a.user_id, a.issuer_name, a.user_phone_number, a.user_telegram_id, a.process_date, a.reception_date, a.type, b.partkey, b.code, b.description, b.quantity, b.unit_price, b.unit_discount, b.amount, b.total, b.itbms
FROM (SELECT date, user_email, cufe, issuer_name, user_phone_number, user_id, user_telegram_id, process_date, reception_date, type FROM invoice_header) a
JOIN (SELECT cufe, partkey, ltrim(code, '0') AS code, description, quantity::double precision AS quantity, unit_price::double precision AS unit_price, unit_discount, amount::double precision AS amount, total::double precision AS total, itbms::double precision AS itbms FROM invoice_detail) b USING (cufe);

CREATE OR REPLACE VIEW public.vw_invoice_header_cleaned AS
SELECT ih.cufe, ih.issuer_name, ih.date, ih.process_date,
    CASE WHEN ih.user_telegram_id IS NULL AND vtu.telegram_id IS NULL THEN '-1' WHEN ih.user_telegram_id IS NULL THEN vtu.telegram_id::text ELSE ih.user_telegram_id END AS user_telegram_id,
    CASE WHEN ih.user_email IS NULL AND vtu.email IS NULL THEN '-1' WHEN ih.user_email IS NULL THEN vtu.email::text ELSE ih.user_email END AS user_email,
    ih.tot_itbms, ih.tot_amount
FROM invoice_header ih LEFT JOIN vw_telegram_users_full vtu ON (ih.user_telegram_id = vtu.telegram_id::text OR ih.user_email = vtu.email::text);

CREATE OR REPLACE VIEW public.vw_usr_compras_p6m AS
WITH ranked_data AS (SELECT user_email, (to_char(date, 'YYYYMM01'))::date AS mes, sum(tot_amount) AS monto, count(DISTINCT cufe) AS num_facturas, count(DISTINCT issuer_name) AS comercios FROM invoice_header WHERE date > (CURRENT_DATE - '6 mons'::interval) GROUP BY user_email, (to_char(date, 'YYYYMM01'))::date)
SELECT user_email, jsonb_agg(jsonb_build_object('mes', mes, 'comercios', comercios, 'num_facturas', num_facturas, 'monto', monto)) AS summ_mes FROM ranked_data GROUP BY user_email;

CREATE OR REPLACE VIEW public.vw_usr_compras_p6m_det AS
WITH ranked_data AS (SELECT a.user_email, (to_char(a.date, 'YYYYMM01'))::date AS mes, sum(d.quantity::double precision) AS qty, sum(CASE WHEN d.unit_discount = '' THEN NULL ELSE d.unit_discount::double precision END * d.quantity::double precision) AS descuentos, count(DISTINCT d.description) AS articulos FROM invoice_header a JOIN invoice_detail d USING (cufe) WHERE a.date > (CURRENT_DATE - '6 mons'::interval) GROUP BY a.user_email, (to_char(a.date, 'YYYYMM01'))::date)
SELECT user_email, jsonb_agg(jsonb_build_object('mes', mes, 'qty', qty, 'descuento', descuentos, 'monto', articulos)) AS summ_mes_det FROM ranked_data GROUP BY user_email;

CREATE OR REPLACE VIEW public.vw_usr_last_invoice AS
WITH t AS (SELECT a.cufe, a.issuer_name, a.date, a.process_date, a.user_telegram_id, a.user_email, a.tot_itbms, a.tot_amount FROM invoice_header a JOIN (SELECT user_email, max(process_date) AS process_date FROM invoice_header GROUP BY user_email) u USING (user_email, process_date))
SELECT user_email, jsonb_agg(jsonb_build_object('issuer_name', issuer_name, 'date', date, 'tot_amount', tot_amount)) AS latest_invoice FROM t GROUP BY user_email;

CREATE OR REPLACE VIEW public.vw_usr_top_issuers AS
WITH ranked_data AS (SELECT user_email, issuer_name, count(*) AS visitas, sum(tot_amount) AS monto, row_number() OVER (PARTITION BY user_email ORDER BY count(*) DESC) AS rank FROM invoice_header WHERE date > (CURRENT_DATE - '6 mons'::interval) GROUP BY user_email, issuer_name),
rd2 AS (SELECT user_email, issuer_name, visitas, monto FROM ranked_data WHERE rank <= 5)
SELECT user_email, jsonb_agg(jsonb_build_object('issuer_name', issuer_name, 'visitas', visitas, 'monto', monto)) AS top_issuers FROM rd2 GROUP BY user_email;

CREATE OR REPLACE VIEW public.vw_usr_top_products AS
WITH ranked_data AS (SELECT a.user_email, d.description, sum(d.quantity::double precision) AS qty, sum(d.amount::double precision) AS amt, row_number() OVER (PARTITION BY a.user_email ORDER BY sum(d.quantity::double precision) DESC) AS rank FROM invoice_header a JOIN invoice_detail d USING (cufe) WHERE a.date > (CURRENT_DATE - '6 mons'::interval) GROUP BY a.user_email, d.description),
rd2 AS (SELECT user_email, description, qty, amt FROM ranked_data WHERE rank <= 10)
SELECT user_email, jsonb_agg(jsonb_build_object('description', description, 'qty', qty, 'amt', amt)) AS top_products FROM rd2 GROUP BY user_email;

CREATE OR REPLACE VIEW public.vw_usr_total_invoices AS
SELECT user_email, jsonb_agg(jsonb_build_object('facturas', num_facturas, 'comercios', num_comercios, 'itbms', tot_itbms)) AS tot_invoices
FROM (SELECT user_email, count(DISTINCT cufe) AS num_facturas, count(DISTINCT issuer_name) AS num_comercios, sum(tot_itbms) AS tot_itbms FROM invoice_header GROUP BY user_email) sub
GROUP BY user_email;

CREATE OR REPLACE VIEW public.vw_usr_general_metrics AS
SELECT t.user_email, t.tot_invoices, c.summ_mes, cd.summ_mes_det, li.latest_invoice, ti.top_issuers, tp.top_products
FROM public.vw_usr_total_invoices t
LEFT JOIN public.vw_usr_compras_p6m c ON t.user_email = c.user_email
LEFT JOIN public.vw_usr_compras_p6m_det cd ON t.user_email = cd.user_email
LEFT JOIN public.vw_usr_last_invoice li ON t.user_email = li.user_email
LEFT JOIN public.vw_usr_top_issuers ti ON t.user_email = ti.user_email
LEFT JOIN public.vw_usr_top_products tp ON t.user_email = tp.user_email;

CREATE OR REPLACE VIEW gamification.v_user_dashboard AS
SELECT us.user_id, u.email, us.total_xp AS total_invoices, COALESCE(fbp.balance, 0::numeric) AS wallet_balance, l.level_id AS current_level, l.level_name, l.level_color, l.benefits_json AS level_benefits, l.min_xp AS level_min_points, l.max_xp AS level_max_points, COALESCE(nl.min_xp - us.total_xp, 0) AS invoices_to_next_level, nl.level_name AS next_level_name,
    COALESCE((SELECT jsonb_object_agg(streak_type, jsonb_build_object('current', current_count, 'max', max_count)) FROM gamification.user_streaks WHERE user_id = us.user_id AND is_active = true), '{}'::jsonb) AS active_streaks,
    (SELECT count(*) FROM gamification.user_mechanics WHERE user_id = us.user_id AND status::text = 'active') AS active_mechanics_count
FROM gamification.user_status us
JOIN dim_users u ON us.user_id = u.id
JOIN gamification.dim_user_levels l ON us.current_level_id = l.level_id
LEFT JOIN gamification.dim_user_levels nl ON nl.level_number = l.level_number + 1
LEFT JOIN rewards.fact_balance_points fbp ON us.user_id = fbp.user_id;

-- =====================================================================
-- PASO 7: RECREAR VISTAS MATERIALIZADAS
-- =====================================================================

CREATE MATERIALIZED VIEW public.user_issuer_integrity_daily AS
SELECT ih.user_email, di.ruc, di.name, di.update_date, count(ih.cufe) AS invoice_count, sum(ih.tot_amount) AS total_amount
FROM invoice_header ih JOIN dim_issuer di ON ih.issuer_ruc = di.ruc
GROUP BY ih.user_email, di.ruc, di.name, di.update_date;

CREATE MATERIALIZED VIEW public.user_header_integrity_daily AS
SELECT user_email, count(cufe) AS total_invoices, sum(tot_amount) AS total_amount, min(date) AS first_invoice, max(date) AS last_invoice, max(update_date) AS last_update
FROM invoice_header GROUP BY user_email;

CREATE MATERIALIZED VIEW public.user_detail_integrity_daily AS
SELECT ih.user_email, count(id.cufe) AS total_details, count(DISTINCT id.description) AS unique_products
FROM invoice_header ih JOIN invoice_detail id ON ih.cufe = id.cufe GROUP BY ih.user_email;

CREATE MATERIALIZED VIEW public.user_product_integrity_daily AS
SELECT ih.user_email, dp.code, dp.name, dp.update_date, sum(id.quantity::numeric) AS total_quantity
FROM invoice_header ih
JOIN invoice_detail id ON ih.cufe = id.cufe
JOIN dim_product dp ON id.code = dp.code
GROUP BY ih.user_email, dp.code, dp.name, dp.update_date;

-- =====================================================================
-- PASO 8: ACTUALIZAR TRIGGER PARA UTC
-- =====================================================================

CREATE OR REPLACE FUNCTION public.update_invoice_header_update_date()
RETURNS TRIGGER AS $$
BEGIN
    NEW.update_date = NOW() AT TIME ZONE 'UTC';
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMIT;
