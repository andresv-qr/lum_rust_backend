# 17 - Colección Postman

## Variables de Entorno

```json
{
  "base_url": "http://localhost:8000",
  "user_token": "{{user_jwt}}",
  "merchant_token": "{{merchant_jwt}}"
}
```

## Requests

### 1. User Login
```
POST {{base_url}}/api/v1/auth/login
Body: {"email": "user@test.com", "password": "pass"}
```

### 2. Get Offers
```
GET {{base_url}}/api/v1/rewards/offers
Headers: Authorization: Bearer {{user_token}}
```

### 3. Create Redemption
```
POST {{base_url}}/api/v1/rewards/redeem
Headers: Authorization: Bearer {{user_token}}
Body: {"offer_id": "uuid", "user_id": 12345}
```

### 4. Merchant Validate
```
POST {{base_url}}/api/v1/merchant/validate
Headers: Authorization: Bearer {{merchant_token}}
Body: {"code": "LUMS-XXXX-XXXX-XXXX"}
```

**Siguiente**: [18-sdk-examples.md](./18-sdk-examples.md)
