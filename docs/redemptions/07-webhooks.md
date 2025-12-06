# 07 - Sistema de Webhooks

## Configuración

Agregar en tabla `merchants`:
```sql
UPDATE rewards.merchants
SET 
  webhook_url = 'https://merchant.com/webhook',
  webhook_secret = 'your-secret',
  webhook_events = ARRAY['redemption.created', 'redemption.confirmed'],
  webhook_enabled = true
WHERE merchant_id = 'uuid';
```

## Eventos Disponibles

- `redemption.created` - Nueva redención
- `redemption.confirmed` - Redención confirmada
- `redemption.expired` - Redención expiró
- `redemption.cancelled` - Redención cancelada

## Formato de Payload

```json
{
  "event": "redemption.confirmed",
  "timestamp": "2025-10-18T18:30:00Z",
  "data": {
    "redemption_id": "uuid",
    "redemption_code": "LUMS-...",
    "offer_name": "Café Americano",
    "confirmed_by": "Merchant Name"
  },
  "merchant_id": "uuid"
}
```

## Verificación de Firma

```javascript
const crypto = require('crypto');

function verifyWebhook(payload, signature, secret) {
  const hmac = crypto.createHmac('sha256', secret);
  hmac.update(payload);
  const expected = hmac.digest('hex');
  return signature === expected;
}
```

**Siguiente**: [08-push-notifications.md](./08-push-notifications.md)
