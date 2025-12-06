-- Migration to update user_issuer_integrity_daily to use dim_issuer_stores
-- Created: 2025-11-14
-- Purpose: Switch from dim_issuer to dim_issuer_stores with composite key (issuer_ruc + store_id)

BEGIN;

-- Drop the old Materialized View and its index
DROP INDEX IF EXISTS idx_uiid_user;
DROP MATERIALIZED VIEW IF EXISTS user_issuer_integrity_daily;

-- Recreate with new structure using dim_issuer_stores
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

-- Recreate the unique index
CREATE UNIQUE INDEX idx_uiid_user ON user_issuer_integrity_daily(user_id);

-- Grant necessary permissions
GRANT SELECT ON user_issuer_integrity_daily TO PUBLIC;

-- Refresh the materialized view with data
REFRESH MATERIALIZED VIEW user_issuer_integrity_daily;

COMMIT;

-- Verification query (optional - run after migration)
-- SELECT user_id, total_count, global_hash, last_update 
-- FROM user_issuer_integrity_daily 
-- ORDER BY user_id 
-- LIMIT 5;
