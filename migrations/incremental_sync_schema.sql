-- ============================================================================
-- Incremental Sync Schema Modifications - Nivel 2
-- ============================================================================
-- Fecha: 2025-11-07
-- Propósito: Habilitar sincronización incremental con integridad de datos
--
-- Características:
-- - Soft delete tracking (is_deleted, deleted_at)
-- - Dataset versioning (auto-increment on changes)
-- - Performance indexes
-- ============================================================================

-- ============================================================================
-- PASO 1: Agregar columnas de soft delete a tablas de dimensiones
-- ============================================================================

-- Tabla: dim_product
ALTER TABLE public.dim_product 
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

COMMENT ON COLUMN public.dim_product.is_deleted IS 'Soft delete flag - TRUE si el producto fue eliminado';
COMMENT ON COLUMN public.dim_product.deleted_at IS 'Timestamp cuando el producto fue marcado como eliminado';

-- Tabla: dim_issuer
ALTER TABLE public.dim_issuer 
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

COMMENT ON COLUMN public.dim_issuer.is_deleted IS 'Soft delete flag - TRUE si el emisor fue eliminado';
COMMENT ON COLUMN public.dim_issuer.deleted_at IS 'Timestamp cuando el emisor fue marcado como eliminado';

-- Tabla: invoice_header
ALTER TABLE public.invoice_header
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

COMMENT ON COLUMN public.invoice_header.is_deleted IS 'Soft delete flag - TRUE si la factura fue eliminada';
COMMENT ON COLUMN public.invoice_header.deleted_at IS 'Timestamp cuando la factura fue marcada como eliminada';

-- Tabla: invoice_detail
ALTER TABLE public.invoice_detail
ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP NULL;

COMMENT ON COLUMN public.invoice_detail.is_deleted IS 'Soft delete flag - TRUE si el detalle fue eliminado';
COMMENT ON COLUMN public.invoice_detail.deleted_at IS 'Timestamp cuando el detalle fue marcado como eliminado';

-- ============================================================================
-- PASO 2: Crear tabla de versiones de datasets
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.dataset_versions (
    table_name VARCHAR(100) PRIMARY KEY,
    version BIGINT NOT NULL DEFAULT 0,
    last_modified TIMESTAMP NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE public.dataset_versions IS 'Tracking de versiones para sincronización incremental';
COMMENT ON COLUMN public.dataset_versions.table_name IS 'Nombre de la tabla versionada';
COMMENT ON COLUMN public.dataset_versions.version IS 'Número de versión incremental (se incrementa en cada INSERT/UPDATE/DELETE)';
COMMENT ON COLUMN public.dataset_versions.last_modified IS 'Timestamp de la última modificación al dataset';
COMMENT ON COLUMN public.dataset_versions.created_at IS 'Timestamp de creación del registro de versión';

-- ============================================================================
-- PASO 3: Inicializar versiones de datasets
-- ============================================================================

INSERT INTO public.dataset_versions (table_name, version, last_modified, created_at) 
VALUES 
    ('dim_product', 1, NOW(), NOW()),
    ('dim_issuer', 1, NOW(), NOW()),
    ('invoice_header', 1, NOW(), NOW()),
    ('invoice_detail', 1, NOW(), NOW())
ON CONFLICT (table_name) DO NOTHING;

-- ============================================================================
-- PASO 4: Function para incrementar version automáticamente
-- ============================================================================

CREATE OR REPLACE FUNCTION public.increment_dataset_version()
RETURNS TRIGGER AS $$
BEGIN
    -- Incrementar version y actualizar timestamp
    UPDATE public.dataset_versions 
    SET version = version + 1, 
        last_modified = NOW()
    WHERE table_name = TG_TABLE_NAME;
    
    -- Si la tabla no existe en dataset_versions, crearla
    IF NOT FOUND THEN
        INSERT INTO public.dataset_versions (table_name, version, last_modified)
        VALUES (TG_TABLE_NAME, 1, NOW());
    END IF;
    
    -- Return NEW para INSERT/UPDATE, OLD para DELETE
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    ELSE
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.increment_dataset_version() IS 'Auto-incrementa la versión del dataset cuando hay cambios (INSERT/UPDATE/DELETE)';

-- ============================================================================
-- PASO 5: Crear triggers para auto-increment version en cambios
-- ============================================================================

-- Trigger para dim_product
DROP TRIGGER IF EXISTS increment_product_version ON public.dim_product;
CREATE TRIGGER increment_product_version
AFTER INSERT OR UPDATE OR DELETE ON public.dim_product
FOR EACH STATEMENT 
EXECUTE FUNCTION public.increment_dataset_version();

COMMENT ON TRIGGER increment_product_version ON public.dim_product IS 'Auto-incrementa version del dataset en cada cambio';

-- Trigger para dim_issuer
DROP TRIGGER IF EXISTS increment_issuer_version ON public.dim_issuer;
CREATE TRIGGER increment_issuer_version
AFTER INSERT OR UPDATE OR DELETE ON public.dim_issuer
FOR EACH STATEMENT 
EXECUTE FUNCTION public.increment_dataset_version();

COMMENT ON TRIGGER increment_issuer_version ON public.dim_issuer IS 'Auto-incrementa version del dataset en cada cambio';

-- Trigger para invoice_header
DROP TRIGGER IF EXISTS increment_header_version ON public.invoice_header;
CREATE TRIGGER increment_header_version
AFTER INSERT OR UPDATE OR DELETE ON public.invoice_header
FOR EACH STATEMENT 
EXECUTE FUNCTION public.increment_dataset_version();

COMMENT ON TRIGGER increment_header_version ON public.invoice_header IS 'Auto-incrementa version del dataset en cada cambio';

-- Trigger para invoice_detail
DROP TRIGGER IF EXISTS increment_detail_version ON public.invoice_detail;
CREATE TRIGGER increment_detail_version
AFTER INSERT OR UPDATE OR DELETE ON public.invoice_detail
FOR EACH STATEMENT 
EXECUTE FUNCTION public.increment_dataset_version();

COMMENT ON TRIGGER increment_detail_version ON public.invoice_detail IS 'Auto-incrementa version del dataset en cada cambio';

-- ============================================================================
-- PASO 6: Crear índices para performance de sync incremental
-- ============================================================================

-- Índices para dim_product
CREATE INDEX IF NOT EXISTS idx_dim_product_update_date_active 
ON public.dim_product(update_date DESC) 
WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_dim_product_deleted 
ON public.dim_product(deleted_at DESC) 
WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_dim_product_code_ruc 
ON public.dim_product(code, issuer_ruc) 
WHERE is_deleted = FALSE;

COMMENT ON INDEX idx_dim_product_update_date_active IS 'Performance para queries de sync incremental (productos activos)';
COMMENT ON INDEX idx_dim_product_deleted IS 'Performance para queries de deleted items';
COMMENT ON INDEX idx_dim_product_code_ruc IS 'Performance para JOINs con invoice_detail';

-- Índices para dim_issuer
CREATE INDEX IF NOT EXISTS idx_dim_issuer_update_date_active 
ON public.dim_issuer(update_date DESC) 
WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_dim_issuer_deleted 
ON public.dim_issuer(deleted_at DESC) 
WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_dim_issuer_ruc_name 
ON public.dim_issuer(issuer_ruc, issuer_name) 
WHERE is_deleted = FALSE;

COMMENT ON INDEX idx_dim_issuer_update_date_active IS 'Performance para queries de sync incremental (emisores activos)';
COMMENT ON INDEX idx_dim_issuer_deleted IS 'Performance para queries de deleted items';
COMMENT ON INDEX idx_dim_issuer_ruc_name IS 'Performance para JOINs con invoice_header';

-- Índices para invoice_header
CREATE INDEX IF NOT EXISTS idx_invoice_header_update_date_active 
ON public.invoice_header(update_date DESC) 
WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_invoice_header_deleted 
ON public.invoice_header(deleted_at DESC) 
WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_invoice_header_user_id_cufe 
ON public.invoice_header(user_id, cufe) 
WHERE is_deleted = FALSE;

COMMENT ON INDEX idx_invoice_header_update_date_active IS 'Performance para queries de sync incremental (headers activos)';
COMMENT ON INDEX idx_invoice_header_deleted IS 'Performance para queries de deleted items';
COMMENT ON INDEX idx_invoice_header_user_id_cufe IS 'Performance para queries por usuario';

-- Índices para invoice_detail
CREATE INDEX IF NOT EXISTS idx_invoice_detail_update_date_active 
ON public.invoice_detail(update_date DESC) 
WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_invoice_detail_deleted 
ON public.invoice_detail(deleted_at DESC) 
WHERE is_deleted = TRUE;

CREATE INDEX IF NOT EXISTS idx_invoice_detail_cufe 
ON public.invoice_detail(cufe) 
WHERE is_deleted = FALSE;

COMMENT ON INDEX idx_invoice_detail_update_date_active IS 'Performance para queries de sync incremental (detalles activos)';
COMMENT ON INDEX idx_invoice_detail_deleted IS 'Performance para queries de deleted items';
COMMENT ON INDEX idx_invoice_detail_cufe IS 'Performance para queries por factura';

-- ============================================================================
-- PASO 7: Helper function para soft delete
-- ============================================================================

CREATE OR REPLACE FUNCTION public.soft_delete_record(
    p_table_name VARCHAR,
    p_id_column VARCHAR,
    p_id_value VARCHAR
)
RETURNS VOID AS $$
BEGIN
    EXECUTE format(
        'UPDATE public.%I SET is_deleted = TRUE, deleted_at = NOW() WHERE %I = $1 AND is_deleted = FALSE',
        p_table_name,
        p_id_column
    ) USING p_id_value;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.soft_delete_record IS 'Helper para marcar un registro como eliminado (soft delete)';

-- Ejemplo de uso:
-- SELECT public.soft_delete_record('dim_product', 'code', 'PROD001');

-- ============================================================================
-- PASO 8: Queries de validación y estadísticas
-- ============================================================================

-- View para monitorear versiones de datasets
CREATE OR REPLACE VIEW public.vw_dataset_sync_status AS
SELECT 
    table_name,
    version,
    last_modified,
    NOW() - last_modified as time_since_last_change,
    CASE 
        WHEN NOW() - last_modified < INTERVAL '1 hour' THEN 'Recent'
        WHEN NOW() - last_modified < INTERVAL '1 day' THEN 'Today'
        WHEN NOW() - last_modified < INTERVAL '7 days' THEN 'This Week'
        ELSE 'Older'
    END as freshness
FROM public.dataset_versions
ORDER BY last_modified DESC;

COMMENT ON VIEW public.vw_dataset_sync_status IS 'Vista para monitorear el estado de sincronización de datasets';

-- ============================================================================
-- VERIFICACIÓN FINAL
-- ============================================================================

-- Verificar columnas agregadas
DO $$
DECLARE
    missing_columns TEXT := '';
BEGIN
    -- Verificar dim_product
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_schema = 'public' 
        AND table_name = 'dim_product' 
        AND column_name = 'is_deleted'
    ) THEN
        missing_columns := missing_columns || 'dim_product.is_deleted, ';
    END IF;
    
    -- Verificar tabla dataset_versions
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_schema = 'public' 
        AND table_name = 'dataset_versions'
    ) THEN
        missing_columns := missing_columns || 'dataset_versions table, ';
    END IF;
    
    -- Reportar resultado
    IF missing_columns != '' THEN
        RAISE WARNING 'Faltan componentes: %', missing_columns;
    ELSE
        RAISE NOTICE '✅ Schema de sincronización incremental instalado correctamente';
        RAISE NOTICE '📊 Versiones inicializadas para 4 datasets';
        RAISE NOTICE '🔄 Triggers automáticos configurados';
        RAISE NOTICE '⚡ Índices de performance creados';
    END IF;
END $$;

-- Mostrar estado actual de versiones
SELECT 
    table_name,
    version,
    last_modified,
    NOW() - last_modified as age
FROM public.dataset_versions
ORDER BY table_name;

-- ============================================================================
-- FIN DEL SCRIPT
-- ============================================================================
