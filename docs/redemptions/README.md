# 🎁 Sistema de Redención de Lümis - Documentación Completa

**Versión**: 3.0  
**Fecha**: 2025-10-18  
**Estado**: ✅ Producción Ready con Monitoreo, Notificaciones y Analytics

---

## 📚 Índice de Documentación

Esta documentación está dividida en módulos temáticos para facilitar la navegación y generación.

### 📖 Documentación por Temas

#### 1. Arquitectura y Conceptos
- **[01-arquitectura.md](./01-arquitectura.md)** - Stack tecnológico, módulos y componentes
- **[02-conceptos.md](./02-conceptos.md)** - Explicación de Lümis, ofertas, redenciones, merchants
- **[03-modelo-datos.md](./03-modelo-datos.md)** - Esquema de base de datos, tablas, relaciones

#### 2. API Endpoints
- **[04-api-usuarios.md](./04-api-usuarios.md)** - Endpoints para usuarios finales (7 endpoints)
- **[05-api-merchants.md](./05-api-merchants.md)** - Endpoints para comercios (5 endpoints)
- **[06-autenticacion.md](./06-autenticacion.md)** - JWT, autenticación, rate limiting

#### 3. Funcionalidades Avanzadas
- **[07-webhooks.md](./07-webhooks.md)** - Sistema de webhooks para merchants
- **[08-push-notifications.md](./08-push-notifications.md)** - Notificaciones push con FCM
- **[09-analytics.md](./09-analytics.md)** - Dashboard y métricas para merchants
- **[10-prometheus-metrics.md](./10-prometheus-metrics.md)** - Monitoreo con Prometheus/Grafana

#### 4. Operaciones y DevOps
- **[11-scheduled-jobs.md](./11-scheduled-jobs.md)** - Jobs automáticos (expiración, limpieza)
- **[12-deployment.md](./12-deployment.md)** - Guía de deployment y configuración
- **[13-troubleshooting.md](./13-troubleshooting.md)** - Solución de problemas comunes

#### 5. Testing y Desarrollo
- **[14-testing.md](./14-testing.md)** - Tests unitarios, integración y carga
- **[15-contributing.md](./15-contributing.md)** - Guía para contribuir al código

#### 6. Ejemplos e Integraciones
- **[16-ejemplos-frontend.md](./16-ejemplos-frontend.md)** - Ejemplos JavaScript/React
- **[17-ejemplos-postman.md](./17-ejemplos-postman.md)** - Colección Postman
- **[18-sdk-examples.md](./18-sdk-examples.md)** - SDKs para diferentes lenguajes

---

## 🚀 Inicio Rápido

### Para Desarrolladores Backend
1. Lee [01-arquitectura.md](./01-arquitectura.md) para entender el sistema
2. Revisa [03-modelo-datos.md](./03-modelo-datos.md) para el esquema DB
3. Consulta [12-deployment.md](./12-deployment.md) para configurar el entorno

### Para Desarrolladores Frontend
1. Lee [02-conceptos.md](./02-conceptos.md) para entender el flujo
2. Revisa [04-api-usuarios.md](./04-api-usuarios.md) para los endpoints
3. Consulta [16-ejemplos-frontend.md](./16-ejemplos-frontend.md) para código de ejemplo

### Para Merchants
1. Lee [02-conceptos.md](./02-conceptos.md) para entender cómo funciona
2. Revisa [05-api-merchants.md](./05-api-merchants.md) para integración
3. Configura [07-webhooks.md](./07-webhooks.md) para recibir notificaciones

### Para DevOps
1. Revisa [12-deployment.md](./12-deployment.md) para deployment
2. Configura [10-prometheus-metrics.md](./10-prometheus-metrics.md) para monitoreo
3. Consulta [13-troubleshooting.md](./13-troubleshooting.md) para debugging

---

## 🎯 Flujos Principales

### Flujo 1: Usuario Redime Oferta
```
Usuario → [GET /api/v1/rewards/offers] → Catálogo
Usuario → [POST /api/v1/rewards/redeem] → Crear Redención
Sistema → Genera QR Code
Sistema → Push Notification al Usuario
Sistema → Webhook al Merchant
Usuario → Muestra QR al Merchant
```

### Flujo 2: Merchant Valida y Confirma
```
Merchant → [POST /api/v1/merchant/validate] → Validar Código
Sistema → Verifica estado, expiración
Merchant → [POST /api/v1/merchant/confirm/:id] → Confirmar
Sistema → Actualiza balance del usuario
Sistema → Push Notification al Usuario
Sistema → Webhook de confirmación al Merchant
Sistema → Actualiza stats del Merchant
```

### Flujo 3: Analytics y Monitoreo
```
Merchant → [GET /api/v1/merchant/analytics] → Dashboard
Sistema → Prometheus scrape /monitoring/metrics
Grafana → Visualiza métricas en tiempo real
Alertmanager → Notifica errores críticos
```

---

## 📊 Stack Tecnológico

| Componente | Tecnología | Versión |
|------------|-----------|---------|
| Backend | Rust + Axum | 0.7.4 |
| Base de Datos | PostgreSQL | 14+ |
| Cache | Redis | 7+ |
| Autenticación | JWT (HS256) | - |
| Hashing | bcrypt | Cost 12 |
| QR Generation | qrcode crate | 0.14 |
| Monitoreo | Prometheus | 0.13 |
| Visualización | Grafana | 10+ |
| Push Notifications | Firebase FCM | - |
| Webhooks | HMAC-SHA256 | - |
| Scheduled Jobs | tokio-cron | 0.10 |

---

## 🔗 Enlaces Útiles

- **Servidor de Producción**: `https://api.lumapp.org`
- **Base de Datos**: `dbmain.lumapp.org`
- **Prometheus**: `http://localhost:8000/monitoring/metrics`
- **Grafana**: `http://grafana.lumapp.org` (si está configurado)
- **Repositorio**: [Interno]

---

## 📞 Contacto y Soporte

**Backend Team**
- Lead: [Nombre]
- Email: backend@lumapp.org

**DevOps Team**
- Lead: [Nombre]
- Email: devops@lumapp.org

**Soporte Técnico**
- Email: soporte@lumapp.org
- Slack: #lumis-redemption

---

## 📝 Notas de Versión

### v3.0 (2025-10-18)
- ✅ Sistema completo de webhooks para merchants
- ✅ Push notifications con FCM
- ✅ Analytics dashboard para merchants
- ✅ Métricas de Prometheus completas
- ✅ Rate limiting con Redis
- ✅ Scheduled jobs (expiración automática)
- ✅ Tests unitarios y de integración
- ✅ Documentación modular por temas

### v2.0 (2025-10-18)
- ✅ Sistema de autenticación para merchants
- ✅ Validación y confirmación de redenciones
- ✅ Balance calculation con triggers
- ✅ QR code generation
- ✅ Endpoints de usuario completados

### v1.0 (2025-10-15)
- ✅ Sistema básico de redenciones
- ✅ Catálogo de ofertas
- ✅ Creación de redenciones

---

## 📄 Licencia

© 2025 Lümis App. Todos los derechos reservados.
Documentación interna - No distribuir.
