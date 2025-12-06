-- ============================================================================
-- Agregar columna update_date a invoice_header e invoice_detail
-- ============================================================================
-- Fecha: 2025-11-07
-- Propósito: Agregar columna update_date faltante para sync incremental
-- ============================================================================

-- Agregar update_date a invoice_header
ALTER TABLE public.invoice_header
ADD COLUMN IF NOT EXISTS update_date TIMESTAMP DEFAULT NOW();

-- Inicializar con process_date si existe, sino NOW()
UPDATE public.invoice_header
SET update_date = COALESCE(process_date, NOW())
WHERE update_date IS NULL;

COMMENT ON COLUMN public.invoice_header.update_date IS 'Timestamp de última actualización del registro (para sync incremental)';

-- Agregar update_date a invoice_detail
ALTER TABLE public.invoice_detail
ADD COLUMN IF NOT EXISTS update_date TIMESTAMP DEFAULT NOW();

-- Inicializar con NOW()
UPDATE public.invoice_detail
SET update_date = NOW()
WHERE update_date IS NULL;

COMMENT ON COLUMN public.invoice_detail.update_date IS 'Timestamp de última actualización del registro (para sync incremental)';

-- Crear índices ahora que las columnas existen
CREATE INDEX IF NOT EXISTS idx_invoice_header_update_date_active 
ON public.invoice_header(update_date DESC) 
WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_invoice_detail_update_date_active 
ON public.invoice_detail(update_date DESC) 
WHERE is_deleted = FALSE;

-- Verificación
DO $$
BEGIN
    RAISE NOTICE '✅ Columnas update_date agregadas correctamente';
    RAISE NOTICE '📊 invoice_header registros: %', (SELECT COUNT(*) FROM public.invoice_header);
    RAISE NOTICE '📊 invoice_detail registros: %', (SELECT COUNT(*) FROM public.invoice_detail);
END $$;

SELECT 'invoice_header' as table_name, COUNT(*) as records, MAX(update_date) as latest_update
FROM public.invoice_header
UNION ALL
SELECT 'invoice_detail', COUNT(*), MAX(update_date)
FROM public.invoice_detail;
