# 01 - Arquitectura del Sistema

**Tema**: Stack tecnológico, componentes y módulos  
**Versión**: 3.0  
**Fecha**: 2025-10-18

---

## 🏗️ Stack Tecnológico

### Backend

```
┌─────────────────────────────────────────────────────────────┐
│  Backend: Rust + Axum Web Framework                         │
│  Puerto: 8000                                                │
│  Runtime: Tokio (async)                                      │
└─────────────────────────────────────────────────────────────┘
```

**Características**:
- **Performance**: Rust ofrece rendimiento nativo sin garbage collection
- **Safety**: Type system previene errores comunes (null pointers, race conditions)
- **Concurrency**: Async/await nativo con Tokio para alta concurrencia
- **Memory Efficiency**: Zero-cost abstractions, bajo consumo de memoria

### Base de Datos

```
┌─────────────────────────────────────────────────────────────┐
│  PostgreSQL 14+                                              │
│  Host: dbmain.lumapp.org                                     │
│  Database: tfactu                                            │
│  Schema: rewards                                             │
└─────────────────────────────────────────────────────────────┘
```

**Características**:
- **ACID Compliance**: Transacciones seguras
- **Triggers**: Actualización automática de balance
- **JSON Support**: Campos JSONB para datos flexibles
- **Indices**: Optimización de queries complejos

### Cache y Rate Limiting

```
┌─────────────────────────────────────────────────────────────┐
│  Redis 7+                                                    │
│  Propósito: Cache, Sessions, Rate Limiting                   │
└─────────────────────────────────────────────────────────────┘
```

**Uso**:
- Cache de ofertas activas (TTL: 5 min)
- Cache de balance de usuarios (TTL: 30 seg)
- Rate limiting distribuido
- Session storage para merchants

### Monitoreo y Observabilidad

```
┌─────────────────────────────────────────────────────────────┐
│  Prometheus + Grafana                                        │
│  Endpoint: /monitoring/metrics                               │
│  Formato: Prometheus text format                             │
└─────────────────────────────────────────────────────────────┘
```

**Métricas Capturadas**:
- HTTP requests (latency, throughput, errors)
- Database queries (duration, connections)
- Cache hit/miss rates
- Business metrics (redemptions, confirmations)
- Custom redemption metrics

---

## 📦 Arquitectura de Módulos

### Estructura del Proyecto

```
lum_rust_ws/
├── src/
│   ├── main.rs                    → Entry point
│   ├── lib.rs                     → App builder
│   ├── state.rs                   → AppState global
│   │
│   ├── domains/                   → Lógica de negocio por dominio
│   │   └── rewards/
│   │       ├── mod.rs
│   │       ├── models.rs          → DTOs y estructuras
│   │       ├── offer_service.rs   → Servicio de ofertas
│   │       ├── redemption_service.rs  → Servicio de redenciones
│   │       └── qr_generator.rs    → Generación de QR codes
│   │
│   ├── api/                       → Handlers HTTP
│   │   ├── rewards/
│   │   │   ├── mod.rs
│   │   │   ├── offers.rs          → GET /api/v1/rewards/offers
│   │   │   ├── redeem.rs          → POST /api/v1/rewards/redeem
│   │   │   ├── history.rs         → GET /api/v1/rewards/history
│   │   │   ├── user.rs            → GET /api/v1/rewards/stats
│   │   │   └── cancel.rs          → POST /api/v1/rewards/cancel/:id
│   │   │
│   │   └── merchant/
│   │       ├── mod.rs
│   │       ├── auth.rs            → POST /api/v1/merchant/auth/login
│   │       ├── validate.rs        → POST /api/v1/merchant/validate
│   │       ├── stats.rs           → GET /api/v1/merchant/stats
│   │       └── analytics.rs       → GET /api/v1/merchant/analytics
│   │
│   ├── middleware/                → Middlewares de Axum
│   │   ├── mod.rs
│   │   └── auth.rs                → JWT validation
│   │
│   ├── services/                  → Servicios compartidos
│   │   ├── mod.rs
│   │   ├── push_notification_service.rs  → FCM integration
│   │   ├── webhook_service.rs            → Webhooks a merchants
│   │   ├── rate_limiter_service.rs       → Rate limiting
│   │   └── scheduled_jobs_service.rs     → Cron jobs
│   │
│   ├── observability/             → Monitoreo
│   │   ├── mod.rs
│   │   ├── metrics.rs             → Prometheus metrics
│   │   ├── middleware.rs          → Metrics middleware
│   │   └── endpoints.rs           → /monitoring/metrics handler
│   │
│   └── monitoring/                → Health checks
│       ├── mod.rs
│       └── endpoints.rs           → /monitoring/health
│
├── tests/                         → Tests
│   └── redemption_system_tests.rs
│
├── docs/                          → Documentación
│   └── redemptions/
│       ├── README.md
│       ├── 01-arquitectura.md
│       └── ...
│
├── Cargo.toml                     → Dependencias
└── .env                           → Configuración
```

---

## 🔄 Flujo de Request HTTP

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. CLIENT REQUEST                                                    │
│    POST /api/v1/rewards/redeem                                      │
│    Authorization: Bearer <JWT>                                       │
│    Body: { offer_id, user_id }                                      │
└─────────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────────┐
│ 2. AXUM ROUTER                                                       │
│    - Parse request                                                   │
│    - Extract headers                                                 │
│    - Route to handler                                                │
└─────────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────────┐
│ 3. MIDDLEWARE CHAIN                                                  │
│    a) metrics_middleware → Record HTTP metrics                       │
│    b) cors_middleware → Handle CORS                                  │
│    c) extract_current_user → Validate JWT, extract claims           │
│    d) rate_limit_middleware → Check rate limits (Redis)              │
└─────────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────────┐
│ 4. API HANDLER                                                       │
│    rewards::redeem::create_redemption()                             │
│    - Validate input                                                  │
│    - Call business logic                                             │
└─────────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────────┐
│ 5. BUSINESS LOGIC (RedemptionService)                               │
│    - Validate offer (OfferService)                                  │
│    - Check user balance (DB query)                                  │
│    - Generate QR code (QrGenerator)                                 │
│    - Create redemption (DB transaction)                             │
│    - Update balance (DB trigger)                                    │
└─────────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────────┐
│ 6. SIDE EFFECTS (Async, non-blocking)                               │
│    - Push notification (FCM)                                         │
│    - Webhook to merchant (HTTP POST)                                │
│    - Record metrics (Prometheus)                                     │
└─────────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────────┐
│ 7. RESPONSE                                                          │
│    200 OK                                                            │
│    {                                                                 │
│      "redemption_id": "...",                                        │
│      "redemption_code": "LUMS-...",                                 │
│      "qr_landing_url": "...",                                       │
│      "expires_at": "2025-10-18T19:00:00Z",                         │
│      "new_balance": 945                                             │
│    }                                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 🔐 Capas de Seguridad

### 1. Network Layer
- **HTTPS Only**: TLS 1.3 en producción
- **Firewall**: Solo puertos 443 (HTTPS) y 8000 (backend) abiertos
- **VPN**: Acceso a base de datos solo desde VPN

### 2. Application Layer
- **JWT Authentication**: Tokens firmados con HS256
- **Rate Limiting**: Redis-based distributed rate limiting
- **Input Validation**: Validación con `validator` crate
- **SQL Injection Prevention**: Prepared statements con sqlx

### 3. Data Layer
- **Encrypted at Rest**: PostgreSQL con encryption
- **Password Hashing**: bcrypt con cost factor 12
- **API Key Storage**: Hashed en base de datos
- **Webhook Signatures**: HMAC-SHA256 para verificación

### 4. Observability Layer
- **Audit Logging**: Todas las confirmaciones registradas
- **Error Tracking**: Logs estructurados con tracing
- **Metrics**: Prometheus para detectar anomalías
- **Alerting**: Alertas automáticas en errores críticos

---

## 📊 Componentes de Infraestructura

```
┌───────────────────────────────────────────────────────────────────┐
│                        LOAD BALANCER                               │
│                        (Nginx/Caddy)                               │
└───────────────────────────────────────────────────────────────────┘
                            │
                            ↓
        ┌───────────────────┴───────────────────┐
        │                                       │
┌───────▼──────────┐                   ┌───────▼──────────┐
│  Rust Backend    │                   │  Rust Backend    │
│  Instance 1      │                   │  Instance 2      │
│  Port 8000       │                   │  Port 8001       │
└──────┬───────────┘                   └──────┬───────────┘
       │                                      │
       └──────────────┬───────────────────────┘
                      │
         ┌────────────┴────────────┐
         │                         │
┌────────▼─────────┐      ┌───────▼────────┐
│  PostgreSQL      │      │  Redis         │
│  Primary         │      │  Cache/Limits  │
│  Port 5432       │      │  Port 6379     │
└──────────────────┘      └────────────────┘
         │
         │ (Replication)
         ↓
┌──────────────────┐
│  PostgreSQL      │
│  Replica (Read)  │
│  Port 5433       │
└──────────────────┘
```

### Escalabilidad Horizontal

**Backend**:
- Stateless design permite múltiples instancias
- Session storage en Redis (compartido)
- Load balancing con round-robin o least-connections

**Base de Datos**:
- Primary para writes
- Replicas para reads
- Connection pooling (max 20 conexiones por instancia)

**Cache**:
- Redis cluster para alta disponibilidad
- Replicación automática
- Failover con Sentinel

---

## 🚀 Performance Characteristics

### Latency Targets

| Operación | P50 | P95 | P99 |
|-----------|-----|-----|-----|
| GET /offers | < 50ms | < 100ms | < 200ms |
| POST /redeem | < 100ms | < 250ms | < 500ms |
| POST /validate | < 50ms | < 100ms | < 150ms |
| POST /confirm | < 150ms | < 300ms | < 600ms |
| GET /analytics | < 200ms | < 500ms | < 1000ms |

### Throughput

- **Sustained**: 1,000 req/s por instancia
- **Peak**: 2,500 req/s por instancia
- **Concurrent Users**: 10,000+ simultáneos

### Resource Usage

- **Memory**: ~200MB por instancia (base)
- **CPU**: ~25% en idle, ~80% en peak
- **Disk I/O**: Minimal (PostgreSQL handles storage)
- **Network**: ~10 Mbps average

---

## 🔧 Configuración y Variables de Entorno

```bash
# Database
DATABASE_URL=postgresql://avalencia:password@dbmain.lumapp.org/tfactu
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=5

# Redis
REDIS_URL=redis://localhost:6379
REDIS_POOL_SIZE=10

# JWT
JWT_SECRET=your-secret-key-here
JWT_EXPIRATION_HOURS=24

# FCM (Push Notifications)
FCM_SERVER_KEY=your-fcm-server-key
FCM_ENDPOINT=https://fcm.googleapis.com/fcm/send

# Server
SERVER_PORT=8000
SERVER_HOST=0.0.0.0
RUST_LOG=info

# Rate Limiting
RATE_LIMIT_ENABLED=true
RATE_LIMIT_REQUESTS_PER_MINUTE=100

# Prometheus
PROMETHEUS_ENABLED=true
METRICS_ENDPOINT=/monitoring/metrics
```

---

## 📈 Próximas Mejoras de Arquitectura

### Q1 2026
- [ ] Migrar a microservicios (API Gateway + Services)
- [ ] Implementar gRPC para comunicación inter-servicios
- [ ] Event-driven architecture con Kafka/NATS
- [ ] Service mesh con Istio

### Q2 2026
- [ ] Kubernetes deployment
- [ ] Auto-scaling basado en métricas
- [ ] Multi-region deployment
- [ ] CDN para QR codes

---

**Siguiente**: [02-conceptos.md](./02-conceptos.md) - Explicación conceptual del sistema
