-- Update OCR test logs table to include comprehensive API call details
-- Adds user tracking, detailed token usage, and cost information

-- Drop existing table and recreate with new schema
DROP TABLE IF EXISTS public.ocr_test_logs CASCADE;

CREATE TABLE public.ocr_test_logs (
    id SERIAL PRIMARY KEY,
    
    -- Request Information
    user_id INTEGER REFERENCES public.dim_users(id),
    image_path TEXT NOT NULL,
    image_size_bytes BIGINT NOT NULL,
    
    -- Model Information
    model_name TEXT NOT NULL,
    provider TEXT DEFAULT 'openrouter',
    
    -- Execution Results
    success BOOLEAN NOT NULL,
    response_time_ms BIGINT NOT NULL,
    error_message TEXT,
    
    -- Token Usage Details
    tokens_prompt INTEGER,           -- Input/prompt tokens
    tokens_completion INTEGER,       -- Output/completion tokens
    tokens_total INTEGER,            -- Total tokens used
    
    -- Cost Information
    cost_prompt_usd NUMERIC(10, 6),      -- Cost for input tokens
    cost_completion_usd NUMERIC(10, 6),  -- Cost for output tokens
    cost_total_usd NUMERIC(10, 6),       -- Total cost in USD
    
    -- API Response Metadata
    generation_id TEXT,              -- OpenRouter generation ID
    model_used TEXT,                 -- Actual model used (may differ from requested)
    finish_reason TEXT,              -- Completion finish reason (stop, length, etc)
    
    -- Extracted Data
    extracted_fields JSONB,          -- All extracted invoice fields
    raw_response JSONB,              -- Full API response for debugging
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance and analytics
CREATE INDEX idx_ocr_test_logs_created_at ON public.ocr_test_logs(created_at DESC);
CREATE INDEX idx_ocr_test_logs_user_id ON public.ocr_test_logs(user_id);
CREATE INDEX idx_ocr_test_logs_model_name ON public.ocr_test_logs(model_name);
CREATE INDEX idx_ocr_test_logs_success ON public.ocr_test_logs(success);
CREATE INDEX idx_ocr_test_logs_image_path ON public.ocr_test_logs(image_path);
CREATE INDEX idx_ocr_test_logs_cost ON public.ocr_test_logs(cost_total_usd) WHERE cost_total_usd IS NOT NULL;

-- Comments for documentation
COMMENT ON TABLE public.ocr_test_logs IS 'Comprehensive logging of OCR API calls for traceability, cost tracking, and performance monitoring';

COMMENT ON COLUMN public.ocr_test_logs.user_id IS 'User ID from dim_users who initiated the OCR request (NULL for test scripts)';
COMMENT ON COLUMN public.ocr_test_logs.image_path IS 'Path to the test image file';
COMMENT ON COLUMN public.ocr_test_logs.image_size_bytes IS 'Size of the image in bytes';

COMMENT ON COLUMN public.ocr_test_logs.model_name IS 'Requested model name (e.g., qwen/qwen3-vl-8b-instruct)';
COMMENT ON COLUMN public.ocr_test_logs.provider IS 'API provider (openrouter, gemini, etc)';

COMMENT ON COLUMN public.ocr_test_logs.success IS 'Whether the OCR extraction was successful';
COMMENT ON COLUMN public.ocr_test_logs.response_time_ms IS 'Response time in milliseconds';
COMMENT ON COLUMN public.ocr_test_logs.error_message IS 'Error message if the attempt failed';

COMMENT ON COLUMN public.ocr_test_logs.tokens_prompt IS 'Number of input/prompt tokens consumed';
COMMENT ON COLUMN public.ocr_test_logs.tokens_completion IS 'Number of output/completion tokens generated';
COMMENT ON COLUMN public.ocr_test_logs.tokens_total IS 'Total tokens consumed (prompt + completion)';

COMMENT ON COLUMN public.ocr_test_logs.cost_prompt_usd IS 'Cost in USD for input tokens';
COMMENT ON COLUMN public.ocr_test_logs.cost_completion_usd IS 'Cost in USD for output tokens';
COMMENT ON COLUMN public.ocr_test_logs.cost_total_usd IS 'Total cost in USD for the API call';

COMMENT ON COLUMN public.ocr_test_logs.generation_id IS 'Unique generation ID from the API provider';
COMMENT ON COLUMN public.ocr_test_logs.model_used IS 'Actual model used (may differ from requested model)';
COMMENT ON COLUMN public.ocr_test_logs.finish_reason IS 'Reason for completion (stop, length, content_filter, etc)';

COMMENT ON COLUMN public.ocr_test_logs.extracted_fields IS 'JSON object with all successfully extracted invoice fields';
COMMENT ON COLUMN public.ocr_test_logs.raw_response IS 'Complete raw API response for debugging and auditing';
COMMENT ON COLUMN public.ocr_test_logs.created_at IS 'Timestamp when the log was created';

-- View for cost analytics
CREATE OR REPLACE VIEW public.ocr_test_logs_summary AS
SELECT 
    DATE(created_at) as date,
    model_name,
    COUNT(*) as total_requests,
    SUM(CASE WHEN success THEN 1 ELSE 0 END) as successful_requests,
    AVG(response_time_ms) as avg_response_time_ms,
    SUM(tokens_total) as total_tokens_used,
    SUM(cost_total_usd) as total_cost_usd,
    AVG(cost_total_usd) as avg_cost_per_request_usd
FROM public.ocr_test_logs
GROUP BY DATE(created_at), model_name
ORDER BY date DESC, model_name;

COMMENT ON VIEW public.ocr_test_logs_summary IS 'Daily summary of OCR requests grouped by model with cost and performance metrics';
