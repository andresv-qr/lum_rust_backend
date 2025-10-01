-- Migración para agregar nuevos campos a la tabla invoice_header
-- Ejecutar este script antes de usar los nuevos campos

-- Agregar campo date_timestamp (fecha y hora actual en GMT-5)
ALTER TABLE public.invoice_header 
ADD COLUMN IF NOT EXISTS date_timestamp TIMESTAMP WITH TIME ZONE;

-- Agregar campo issuer_dv (dígito verificador del RUC)
ALTER TABLE public.invoice_header 
ADD COLUMN IF NOT EXISTS issuer_dv VARCHAR(10);

-- Agregar campo issuer_address (dirección del comercio)
ALTER TABLE public.invoice_header 
ADD COLUMN IF NOT EXISTS issuer_address TEXT;

-- Agregar campo issuer_ws (parámetro opcional de la API)
ALTER TABLE public.invoice_header 
ADD COLUMN IF NOT EXISTS issuer_ws VARCHAR(255);

-- Verificar que los campos se agregaron correctamente
SELECT column_name, data_type, is_nullable 
FROM information_schema.columns 
WHERE table_name = 'invoice_header' 
AND table_schema = 'public'
AND column_name IN ('date_timestamp', 'issuer_dv', 'issuer_address', 'issuer_ws')
ORDER BY column_name;