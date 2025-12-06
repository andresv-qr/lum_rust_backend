# 04 - API Endpoints para Usuarios

Base URL: `https://api.lumapp.org/api/v1/rewards`

## Endpoints Disponibles

| Método | Endpoint | Autenticación | Descripción |
|--------|----------|---------------|-------------|
| GET | `/offers` | ✅ JWT | Listar ofertas disponibles |
| POST | `/redeem` | ✅ JWT | Crear redención |
| GET | `/history` | ✅ JWT | Historial de redenciones |
| GET | `/stats` | ✅ JWT | Estadísticas del usuario |
| GET | `/redemptions/:id` | ✅ JWT | Detalles de redención |
| POST | `/cancel/:id` | ✅ JWT | Cancelar redención |
| GET | `/balance` | ✅ JWT | Balance actual |

Ver [16-ejemplos-frontend.md](./16-ejemplos-frontend.md) para código completo.
Ver [17-ejemplos-postman.md](./17-ejemplos-postman.md) para colección Postman.

**Siguiente**: [05-api-merchants.md](./05-api-merchants.md)
