# 🚀 INICIO RÁPIDO - Sistema de Redenciones Lümis

**Versión**: 1.0.0  
**Fecha**: 19 de octubre, 2024  
**Status**: ✅ Listo para producción

---

## ⚡ Inicio en 5 Minutos

### 1. Verificar Binario (5 segundos)
```bash
ls -lh target/release/lum_rust_ws
# Debe mostrar: 66MB aproximadamente
```

### 2. Configurar Variables de Entorno (2 minutos)

Edita el archivo `.env` en la raíz del proyecto:

```bash
# Database principal
DATABASE_URL=postgresql://username:password@dbmain.lumapp.org/tfactu

# Redis (cache y rate limiting)
REDIS_URL=redis://localhost:6379

# JWT Authentication
JWT_SECRET=tu_secreto_jwt_aqui

# Firebase Cloud Messaging (Push Notifications)
FCM_SERVER_KEY=AAAA... # Obtener de Firebase Console
FCM_ENDPOINT=https://fcm.googleapis.com/fcm/send

# WhatsApp (ya configurado)
WHATSAPP_TOKEN=tu_token_existente
PHONE_NUMBER_ID=tu_phone_id_existente
WHATSAPP_API_BASE_URL=https://graph.facebook.com/v18.0

# Ofertas WS Database (opcional)
WS_DATABASE_URL=postgresql://username:password@dbmain.lumapp.org/ofertas_ws

# Features
RATE_LIMIT_ENABLED=true
PROMETHEUS_ENABLED=true
SCHEDULED_JOBS_ENABLED=true

# Server
PORT=8000
```

### 3. Iniciar Servidor (10 segundos)
```bash
cd /home/client_1099_1/scripts/lum_rust_ws

# Opción 1: Producción (optimizado)
./target/release/lum_rust_ws

# Opción 2: Con logs detallados
RUST_LOG=info ./target/release/lum_rust_ws

# Opción 3: Debug completo
RUST_LOG=debug ./target/release/lum_rust_ws
```

**Deberías ver**:
```
🔍 Monitoring system initialized
🚀 Application state initialized with optimized configuration
🤖 ONNX ML models initialized for enhanced QR detection
📲 Push notification service initialized (FCM ready)
🔗 Webhook service initialized (merchant notifications ready)
🚦 Rate limiter service initialized (abuse prevention active)
⏰ Scheduled jobs service started (nightly validation, expiration checks)
⏰ OfertasWs refresh scheduler initialized (10am & 3pm Panamá)
listening on 0.0.0.0:8000
```

### 4. Verificar que Funciona (1 minuto)

```bash
# Test 1: Health check
curl http://localhost:8000/health
# Respuesta esperada: {"status":"ok"}

# Test 2: Métricas Prometheus
curl http://localhost:8000/monitoring/metrics | grep redemptions
# Respuesta esperada: Lista de métricas de redenciones

# Test 3: Balance de usuario (necesitas JWT válido)
curl http://localhost:8000/api/v1/rewards/balance \
  -H "Authorization: Bearer TU_JWT_TOKEN_AQUI"
# Respuesta esperada: {"user_id":12345,"balance_points":X,"balance_lumis":Y}
```

---

## 🧪 Testing Completo (15 minutos)

### Generar JWT de Prueba

```bash
cd /home/client_1099_1/scripts/lum_rust_ws

# Usar script de generación
python3 generate_test_jwt.py --user-id 12345 --name "Usuario Test"

# O manualmente desde https://jwt.io con este payload:
{
  "sub": "12345",
  "name": "Usuario Test",
  "exp": 1735689600,  # 31 dic 2024
  "iat": 1729296000
}
```

### Test Suite Completo

```bash
# Variables
export JWT_USER="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." # Tu JWT aquí
export JWT_MERCHANT="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." # JWT de merchant
export BASE_URL="http://localhost:8000"

# 1. Consultar balance
curl $BASE_URL/api/v1/rewards/balance \
  -H "Authorization: Bearer $JWT_USER"

# 2. Ver ofertas disponibles
curl $BASE_URL/api/v1/rewards/offers \
  -H "Authorization: Bearer $JWT_USER"

# 3. Crear redención (guarda el ID de la respuesta)
curl -X POST $BASE_URL/api/v1/rewards/redeem \
  -H "Authorization: Bearer $JWT_USER" \
  -H "Content-Type: application/json" \
  -d '{
    "offer_id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": 12345
  }'

# Respuesta esperada:
# {
#   "redemption_id": "uuid-aqui",
#   "status": "pending",
#   "redemption_code": "ABC123",
#   "created_at": "2024-10-19T..."
# }

# 4. Ver detalle de redención
export REDEMPTION_ID="uuid-de-respuesta-anterior"
curl $BASE_URL/api/v1/rewards/redemptions/$REDEMPTION_ID \
  -H "Authorization: Bearer $JWT_USER"

# 5. Ver historial
curl "$BASE_URL/api/v1/rewards/history?limit=10&offset=0" \
  -H "Authorization: Bearer $JWT_USER"

# 6. Ver acumulaciones
curl "$BASE_URL/api/v1/rewards/accumulations?limit=10&offset=0" \
  -H "Authorization: Bearer $JWT_USER"

# === MERCHANT ENDPOINTS ===

# 7. Ver redenciones pendientes (merchant)
curl $BASE_URL/api/v1/merchant/pending \
  -H "Authorization: Bearer $JWT_MERCHANT"

# 8. Validar redención
curl -X POST $BASE_URL/api/v1/merchant/validate/$REDEMPTION_ID \
  -H "Authorization: Bearer $JWT_MERCHANT" \
  -H "Content-Type: application/json" \
  -d '{
    "redemption_code": "ABC123"
  }'

# 9. Confirmar redención
curl -X POST $BASE_URL/api/v1/merchant/confirm/$REDEMPTION_ID \
  -H "Authorization: Bearer $JWT_MERCHANT"

# 10. Analytics (merchant)
curl "$BASE_URL/api/v1/merchant/analytics?start_date=2024-10-01&end_date=2024-10-19" \
  -H "Authorization: Bearer $JWT_MERCHANT"
```

---

## 📊 Monitoreo

### Logs en Tiempo Real
```bash
# Ver todos los logs
tail -f /var/log/lum_rust_ws.log

# Solo errores
tail -f /var/log/lum_rust_ws.log | grep ERROR

# Solo redenciones
tail -f /var/log/lum_rust_ws.log | grep redemption

# Métricas específicas
tail -f /var/log/lum_rust_ws.log | grep -E "(balance|redemption|webhook|rate_limit)"
```

### Métricas Prometheus
```bash
# Ver todas las métricas
curl http://localhost:8000/monitoring/metrics

# Solo métricas de redenciones
curl http://localhost:8000/monitoring/metrics | grep redemptions

# Exportar a archivo
curl http://localhost:8000/monitoring/metrics > metricas_$(date +%Y%m%d_%H%M%S).txt
```

### Métricas Disponibles
- `redemptions_created_total` - Total de redenciones creadas
- `redemptions_confirmed_total` - Total confirmadas
- `redemptions_cancelled_total` - Total canceladas
- `redemptions_expired_total` - Total expiradas
- `redemptions_rejected_total` - Total rechazadas
- `redemptions_active` - Redenciones activas ahora
- `redemptions_processing_duration_seconds` - Tiempo de procesamiento
- `lumis_redeemed_total` - Total de lümis gastados
- `offers_created_total` - Total de ofertas creadas
- `offers_active` - Ofertas activas
- `rate_limit_exceeded_total` - Rate limits excedidos
- `webhook_delivery_duration_seconds` - Tiempo de entrega de webhooks

### Conexión a Base de Datos
```bash
# Conectar a PostgreSQL
psql postgresql://username:password@dbmain.lumapp.org/tfactu

# Verificar balances
SELECT user_id, balance_points, balance_lumis, updated_at 
FROM rewards.fact_balance_points 
ORDER BY updated_at DESC LIMIT 10;

# Verificar redenciones recientes
SELECT redemption_id, user_id, status, created_at, confirmed_at
FROM rewards.user_redemptions
ORDER BY created_at DESC LIMIT 10;

# Verificar acumulaciones
SELECT user_id, accum_type, dtype, quantity, created_at
FROM rewards.fact_accumulations
ORDER BY created_at DESC LIMIT 20;
```

---

## 🔧 Troubleshooting

### Problema: No inicia el servidor
**Síntoma**: Error al ejecutar `./target/release/lum_rust_ws`

**Solución**:
```bash
# 1. Verificar permisos
chmod +x target/release/lum_rust_ws

# 2. Verificar variables de entorno
cat .env | grep -E "(DATABASE_URL|REDIS_URL|JWT_SECRET)"

# 3. Verificar conexión a DB
psql $DATABASE_URL -c "SELECT 1"

# 4. Verificar conexión a Redis
redis-cli -u $REDIS_URL ping
```

### Problema: Error de JWT inválido
**Síntoma**: `{"error":"Invalid token"}`

**Solución**:
```bash
# Verificar que JWT_SECRET en .env coincide con el usado para firmar
echo $JWT_SECRET

# Generar nuevo JWT con el secreto correcto
python3 generate_test_jwt.py --user-id 12345
```

### Problema: Balance no se actualiza
**Síntoma**: Balance no cambia después de redención

**Solución**:
```bash
# Verificar triggers en DB
psql $DATABASE_URL << EOF
SELECT tgname, tgenabled 
FROM pg_trigger 
WHERE tgname IN (
  'trigger_accumulations_points_updatebalance',
  'trigger_subtract_redemption'
);
EOF

# Deben mostrar 'O' (enabled)
# Si no, ejecutar: \i fix_balance_triggers.sql
```

### Problema: Push notifications no llegan
**Síntoma**: Redenciones se crean pero no hay notificación

**Solución**:
```bash
# Verificar FCM_SERVER_KEY en .env
echo $FCM_SERVER_KEY

# Verificar tokens FCM en DB
psql $DATABASE_URL -c "SELECT user_id, fcm_token FROM users.user_devices WHERE fcm_token IS NOT NULL LIMIT 5;"

# Ver logs de push service
tail -f /var/log/lum_rust_ws.log | grep "push_notification"
```

### Problema: Rate limiting muy agresivo
**Síntoma**: `{"error":"Rate limit exceeded"}`

**Solución**:
```bash
# Opción 1: Deshabilitar temporalmente
# En .env cambiar: RATE_LIMIT_ENABLED=false

# Opción 2: Resetear contador para un usuario
redis-cli -u $REDIS_URL DEL "rate_limit:redeem:user:12345"

# Opción 3: Ver configuración actual en código
grep -A 5 "RateLimitConfig" src/services/rate_limiter_service.rs
```

---

## 🚀 Deploy a Producción

### Opción 1: Systemd Service (Recomendado)

```bash
# 1. Crear archivo de servicio
sudo nano /etc/systemd/system/lum_rust_ws.service
```

Contenido:
```ini
[Unit]
Description=Lum Rust WS - Sistema de Redenciones
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=lum_app
Group=lum_app
WorkingDirectory=/opt/lum_rust_ws
Environment="RUST_LOG=info"
EnvironmentFile=/opt/lum_rust_ws/.env
ExecStart=/opt/lum_rust_ws/lum_rust_ws
Restart=always
RestartSec=10
StandardOutput=append:/var/log/lum_rust_ws.log
StandardError=append:/var/log/lum_rust_ws_error.log

[Install]
WantedBy=multi-user.target
```

```bash
# 2. Copiar archivos
sudo mkdir -p /opt/lum_rust_ws
sudo cp target/release/lum_rust_ws /opt/lum_rust_ws/
sudo cp .env /opt/lum_rust_ws/

# 3. Configurar permisos
sudo useradd -r -s /bin/false lum_app
sudo chown -R lum_app:lum_app /opt/lum_rust_ws
sudo chmod 600 /opt/lum_rust_ws/.env

# 4. Habilitar y iniciar servicio
sudo systemctl daemon-reload
sudo systemctl enable lum_rust_ws
sudo systemctl start lum_rust_ws

# 5. Verificar status
sudo systemctl status lum_rust_ws

# 6. Ver logs
sudo journalctl -u lum_rust_ws -f
```

### Opción 2: Docker (Alternativa)

```bash
# 1. Crear Dockerfile
cat > Dockerfile << 'EOF'
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY target/release/lum_rust_ws /app/
COPY .env /app/

EXPOSE 8000

CMD ["./lum_rust_ws"]
EOF

# 2. Build imagen
docker build -t lum_rust_ws:1.0.0 .

# 3. Run container
docker run -d \
  --name lum_rust_ws \
  --restart unless-stopped \
  -p 8000:8000 \
  -v /var/log/lum_rust_ws:/var/log \
  lum_rust_ws:1.0.0

# 4. Verificar logs
docker logs -f lum_rust_ws
```

### Opción 3: PM2 (Node.js style)

```bash
# 1. Instalar PM2
npm install -g pm2

# 2. Crear ecosystem.config.js
cat > ecosystem.config.js << 'EOF'
module.exports = {
  apps: [{
    name: 'lum_rust_ws',
    script: './target/release/lum_rust_ws',
    cwd: '/home/client_1099_1/scripts/lum_rust_ws',
    instances: 1,
    autorestart: true,
    watch: false,
    max_memory_restart: '1G',
    env: {
      RUST_LOG: 'info'
    }
  }]
}
EOF

# 3. Iniciar con PM2
pm2 start ecosystem.config.js

# 4. Ver status
pm2 status

# 5. Ver logs
pm2 logs lum_rust_ws

# 6. Guardar para auto-inicio
pm2 save
pm2 startup
```

---

## 📱 Entregar a Frontend

### 1. Enviar Documentación
```bash
# Archivo principal para frontend
cat docs/DOCUMENTACION_FRONTEND_USUARIOS.md

# O enviarlo por email/Slack
# Tamaño: 15KB, ~1,100 líneas
# Contiene: 7 APIs, ejemplos React Native y Flutter
```

### 2. Endpoints Base
```
Producción: https://api.lumapp.org
Staging: http://staging.lumapp.org:8000
Local: http://localhost:8000
```

### 3. Autenticación
Todos los endpoints requieren:
```
Authorization: Bearer {JWT_TOKEN}
```

El JWT debe incluir:
- User endpoints: `sub` = user_id
- Merchant endpoints: `merchant_id` en claims

### 4. Ejemplos de Integración

**React Native**:
```javascript
// Ver DOCUMENTACION_FRONTEND_USUARIOS.md líneas 200-400
import { useRedemptions } from './hooks/useRedemptions';

function OffersScreen() {
  const { offers, loading, redeem } = useRedemptions();
  
  const handleRedeem = async (offerId) => {
    await redeem(offerId);
  };
  
  // ... ver archivo completo
}
```

**Flutter**:
```dart
// Ver DOCUMENTACION_FRONTEND_USUARIOS.md líneas 450-600
class RedemptionService {
  Future<UserBalance> getBalance() async {
    final response = await http.get(
      Uri.parse('$baseUrl/api/v1/rewards/balance'),
      headers: {'Authorization': 'Bearer $token'},
    );
    // ... ver archivo completo
  }
}
```

---

## ✅ Checklist Post-Deploy

Después de desplegar, verificar:

- [ ] Servidor inicia correctamente (ver logs)
- [ ] Health check responde: `curl https://api.lumapp.org/health`
- [ ] Métricas disponibles: `curl https://api.lumapp.org/monitoring/metrics`
- [ ] Balance endpoint funciona con JWT real
- [ ] Push notifications llegan (crear redención de prueba)
- [ ] Rate limiting activo (hacer 10 requests rápidos)
- [ ] Scheduled jobs corriendo (verificar logs a las 3:00 AM)
- [ ] Webhooks funcionan (confirmar redención y verificar en merchant)
- [ ] Base de datos se actualiza (verificar triggers)
- [ ] Frontend puede consumir APIs

---

## 📞 Soporte

### Documentación Adicional
- **Frontend**: `docs/DOCUMENTACION_FRONTEND_USUARIOS.md`
- **Estado técnico**: `ESTADO_ACTUAL_IMPLEMENTACION.md`
- **Resumen visual**: `RESUMEN_VISUAL.md`
- **Sistema completo**: `SISTEMA_LISTO_PARA_PRODUCCION.md`

### Comandos Útiles
```bash
# Recompilar (si cambias código)
cargo build --release

# Ver tamaño del binario
ls -lh target/release/lum_rust_ws

# Verificar dependencias
cargo tree | grep -E "(axum|sqlx|tokio)"

# Actualizar dependencias
cargo update

# Aplicar sugerencias de compilador
cargo fix --lib -p lum_rust_ws
```

---

**¡Sistema listo! 🎉**

Tiempo total de setup: ~5-10 minutos  
Próximos pasos: Testing con datos reales y deploy a staging
