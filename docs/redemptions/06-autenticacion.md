# 06 - Autenticación y Seguridad

## JWT para Usuarios

```json
{
  "sub": "12345",
  "email": "user@example.com",
  "role": "user",
  "exp": 1729281600,
  "iat": 1729195200
}
```

## JWT para Merchants

```json
{
  "sub": "merchant-uuid",
  "merchant_name": "Starbucks",
  "role": "merchant",
  "exp": 1729281600,
  "iat": 1729195200
}
```

## Rate Limiting

- IP: 100 req/min
- Usuario: 10 redenciones/día
- Merchant: 500 validaciones/min

**Siguiente**: [07-webhooks.md](./07-webhooks.md)
