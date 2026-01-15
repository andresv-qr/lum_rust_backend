-- ============================================================================
-- Redemptions: Enforce unique redemption_code (safety net)
-- Date: 2025-12-16
-- Notes:
-- - Codes are generated with high entropy, but this adds a DB-level guarantee.
-- - If duplicates already exist, the migration will SKIP creating the unique index.
-- ============================================================================

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM rewards.user_redemptions
        GROUP BY redemption_code
        HAVING COUNT(*) > 1
        LIMIT 1
    ) THEN
        RAISE NOTICE 'Skipping unique index on redemption_code: duplicates exist. Please dedupe first.';
    ELSE
        CREATE UNIQUE INDEX IF NOT EXISTS idx_user_redemptions_code_unique
        ON rewards.user_redemptions (redemption_code);

        RAISE NOTICE 'Unique index idx_user_redemptions_code_unique ensured.';
    END IF;
END $$;
