# 📱 Guía de Uso del Sistema de Notificaciones Lüm

> **Documento para:** Equipo de Negocio, Marketing y Operaciones  
> **Versión:** 1.0  
> **Fecha:** Diciembre 2025

---

## 📋 Índice

1. [¿Qué es el Sistema de Notificaciones?](#1-qué-es-el-sistema-de-notificaciones)
2. [Tipos de Notificaciones Disponibles](#2-tipos-de-notificaciones-disponibles)
3. [Flujo: ¿Cómo llega una notificación al usuario?](#3-flujo-cómo-llega-una-notificación-al-usuario)
4. [Notificaciones Automáticas (sin intervención)](#4-notificaciones-automáticas-sin-intervención)
5. [Notificaciones Manuales (Marketing/Operaciones)](#5-notificaciones-manuales-marketingoperaciones)
6. [Ejemplos Prácticos con SQL](#6-ejemplos-prácticos-con-sql)
7. [Métricas y Seguimiento](#7-métricas-y-seguimiento)
8. [Preguntas Frecuentes](#8-preguntas-frecuentes)

---

## 1. ¿Qué es el Sistema de Notificaciones?

El sistema de notificaciones de Lüm permite comunicarse con los usuarios de dos formas:

| Canal | Descripción | Cuándo se ve |
|-------|-------------|--------------|
| **In-App** | Notificación dentro de la app (bandeja de notificaciones) | Cuando el usuario abre la app |
| **Push** | Notificación del sistema operativo (Android/iOS) | Inmediatamente, aunque la app esté cerrada |

**Ambas se crean automáticamente** cuando se inserta un registro en la tabla `notifications`. El sistema se encarga de:
1. Guardar la notificación in-app
2. Encolarla para envío push
3. Enviarla a todos los dispositivos del usuario

---

## 2. Tipos de Notificaciones Disponibles

| Tipo | Uso | Prioridad Recomendada | Ejemplo |
|------|-----|----------------------|---------|
| `invoice` | Factura procesada | Normal | "Tu factura de Supermercado fue procesada" |
| `achievement` | Logro desbloqueado | Alta | "🏆 ¡Completaste 'Primera Semana'!" |
| `level_up` | Subida de nivel | Alta | "🎉 ¡Subiste al nivel 5: Explorador!" |
| `reward` | Lümis ganados | Normal | "Ganaste 50 Lümis por tu compra" |
| `streak` | Racha en riesgo/completada | Alta | "🔥 ¡Tu racha de 7 días está en peligro!" |
| `promo` | Promociones y ofertas | Baja | "20% de descuento en tu próxima redención" |
| `system` | Avisos del sistema | Normal | "Actualiza la app para nuevas funciones" |
| `challenge` | Retos y misiones | Normal | "Nuevo reto disponible: Escanea 3 facturas" |
| `reminder` | Recordatorios | Baja | "No olvides escanear tus facturas de hoy" |

### Prioridades

| Prioridad | Comportamiento en el teléfono |
|-----------|------------------------------|
| `urgent` | Sonido + Vibración + Pantalla encendida |
| `high` | Sonido + Vibración |
| `normal` | Sonido suave |
| `low` | Silenciosa (solo badge) |

---

## 3. Flujo: ¿Cómo llega una notificación al usuario?

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        FLUJO DE NOTIFICACIONES                              │
└─────────────────────────────────────────────────────────────────────────────┘

     ┌──────────────┐
     │   ORIGEN     │
     │              │
     │ • Trigger DB │ ──────┐
     │ • SQL Manual │       │
     │ • API Rust   │       │
     └──────────────┘       │
                            ▼
                   ┌─────────────────┐
                   │  TABLA          │
                   │  notifications  │
                   │                 │
                   │ (se inserta     │
                   │  el registro)   │
                   └────────┬────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
     ┌─────────────────┐        ┌─────────────────────┐
     │   IN-APP        │        │   COLA DE PUSH      │
     │   (Inmediato)   │        │   notification_     │
     │                 │        │   push_queue        │
     │ Usuario ve en   │        └──────────┬──────────┘
     │ su bandeja de   │                   │
     │ notificaciones  │                   ▼
     └─────────────────┘        ┌─────────────────────┐
                                │   WORKER            │
                                │   (cada 5 seg)      │
                                │                     │
                                │ Procesa la cola     │
                                │ y envía a FCM       │
                                └──────────┬──────────┘
                                           │
                                           ▼
                                ┌─────────────────────┐
                                │   FIREBASE (FCM)    │
                                │                     │
                                │ Envía a dispositivo │
                                └──────────┬──────────┘
                                           │
                                           ▼
                                ┌─────────────────────┐
                                │   📱 DISPOSITIVO    │
                                │                     │
                                │ • Android           │
                                │ • iOS               │
                                │ • Web               │
                                └─────────────────────┘
```

### Tiempo de entrega

| Etapa | Tiempo aproximado |
|-------|-------------------|
| Inserción en `notifications` | Instantáneo |
| Visible en app (in-app) | Instantáneo |
| Procesamiento del worker | 0-5 segundos |
| Entrega push (FCM → dispositivo) | 0-2 segundos |
| **Total** | **< 10 segundos** |

---

## 4. Notificaciones Automáticas (sin intervención)

Estas notificaciones se generan **automáticamente** por el sistema:

### 4.1 Factura Procesada
- **Trigger:** Cuando se inserta una factura en `invoice_header`
- **Destinatario:** El dueño de la factura (`user_id`)
- **No requiere acción manual**

### 4.2 Logro Desbloqueado
- **Trigger:** Cuando el sistema de gamificación otorga un achievement
- **Llamada automática desde:** `gamification.grant_achievement_reward()`

### 4.3 Subida de Nivel
- **Trigger:** Cuando el usuario acumula suficientes XP
- **Llamada automática desde:** Sistema de niveles

### 4.4 Racha en Riesgo
- **Trigger:** Job programado (cada hora, 10am-10pm)
- **Destinatarios:** Usuarios con racha >= 3 días que no han entrado hoy

---

## 5. Notificaciones Manuales (Marketing/Operaciones)

Para enviar notificaciones manuales (promociones, avisos, etc.), hay dos opciones:

### Opción A: Usar la función SQL `notify_promo()` (Recomendado)

```sql
-- Enviar promoción a UN usuario específico
SELECT public.notify_promo(
    p_user_id := 12345,                              -- ID del usuario
    p_title := '🎁 ¡Oferta especial para ti!',       -- Título (max 200 chars)
    p_body := 'Obtén 2x Lümis en tu próxima factura hasta el viernes',
    p_action_url := '/offers/double-lumis',          -- Deep link en la app
    p_image_url := 'https://cdn.lum.app/promos/2x.png',  -- Imagen (opcional)
    p_campaign_id := 'PROMO_DIC_2025_001',           -- ID de campaña (para tracking)
    p_expires_at := '2025-12-15 23:59:59'::TIMESTAMPTZ  -- Fecha de expiración
);
```

### Opción B: Insertar directamente con `create_notification()`

```sql
SELECT public.create_notification(
    p_user_id := 12345,
    p_title := 'Aviso importante',
    p_body := 'Actualizamos nuestros términos de servicio',
    p_type := 'system',
    p_priority := 'normal',
    p_action_url := '/legal/terms',
    p_image_url := NULL,
    p_payload := '{"version": "2.0"}'::JSONB,
    p_idempotency_key := 'terms_update_v2_12345',  -- Evita duplicados
    p_expires_at := NULL,
    p_send_push := TRUE  -- FALSE si solo quieres in-app
);
```

---

## 6. Ejemplos Prácticos con SQL

### 6.1 Enviar promoción a TODOS los usuarios activos

```sql
-- Promoción masiva: Doble Lümis el fin de semana
INSERT INTO public.notifications (
    user_id, title, body, type, priority, 
    action_url, idempotency_key, expires_at
)
SELECT 
    id as user_id,
    '🎉 ¡Doble Lümis este finde!' as title,
    'Escanea facturas sábado y domingo y gana el doble' as body,
    'promo' as type,
    'normal' as priority,
    '/earn' as action_url,
    'promo_double_weekend_' || id as idempotency_key,  -- Único por usuario
    '2025-12-08 23:59:59'::TIMESTAMPTZ as expires_at
FROM public.dim_users
WHERE is_active = TRUE
AND id NOT IN (
    -- Excluir usuarios que ya recibieron esta promo
    SELECT user_id FROM public.notifications 
    WHERE idempotency_key LIKE 'promo_double_weekend_%'
);

-- Ver cuántos se enviaron
SELECT COUNT(*) as usuarios_notificados 
FROM public.notifications 
WHERE idempotency_key LIKE 'promo_double_weekend_%';
```

### 6.2 Notificar a usuarios de un segmento específico

```sql
-- Notificar a usuarios "Premium" (con más de 1000 Lümis)
SELECT public.notify_promo(
    p_user_id := u.id,
    p_title := '⭐ Beneficio exclusivo Premium',
    p_body := 'Por ser cliente VIP, tienes acceso anticipado a nuevas ofertas',
    p_action_url := '/offers/premium',
    p_campaign_id := 'VIP_EARLY_ACCESS_DIC'
)
FROM public.dim_users u
JOIN gamification.user_balances b ON u.id = b.user_id
WHERE b.current_balance >= 1000;
```

### 6.3 Recordatorio a usuarios inactivos

```sql
-- Usuarios que no han abierto la app en 7 días
SELECT public.create_notification(
    p_user_id := u.id,
    p_title := '¡Te extrañamos! 👋',
    p_body := 'Tienes Lümis esperándote. Escanea una factura hoy.',
    p_type := 'reminder',
    p_priority := 'low',
    p_action_url := '/earn',
    p_idempotency_key := 'reengagement_7d_' || u.id || '_' || CURRENT_DATE,
    p_send_push := TRUE
)
FROM public.dim_users u
WHERE u.last_login_at < NOW() - INTERVAL '7 days'
AND u.is_active = TRUE;
```

### 6.4 Anuncio de mantenimiento programado

```sql
-- Aviso de sistema a TODOS los usuarios
SELECT public.create_notification(
    p_user_id := id,
    p_title := '🔧 Mantenimiento programado',
    p_body := 'El domingo 8 de diciembre de 2-4am habrá mantenimiento. La app no estará disponible.',
    p_type := 'system',
    p_priority := 'high',
    p_action_url := NULL,
    p_idempotency_key := 'maintenance_20251208_' || id,
    p_send_push := TRUE
)
FROM public.dim_users
WHERE is_active = TRUE;
```

---

## 7. Métricas y Seguimiento

### 7.1 Ver notificaciones enviadas hoy

```sql
SELECT 
    type,
    COUNT(*) as total,
    COUNT(*) FILTER (WHERE is_read) as leidas,
    ROUND(100.0 * COUNT(*) FILTER (WHERE is_read) / COUNT(*), 1) as tasa_lectura
FROM public.notifications
WHERE created_at >= CURRENT_DATE
GROUP BY type
ORDER BY total DESC;
```

### 7.2 Estado de la cola de push

```sql
SELECT 
    status,
    COUNT(*) as cantidad,
    MAX(created_at) as ultimo
FROM public.notification_push_queue
GROUP BY status;
```

### 7.3 Tokens de dispositivo por plataforma

```sql
SELECT 
    platform,
    COUNT(*) as dispositivos_activos
FROM public.device_tokens
WHERE is_active = TRUE
GROUP BY platform;
```

### 7.4 Usuarios sin token push (no recibirán push)

```sql
SELECT COUNT(DISTINCT u.id) as usuarios_sin_push
FROM public.dim_users u
LEFT JOIN public.device_tokens dt ON u.id = dt.user_id AND dt.is_active = TRUE
WHERE u.is_active = TRUE
AND dt.id IS NULL;
```

---

## 8. Preguntas Frecuentes

### ❓ ¿Qué pasa si envío la misma notificación dos veces?

**R:** Si usas `idempotency_key`, el sistema ignora duplicados automáticamente. Esto es seguro para reintentos.

```sql
-- Ejemplo: Esto solo crea UNA notificación aunque se ejecute 10 veces
SELECT public.notify_promo(
    p_user_id := 123,
    p_title := 'Promoción única',
    p_body := 'Solo la verás una vez',
    p_campaign_id := 'UNICA_123'  -- Este ID previene duplicados
);
```

---

### ❓ ¿Puedo enviar solo in-app sin push?

**R:** Sí, usa `p_send_push := FALSE`:

```sql
SELECT public.create_notification(
    p_user_id := 123,
    p_title := 'Solo para la bandeja',
    p_body := 'No enviar push',
    p_type := 'system',
    p_send_push := FALSE  -- Solo in-app
);
```

---

### ❓ ¿Cómo sé si un usuario recibió el push?

**R:** Revisa la cola de push:

```sql
SELECT 
    q.status,
    q.attempts,
    q.error_message,
    n.title
FROM public.notification_push_queue q
JOIN public.notifications n ON q.notification_id = n.id
WHERE n.user_id = 12345
ORDER BY q.created_at DESC
LIMIT 10;
```

| Status | Significado |
|--------|-------------|
| `sent` | Entregado a FCM exitosamente |
| `failed` | Falló después de 3 intentos |
| `skipped` | Usuario sin token de dispositivo |
| `pending` | En espera de procesamiento |
| `retrying` | Reintentando con backoff |

---

### ❓ ¿Puedo programar una notificación para el futuro?

**R:** No directamente, pero puedes usar `expires_at` junto con un job de pg_cron:

```sql
-- Crear la notificación ahora pero que expire en 24h
SELECT public.notify_promo(
    p_user_id := 123,
    p_title := 'Oferta de 24 horas',
    p_body := 'Esta oferta desaparece mañana',
    p_expires_at := NOW() + INTERVAL '24 hours'
);
```

Para notificaciones programadas verdaderas, contacta al equipo de desarrollo.

---

### ❓ ¿Cuántas notificaciones puedo enviar?

**R:** Límites actuales:

| Límite | Valor |
|--------|-------|
| Notificaciones por usuario por hora | 10 |
| Promos por usuario por día | 3 |
| Batch por ejecución SQL | Sin límite (usa transacciones) |

---

### ❓ ¿Cómo elimino una notificación enviada por error?

**R:** Puedes marcarla como "dismissed" (el usuario no la verá):

```sql
UPDATE public.notifications
SET is_dismissed = TRUE
WHERE idempotency_key = 'CAMPAIGN_ERRONEA_123';
```

---

## 📞 Contacto

Para soporte técnico o nuevos requerimientos de notificaciones:
- **Slack:** #backend-team
- **Email:** backend@lum.app

---

*Documento generado automáticamente. Última actualización: Diciembre 2025*
