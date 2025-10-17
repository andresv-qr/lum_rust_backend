# 🎯 API de Ofertas con Cache Redis - Documentación Completa

## 📋 Resumen

Nueva API optimizada para consultar ofertas de `wsf_consolidado` con cache Redis, compresión GZIP y E-Tag para máxima eficiencia.

**Performance esperado:**
- Primera carga: ~500-800ms
- Con cache: ~5-15ms
- Con E-Tag (304): ~3-5ms, **0 bytes transferidos**

---

## 🔧 Configuración

### 1. Variables de Entorno

Agregar a `.env`:

```bash
# Base de datos WS (ofertas)
WS_DATABASE_URL=postgresql://avalencia:Jacobo23@dbws.lumapp.org/ws

# Redis (ya existe)
REDIS_URL=redis://localhost:6379
```

### 2. Migración de Base de Datos

Ejecutar en la base de datos **ws**:

```bash
psql -h dbws.lumapp.org -U avalencia -d ws -f ofertas_refresh_log.sql
```

O manualmente:

```sql
CREATE TABLE IF NOT EXISTS ofertas_cache_refresh_log (
    id SERIAL PRIMARY KEY,
    executed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    status VARCHAR(20) NOT NULL,
    records_count INTEGER,
    execution_time_ms INTEGER,
    error_message TEXT,
    redis_key VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ofertas_log_executed_at 
ON ofertas_cache_refresh_log(executed_at DESC);
```

### 3. Índice de Base de Datos (Opcional pero Recomendado)

Para optimizar la query:

```sql
CREATE INDEX IF NOT EXISTS idx_wsf_consolidado_precios 
ON wsf_consolidado(precio_anterior, precio_actual) 
WHERE precio_actual IS NOT NULL AND precio_anterior IS NOT NULL;
```

---

## 📡 API Endpoints

### GET /api/v4/ofertas

Obtiene todas las ofertas con diferencia de precio > $3.

**Autenticación:** JWT Bearer Token (protegido)

**Headers:**
```
Authorization: Bearer <token>
If-None-Match: "ofertas-2025-10-15-15:00" (opcional)
```

**Response Headers:**
```
Content-Type: application/json
Content-Encoding: gzip
ETag: "ofertas-2025-10-15-15:00"
Cache-Control: public, max-age=18000
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "ofertas": [
      {
        "comercio": "El Machetazo",
        "producto": "Arroz Diana 500g",
        "codigo": "7891234567890",
        "precio_actual": 1.99,
        "precio_anterior": 2.50,
        "precio_minimo_2m": 1.85,
        "diferencia": 0.51,
        "porcentaje_descuento": 20.4,
        "ahorro": 0.51,
        "es_precio_mas_bajo": false,
        "latest_date": "2025-10-15",
        "dias_con_precio_actual": 3,
        "link": "https://...",
        "imagen": "https://..."
      }
      // ... ~7000 registros
    ],
    "metadata": {
      "total_count": 7000,
      "generated_at": "2025-10-15T20:00:00Z",
      "next_update": "2025-10-16T15:00:00Z",
      "version": "ofertas:cache:2025-10-15-15:00"
    }
  }
}
```

**Response (304 Not Modified):**
Si el E-Tag coincide, devuelve 304 sin body (0 bytes).

**Response (503 Service Unavailable):**
Si WS_DATABASE_URL no está configurado:
```json
{
  "success": false,
  "error": "WS database not configured. Ofertas API is unavailable."
}
```

---

### POST /api/v4/ofertas/refresh

Fuerza un refresh manual del cache (invalida y regenera).

**Autenticación:** JWT Bearer Token (protegido)

**Headers:**
```
Authorization: Bearer <token>
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "message": "Cache refreshed successfully",
    "records_count": 7000,
    "compressed_size_bytes": 482304,
    "execution_time_ms": 456,
    "cache_key": "ofertas:cache:2025-10-15-15:00"
  }
}
```

---

## 🔄 Auto-Refresh Scheduler

El sistema ejecuta automáticamente refresh del cache **2 veces al día**:

- **10:00 AM** hora Panamá (UTC-5) = 3:00 PM UTC
- **3:00 PM** hora Panamá (UTC-5) = 8:00 PM UTC

**Logs del scheduler:**
```
⏰ Executing scheduled ofertas refresh (10am Panamá)
🔄 Starting ofertas refresh for key: ofertas:cache:2025-10-15-10:00
✅ Scheduled refresh completed: 7000 ofertas, 482304 bytes, 456ms
```

Los logs se guardan automáticamente en `ofertas_cache_refresh_log`.

---

## 🗄️ Cache Strategy

### Redis Keys

Formato: `ofertas:cache:{YYYY-MM-DD}-{HH}:00`

Ejemplos:
- `ofertas:cache:2025-10-15-10:00` (versión 10am)
- `ofertas:cache:2025-10-15-15:00` (versión 3pm)

### E-Tag Format

Formato: `"ofertas-{YYYY-MM-DD}-{HH}:00"`

Ejemplos:
- `"ofertas-2025-10-15-10:00"`
- `"ofertas-2025-10-15-15:00"`

### TTL

- **Redis TTL:** 12 horas
- **HTTP Cache-Control:** `public, max-age=18000` (5 horas)

### Lógica de Slot

```
Si hora actual < 15 (3pm):
  → Usar slot 10am
Sino:
  → Usar slot 3pm
```

Esto significa:
- 00:00 - 14:59 → Usa cache de 10am
- 15:00 - 23:59 → Usa cache de 3pm

---

## 📊 Monitoreo

### Consultar Logs de Refresh

```sql
-- Últimas 10 ejecuciones
SELECT 
    executed_at,
    status,
    records_count,
    execution_time_ms,
    redis_key
FROM ofertas_cache_refresh_log
ORDER BY executed_at DESC
LIMIT 10;

-- Ejecuciones fallidas
SELECT 
    executed_at,
    error_message,
    redis_key
FROM ofertas_cache_refresh_log
WHERE status = 'error'
ORDER BY executed_at DESC;

-- Estadísticas del día
SELECT 
    DATE(executed_at) as fecha,
    COUNT(*) as total_ejecuciones,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as exitosas,
    AVG(execution_time_ms) as tiempo_promedio_ms,
    AVG(records_count) as promedio_ofertas
FROM ofertas_cache_refresh_log
WHERE executed_at >= CURRENT_DATE
GROUP BY DATE(executed_at);
```

### Verificar Cache en Redis

```bash
# Conectar a Redis
redis-cli

# Ver keys de ofertas
KEYS ofertas:cache:*

# Ver tamaño de un cache
STRLEN ofertas:cache:2025-10-15-15:00

# Ver TTL restante (segundos)
TTL ofertas:cache:2025-10-15-15:00

# Eliminar cache específico
DEL ofertas:cache:2025-10-15-15:00
```

### Logs de Aplicación

```bash
# Buscar logs de ofertas
grep "ofertas" nohup.out | tail -20

# Cache hits
grep "Cache HIT" nohup.out | tail -10

# Cache misses
grep "Cache MISS" nohup.out | tail -10

# E-Tag matches
grep "E-Tag match" nohup.out | tail -10
```

---

## 🧪 Testing

### 1. Compilar y Ejecutar

```bash
cd /home/client_1099_1/scripts/lum_rust_ws
cargo build --release
cargo run --release
```

### 2. Generar Token JWT

```bash
python3 generate_test_jwt.py
# Copiar token generado
```

### 3. Test GET (Primera vez - Cache Miss)

```bash
TOKEN="eyJ..."

curl -X GET "http://localhost:8000/api/v4/ofertas" \
  -H "Authorization: Bearer $TOKEN" \
  -w "\nStatus: %{http_code}\nSize: %{size_download} bytes\nTime: %{time_total}s\n" \
  --compressed
```

Esperado:
- Status: 200
- Headers: ETag, Content-Encoding: gzip
- Time: ~500-800ms (primera vez)

### 4. Test GET (Segunda vez - Cache Hit)

```bash
curl -X GET "http://localhost:8000/api/v4/ofertas" \
  -H "Authorization: Bearer $TOKEN" \
  -w "\nStatus: %{http_code}\nSize: %{size_download} bytes\nTime: %{time_total}s\n" \
  --compressed
```

Esperado:
- Status: 200
- Time: ~5-15ms (desde Redis)

### 5. Test E-Tag (304 Not Modified)

```bash
# Primero, obtener E-Tag
ETAG=$(curl -s -X GET "http://localhost:8000/api/v4/ofertas" \
  -H "Authorization: Bearer $TOKEN" \
  -I | grep -i "etag" | cut -d' ' -f2 | tr -d '\r')

# Usar E-Tag en siguiente request
curl -X GET "http://localhost:8000/api/v4/ofertas" \
  -H "Authorization: Bearer $TOKEN" \
  -H "If-None-Match: $ETAG" \
  -w "\nStatus: %{http_code}\nSize: %{size_download} bytes\nTime: %{time_total}s\n"
```

Esperado:
- Status: 304
- Size: 0 bytes
- Time: ~3-5ms

### 6. Test Refresh Manual

```bash
curl -X POST "http://localhost:8000/api/v4/ofertas/refresh" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  | jq
```

Esperado:
```json
{
  "success": true,
  "data": {
    "message": "Cache refreshed successfully",
    "records_count": 7000,
    ...
  }
}
```

### 7. Verificar Logs en BD

```bash
psql -h dbws.lumapp.org -U avalencia -d ws \
  -c "SELECT * FROM ofertas_cache_refresh_log ORDER BY executed_at DESC LIMIT 5;"
```

---

## 🚀 Deployment en Producción

### 1. Verificar Variables de Entorno

```bash
# En servidor
cd /home/client_1099_1/scripts/lum_rust_ws
cat .env | grep WS_DATABASE_URL
```

### 2. Build Release

```bash
cargo build --release
```

### 3. Detener Servidor Actual

```bash
# Encontrar PID
ps aux | grep lum_rust_ws

# Detener gracefully
kill -TERM <PID>
```

### 4. Iniciar Nueva Versión

```bash
nohup ./target/release/lum_rust_ws > nohup_ofertas.out 2>&1 &
```

### 5. Verificar Inicio

```bash
# Logs de inicio
tail -f nohup_ofertas.out

# Buscar mensajes clave:
# - "✅ WS database pool initialized for ofertas"
# - "⏰ Ofertas refresh scheduler initialized (10am & 3pm Panamá)"
```

### 6. Health Check

```bash
curl http://localhost:8000/health
```

---

## 📈 Optimizaciones Futuras

### 1. Paginación (Si se requiere)

```
GET /api/v4/ofertas?page=1&limit=100
```

- Reduce payload inicial
- Mejora UX en Flutter
- Cache por página

### 2. Filtros

```
GET /api/v4/ofertas?comercio=El%20Machetazo
GET /api/v4/ofertas?min_descuento=20
GET /api/v4/ofertas?categoria=Alimentos
```

### 3. Compresión Brotli

Alternativa a GZIP (~15% mejor):
```rust
Content-Encoding: br
```

### 4. Streaming Response

Para datasets muy grandes:
```rust
// Stream línea por línea (NDJSON)
{"comercio": "..."}
{"comercio": "..."}
```

### 5. CDN Integration

- Cloudflare cache
- Edge caching global
- Reducir latencia internacional

---

## 🔒 Seguridad

### Autenticación

- ✅ JWT Bearer Token requerido
- ✅ Middleware `extract_current_user`
- ✅ Token expiration validation

### Rate Limiting

- ✅ Implementado a nivel global
- ⚠️ Considerar rate limit específico para /ofertas si hay abuso

### Datos Sensibles

- ✅ Credenciales en variables de entorno
- ✅ No se exponen en logs
- ✅ Conexión PostgreSQL con SSL

---

## 📞 Troubleshooting

### Error: "WS database not configured"

**Causa:** Variable `WS_DATABASE_URL` no está configurada o es inválida.

**Solución:**
```bash
echo "WS_DATABASE_URL=postgresql://avalencia:Jacobo23@dbws.lumapp.org/ws" >> .env
```

### Error: "Redis connection error"

**Causa:** Redis no está corriendo o REDIS_URL es incorrecta.

**Solución:**
```bash
# Verificar Redis
redis-cli PING
# Debe responder: PONG

# Si no está corriendo
sudo systemctl start redis
```

### Cache siempre MISS

**Causa:** TTL expirado o key incorrecta.

**Solución:**
```bash
# Verificar keys en Redis
redis-cli KEYS "ofertas:cache:*"

# Ver logs
grep "Cache key:" nohup.out | tail -5
```

### Scheduler no ejecuta

**Causa:** Timezone incorrecta o cron pattern inválido.

**Solución:**
```bash
# Verificar logs de scheduler
grep "Executing scheduled ofertas" nohup.out

# Verificar timezone del sistema
date
timedatectl
```

### Query muy lenta

**Causa:** Falta índice o tabla muy grande.

**Solución:**
```sql
-- Crear índice
CREATE INDEX CONCURRENTLY idx_wsf_consolidado_precios 
ON wsf_consolidado(precio_anterior, precio_actual) 
WHERE precio_actual IS NOT NULL AND precio_anterior IS NOT NULL;

-- Analizar query
EXPLAIN ANALYZE 
SELECT * FROM wsf_consolidado
WHERE abs(precio_anterior - precio_actual) > 3
  AND precio_actual IS NOT NULL;
```

---

## 📚 Recursos

- **Código fuente:** `src/api/ofertas_v4.rs`
- **Scheduler:** `src/tasks/ofertas_refresh.rs`
- **WS Pool:** `src/db/ws_pool.rs`
- **Migración SQL:** `ofertas_refresh_log.sql`

---

## ✅ Checklist de Implementación

- [x] Crear módulo `ofertas_v4.rs`
- [x] Crear scheduler `ofertas_refresh.rs`
- [x] Crear pool WS `ws_pool.rs`
- [x] Migración SQL tabla logs
- [x] Agregar dependencias Cargo.toml
- [x] Integrar en `mod.rs`
- [x] Integrar en `main.rs`
- [x] Modificar `AppState`
- [ ] **Ejecutar migración SQL en BD ws**
- [ ] **Configurar WS_DATABASE_URL en .env**
- [ ] **Compilar y testear**
- [ ] **Deploy a producción**
- [ ] **Verificar scheduler ejecuta correctamente**
- [ ] **Documentar para equipo frontend**

---

**Fecha:** 15 de Octubre, 2025  
**Versión:** 1.0  
**Autor:** GitHub Copilot
