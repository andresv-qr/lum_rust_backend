# 08 - Push Notifications (FCM)

## Configuración

Variables de entorno:
```bash
FCM_SERVER_KEY=your-fcm-server-key
FCM_ENDPOINT=https://fcm.googleapis.com/fcm/send
```

## Eventos Notificados

1. **Redención Creada**
   - Título: "🎁 Nueva redención creada"
   - Body: "Muestra el código {code} al comercio"

2. **Redención Confirmada**
   - Título: "¡Redención confirmada!"
   - Body: "Tu redención de {offer} fue confirmada"

3. **Redención por Expirar**
   - Título: "⏰ Tu redención expira pronto"
   - Body: "{offer} expira en {minutes} minutos"

## Tabla de Dispositivos

```sql
CREATE TABLE public.user_devices (
    device_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    fcm_token TEXT NOT NULL,
    device_type TEXT,
    is_active BOOLEAN DEFAULT true
);
```

**Siguiente**: [09-analytics.md](./09-analytics.md)
