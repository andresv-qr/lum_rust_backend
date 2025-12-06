# 05 - API Endpoints para Merchants

Base URL: `https://api.lumapp.org/api/v1/merchant`

## Endpoints Disponibles

| Método | Endpoint | Autenticación | Descripción |
|--------|----------|---------------|-------------|
| POST | `/auth/login` | ❌ No | Login merchant |
| POST | `/validate` | ✅ Merchant JWT | Validar código |
| POST | `/confirm/:id` | ✅ Merchant JWT | Confirmar redención |
| GET | `/stats` | ✅ Merchant JWT | Estadísticas |
| GET | `/analytics` | ✅ Merchant JWT | Dashboard analytics |

**Ejemplo Login**:
```bash
curl -X POST https://api.lumapp.org/api/v1/merchant/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "merchant_name": "Starbucks Test",
    "api_key": "your-api-key"
  }'
```

**Siguiente**: [06-autenticacion.md](./06-autenticacion.md)
