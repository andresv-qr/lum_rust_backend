-- Migration to create Materialized Views for daily integrity checks
-- Created: 2025-11-08
-- Purpose: Enable fast integrity validation for 50K+ users
-- Refresh: Daily at 3 AM UTC via scheduled job

BEGIN;

-- ============================================================================
-- PRODUCTS INTEGRITY VIEW
-- ============================================================================
CREATE MATERIALIZED VIEW user_product_integrity_daily AS
SELECT 
    ih.user_id,
    COUNT(DISTINCT dp.code) as total_count,
    COALESCE(BIT_XOR(HASHTEXT(dp.code)::BIGINT), 0) as global_hash,
    MAX(dp.update_date) as last_update,
    NOW() as snapshot_time
FROM dim_product dp
INNER JOIN invoice_detail id ON dp.code = id.code
INNER JOIN invoice_header ih ON id.cufe = ih.cufe
WHERE dp.is_deleted = FALSE
GROUP BY ih.user_id;

CREATE UNIQUE INDEX idx_upid_user ON user_product_integrity_daily(user_id);

-- ============================================================================
-- ISSUERS INTEGRITY VIEW
-- ============================================================================
CREATE MATERIALIZED VIEW user_issuer_integrity_daily AS
SELECT 
    ih.user_id,
    COUNT(DISTINCT (dis.issuer_ruc, dis.store_id)) as total_count,
    COALESCE(BIT_XOR(HASHTEXT(dis.issuer_ruc || '-' || dis.store_id)::BIGINT), 0) as global_hash,
    MAX(dis.update_date) as last_update,
    NOW() as snapshot_time
FROM dim_issuer_stores dis
INNER JOIN invoice_header ih ON dis.issuer_ruc = ih.issuer_ruc AND dis.store_id = ih.store_id
WHERE ih.user_id IS NOT NULL
GROUP BY ih.user_id;

CREATE UNIQUE INDEX idx_uiid_user ON user_issuer_integrity_daily(user_id);

-- ============================================================================
-- HEADERS INTEGRITY VIEW
-- ============================================================================
CREATE MATERIALIZED VIEW user_header_integrity_daily AS
SELECT 
    user_id,
    COUNT(*) as total_count,
    COALESCE(BIT_XOR(HASHTEXT(cufe)::BIGINT), 0) as global_hash,
    MAX(update_date) as last_update,
    NOW() as snapshot_time
FROM invoice_header
WHERE is_deleted = FALSE
  AND user_id IS NOT NULL
GROUP BY user_id;

CREATE UNIQUE INDEX idx_uhid_user ON user_header_integrity_daily(user_id);

-- ============================================================================
-- DETAILS INTEGRITY VIEW
-- ============================================================================
CREATE MATERIALIZED VIEW user_detail_integrity_daily AS
SELECT 
    ih.user_id,
    COUNT(*) as total_count,
    COALESCE(BIT_XOR(HASHTEXT(id.cufe || '_' || COALESCE(id.code, ''))::BIGINT), 0) as global_hash,
    MAX(id.update_date) as last_update,
    NOW() as snapshot_time
FROM invoice_detail id
INNER JOIN invoice_header ih ON id.cufe = ih.cufe
WHERE id.is_deleted = FALSE
  AND ih.user_id IS NOT NULL
GROUP BY ih.user_id;

CREATE UNIQUE INDEX idx_udid_user ON user_detail_integrity_daily(user_id);

COMMIT;

-- ============================================================================
-- NOTES
-- ============================================================================
-- To refresh manually:
--   REFRESH MATERIALIZED VIEW CONCURRENTLY user_product_integrity_daily;
--   REFRESH MATERIALIZED VIEW CONCURRENTLY user_issuer_integrity_daily;
--   REFRESH MATERIALIZED VIEW CONCURRENTLY user_header_integrity_daily;
--   REFRESH MATERIALIZED VIEW CONCURRENTLY user_detail_integrity_daily;
--
-- Estimated refresh time with 50K users: 2-5 minutes
-- Scheduled via Rust cron job at 3 AM UTC daily
