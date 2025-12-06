-- Migration: add soft delete columns and dataset versioning for incremental sync (Nivel 2)
-- Date: 2025-11-08
-- Purpose: Add is_deleted/deleted_at to dimension & invoice tables, create dataset_versions table,
-- and add triggers to increment version on changes.

BEGIN;

-- 1) Add soft-delete columns to dimensions and invoice tables
ALTER TABLE public.dim_product
    ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

ALTER TABLE public.dim_issuer
    ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

ALTER TABLE public.invoice_header
    ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

ALTER TABLE public.invoice_detail
    ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

-- 2) Create dataset_versions table (idempotent)
CREATE TABLE IF NOT EXISTS public.dataset_versions (
    table_name VARCHAR(100) PRIMARY KEY,
    version BIGINT DEFAULT 0,
    last_modified TIMESTAMP DEFAULT NOW()
);

-- 3) Initialize entries for our tables if not present
INSERT INTO public.dataset_versions (table_name, version)
VALUES
    ('dim_product', 1),
    ('dim_issuer', 1),
    ('invoice_header', 1),
    ('invoice_detail', 1)
ON CONFLICT (table_name) DO NOTHING;

-- 4) Function to increment dataset version per table (statement-level trigger)
CREATE OR REPLACE FUNCTION public.increment_dataset_version()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE public.dataset_versions
    SET version = version + 1,
        last_modified = NOW()
    WHERE table_name = TG_TABLE_NAME;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 5) Attach triggers to increment version after changes (statement-level)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'increment_product_version'
    ) THEN
        CREATE TRIGGER increment_product_version
        AFTER INSERT OR UPDATE OR DELETE ON public.dim_product
        FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'increment_issuer_version'
    ) THEN
        CREATE TRIGGER increment_issuer_version
        AFTER INSERT OR UPDATE OR DELETE ON public.dim_issuer
        FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'increment_header_version'
    ) THEN
        CREATE TRIGGER increment_header_version
        AFTER INSERT OR UPDATE OR DELETE ON public.invoice_header
        FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'increment_detail_version'
    ) THEN
        CREATE TRIGGER increment_detail_version
        AFTER INSERT OR UPDATE OR DELETE ON public.invoice_detail
        FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();
    END IF;
END$$;

-- 6) Create indexes for update_date and deleted_at
CREATE INDEX IF NOT EXISTS idx_dim_product_update_date ON public.dim_product(update_date) WHERE is_deleted = FALSE;
CREATE INDEX IF NOT EXISTS idx_dim_product_deleted ON public.dim_product(deleted_at) WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_dim_issuer_update_date ON public.dim_issuer(update_date) WHERE is_deleted = FALSE;
CREATE INDEX IF NOT EXISTS idx_dim_issuer_deleted ON public.dim_issuer(deleted_at) WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_invoice_header_update_date ON public.invoice_header(update_date) WHERE is_deleted = FALSE;
CREATE INDEX IF NOT EXISTS idx_invoice_detail_update_date ON public.invoice_detail(update_date) WHERE is_deleted = FALSE;

COMMIT;

-- Notes:
-- - This migration is safe to run multiple times.
-- - For very large tables consider running index creation concurrently in maintenance window.
