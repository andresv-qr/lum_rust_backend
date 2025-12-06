-- ============================================================================
-- MIGRACIÓN: Sistema completo de Redenciones con Webhooks y Notificaciones
-- ============================================================================
-- Fecha: 2025-10-18
-- Descripción: Agregar soporte completo para webhooks, push notifications,
--              rate limiting y analytics para el sistema de redenciones

-- ============================================================================
-- 1. AGREGAR COLUMNAS A TABLA merchants PARA WEBHOOKS
-- ============================================================================

ALTER TABLE IF EXISTS rewards.merchants
ADD COLUMN IF NOT EXISTS webhook_url TEXT,
ADD COLUMN IF NOT EXISTS webhook_secret TEXT,
ADD COLUMN IF NOT EXISTS webhook_events TEXT[] DEFAULT ARRAY['redemption.created', 'redemption.confirmed', 'redemption.expired', 'redemption.cancelled'],
ADD COLUMN IF NOT EXISTS webhook_enabled BOOLEAN DEFAULT false,
ADD COLUMN IF NOT EXISTS last_stats_update TIMESTAMP WITH TIME ZONE,
ADD COLUMN IF NOT EXISTS total_redemptions INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_revenue BIGINT DEFAULT 0;

COMMENT ON COLUMN rewards.merchants.webhook_url IS 'URL del webhook del merchant para recibir notificaciones';
COMMENT ON COLUMN rewards.merchants.webhook_secret IS 'Secret para firmar webhooks con HMAC-SHA256';
COMMENT ON COLUMN rewards.merchants.webhook_events IS 'Array de eventos suscritos (redemption.created, redemption.confirmed, etc)';
COMMENT ON COLUMN rewards.merchants.webhook_enabled IS 'Si el webhook está activo';

-- ============================================================================
-- 2. CREAR TABLA DE LOG DE WEBHOOKS
-- ============================================================================

CREATE TABLE IF NOT EXISTS rewards.webhook_logs (
    log_id SERIAL PRIMARY KEY,
    merchant_id UUID NOT NULL REFERENCES rewards.merchants(merchant_id),
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    sent_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    response_time_ms INTEGER,
    
    CONSTRAINT fk_merchant FOREIGN KEY (merchant_id) 
        REFERENCES rewards.merchants(merchant_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_webhook_logs_merchant_id ON rewards.webhook_logs(merchant_id);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_sent_at ON rewards.webhook_logs(sent_at DESC);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_event_type ON rewards.webhook_logs(event_type);

COMMENT ON TABLE rewards.webhook_logs IS 'Log de webhooks enviados a merchants';

-- ============================================================================
-- 3. CREAR TABLA PARA TOKENS FCM DE USUARIOS
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.user_devices (
    device_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    fcm_token TEXT NOT NULL,
    device_type TEXT, -- 'ios', 'android', 'web'
    device_name TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    
    UNIQUE(user_id, fcm_token)
);

CREATE INDEX IF NOT EXISTS idx_user_devices_user_id ON public.user_devices(user_id);
CREATE INDEX IF NOT EXISTS idx_user_devices_fcm_token ON public.user_devices(fcm_token);
CREATE INDEX IF NOT EXISTS idx_user_devices_active ON public.user_devices(is_active) WHERE is_active = true;

COMMENT ON TABLE public.user_devices IS 'Dispositivos registrados de usuarios para push notifications';

-- ============================================================================
-- 4. CREAR TABLA DE LOG DE PUSH NOTIFICATIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.push_notifications_log (
    notification_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    data JSONB,
    sent_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    success BOOLEAN DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_push_log_user_id ON public.push_notifications_log(user_id);
CREATE INDEX IF NOT EXISTS idx_push_log_sent_at ON public.push_notifications_log(sent_at DESC);

COMMENT ON TABLE public.push_notifications_log IS 'Historial de notificaciones push enviadas';

-- ============================================================================
-- 5. AGREGAR COLUMNA expiration_alert_sent A user_redemptions
-- ============================================================================

ALTER TABLE IF EXISTS rewards.user_redemptions
ADD COLUMN IF NOT EXISTS expiration_alert_sent BOOLEAN DEFAULT false;

COMMENT ON COLUMN rewards.user_redemptions.expiration_alert_sent IS 'Si se envió alerta de expiración próxima';

-- ============================================================================
-- 6. CREAR TABLA PARA CACHE DE QR CODES (OPCIONAL)
-- ============================================================================

CREATE TABLE IF NOT EXISTS rewards.qr_code_cache (
    qr_id SERIAL PRIMARY KEY,
    redemption_code TEXT NOT NULL UNIQUE,
    qr_image_data BYTEA,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_qr_cache_redemption_code ON rewards.qr_code_cache(redemption_code);
CREATE INDEX IF NOT EXISTS idx_qr_cache_expires_at ON rewards.qr_code_cache(expires_at);

COMMENT ON TABLE rewards.qr_code_cache IS 'Cache de imágenes QR generadas (opcional)';

-- ============================================================================
-- 7. CREAR VISTA DE ANALYTICS PARA MERCHANTS
-- ============================================================================

CREATE OR REPLACE VIEW rewards.vw_merchant_analytics AS
SELECT 
    m.merchant_id,
    m.merchant_name,
    COUNT(ur.redemption_id) as total_redemptions,
    COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed') as confirmed_redemptions,
    COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'pending') as pending_redemptions,
    COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'expired') as expired_redemptions,
    COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'cancelled') as cancelled_redemptions,
    COALESCE(SUM(ur.lumis_spent), 0) as total_lumis,
    ROUND(AVG(EXTRACT(EPOCH FROM (ur.validated_at - ur.created_at)) / 60.0)::numeric, 2) as avg_confirmation_minutes,
    ROUND(
        (COUNT(*) FILTER (WHERE ur.redemption_status = 'expired')::numeric / 
         NULLIF(COUNT(*)::numeric, 0) * 100), 
        2
    ) as expiration_rate_pct
FROM rewards.merchants m
LEFT JOIN rewards.redemption_offers ro ON m.merchant_id = ro.merchant_id
LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
WHERE m.is_active = true
GROUP BY m.merchant_id, m.merchant_name;

COMMENT ON VIEW rewards.vw_merchant_analytics IS 'Vista consolidada de analytics por merchant';

-- ============================================================================
-- 8. FUNCIÓN PARA ACTUALIZAR STATS DE MERCHANTS (PARA CRON JOB)
-- ============================================================================

CREATE OR REPLACE FUNCTION rewards.fn_update_merchant_stats()
RETURNS void AS $$
BEGIN
    UPDATE rewards.merchants m
    SET 
        total_redemptions = COALESCE(stats.confirmed_count, 0),
        total_revenue = COALESCE(stats.total_lumis, 0),
        last_stats_update = NOW()
    FROM (
        SELECT 
            ro.merchant_id,
            COUNT(*) FILTER (WHERE ur.redemption_status = 'confirmed') as confirmed_count,
            SUM(ur.lumis_spent) FILTER (WHERE ur.redemption_status = 'confirmed') as total_lumis
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        GROUP BY ro.merchant_id
    ) stats
    WHERE m.merchant_id = stats.merchant_id
      AND m.is_active = true;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION rewards.fn_update_merchant_stats() IS 'Recalcula estadísticas de merchants (llamar desde cron job)';

-- ============================================================================
-- 9. ÍNDICES ADICIONALES PARA OPTIMIZACIÓN
-- ============================================================================

-- Índice compuesto para queries de analytics por merchant y fecha
CREATE INDEX IF NOT EXISTS idx_user_redemptions_merchant_date 
ON rewards.user_redemptions(merchant_id, created_at DESC) 
WHERE redemption_status IN ('confirmed', 'pending', 'expired');

-- Índice para búsqueda de redenciones pendientes a expirar
CREATE INDEX IF NOT EXISTS idx_user_redemptions_expiring 
ON rewards.user_redemptions(code_expires_at) 
WHERE redemption_status = 'pending' AND expiration_alert_sent = false;

-- Índice para redenciones por hora (analytics)
CREATE INDEX IF NOT EXISTS idx_user_redemptions_hour 
ON rewards.user_redemptions(EXTRACT(HOUR FROM created_at));

-- ============================================================================
-- 10. PERMISOS
-- ============================================================================

-- Otorgar permisos al usuario de la aplicación (ajusta según tu usuario)
GRANT SELECT, INSERT, UPDATE ON rewards.webhook_logs TO avalencia;
GRANT USAGE, SELECT ON SEQUENCE rewards.webhook_logs_log_id_seq TO avalencia;

GRANT SELECT, INSERT, UPDATE ON public.user_devices TO avalencia;
GRANT USAGE, SELECT ON SEQUENCE public.user_devices_device_id_seq TO avalencia;

GRANT SELECT, INSERT ON public.push_notifications_log TO avalencia;
GRANT USAGE, SELECT ON SEQUENCE public.push_notifications_log_notification_id_seq TO avalencia;

GRANT SELECT, INSERT, DELETE ON rewards.qr_code_cache TO avalencia;
GRANT USAGE, SELECT ON SEQUENCE rewards.qr_code_cache_qr_id_seq TO avalencia;

GRANT SELECT ON rewards.vw_merchant_analytics TO avalencia;

-- ============================================================================
-- VERIFICACIÓN
-- ============================================================================

-- Verificar que todo se creó correctamente
SELECT 'Tablas creadas:' as status;
SELECT table_name FROM information_schema.tables 
WHERE table_schema = 'rewards' 
  AND table_name IN ('webhook_logs', 'qr_code_cache');

SELECT table_name FROM information_schema.tables 
WHERE table_schema = 'public' 
  AND table_name IN ('user_devices', 'push_notifications_log');

SELECT 'Columnas agregadas a merchants:' as status;
SELECT column_name FROM information_schema.columns 
WHERE table_schema = 'rewards' 
  AND table_name = 'merchants' 
  AND column_name IN ('webhook_url', 'webhook_secret', 'webhook_events', 'webhook_enabled');

SELECT 'Índices creados:' as status;
SELECT indexname FROM pg_indexes 
WHERE schemaname = 'rewards' 
  AND tablename IN ('webhook_logs', 'user_redemptions', 'qr_code_cache');

SELECT '✅ Migración completada exitosamente' as result;
