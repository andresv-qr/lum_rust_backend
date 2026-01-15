-- ============================================================================
-- FIX: Auto-update update_date en invoice_header + Zona Horaria Panamá
-- ============================================================================
-- Fecha: 2026-01-14
-- Objetivo: 
--   1. Crear trigger para actualizar update_date en cada UPDATE
--   2. Configurar zona horaria a America/Panama (GMT-5)
--   3. Convertir update_date a timestamptz (con zona horaria)
-- ============================================================================

BEGIN;

-- ============================================================================
-- PASO 1: Verificar/Crear función genérica para actualizar timestamps
-- ============================================================================

-- Esta función ya existe según la consulta, pero la recreamos para asegurar
CREATE OR REPLACE FUNCTION public.update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.update_date = CURRENT_TIMESTAMP AT TIME ZONE 'America/Panama';
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.update_updated_at_column() IS 
'Función trigger que actualiza el campo update_date con la hora actual de Panamá (GMT-5)';

-- ============================================================================
-- PASO 2: Crear trigger en invoice_header para UPDATE
-- ============================================================================

-- Eliminar trigger si ya existe (idempotente)
DROP TRIGGER IF EXISTS trg_invoice_header_update_date ON public.invoice_header;

-- Crear el trigger
CREATE TRIGGER trg_invoice_header_update_date
    BEFORE UPDATE ON public.invoice_header
    FOR EACH ROW
    EXECUTE FUNCTION public.update_updated_at_column();

COMMENT ON TRIGGER trg_invoice_header_update_date ON public.invoice_header IS 
'Actualiza automáticamente update_date en cada UPDATE con hora de Panamá';

-- ============================================================================
-- PASO 3: Convertir update_date a TIMESTAMPTZ (con zona horaria)
-- ============================================================================

-- Nota: Hay vistas materializadas que dependen de esta columna.
-- Para cambiar el tipo necesitamos:
-- 1. Identificar las vistas dependientes
-- 2. Recrearlas después del cambio
-- Por ahora, solo actualizamos el DEFAULT. El tipo timestamp sin zona
-- funciona correctamente con el trigger que ya maneja la zona horaria.

-- Cambiar el default para usar zona horaria de Panamá explícitamente
ALTER TABLE public.invoice_header 
    ALTER COLUMN update_date SET DEFAULT (CURRENT_TIMESTAMP AT TIME ZONE 'America/Panama');

-- ============================================================================
-- PASO 4: Configurar zona horaria por defecto para la base de datos (Opcional)
-- ============================================================================

-- Nota: Esto afecta a TODA la base de datos. Si solo quieres invoice_header,
-- comenta esta línea. La función del trigger ya maneja la zona horaria.

-- ALTER DATABASE tfactu SET timezone = 'America/Panama';

-- ============================================================================
-- VERIFICACIÓN
-- ============================================================================

-- Verificar que el trigger fue creado
SELECT 
    tgname AS trigger_name,
    tgtype AS trigger_type,
    tgenabled AS enabled,
    pg_get_triggerdef(oid) AS definition
FROM pg_trigger 
WHERE tgrelid = 'public.invoice_header'::regclass 
  AND tgname = 'trg_invoice_header_update_date';

-- Verificar tipo de columna
SELECT 
    column_name, 
    data_type, 
    column_default 
FROM information_schema.columns 
WHERE table_schema = 'public' 
  AND table_name = 'invoice_header' 
  AND column_name = 'update_date';

COMMIT;

-- ============================================================================
-- TEST (Descomentar para probar)
-- ============================================================================

-- BEGIN;
-- 
-- -- Obtener un registro existente
-- SELECT cufe, update_date FROM public.invoice_header LIMIT 1;
-- 
-- -- Actualizarlo (cualquier campo)
-- UPDATE public.invoice_header 
-- SET tot_amount = tot_amount 
-- WHERE cufe = (SELECT cufe FROM public.invoice_header LIMIT 1);
-- 
-- -- Verificar que update_date cambió
-- SELECT cufe, update_date FROM public.invoice_header LIMIT 1;
-- 
-- ROLLBACK; -- No aplicar cambios, solo prueba
