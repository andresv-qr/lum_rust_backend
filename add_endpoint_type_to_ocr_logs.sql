-- Agregar campo endpoint_type a la tabla ocr_test_logs
-- para diferenciar entre llamadas del endpoint principal vs retry

ALTER TABLE public.ocr_test_logs 
ADD COLUMN IF NOT EXISTS endpoint_type VARCHAR(20) DEFAULT 'upload';

-- Crear índice para consultas por tipo de endpoint
CREATE INDEX IF NOT EXISTS idx_ocr_test_logs_endpoint_type 
ON public.ocr_test_logs(endpoint_type);

-- Agregar comentario explicativo
COMMENT ON COLUMN public.ocr_test_logs.endpoint_type IS 
'Tipo de endpoint que generó el registro: "upload" (primera imagen) o "retry" (imagen adicional)';

-- Verificar la alteración
SELECT column_name, data_type, column_default 
FROM information_schema.columns 
WHERE table_schema = 'public' 
  AND table_name = 'ocr_test_logs' 
  AND column_name = 'endpoint_type';
