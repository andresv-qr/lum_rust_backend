-- Tabla de logs para ofertasws cache refresh
-- Base de datos: ws
-- Se ejecuta en el mismo esquema que wsf_consolidado

CREATE TABLE IF NOT EXISTS ofertasws_cache_refresh_log (
    id SERIAL PRIMARY KEY,
    executed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    status VARCHAR(20) NOT NULL, -- 'success', 'error', 'partial'
    records_count INTEGER, -- Número de ofertasws cacheadas
    execution_time_ms INTEGER, -- Tiempo de ejecución en milisegundos
    request_size_kb INTEGER, -- Tamaño del payload comprimido en kilobytes
    error_message TEXT, -- Mensaje de error si status='error'
    redis_key VARCHAR(100), -- Key de Redis utilizada
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Índice para consultas por fecha
CREATE INDEX IF NOT EXISTS idx_ofertasws_log_executed_at 
ON ofertasws_cache_refresh_log(executed_at DESC);

-- Comentarios
COMMENT ON TABLE ofertasws_cache_refresh_log IS 'Registro de ejecuciones del cache refresh de ofertasws';
COMMENT ON COLUMN ofertasws_cache_refresh_log.status IS 'Estado: success, error, partial';
COMMENT ON COLUMN ofertasws_cache_refresh_log.records_count IS 'Cantidad de ofertasws encontradas y cacheadas';
COMMENT ON COLUMN ofertasws_cache_refresh_log.execution_time_ms IS 'Tiempo total de ejecución en milisegundos';
COMMENT ON COLUMN ofertasws_cache_refresh_log.request_size_kb IS 'Tamaño del payload comprimido en kilobytes';
