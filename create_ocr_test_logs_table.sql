-- Table for OCR test logging and traceability
-- This table stores all OCR attempts with model info, performance metrics, and results

CREATE TABLE IF NOT EXISTS public.ocr_test_logs (
    id SERIAL PRIMARY KEY,
    image_path TEXT NOT NULL,
    model_name TEXT NOT NULL,
    image_size_bytes BIGINT NOT NULL,
    success BOOLEAN NOT NULL,
    response_time_ms BIGINT NOT NULL,
    tokens_used INTEGER,
    error_message TEXT,
    extracted_fields JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_ocr_test_logs_created_at ON public.ocr_test_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ocr_test_logs_model_name ON public.ocr_test_logs(model_name);
CREATE INDEX IF NOT EXISTS idx_ocr_test_logs_success ON public.ocr_test_logs(success);
CREATE INDEX IF NOT EXISTS idx_ocr_test_logs_image_path ON public.ocr_test_logs(image_path);

-- Comments for documentation
COMMENT ON TABLE public.ocr_test_logs IS 'Logs all OCR test attempts for traceability and performance monitoring';
COMMENT ON COLUMN public.ocr_test_logs.image_path IS 'Path to the test image file';
COMMENT ON COLUMN public.ocr_test_logs.model_name IS 'OpenRouter model used (e.g., qwen/qwen3-vl-8b-instruct)';
COMMENT ON COLUMN public.ocr_test_logs.image_size_bytes IS 'Size of the image in bytes';
COMMENT ON COLUMN public.ocr_test_logs.success IS 'Whether the OCR extraction was successful';
COMMENT ON COLUMN public.ocr_test_logs.response_time_ms IS 'Response time in milliseconds';
COMMENT ON COLUMN public.ocr_test_logs.tokens_used IS 'Number of tokens consumed by the API call';
COMMENT ON COLUMN public.ocr_test_logs.error_message IS 'Error message if the attempt failed';
COMMENT ON COLUMN public.ocr_test_logs.extracted_fields IS 'JSON object with all extracted fields (if successful)';
COMMENT ON COLUMN public.ocr_test_logs.created_at IS 'Timestamp when the log was created';
