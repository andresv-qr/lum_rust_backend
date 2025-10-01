-- ============================================================================
-- INVOICE PROCESSING API - DATABASE SCHEMA
-- ============================================================================
-- This script creates the necessary tables for the robust invoice processing API
-- as documented in INVOICE_EXTRACTION_DOCUMENTATION.md

-- ============================================================================
-- LOGGING SCHEMA
-- ============================================================================

-- Create logs schema if it doesn't exist
CREATE SCHEMA IF NOT EXISTS logs;

-- ============================================================================
-- MAIN INVOICE TABLES (should already exist)
-- ============================================================================

-- These tables should already exist, but including for reference:

-- public.invoice_header table (already exists)
-- Contains main invoice data - one record per invoice
-- Fields: no, date, cufe, issuer_name, issuer_ruc, issuer_dv, issuer_address, 
--         issuer_phone, tot_amount, tot_itbms, url, type, process_date, 
--         reception_date, user_id, origin, user_email

-- public.invoice_detail table (already exists) 
-- Contains invoice line items - multiple records per invoice
-- Fields: cufe, quantity, code, description, unit_discount, unit_price, 
--         itbms, information_of_interest (ALL VARCHAR)

-- public.invoice_payment table (already exists)
-- Contains payment information - one record per invoice  
-- Fields: cufe, vuelto, total_pagado (ALL VARCHAR)

-- ============================================================================
-- NEW LOGGING TABLE FOR BOT OPERATIONS
-- ============================================================================

-- Drop existing table if it exists (for clean setup)
DROP TABLE IF EXISTS logs.bot_rust_scrapy;

-- Create the comprehensive logging table for bot operations
CREATE TABLE logs.bot_rust_scrapy (
    -- Primary key
    id SERIAL PRIMARY KEY,
    
    -- Request information
    url VARCHAR NOT NULL,                           -- URL procesada
    cufe VARCHAR,                                   -- CUFE extraído (si exitoso)
    origin VARCHAR NOT NULL,                        -- Origen de la solicitud
    user_id VARCHAR NOT NULL,                       -- ID del usuario solicitante
    user_email VARCHAR NOT NULL,                    -- Email del usuario
    
    -- Performance metrics
    execution_time_ms INTEGER,                      -- Tiempo de ejecución del scraping (ms)
    scraped_fields_count INTEGER DEFAULT 0,        -- Número de campos extraídos exitosamente
    retry_attempts INTEGER DEFAULT 0,              -- Número de intentos de retry
    
    -- Status and error tracking
    status VARCHAR NOT NULL DEFAULT 'PROCESSING',  -- Estado final de la operación
    error_message TEXT,                             -- Mensaje de error detallado
    error_type VARCHAR,                             -- Tipo de error categorizado
    
    -- Timestamps
    request_timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),  -- Timestamp de recepción
    response_timestamp TIMESTAMP WITH TIME ZONE,   -- Timestamp de respuesta
    
    -- Indexes for common queries
    CONSTRAINT chk_status CHECK (status IN ('SUCCESS', 'DUPLICATE', 'VALIDATION_ERROR', 
                                           'SCRAPING_ERROR', 'DATABASE_ERROR', 
                                           'TIMEOUT_ERROR', 'NETWORK_ERROR', 'PROCESSING')),
    
    CONSTRAINT chk_error_type CHECK (error_type IS NULL OR 
                                   error_type IN ('INVALID_URL', 'MISSING_FIELDS', 
                                                'CUFE_NOT_FOUND', 'HTML_PARSE_ERROR',
                                                'DB_CONNECTION_ERROR', 'DB_TRANSACTION_ERROR',
                                                'TIMEOUT', 'UNKNOWN'))
);

-- ============================================================================
-- INDEXES FOR PERFORMANCE
-- ============================================================================

-- Index for status queries
CREATE INDEX idx_bot_rust_scrapy_status ON logs.bot_rust_scrapy(status);

-- Index for user queries
CREATE INDEX idx_bot_rust_scrapy_user_id ON logs.bot_rust_scrapy(user_id);

-- Index for time-based queries
CREATE INDEX idx_bot_rust_scrapy_request_timestamp ON logs.bot_rust_scrapy(request_timestamp);

-- Index for CUFE lookups
CREATE INDEX idx_bot_rust_scrapy_cufe ON logs.bot_rust_scrapy(cufe) WHERE cufe IS NOT NULL;

-- Index for origin analysis
CREATE INDEX idx_bot_rust_scrapy_origin ON logs.bot_rust_scrapy(origin);

-- Composite index for user stats
CREATE INDEX idx_bot_rust_scrapy_user_stats ON logs.bot_rust_scrapy(user_id, request_timestamp, status);

-- ============================================================================
-- SAMPLE DATA AND VALIDATION
-- ============================================================================

-- Insert a sample log entry for testing
INSERT INTO logs.bot_rust_scrapy (
    url, 
    origin, 
    user_id, 
    user_email, 
    status,
    execution_time_ms,
    scraped_fields_count,
    cufe
) VALUES (
    'https://dgi-fep.mef.gob.pa/FacturasPorQR?chFE=FE012000...', 
    'whatsapp', 
    'test_user_123', 
    'test@example.com',
    'SUCCESS',
    1250,
    17,
    'FE01200002679372-1-844914-7300002025051500311570140020317481978892'
);

-- ============================================================================
-- GRANTS AND PERMISSIONS
-- ============================================================================

-- Grant appropriate permissions to application user
-- (Replace 'app_user' with your actual application database user)
-- GRANT SELECT, INSERT, UPDATE ON logs.bot_rust_scrapy TO app_user;
-- GRANT USAGE ON SEQUENCE logs.bot_rust_scrapy_id_seq TO app_user;

-- ============================================================================
-- VERIFICATION QUERIES
-- ============================================================================

-- Verify table creation
SELECT 
    schemaname, 
    tablename, 
    tableowner 
FROM pg_tables 
WHERE schemaname = 'logs' AND tablename = 'bot_rust_scrapy';

-- Check table structure
\d logs.bot_rust_scrapy

-- Verify sample data
SELECT 
    id,
    url,
    status,
    user_id,
    execution_time_ms,
    scraped_fields_count,
    request_timestamp
FROM logs.bot_rust_scrapy 
ORDER BY request_timestamp DESC 
LIMIT 5;

-- ============================================================================
-- MAINTENANCE QUERIES
-- ============================================================================

-- Query for system statistics (last 24 hours)
SELECT 
    COUNT(*) as total_requests,
    COUNT(CASE WHEN status = 'SUCCESS' THEN 1 END) as successful_requests,
    COUNT(CASE WHEN status = 'DUPLICATE' THEN 1 END) as duplicate_requests,
    COUNT(DISTINCT user_id) as unique_users,
    AVG(execution_time_ms) as avg_execution_time_ms,
    MAX(execution_time_ms) as max_execution_time_ms
FROM logs.bot_rust_scrapy 
WHERE request_timestamp >= NOW() - INTERVAL '24 hours';

-- Query for user statistics (last 30 days)
SELECT 
    user_id,
    COUNT(*) as total_requests,
    COUNT(CASE WHEN status = 'SUCCESS' THEN 1 END) as successful_requests,
    AVG(execution_time_ms) as avg_execution_time_ms
FROM logs.bot_rust_scrapy 
WHERE request_timestamp >= NOW() - INTERVAL '30 days'
GROUP BY user_id
ORDER BY total_requests DESC
LIMIT 10;

-- ============================================================================
-- CLEANUP PROCEDURE (Optional)
-- ============================================================================

-- Create a function to clean old log entries (older than 90 days)
-- This helps maintain performance and storage efficiency
CREATE OR REPLACE FUNCTION cleanup_old_bot_logs()
RETURNS INTEGER AS $$
DECLARE
    rows_deleted INTEGER;
BEGIN
    DELETE FROM logs.bot_rust_scrapy 
    WHERE request_timestamp < NOW() - INTERVAL '90 days';
    
    GET DIAGNOSTICS rows_deleted = ROW_COUNT;
    
    -- Log the cleanup operation
    INSERT INTO logs.bot_rust_scrapy (
        url, origin, user_id, user_email, status, error_message, execution_time_ms
    ) VALUES (
        'SYSTEM_CLEANUP', 'system', 'system', 'system@internal', 'SUCCESS', 
        format('Cleaned up %s old log entries', rows_deleted), 0
    );
    
    RETURN rows_deleted;
END;
$$ LANGUAGE plpgsql;

-- Example: Schedule cleanup to run weekly (uncomment if using pg_cron)
-- SELECT cron.schedule('cleanup-bot-logs', '0 2 * * 0', 'SELECT cleanup_old_bot_logs();');

COMMENT ON TABLE logs.bot_rust_scrapy IS 'Comprehensive logging for invoice processing bot operations - tracks all requests, performance metrics, and errors';
COMMENT ON FUNCTION cleanup_old_bot_logs() IS 'Maintenance function to clean up log entries older than 90 days';
