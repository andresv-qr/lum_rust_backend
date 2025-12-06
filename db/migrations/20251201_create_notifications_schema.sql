-- Migration: Create Notifications System Schema
-- Date: 2025-12-01
-- Version: 2.2
-- Description: Tables for in-app notifications and push notification management
-- Author: Backend Team

BEGIN;

-- ============================================================================
-- 1. TABLA PRINCIPAL: notifications
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.notifications (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES public.dim_users(id) ON DELETE CASCADE,
    
    -- Contenido
    title VARCHAR(200) NOT NULL,
    body TEXT NOT NULL,
    type VARCHAR(50) NOT NULL,
    priority VARCHAR(20) NOT NULL DEFAULT 'normal',
    
    -- Estado
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    is_dismissed BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Contenido opcional
    image_url TEXT,
    action_url VARCHAR(255),
    payload JSONB DEFAULT '{}',
    
    -- Deduplicación
    idempotency_key VARCHAR(100),
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    read_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    
    -- Constraints de validación
    CONSTRAINT chk_notifications_type CHECK (type IN (
        'reward', 'achievement', 'streak', 'invoice', 
        'promo', 'system', 'challenge', 'level_up', 'reminder'
    )),
    CONSTRAINT chk_notifications_priority CHECK (priority IN ('low', 'normal', 'high', 'urgent'))
);

-- Índice principal para listado por usuario (ordenado por fecha)
CREATE INDEX IF NOT EXISTS idx_notifications_user_created 
    ON public.notifications(user_id, created_at DESC);

-- Partial index para badge count (super rápido para notificaciones no leídas)
CREATE INDEX IF NOT EXISTS idx_notifications_user_unread 
    ON public.notifications(user_id) 
    WHERE is_read = FALSE AND is_dismissed = FALSE;

-- Índice para limpieza automática de expiradas
CREATE INDEX IF NOT EXISTS idx_notifications_expires 
    ON public.notifications(expires_at) 
    WHERE expires_at IS NOT NULL;

-- Índice único de deduplicación (evita duplicados en retries)
CREATE UNIQUE INDEX IF NOT EXISTS idx_notifications_idempotency 
    ON public.notifications(user_id, idempotency_key) 
    WHERE idempotency_key IS NOT NULL;

-- Índice para filtrado por tipo
CREATE INDEX IF NOT EXISTS idx_notifications_user_type
    ON public.notifications(user_id, type, created_at DESC)
    WHERE is_dismissed = FALSE;

-- Comentarios
COMMENT ON TABLE public.notifications IS 'Notificaciones in-app y push para usuarios Lüm';
COMMENT ON COLUMN public.notifications.idempotency_key IS 'Clave única para evitar duplicados. Formato: {type}_{reference_id}_{date}';
COMMENT ON COLUMN public.notifications.action_url IS 'Deep link para navegación en app. Ej: /invoices/ABC123, /achievements';
COMMENT ON COLUMN public.notifications.payload IS 'Datos adicionales en formato JSON para el frontend';

-- ============================================================================
-- 2. TABLA: device_tokens (FCM Push Notifications)
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.device_tokens (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES public.dim_users(id) ON DELETE CASCADE,
    
    fcm_token TEXT NOT NULL,
    platform VARCHAR(20) NOT NULL,
    device_id VARCHAR(255),
    device_name VARCHAR(100),
    app_version VARCHAR(20),
    
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    
    -- Constraint de validación
    CONSTRAINT chk_device_tokens_platform CHECK (platform IN ('android', 'ios', 'web'))
);

-- Índice único: un token FCM activo solo puede pertenecer a un usuario
CREATE UNIQUE INDEX IF NOT EXISTS idx_device_tokens_fcm_active 
    ON public.device_tokens(fcm_token) 
    WHERE is_active = TRUE;

-- Índice para buscar tokens activos de un usuario
CREATE INDEX IF NOT EXISTS idx_device_tokens_user_active 
    ON public.device_tokens(user_id) 
    WHERE is_active = TRUE;

-- Índice para limpieza de tokens inactivos antiguos
CREATE INDEX IF NOT EXISTS idx_device_tokens_inactive_cleanup
    ON public.device_tokens(updated_at)
    WHERE is_active = FALSE;

-- Comentarios
COMMENT ON TABLE public.device_tokens IS 'Tokens FCM para push notifications';
COMMENT ON COLUMN public.device_tokens.device_id IS 'Identificador único del dispositivo físico';
COMMENT ON COLUMN public.device_tokens.is_active IS 'FALSE cuando el token es inválido o el usuario hizo logout';

-- ============================================================================
-- 3. TABLA: notification_push_queue (Cola asíncrona de push)
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.notification_push_queue (
    id BIGSERIAL PRIMARY KEY,
    notification_id BIGINT NOT NULL REFERENCES public.notifications(id) ON DELETE CASCADE,
    
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    
    last_attempt_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ DEFAULT NOW(),
    
    error_message TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    
    -- Constraint de validación
    CONSTRAINT chk_push_queue_status CHECK (status IN ('pending', 'processing', 'sent', 'failed', 'skipped'))
);

-- Índice para el worker: obtener pendientes ordenados por prioridad
CREATE INDEX IF NOT EXISTS idx_push_queue_pending 
    ON public.notification_push_queue(status, next_attempt_at)
    WHERE status IN ('pending', 'processing');

-- Índice para limpieza de procesados antiguos
CREATE INDEX IF NOT EXISTS idx_push_queue_completed
    ON public.notification_push_queue(completed_at)
    WHERE status IN ('sent', 'skipped', 'failed');

-- Comentarios
COMMENT ON TABLE public.notification_push_queue IS 'Cola de push notifications para procesamiento asíncrono';
COMMENT ON COLUMN public.notification_push_queue.next_attempt_at IS 'Siguiente intento con backoff exponencial';
COMMENT ON COLUMN public.notification_push_queue.status IS 'pending: en espera, processing: siendo procesado, sent: enviado OK, failed: falló después de max_attempts, skipped: no se envió (ej: usuario sin tokens)';

-- ============================================================================
-- 4. TRIGGER: Manejo de tokens FCM en cambio de cuenta
-- ============================================================================

-- Función: Al registrar un token, desactivar el mismo token en otras cuentas
CREATE OR REPLACE FUNCTION public.handle_fcm_token_registration()
RETURNS TRIGGER AS $$
BEGIN
    -- Desactivar el mismo token para otros usuarios (cambio de cuenta)
    UPDATE public.device_tokens 
    SET is_active = FALSE, 
        updated_at = NOW()
    WHERE fcm_token = NEW.fcm_token 
    AND user_id != NEW.user_id 
    AND is_active = TRUE;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Crear trigger solo si no existe
DROP TRIGGER IF EXISTS trg_fcm_token_registration ON public.device_tokens;
CREATE TRIGGER trg_fcm_token_registration
    BEFORE INSERT ON public.device_tokens
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_fcm_token_registration();

COMMENT ON FUNCTION public.handle_fcm_token_registration() IS 
'Desactiva tokens duplicados en otras cuentas cuando un usuario registra un token que ya existía (cambio de cuenta en mismo dispositivo)';

-- ============================================================================
-- 5. FUNCIÓN: Crear notificación con deduplicación
-- ============================================================================

CREATE OR REPLACE FUNCTION public.create_notification(
    p_user_id BIGINT,
    p_title VARCHAR(200),
    p_body TEXT,
    p_type VARCHAR(50),
    p_priority VARCHAR(20) DEFAULT 'normal',
    p_action_url VARCHAR(255) DEFAULT NULL,
    p_image_url TEXT DEFAULT NULL,
    p_payload JSONB DEFAULT '{}',
    p_idempotency_key VARCHAR(100) DEFAULT NULL,
    p_expires_at TIMESTAMPTZ DEFAULT NULL,
    p_send_push BOOLEAN DEFAULT TRUE
)
RETURNS BIGINT AS $$
DECLARE
    v_notification_id BIGINT;
BEGIN
    -- Insertar notificación con deduplicación
    INSERT INTO public.notifications (
        user_id, title, body, type, priority,
        action_url, image_url, payload, idempotency_key, expires_at
    ) VALUES (
        p_user_id, p_title, p_body, p_type, p_priority,
        p_action_url, p_image_url, p_payload, p_idempotency_key, p_expires_at
    )
    ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL
    DO NOTHING
    RETURNING id INTO v_notification_id;
    
    -- Si fue duplicado, v_notification_id será NULL
    IF v_notification_id IS NULL THEN
        RAISE NOTICE 'create_notification: Duplicado ignorado (idempotency_key=%)', p_idempotency_key;
        RETURN NULL;
    END IF;
    
    -- Encolar push notification si está habilitado
    IF p_send_push THEN
        INSERT INTO public.notification_push_queue (notification_id, status)
        VALUES (v_notification_id, 'pending');
    END IF;
    
    RETURN v_notification_id;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.create_notification IS 
'Crea notificación con deduplicación automática por idempotency_key.
Retorna NULL si es duplicado, BIGINT id si se creó.
Formato recomendado para idempotency_key: {type}_{reference_id}_{date}
Ejemplo: invoice_ABC123_2025-12-01, achievement_first_week_12345_2025-12-01';

-- ============================================================================
-- 6. FUNCIÓN: Notificar logro desbloqueado (callable desde Rust)
-- ============================================================================

CREATE OR REPLACE FUNCTION public.notify_achievement_unlocked(
    p_user_id BIGINT,
    p_achievement_code VARCHAR(50),
    p_achievement_name VARCHAR(200),
    p_lumis_reward INTEGER
)
RETURNS BIGINT AS $$
BEGIN
    RETURN public.create_notification(
        p_user_id := p_user_id,
        p_title := '🏆 ¡Logro desbloqueado!',
        p_body := FORMAT('Completaste "%s" y ganaste %s Lümis', p_achievement_name, p_lumis_reward),
        p_type := 'achievement',
        p_priority := 'high',
        p_action_url := '/achievements',
        p_payload := jsonb_build_object(
            'achievement_code', p_achievement_code,
            'achievement_name', p_achievement_name,
            'lumis_reward', p_lumis_reward
        ),
        p_idempotency_key := FORMAT('achievement_%s_%s_%s', p_user_id, p_achievement_code, CURRENT_DATE),
        p_send_push := TRUE
    );
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.notify_achievement_unlocked IS 
'Notifica logro desbloqueado. Llamar desde gamification.grant_achievement_reward().
Idempotency garantiza una notificación por logro por día.';

-- ============================================================================
-- 7. FUNCIÓN: Limpieza automática de datos antiguos
-- ============================================================================

CREATE OR REPLACE FUNCTION public.cleanup_notifications()
RETURNS TABLE(
    notifications_expired INTEGER,
    notifications_old_read INTEGER,
    tokens_inactive INTEGER,
    queue_processed INTEGER
) AS $$
DECLARE
    v_notifications_expired INTEGER := 0;
    v_notifications_old_read INTEGER := 0;
    v_tokens_inactive INTEGER := 0;
    v_queue_processed INTEGER := 0;
BEGIN
    -- 1. Eliminar notificaciones expiradas
    DELETE FROM public.notifications 
    WHERE expires_at IS NOT NULL 
    AND expires_at < NOW();
    GET DIAGNOSTICS v_notifications_expired = ROW_COUNT;
    
    -- 2. Eliminar notificaciones leídas antiguas (>90 días)
    DELETE FROM public.notifications 
    WHERE is_read = TRUE 
    AND created_at < NOW() - INTERVAL '90 days';
    GET DIAGNOSTICS v_notifications_old_read = ROW_COUNT;
    
    -- 3. Eliminar tokens inactivos antiguos (>60 días)
    DELETE FROM public.device_tokens 
    WHERE is_active = FALSE 
    AND updated_at < NOW() - INTERVAL '60 days';
    GET DIAGNOSTICS v_tokens_inactive = ROW_COUNT;
    
    -- 4. Eliminar items procesados de la cola (>7 días)
    DELETE FROM public.notification_push_queue
    WHERE status IN ('sent', 'skipped', 'failed')
    AND completed_at < NOW() - INTERVAL '7 days';
    GET DIAGNOSTICS v_queue_processed = ROW_COUNT;
    
    RETURN QUERY SELECT 
        v_notifications_expired,
        v_notifications_old_read,
        v_tokens_inactive,
        v_queue_processed;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION public.cleanup_notifications IS 
'Job de limpieza para ejecutar diariamente vía pg_cron.
Elimina: notificaciones expiradas, leídas >90 días, tokens inactivos >60 días, cola procesada >7 días.';

-- ============================================================================
-- 8. VERIFICACIÓN DE LA MIGRACIÓN
-- ============================================================================

DO $$
DECLARE
    v_tables_created INTEGER := 0;
    v_indexes_created INTEGER := 0;
    v_functions_created INTEGER := 0;
BEGIN
    -- Verificar tablas
    SELECT COUNT(*) INTO v_tables_created
    FROM information_schema.tables 
    WHERE table_schema = 'public' 
    AND table_name IN ('notifications', 'device_tokens', 'notification_push_queue');
    
    -- Verificar índices
    SELECT COUNT(*) INTO v_indexes_created
    FROM pg_indexes 
    WHERE schemaname = 'public' 
    AND indexname LIKE 'idx_notifications%' 
    OR indexname LIKE 'idx_device_tokens%'
    OR indexname LIKE 'idx_push_queue%';
    
    -- Verificar funciones
    SELECT COUNT(*) INTO v_functions_created
    FROM information_schema.routines 
    WHERE routine_schema = 'public' 
    AND routine_name IN ('create_notification', 'notify_achievement_unlocked', 'cleanup_notifications', 'handle_fcm_token_registration');
    
    RAISE NOTICE '';
    RAISE NOTICE '╔═══════════════════════════════════════════════════════════════════╗';
    RAISE NOTICE '║          MIGRACIÓN NOTIFICATIONS SCHEMA v2.2 COMPLETADA           ║';
    RAISE NOTICE '╠═══════════════════════════════════════════════════════════════════╣';
    RAISE NOTICE '║  Tablas creadas:    % / 3                                         ║', v_tables_created;
    RAISE NOTICE '║  Índices creados:   % (incluye parciales)                        ║', v_indexes_created;
    RAISE NOTICE '║  Funciones creadas: % / 4                                         ║', v_functions_created;
    RAISE NOTICE '╠═══════════════════════════════════════════════════════════════════╣';
    RAISE NOTICE '║  TABLAS:                                                          ║';
    RAISE NOTICE '║    ✅ public.notifications                                        ║';
    RAISE NOTICE '║    ✅ public.device_tokens                                        ║';
    RAISE NOTICE '║    ✅ public.notification_push_queue                              ║';
    RAISE NOTICE '║  FUNCIONES:                                                       ║';
    RAISE NOTICE '║    ✅ public.create_notification()                                ║';
    RAISE NOTICE '║    ✅ public.notify_achievement_unlocked()                        ║';
    RAISE NOTICE '║    ✅ public.cleanup_notifications()                              ║';
    RAISE NOTICE '║    ✅ public.handle_fcm_token_registration()                      ║';
    RAISE NOTICE '╚═══════════════════════════════════════════════════════════════════╝';
    RAISE NOTICE '';
END $$;

COMMIT;
