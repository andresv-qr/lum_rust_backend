-- Migration to remove dataset_versions system (not needed for multi-user scenario)
-- Created: 2025-11-08
-- Reason: Dataset versioning doesn't apply when each user has different data

BEGIN;

-- Drop triggers
DROP TRIGGER IF EXISTS increment_product_version ON dim_product;
DROP TRIGGER IF EXISTS increment_issuer_version ON dim_issuer;
DROP TRIGGER IF EXISTS increment_header_version ON invoice_header;
DROP TRIGGER IF EXISTS increment_detail_version ON invoice_detail;

-- Drop function
DROP FUNCTION IF EXISTS increment_dataset_version();

-- Drop dependent views first
DROP VIEW IF EXISTS vw_dataset_sync_status;

-- Drop table with CASCADE to handle any other dependencies
DROP TABLE IF EXISTS dataset_versions CASCADE;

COMMIT;
