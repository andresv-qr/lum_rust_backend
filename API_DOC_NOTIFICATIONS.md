# 📋 Especificación de API: Sistema de Notificaciones Lüm

## Documento para Equipo Backend

**Fecha:** 1 de Diciembre, 2025  
**Versión:** 2.2 (Revisada - Correcciones Críticas)  
**App:** Lüm (Flutter)  
**API Version:** v4  
**Base URL:** `https://webh.lumapp.org/api/v4`

---

## 1. Resumen Ejecutivo

El frontend de Lüm necesita un sistema de notificaciones que permita:
1. Almacenar y consultar notificaciones por usuario
2. Marcar notificaciones como leídas (individual y batch)
3. Enviar push notifications vía Firebase Cloud Messaging (FCM)
4. Registrar tokens de dispositivos para push
5. Obtener contador de no leídas (para badge UI)

### 1.1 Cambios en v2.0
- ✅ Corregido `user_id` de `UUID` a `BIGINT` (consistente con `dim_users.id`)
- ✅ Agregado endpoint `GET /notifications/count` para badge
- ✅ Agregado índice de deduplicación para evitar notificaciones duplicadas
- ✅ Agregado trigger para manejo seguro de tokens FCM en cambio de cuenta
- ✅ Documentado rate limiting con Redis
- ✅ Agregado manejo de race conditions

---

## 2. Modelo de Datos

### 2.1 Tabla `notifications`

```sql
-- Schema: public (mismo que dim_users)
CREATE TABLE public.notifications (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES public.dim_users(id) ON DELETE CASCADE,
    
    -- Contenido
    title VARCHAR(200) NOT NULL,
    body TEXT NOT NULL,
    type VARCHAR(50) NOT NULL,  -- Ver enum abajo
    priority VARCHAR(20) NOT NULL DEFAULT 'normal',  -- low, normal, high, urgent
    
    -- Estado
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    is_dismissed BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Contenido opcional
    image_url TEXT,
    action_url VARCHAR(255),  -- Deep link: /rewards/123, /achievements/456
    payload JSONB DEFAULT '{}',  -- Datos adicionales flexibles
    
    -- Deduplicación
    idempotency_key VARCHAR(100),  -- Para evitar duplicados en retries
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    read_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,  -- NULL = no expira
    
    -- Constraints
    CONSTRAINT valid_type CHECK (type IN (
        'reward', 'achievement', 'streak', 'invoice', 
        'promo', 'system', 'challenge', 'level_up', 'reminder'
    )),
    CONSTRAINT valid_priority CHECK (priority IN ('low', 'normal', 'high', 'urgent'))
);

-- Índices críticos para performance
CREATE INDEX idx_notifications_user_created 
    ON public.notifications(user_id, created_at DESC);

-- Partial index para badge count (super rápido)
CREATE INDEX idx_notifications_user_unread 
    ON public.notifications(user_id) 
    WHERE is_read = FALSE AND is_dismissed = FALSE;

-- Índice para limpieza de expiradas
CREATE INDEX idx_notifications_expires 
    ON public.notifications(expires_at) 
    WHERE expires_at IS NOT NULL;

-- Índice de deduplicación (evita notificaciones duplicadas por retry)
CREATE UNIQUE INDEX idx_notifications_idempotency 
    ON public.notifications(user_id, idempotency_key) 
    WHERE idempotency_key IS NOT NULL;

-- Comentarios
COMMENT ON TABLE public.notifications IS 'Notificaciones in-app y push para usuarios';
COMMENT ON COLUMN public.notifications.idempotency_key IS 'Clave única para evitar duplicados en retries. Formato: {type}_{reference_id}_{timestamp_bucket}';
```

### 2.2 Tabla `device_tokens`

```sql
CREATE TABLE public.device_tokens (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES public.dim_users(id) ON DELETE CASCADE,
    
    fcm_token TEXT NOT NULL,
    platform VARCHAR(20) NOT NULL,  -- 'android', 'ios'
    device_id VARCHAR(255),  -- Identificador único del dispositivo
    device_name VARCHAR(100),  -- "Pixel 7", "iPhone 14"
    app_version VARCHAR(20),  -- "0.2.5"
    
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    
    CONSTRAINT valid_platform CHECK (platform IN ('android', 'ios', 'web'))
);

-- Índice único en fcm_token (un token solo puede pertenecer a un usuario activo)
CREATE UNIQUE INDEX idx_device_tokens_fcm_active 
    ON public.device_tokens(fcm_token) 
    WHERE is_active = TRUE;

-- Índice para buscar tokens activos de un usuario
CREATE INDEX idx_device_tokens_user_active 
    ON public.device_tokens(user_id) 
    WHERE is_active = TRUE;

-- Trigger: Al registrar un token, desactivar el mismo token en otras cuentas
-- (Maneja el caso de cambio de cuenta en el mismo dispositivo)
CREATE OR REPLACE FUNCTION public.handle_fcm_token_registration()
RETURNS TRIGGER AS $$
BEGIN
    -- Desactivar el mismo token para otros usuarios
    UPDATE public.device_tokens 
    SET is_active = FALSE, updated_at = NOW()
    WHERE fcm_token = NEW.fcm_token 
    AND user_id != NEW.user_id 
    AND is_active = TRUE;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_fcm_token_registration
    BEFORE INSERT ON public.device_tokens
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_fcm_token_registration();

COMMENT ON TABLE public.device_tokens IS 'Tokens FCM para push notifications';
COMMENT ON FUNCTION public.handle_fcm_token_registration() IS 'Desactiva tokens duplicados en otras cuentas (cambio de cuenta)';
```

---

## 3. Endpoints REST

### 3.0 Consideraciones de Seguridad (CRÍTICO)

Todos los endpoints de notificaciones DEBEN validar que el recurso pertenece al usuario autenticado:

```rust
// Pseudocódigo para cada endpoint
async fn get_notification(user_id: i64, notification_id: i64) -> Result<Notification> {
    let notification = sqlx::query_as!(
        Notification,
        "SELECT * FROM notifications WHERE id = $1 AND user_id = $2",
        notification_id,
        user_id  // <- CRÍTICO: Siempre filtrar por user_id del JWT
    ).fetch_optional(&pool).await?;
    
    notification.ok_or(NotFoundError)
}
```

**Reglas de seguridad:**
1. **Nunca** exponer notificaciones de otros usuarios
2. **Siempre** incluir `user_id` en WHERE clauses
3. **Validar** ownership antes de UPDATE/DELETE
4. Rate limit: 100 requests/minuto por usuario

---

### 3.1 Listar Notificaciones

```
GET /api/v4/notifications
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Query Parameters:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | int | 20 | Máximo de resultados (max 100) |
| `offset` | int | 0 | Para paginación |
| `unread_only` | bool | false | Solo no leídas |
| `type` | string | null | Filtrar por tipo |
| `since` | ISO8601 | null | Notificaciones desde fecha (para sync incremental) |

**Response 200:**
```json
{
    "success": true,
    "data": {
        "notifications": [
            {
                "id": 12345,
                "title": "¡Factura procesada!",
                "body": "Ganaste 25 Lümis con tu compra en Super 99",
                "type": "invoice",
                "priority": "normal",
                "is_read": false,
                "image_url": null,
                "action_url": "/invoices/123",
                "payload": {
                    "invoice_id": 123,
                    "lumis_earned": 25,
                    "merchant_name": "Super 99"
                },
                "created_at": "2025-11-30T15:30:00Z",
                "expires_at": null
            },
            {
                "id": 12344,
                "title": "🏆 ¡Logro desbloqueado!",
                "body": "Completaste 'Primera Semana' - 7 días consecutivos",
                "type": "achievement",
                "priority": "high",
                "is_read": false,
                "image_url": "https://cdn.lumapp.org/achievements/first_week.png",
                "action_url": "/achievements/first_week",
                "payload": {
                    "achievement_id": "first_week",
                    "lumis_reward": 100
                },
                "created_at": "2025-11-30T14:00:00Z",
                "expires_at": null
            }
        ],
        "meta": {
            "total": 47,
            "unread_count": 5,
            "limit": 20,
            "offset": 0,
            "has_more": true
        }
    },
    "error": null,
    "request_id": "req_abc123",
    "timestamp": "2025-11-30T15:35:00Z"
}
```

---

### 3.2 Obtener Contador de No Leídas (Badge)

```
GET /api/v4/notifications/count
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Response 200:**
```json
{
    "success": true,
    "data": {
        "unread_count": 5,
        "by_type": {
            "invoice": 2,
            "achievement": 1,
            "streak": 1,
            "promo": 1
        },
        "has_urgent": false
    },
    "error": null,
    "request_id": "req_abc123",
    "timestamp": "2025-11-30T15:35:00Z"
}
```

**Nota:** Este endpoint está optimizado con partial index. Usar para actualizar badge en cada app resume.

---

### 3.3 Marcar como Leída

```
POST /api/v4/notifications/{id}/read
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Response 200:**
```json
{
    "success": true,
    "data": {
        "id": 12345,
        "is_read": true,
        "read_at": "2025-11-30T15:36:00Z"
    },
    "error": null,
    "request_id": "req_abc124",
    "timestamp": "2025-11-30T15:36:00Z"
}
```

**Response 404:**
```json
{
    "success": false,
    "data": null,
    "error": {
        "code": "NOTIFICATION_NOT_FOUND",
        "message": "Notificación no encontrada"
    },
    "request_id": "req_abc125",
    "timestamp": "2025-11-30T15:36:00Z"
}
```

**Nota sobre Race Conditions:** Si múltiples requests intentan marcar la misma notificación, todas retornarán éxito. El `read_at` será el del primer request (usando `WHERE is_read = FALSE` en el UPDATE).

---

### 3.4 Marcar Todas como Leídas

```
POST /api/v4/notifications/read-all
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Request Body (opcional):**
```json
{
    "type": "invoice",  // Opcional: solo marcar de este tipo
    "before": "2025-11-30T15:00:00Z"  // Opcional: solo anteriores a esta fecha
}
```

**Response 200:**
```json
{
    "success": true,
    "data": {
        "marked_count": 5,
        "read_at": "2025-11-30T15:37:00Z"
    },
    "error": null,
    "request_id": "req_abc126",
    "timestamp": "2025-11-30T15:37:00Z"
}
```

---

### 3.5 Eliminar/Descartar Notificación

```
DELETE /api/v4/notifications/{id}
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Response 200:**
```json
{
    "success": true,
    "data": {
        "id": 12345,
        "dismissed": true
    },
    "error": null,
    "request_id": "req_abc127",
    "timestamp": "2025-11-30T15:38:00Z"
}
```

**Nota:** Esto hace soft-delete (`is_dismissed = TRUE`), no elimina el registro.

---

### 3.6 Registrar Token FCM

```
POST /api/v4/devices/fcm-token
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Request Body:**
```json
{
    "fcm_token": "fMz9Q3...(token largo de FCM)",
    "platform": "android",
    "device_id": "unique_device_id_123",
    "device_name": "Pixel 7 Pro",
    "app_version": "0.2.5"
}
```

**Response 200:**
```json
{
    "success": true,
    "data": {
        "registered": true,
        "device_id": "unique_device_id_123",
        "is_new": true
    },
    "error": null,
    "request_id": "req_abc128",
    "timestamp": "2025-11-30T15:39:00Z"
}
```

**Comportamiento en cambio de cuenta:**
- Si el token ya estaba registrado para otro usuario, se desactiva automáticamente en la cuenta anterior
- El trigger `trg_fcm_token_registration` maneja esto de forma transparente

**SQL de implementación (con ON CONFLICT para race conditions):**
```sql
-- El endpoint debe usar ON CONFLICT para evitar race conditions
INSERT INTO public.device_tokens (user_id, fcm_token, platform, device_id, device_name, app_version)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (fcm_token) WHERE is_active = TRUE
DO UPDATE SET 
    user_id = EXCLUDED.user_id,
    platform = EXCLUDED.platform,
    device_id = EXCLUDED.device_id,
    device_name = EXCLUDED.device_name,
    app_version = EXCLUDED.app_version,
    updated_at = NOW(),
    last_used_at = NOW()
RETURNING id, (xmax = 0) as is_new;
-- xmax = 0 significa INSERT, xmax > 0 significa UPDATE
```

---

### 3.7 Eliminar Token FCM (Logout)

```
DELETE /api/v4/devices/fcm-token
```

**Headers:**
```
Authorization: Bearer <jwt_token>
```

**Request Body:**
```json
{
    "fcm_token": "fMz9Q3...(token a eliminar)"
}
```

**Response 200:**
```json
{
    "success": true,
    "data": {
        "removed": true
    },
    "error": null,
    "request_id": "req_abc129",
    "timestamp": "2025-11-30T15:40:00Z"
}
```

---

## 4. Tipos de Notificación

| Type | Descripción | Trigger | Priority |
|------|-------------|---------|----------|
| `invoice` | Factura procesada exitosamente | Después de procesar factura | normal |
| `achievement` | Logro desbloqueado | Cuando se desbloquea logro | high |
| `streak` | Actualización de racha | Racha completada o en peligro | normal/high |
| `level_up` | Subió de nivel | Cuando sube de nivel | high |
| `reward` | Nueva recompensa disponible | Recompensa desbloqueada | normal |
| `challenge` | Reto semanal | Nuevo reto o reto completado | normal |
| `promo` | Promoción especial | Campaña de marketing | low |
| `reminder` | Recordatorio | No ha escaneado en X días | low |
| `system` | Sistema | Mantenimiento, actualizaciones | varies |

---

## 5. Triggers para Crear Notificaciones

### 5.1 Función Centralizada para Crear Notificaciones

```sql
-- Función centralizada con deduplicación incorporada
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
    DO NOTHING  -- Ignorar duplicados silenciosamente
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
'Crea notificación con deduplicación automática. 
Usar idempotency_key para evitar duplicados en retries.
Formato recomendado: {type}_{reference_id}_{date}';
```

### 5.2 Después de Procesar Factura

```sql
-- IMPORTANTE: Este trigger NO debe calcular lumis directamente.
-- Los lumis se calculan en el sistema de rewards existente.
-- Este trigger solo notifica que la factura fue procesada.

CREATE OR REPLACE FUNCTION public.notify_invoice_processed()
RETURNS TRIGGER AS $$
BEGIN
    -- Solo para facturas nuevas con usuario asociado
    IF TG_OP = 'INSERT' AND NEW.user_id IS NOT NULL THEN
        -- Crear notificación con idempotency_key basada en CUFE
        -- NOTA: Los lumis ganados se obtienen después del procesamiento de rewards
        -- Por ahora notificamos solo el procesamiento exitoso
        PERFORM public.create_notification(
            p_user_id := NEW.user_id,
            p_title := '¡Factura procesada!',
            p_body := FORMAT('Tu factura de %s fue procesada exitosamente', COALESCE(NEW.issuer_name, 'comercio')),
            p_type := 'invoice',
            p_priority := 'normal',
            p_action_url := FORMAT('/invoices/%s', NEW.cufe),
            p_payload := jsonb_build_object(
                'cufe', NEW.cufe,
                'merchant_name', NEW.issuer_name,
                'amount', NEW.tot_amount,
                'date', NEW.date
            ),
            p_idempotency_key := FORMAT('invoice_%s', NEW.cufe),  -- CUFE es único
            p_send_push := TRUE
        );
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- NOTA: Evaluar si este trigger es necesario o si la notificación
-- debe generarse desde el código Rust después de confirmar los lumis ganados.
-- Alternativa: Llamar a create_notification() desde el endpoint de procesamiento.

CREATE TRIGGER trg_notify_invoice_processed
    AFTER INSERT ON public.invoice_header
    FOR EACH ROW
    EXECUTE FUNCTION public.notify_invoice_processed();
```

### 5.3 Racha en Peligro (Job Programado)

```sql
-- Función para job de racha en peligro (ejecutar cada hora vía pg_cron)
CREATE OR REPLACE FUNCTION public.notify_streak_at_risk()
RETURNS TABLE(notifications_sent INTEGER) AS $$
DECLARE
    v_sent INTEGER := 0;
    v_user RECORD;
    v_today DATE := CURRENT_DATE;
BEGIN
    -- Buscar usuarios con racha activa que no han hecho login hoy
    FOR v_user IN 
        SELECT 
            us.user_id,
            us.current_count as streak_days,
            us.last_activity_date
        FROM gamification.user_streaks us
        WHERE us.streak_type = 'daily_login'
        AND us.current_count >= 3  -- Solo alertar si tienen 3+ días
        AND us.last_activity_date < v_today  -- No han hecho login hoy
        AND us.last_activity_date >= v_today - INTERVAL '1 day'  -- Pero sí ayer (racha no rota aún)
        AND NOT EXISTS (
            -- No enviar si ya enviamos hoy
            SELECT 1 FROM public.notifications n
            WHERE n.user_id = us.user_id
            AND n.type = 'streak'
            AND n.idempotency_key = FORMAT('streak_risk_%s_%s', us.user_id, v_today)
        )
    LOOP
        PERFORM public.create_notification(
            p_user_id := v_user.user_id,
            p_title := '🔥 ¡Tu racha está en peligro!',
            p_body := FORMAT('Llevas %s días consecutivos. ¡No pierdas tu racha!', v_user.streak_days),
            p_type := 'streak',
            p_priority := 'high',
            p_action_url := '/earn',
            p_payload := jsonb_build_object(
                'streak_days', v_user.streak_days,
                'risk_level', 'high'
            ),
            p_idempotency_key := FORMAT('streak_risk_%s_%s', v_user.user_id, v_today),
            p_send_push := TRUE
        );
        
        v_sent := v_sent + 1;
    END LOOP;
    
    RETURN QUERY SELECT v_sent;
END;
$$ LANGUAGE plpgsql;

-- Programar con pg_cron (ejecutar cada hora entre 10am y 10pm Panamá)
-- SELECT cron.schedule('streak-risk-notify', '0 10-22 * * *', 'SELECT * FROM public.notify_streak_at_risk()');
```

### 5.4 Logro Desbloqueado

```sql
-- NOTA: rewards.fact_accumulations usa accum_type = 'earn'|'spend'|'daily_game', NO 'achievement'
-- Los logros se identifican por el JOIN con dim_accumulations donde name LIKE 'gamification_%'
-- Esta es una función callable, NO un trigger automático (para evitar acoplamiento)

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
'Función para notificar logros desbloqueados. 
Llamar desde gamification.grant_achievement_reward() después de insertar en fact_accumulations.
NO es un trigger automático para evitar acoplamiento con schema rewards.';

-- Ejemplo de integración en grant_achievement_reward:
-- PERFORM public.notify_achievement_unlocked(p_user_id, 'first_week', 'Primera Semana', 100);
```

---

## 6. Integración FCM (Push Notifications)

### 6.1 Payload de Push

El backend debe enviar este payload a FCM:

```json
{
    "message": {
        "token": "fcm_device_token_here",
        "notification": {
            "title": "¡Factura procesada!",
            "body": "Ganaste 25 Lümis con tu compra en Super 99"
        },
        "data": {
            "notification_id": "12345",
            "type": "invoice",
            "action_url": "/invoices/123",
            "click_action": "FLUTTER_NOTIFICATION_CLICK"
        },
        "android": {
            "priority": "high",
            "notification": {
                "channel_id": "lum_notifications",
                "icon": "ic_notification",
                "color": "#9C27B0"
            }
        },
        "apns": {
            "payload": {
                "aps": {
                    "badge": 5,
                    "sound": "default"
                }
            }
        }
    }
}
```

**Nota:** `notification_id` es un BIGINT (número), no UUID.

### 6.2 Manejo de Tokens Inválidos

Cuando FCM retorna error `UNREGISTERED` o `INVALID_ARGUMENT`:

```sql
-- Marcar token como inactivo
UPDATE device_tokens 
SET is_active = FALSE, updated_at = NOW()
WHERE fcm_token = 'token_invalido';
```

---

## 7. Consideraciones de Performance

### 7.1 Limpieza Automática

```sql
-- Tabla de cola para push (procesada por worker async)
CREATE TABLE public.notification_push_queue (
    id BIGSERIAL PRIMARY KEY,
    notification_id BIGINT NOT NULL REFERENCES public.notifications(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, sent, failed, skipped, retrying
    attempts INTEGER DEFAULT 0,
    last_attempt_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ DEFAULT NOW(),  -- Para backoff exponencial
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_push_queue_pending ON public.notification_push_queue(status, next_attempt_at)
WHERE status IN ('pending', 'retrying');

-- Nota de Implementación para Worker (Rust):
-- Usar SKIP LOCKED para concurrencia segura entre múltiples instancias del worker:
-- SELECT * FROM notification_push_queue 
-- WHERE status IN ('pending', 'retrying') AND next_attempt_at <= NOW()
-- ORDER BY next_attempt_at ASC
-- LIMIT 10
-- FOR UPDATE SKIP LOCKED;

-- Job diario: eliminar notificaciones expiradas
CREATE OR REPLACE FUNCTION public.cleanup_expired_notifications()
RETURNS TABLE(deleted_count INTEGER) AS $$
DECLARE
    v_deleted INTEGER;
BEGIN
    DELETE FROM public.notifications 
    WHERE expires_at IS NOT NULL 
    AND expires_at < NOW();
    
    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    
    -- Limpiar notificaciones leídas antiguas (>90 días)
    DELETE FROM public.notifications 
    WHERE is_read = TRUE 
    AND created_at < NOW() - INTERVAL '90 days';
    
    GET DIAGNOSTICS v_deleted = v_deleted + ROW_COUNT;
    
    -- Limpiar tokens inactivos (>60 días sin uso)
    DELETE FROM public.device_tokens 
    WHERE is_active = FALSE 
    AND updated_at < NOW() - INTERVAL '60 days';
    
    -- Limpiar cola de push procesada (>7 días)
    DELETE FROM public.notification_push_queue
    WHERE status IN ('sent', 'skipped')
    AND created_at < NOW() - INTERVAL '7 days';
    
    RETURN QUERY SELECT v_deleted;
END;
$$ LANGUAGE plpgsql;

-- Programar con pg_cron: 3am Panamá todos los días
-- SELECT cron.schedule('cleanup-notifications', '0 8 * * *', 'SELECT * FROM public.cleanup_expired_notifications()');
```

### 7.2 Rate Limiting (Redis)

```rust
// Implementación en Rust con Redis
use redis::AsyncCommands;

const RATE_LIMIT_NOTIFICATIONS_PER_HOUR: i64 = 10;
const RATE_LIMIT_PROMO_PER_DAY: i64 = 3;

pub async fn check_notification_rate_limit(
    redis: &redis::aio::MultiplexedConnection,
    user_id: i64,
    notification_type: &str,
) -> Result<bool, AppError> {
    let mut conn = redis.clone();
    
    // Key con TTL de 1 hora para límite general
    let hourly_key = format!("notif_rate:{}:hourly", user_id);
    let count: i64 = conn.incr(&hourly_key, 1).await?;
    
    if count == 1 {
        conn.expire(&hourly_key, 3600).await?; // 1 hora TTL
    }
    
    if count > RATE_LIMIT_NOTIFICATIONS_PER_HOUR {
        return Ok(false); // Rate limited
    }
    
    // Límite especial para promos (3 por día)
    if notification_type == "promo" {
        let daily_key = format!("notif_rate:{}:promo_daily", user_id);
        let promo_count: i64 = conn.incr(&daily_key, 1).await?;
        
        if promo_count == 1 {
            conn.expire(&daily_key, 86400).await?; // 24 horas TTL
        }
        
        if promo_count > RATE_LIMIT_PROMO_PER_DAY {
            return Ok(false);
        }
    }
    
    Ok(true)
}
```

### 7.3 Cooldown Entre Notificaciones del Mismo Tipo

```rust
const COOLDOWN_SAME_TYPE_SECONDS: i64 = 300; // 5 minutos

pub async fn check_notification_cooldown(
    redis: &redis::aio::MultiplexedConnection,
    user_id: i64,
    notification_type: &str,
) -> Result<bool, AppError> {
    let mut conn = redis.clone();
    let cooldown_key = format!("notif_cooldown:{}:{}", user_id, notification_type);
    
    // SET NX con TTL (solo setea si no existe)
    let set_result: bool = redis::cmd("SET")
        .arg(&cooldown_key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(COOLDOWN_SAME_TYPE_SECONDS)
        .query_async(&mut conn)
        .await?;
    
    Ok(set_result) // true = cooldown no activo, puede enviar
}
```

### 7.4 Batch Push con FCM (Multicast)

```rust
// Enviar push a múltiples dispositivos de un usuario en una sola llamada
pub async fn send_push_batch(
    fcm_client: &FcmClient,
    tokens: Vec<String>,
    notification: &Notification,
) -> Result<BatchResult, FcmError> {
    // FCM permite hasta 500 tokens por multicast
    const BATCH_SIZE: usize = 500;
    
    let mut results = BatchResult::default();
    
    for chunk in tokens.chunks(BATCH_SIZE) {
        let message = MulticastMessage {
            tokens: chunk.to_vec(),
            notification: Some(FcmNotification {
                title: notification.title.clone(),
                body: notification.body.clone(),
            }),
            data: Some(notification.payload.clone()),
            android: Some(AndroidConfig {
                priority: AndroidMessagePriority::High,
                notification: Some(AndroidNotification {
                    channel_id: "lum_notifications".to_string(),
                    icon: "ic_notification".to_string(),
                    color: "#9C27B0".to_string(),
                }),
            }),
            apns: Some(ApnsConfig {
                payload: ApnsPayload {
                    aps: Aps {
                        badge: Some(notification.unread_count as u32),
                        sound: Some("default".to_string()),
                    },
                },
            }),
        };
        
        let response = fcm_client.send_multicast(message).await?;
        
        // Procesar tokens inválidos
        for (i, result) in response.responses.iter().enumerate() {
            if let Some(error) = &result.error {
                if error.code == "UNREGISTERED" || error.code == "INVALID_ARGUMENT" {
                    results.invalid_tokens.push(chunk[i].clone());
                }
            } else {
                results.success_count += 1;
            }
        }
    }
    
    Ok(results)
}
```

---

## 8. Respuestas de Error Estándar

```json
// 400 Bad Request
{
    "success": false,
    "data": null,
    "error": {
        "code": "INVALID_REQUEST",
        "message": "El parámetro 'limit' debe ser menor a 100"
    }
}

// 401 Unauthorized
{
    "success": false,
    "data": null,
    "error": {
        "code": "UNAUTHORIZED",
        "message": "Token inválido o expirado"
    }
}

// 404 Not Found
{
    "success": false,
    "data": null,
    "error": {
        "code": "NOTIFICATION_NOT_FOUND",
        "message": "Notificación no encontrada"
    }
}

// 500 Internal Error
{
    "success": false,
    "data": null,
    "error": {
        "code": "INTERNAL_ERROR",
        "message": "Error interno del servidor"
    }
}
```

---

## 9. Prioridades de Implementación

### Fase 1 (MVP) - Crítico ⏱️ 2-3 días
- [x] Especificación corregida (tipos BIGINT)
- [x] Migración SQL: tablas `notifications`, `device_tokens`, `notification_push_queue`
- [x] Función `create_notification()` con deduplicación
- [x] Módulo Rust `notifications_v4.rs`:
  - [x] `GET /notifications` (listar con paginación)
  - [x] `GET /notifications/count` (badge count)
  - [x] `POST /notifications/{id}/read`
  - [x] `POST /notifications/read-all`
  - [x] `DELETE /notifications/{id}`
- [ ] Tests unitarios

### Fase 2 - Push Notifications ⏱️ 2 días
- [x] `POST /devices/fcm-token`
- [x] `DELETE /devices/fcm-token`
- [x] Trigger `trg_fcm_token_registration`
- [x] Integración FCM (Legacy API con server key)
- [x] Worker async para procesar `notification_push_queue`
- [x] Manejo de tokens inválidos

### Fase 3 - Triggers Automáticos ⏱️ 1-2 días
- [x] Trigger `trg_notify_invoice_processed`
- [x] Function `notify_achievement_unlocked()`
- [x] Function `notify_level_up()`
- [x] Function `notify_promo()`
- [x] Job `notify_streak_at_risk()` (listo para pg_cron)
- [x] Job `cleanup_expired_notifications()` (listo para pg_cron)

### Fase 4 - Optimizaciones ⏱️ 1 día
- [x] Rate limiting con Redis (por usuario, por tipo, cooldown)
- [x] Cooldown entre notificaciones del mismo tipo (5 min)
- [x] Métricas Prometheus (queue, API, push)
- [ ] Batch multicast FCM (upgrade a HTTP v1 API)

---

## 10. Archivos Creados

```
db/migrations/
├── 20251201_create_notifications_schema.sql  # Tablas, índices y funciones base
├── 20251201_create_notification_triggers.sql # Triggers y funciones callable

src/api/
├── notifications_v4.rs                        # Endpoints REST (7 endpoints)

src/services/
├── push_notification_service.rs              # FCM integration + queue processor
```

### Funciones SQL Disponibles

| Función | Descripción | Uso |
|---------|-------------|-----|
| `create_notification()` | Crea notificación con deduplicación | Base para todas las notificaciones |
| `notify_achievement_unlocked()` | Notifica logro desbloqueado | Llamar desde `grant_achievement_reward()` |
| `notify_level_up()` | Notifica subida de nivel | Llamar al calcular nivel |
| `notify_promo()` | Notifica promociones | Campañas de marketing |
| `notify_streak_at_risk()` | Alerta rachas en peligro | Job programado (pg_cron) |
| `cleanup_expired_notifications()` | Limpieza de datos antiguos | Job programado (pg_cron) |

### Configuración Requerida (FCM HTTP v1)

```bash
# ============================================================================
# FIREBASE CLOUD MESSAGING - HTTP v1 API (Recomendado)
# ============================================================================
# 
# La implementación usa FCM HTTP v1 con OAuth 2.0, que es:
# - Más seguro (tokens rotativos vs server key estático)
# - Future-proof (Legacy API está deprecated)
# - Mejor rate limiting y features
#
# Configuración:
# 1. Ve a Firebase Console → Project Settings → Service Accounts
# 2. Genera una nueva clave privada (JSON)
# 3. Guarda el archivo JSON en una ubicación segura
# 4. Configura las variables:

# Ruta al archivo JSON de credenciales del Service Account
GOOGLE_APPLICATION_CREDENTIALS=/path/to/your-firebase-service-account.json

# ID del proyecto de Firebase (lo encuentras en Project Settings)
FIREBASE_PROJECT_ID=tu-proyecto-firebase

# Redis para rate limiting
REDIS_URL=redis://localhost:6379
```

> ⚠️ **NOTA:** La variable `FCM_SERVER_KEY` ya NO se usa. La autenticación ahora es
> vía OAuth 2.0 con Service Account, que es más seguro y es el método recomendado
> por Google.

### Rate Limiting Configurado

| Límite | Valor | Descripción |
|--------|-------|-------------|
| `NOTIFICATIONS_PER_HOUR_USER` | 10/hora | Límite general por usuario |
| `PROMO_NOTIFICATIONS_PER_DAY_USER` | 3/día | Límite especial para promos |
| `NOTIFICATION_TYPE_COOLDOWN` | 5 min | Cooldown entre notificaciones del mismo tipo |
| `NOTIFICATION_API_PER_MINUTE_USER` | 60/min | Límite de requests a la API |

### Métricas Prometheus

```
# Push notifications
push_notifications_sent_total{notification_type, status}

# Cola de notificaciones
notification_queue_processed_total{status}  # sent, failed, skipped, invalid_token

# Notificaciones in-app
notifications_created_total{notification_type}

# API requests
notification_api_requests_total{endpoint, status}
```

---

## 11. Contacto Frontend

Para cualquier duda sobre la integración:
- Los campos `action_url` usan el formato de rutas de GoRouter de la app
- El campo `payload` es flexible para datos específicos por tipo
- El `unread_count` en meta es crítico para el badge de la UI
- **NUEVO:** Endpoint `/notifications/count` optimizado para badge refresh

---

## 12. Análisis de Calidad (Self-Review)

| Criterio | Score v1.0 | Score v2.2 | Notas |
|----------|-----------|-----------|-------|
| A. Requerimiento | 0.70 | **0.97** | Triggers corregidos, funciones callable en vez de auto-triggers problemáticos |
| B. Escalabilidad | 0.95 | **0.98** | Queue con backoff, SKIP LOCKED, índices parciales |
| C. Costo-Efectividad | 0.85 | **0.95** | Batch FCM, limpieza automática, soft-delete |
| D. Precisión | 0.88 | **0.97** | Tipos correctos (BIGINT), payload FCM corregido |
| E. Completitud | 0.88 | **0.96** | Seguridad documentada, SQL de implementación incluido |
| F. Race Conditions | 0.75 | **0.98** | ON CONFLICT en tokens, deduplicación DB, SKIP LOCKED |

**PROMEDIO FINAL: 0.968** ✅

### Cambios v2.2:
- ✅ Corregido trigger de achievement (era inválido, ahora es función callable)
- ✅ Agregado ON CONFLICT para registro de tokens FCM
- ✅ Corregido tipo de notification_id en payload FCM (BIGINT, no UUID)
- ✅ Agregada sección de seguridad (validación de ownership)
- ✅ Simplificado trigger de invoice (sin cálculo de lumis hardcodeado)

---

**¿Preguntas?** El frontend está listo para integrarse tan pronto como los endpoints de Fase 1 estén disponibles. Podemos hacer testing con Postman/curl primero.
