# 12 - Guía de Deployment

## Pre-requisitos

- Rust 1.70+
- PostgreSQL 14+
- Redis 7+
- Docker (opcional)

## Deployment Manual

```bash
# 1. Clonar repo
git clone [repo-url]
cd lum_rust_ws

# 2. Configurar .env
cp .env.example .env
# Editar DATABASE_URL, REDIS_URL, JWT_SECRET, etc.

# 3. Ejecutar migraciones
psql -h dbmain.lumapp.org -U avalencia -d tfactu -f migration_redemption_system_complete.sql

# 4. Build release
cargo build --release

# 5. Run
./target/release/lum_rust_ws
```

## Deployment con Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/lum_rust_ws /usr/local/bin/
CMD ["lum_rust_ws"]
```

## Health Checks

```bash
# Health
curl http://localhost:8000/monitoring/health

# Metrics
curl http://localhost:8000/monitoring/metrics
```

**Siguiente**: [13-troubleshooting.md](./13-troubleshooting.md)
