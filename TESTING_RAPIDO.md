# 🧪 TESTING RÁPIDO - Comandos Copy/Paste

**Para testing inmediato del sistema**

---

## 🚀 Paso 1: Iniciar Servidor (10 segundos)

```bash
cd /home/client_1099_1/scripts/lum_rust_ws
./target/release/lum_rust_ws
```

**Esperado**:
```
📲 Push notification service initialized (FCM ready)
🔗 Webhook service initialized (merchant notifications ready)
🚦 Rate limiter service initialized (abuse prevention active)
⏰ Scheduled jobs service started (nightly validation, expiration checks)
listening on 0.0.0.0:8000
```

---

## 🔍 Paso 2: Health Check (5 segundos)

En otra terminal:

```bash
curl http://localhost:8000/health
```

**Esperado**: `{"status":"ok"}`

---

## 📊 Paso 3: Ver Métricas (10 segundos)

```bash
# Ver todas las métricas
curl http://localhost:8000/monitoring/metrics

# Solo métricas de redenciones
curl http://localhost:8000/monitoring/metrics | grep redemptions

# Contar métricas
curl http://localhost:8000/monitoring/metrics | grep redemptions | wc -l
```

**Esperado**: ~12 métricas de redemptions

---

## 🔑 Paso 4: Generar JWT de Prueba

### Opción A: Usar JWT existente de base de datos

```bash
# Conectar a DB y obtener un user_id real
psql postgresql://username:password@dbmain.lumapp.org/tfactu \
  -c "SELECT user_id FROM rewards.fact_balance_points LIMIT 1;"

# Usar ese user_id para generar JWT en https://jwt.io
# Payload:
{
  "sub": "12345",  # Reemplazar con user_id real
  "name": "Usuario Test",
  "exp": 1735689600,  # 31 dic 2024
  "iat": 1729296000
}
# Secret: El mismo del .env (JWT_SECRET)
```

### Opción B: Usar script Python (si existe)

```bash
python3 generate_test_jwt.py --user-id 12345 --name "Test User"
```

### Opción C: JWT de prueba hardcoded (SOLO PARA TESTING LOCAL)

```bash
# ⚠️ Este JWT solo funciona si JWT_SECRET="test_secret"
export JWT_TEST="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NSIsIm5hbWUiOiJUZXN0IFVzZXIiLCJleHAiOjE3MzU2ODk2MDAsImlhdCI6MTcyOTI5NjAwMH0.X8YQZ_k3vN2pL5mR7jT9wV1xC4sA6fE8hG0iJ2kM4nO"
```

---

## 🧪 Paso 5: Test Balance Endpoint

```bash
# Configurar JWT (usar uno de los métodos anteriores)
export JWT="TU_JWT_AQUI"

# Test balance
curl http://localhost:8000/api/v1/rewards/balance \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json"
```

**Esperado**:
```json
{
  "user_id": 12345,
  "balance_points": 150,
  "balance_lumis": 450,
  "last_updated": "2024-10-19T..."
}
```

**Si error 401**: JWT inválido o expirado
**Si error 404**: user_id no existe en fact_balance_points

---

## 🎁 Paso 6: Test Ofertas

```bash
curl http://localhost:8000/api/v1/rewards/offers \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json"
```

**Esperado**:
```json
{
  "offers": [
    {
      "offer_id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Café Gratis",
      "description": "Un café gratis en tiendas participantes",
      "lumis_cost": 50,
      "merchant_name": "Café Duran",
      "valid_until": "2024-12-31T23:59:59Z",
      "is_active": true
    }
  ],
  "total": 1
}
```

---

## 🔄 Paso 7: Test Redención Completa

### 7.1 Obtener offer_id

```bash
# Listar ofertas y obtener un offer_id
curl http://localhost:8000/api/v1/rewards/offers \
  -H "Authorization: Bearer $JWT" | jq '.offers[0].offer_id'

# Guardar en variable
export OFFER_ID="550e8400-e29b-41d4-a716-446655440000"
```

### 7.2 Obtener user_id del JWT

```bash
# Extraer user_id del JWT (sub claim)
export USER_ID=$(echo $JWT | cut -d'.' -f2 | base64 -d 2>/dev/null | jq -r '.sub')
echo "User ID: $USER_ID"
```

### 7.3 Crear Redención

```bash
curl -X POST http://localhost:8000/api/v1/rewards/redeem \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"offer_id\": \"$OFFER_ID\",
    \"user_id\": $USER_ID
  }"
```

**Esperado**:
```json
{
  "redemption_id": "uuid-generado",
  "status": "pending",
  "redemption_code": "ABC123",
  "lumis_spent": 50,
  "created_at": "2024-10-19T...",
  "expires_at": "2024-10-26T..."
}
```

**Guardar redemption_id y code**:
```bash
export REDEMPTION_ID="uuid-de-respuesta"
export REDEMPTION_CODE="ABC123"
```

### 7.4 Verificar Balance Disminuyó

```bash
curl http://localhost:8000/api/v1/rewards/balance \
  -H "Authorization: Bearer $JWT"
```

**Esperado**: balance_lumis debe haber disminuido 50

---

## 📜 Paso 8: Test Historial

```bash
curl "http://localhost:8000/api/v1/rewards/history?limit=10&offset=0" \
  -H "Authorization: Bearer $JWT"
```

**Esperado**: Lista con la redención recién creada

---

## 🏪 Paso 9: Test Merchant Endpoints

### 9.1 Generar JWT de Merchant

```bash
# En https://jwt.io con payload:
{
  "sub": "merchant_123",
  "merchant_id": "uuid-del-merchant",
  "name": "Café Duran",
  "exp": 1735689600,
  "iat": 1729296000
}

export JWT_MERCHANT="eyJhbGciOi..."
```

### 9.2 Ver Pendientes

```bash
curl http://localhost:8000/api/v1/merchant/pending \
  -H "Authorization: Bearer $JWT_MERCHANT"
```

### 9.3 Validar Código

```bash
curl -X POST http://localhost:8000/api/v1/merchant/validate/$REDEMPTION_ID \
  -H "Authorization: Bearer $JWT_MERCHANT" \
  -H "Content-Type: application/json" \
  -d "{
    \"redemption_code\": \"$REDEMPTION_CODE\"
  }"
```

**Esperado**: `{"valid": true}`

### 9.4 Confirmar Redención

```bash
curl -X POST http://localhost:8000/api/v1/merchant/confirm/$REDEMPTION_ID \
  -H "Authorization: Bearer $JWT_MERCHANT"
```

**Esperado**: `{"status": "confirmed"}`

---

## ❌ Paso 10: Test Cancelación

```bash
curl -X POST http://localhost:8000/api/v1/rewards/redemptions/$REDEMPTION_ID/cancel \
  -H "Authorization: Bearer $JWT"
```

**Esperado**: `{"status": "cancelled"}`

**Verificar balance restaurado**:
```bash
curl http://localhost:8000/api/v1/rewards/balance \
  -H "Authorization: Bearer $JWT"
```

---

## 📊 Paso 11: Test Analytics (Merchant)

```bash
curl "http://localhost:8000/api/v1/merchant/analytics?start_date=2024-10-01&end_date=2024-10-19" \
  -H "Authorization: Bearer $JWT_MERCHANT"
```

**Esperado**:
```json
{
  "total_redemptions": 3,
  "confirmed_redemptions": 1,
  "pending_redemptions": 2,
  "total_lumis_redeemed": 111,
  "average_confirmation_time_minutes": 15.5
}
```

---

## 🔍 Paso 12: Test Acumulaciones

```bash
curl "http://localhost:8000/api/v1/rewards/accumulations?limit=20&offset=0" \
  -H "Authorization: Bearer $JWT"
```

**Esperado**: Lista de acumulaciones del usuario

---

## 🚦 Paso 13: Test Rate Limiting

```bash
# Hacer 10 requests rápidos
for i in {1..10}; do
  curl http://localhost:8000/api/v1/rewards/balance \
    -H "Authorization: Bearer $JWT" \
    -s -o /dev/null -w "Request $i: %{http_code}\n"
  sleep 0.1
done
```

**Esperado**: Primeros 5-6 devuelven 200, luego 429 (Too Many Requests)

---

## 💾 Paso 14: Verificar Base de Datos

```bash
# Conectar a DB
psql postgresql://username:password@dbmain.lumapp.org/tfactu

# Ver redenciones recientes
SELECT redemption_id, user_id, status, lumis_spent, created_at, confirmed_at
FROM rewards.user_redemptions
ORDER BY created_at DESC LIMIT 5;

# Ver balance del usuario
SELECT user_id, balance_points, balance_lumis, updated_at
FROM rewards.fact_balance_points
WHERE user_id = 12345;

# Ver acumulaciones recientes
SELECT user_id, accum_type, dtype, quantity, created_at
FROM rewards.fact_accumulations
ORDER BY created_at DESC LIMIT 10;

# Verificar triggers activos
SELECT tgname, tgenabled
FROM pg_trigger
WHERE tgname IN (
  'trigger_accumulations_points_updatebalance',
  'trigger_subtract_redemption'
);
```

---

## 📈 Paso 15: Ver Métricas Post-Testing

```bash
# Ver métricas actualizadas
curl http://localhost:8000/monitoring/metrics | grep -E "redemptions_(created|confirmed|cancelled)_total"

# Exportar métricas
curl http://localhost:8000/monitoring/metrics > metricas_$(date +%Y%m%d_%H%M%S).txt
```

---

## 🔍 Paso 16: Revisar Logs

```bash
# Ver logs del servidor en la terminal donde está corriendo

# O si usaste systemd:
sudo journalctl -u lum_rust_ws -f

# Buscar errores
tail -f /var/log/lum_rust_ws.log | grep ERROR

# Buscar redenciones
tail -f /var/log/lum_rust_ws.log | grep redemption

# Buscar push notifications
tail -f /var/log/lum_rust_ws.log | grep "push_notification"
```

---

## ✅ Checklist de Testing

```bash
# Copiar y pegar todo esto:

echo "=== TESTING CHECKLIST ==="
echo ""

# 1. Health check
echo -n "1. Health check: "
curl -s http://localhost:8000/health | jq -r '.status' && echo "✅" || echo "❌"

# 2. Metrics
echo -n "2. Metrics: "
curl -s http://localhost:8000/monitoring/metrics | grep -q redemptions && echo "✅" || echo "❌"

# 3. Balance (necesita JWT)
echo -n "3. Balance endpoint: "
curl -s http://localhost:8000/api/v1/rewards/balance \
  -H "Authorization: Bearer $JWT" | grep -q user_id && echo "✅" || echo "❌"

# 4. Offers
echo -n "4. Offers endpoint: "
curl -s http://localhost:8000/api/v1/rewards/offers \
  -H "Authorization: Bearer $JWT" | grep -q offers && echo "✅" || echo "❌"

# 5. Database connection
echo -n "5. Database: "
psql $DATABASE_URL -c "SELECT 1" > /dev/null 2>&1 && echo "✅" || echo "❌"

# 6. Redis connection
echo -n "6. Redis: "
redis-cli -u $REDIS_URL ping > /dev/null 2>&1 && echo "✅" || echo "❌"

echo ""
echo "=== TESTING COMPLETE ==="
```

---

## 🆘 Troubleshooting Rápido

### Error 401 Unauthorized
```bash
# Verificar JWT
echo $JWT | cut -d'.' -f2 | base64 -d 2>/dev/null | jq

# Verificar JWT_SECRET en .env
grep JWT_SECRET .env
```

### Error 500 Internal Server Error
```bash
# Ver logs del servidor
tail -30 /var/log/lum_rust_ws.log

# Verificar conexión DB
psql $DATABASE_URL -c "SELECT 1"
```

### Error 429 Rate Limit
```bash
# Resetear rate limit en Redis
redis-cli -u $REDIS_URL FLUSHDB

# O esperar 1 minuto
sleep 60
```

### Servidor no inicia
```bash
# Verificar puerto 8000 libre
lsof -i :8000

# Matar proceso si necesario
kill -9 $(lsof -t -i:8000)

# Verificar permisos
chmod +x target/release/lum_rust_ws

# Verificar .env
cat .env | grep -E "(DATABASE_URL|REDIS_URL|JWT_SECRET)"
```

---

## 🎯 Suite Completa en 1 Comando

```bash
#!/bin/bash
# Guardar como test_suite.sh y ejecutar: bash test_suite.sh

echo "🧪 TESTING LÜMIS REDEMPTION SYSTEM"
echo "=================================="
echo ""

# Variables
BASE_URL="http://localhost:8000"
JWT="$JWT"  # Debe estar definido

if [ -z "$JWT" ]; then
  echo "❌ Error: JWT no definido"
  echo "Ejecuta: export JWT='tu_jwt_aqui'"
  exit 1
fi

echo "✅ JWT configurado"
echo ""

# Test 1: Health
echo "Test 1: Health check"
curl -s $BASE_URL/health | jq '.'
echo ""

# Test 2: Balance
echo "Test 2: Balance"
curl -s $BASE_URL/api/v1/rewards/balance \
  -H "Authorization: Bearer $JWT" | jq '.'
echo ""

# Test 3: Offers
echo "Test 3: Offers"
OFFERS=$(curl -s $BASE_URL/api/v1/rewards/offers \
  -H "Authorization: Bearer $JWT")
echo $OFFERS | jq '.'
OFFER_ID=$(echo $OFFERS | jq -r '.offers[0].offer_id')
echo "Offer ID: $OFFER_ID"
echo ""

# Test 4: Redeem
echo "Test 4: Create redemption"
USER_ID=$(echo $JWT | cut -d'.' -f2 | base64 -d 2>/dev/null | jq -r '.sub')
REDEMPTION=$(curl -s -X POST $BASE_URL/api/v1/rewards/redeem \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{\"offer_id\":\"$OFFER_ID\",\"user_id\":$USER_ID}")
echo $REDEMPTION | jq '.'
REDEMPTION_ID=$(echo $REDEMPTION | jq -r '.redemption_id')
echo "Redemption ID: $REDEMPTION_ID"
echo ""

# Test 5: History
echo "Test 5: History"
curl -s "$BASE_URL/api/v1/rewards/history?limit=5" \
  -H "Authorization: Bearer $JWT" | jq '.'
echo ""

# Test 6: Cancel
echo "Test 6: Cancel redemption"
curl -s -X POST $BASE_URL/api/v1/rewards/redemptions/$REDEMPTION_ID/cancel \
  -H "Authorization: Bearer $JWT" | jq '.'
echo ""

# Test 7: Balance after cancel
echo "Test 7: Balance after cancel"
curl -s $BASE_URL/api/v1/rewards/balance \
  -H "Authorization: Bearer $JWT" | jq '.'
echo ""

echo "=================================="
echo "✅ TESTING COMPLETE"
```

---

**Para ejecutar la suite completa**:

```bash
# 1. Definir JWT
export JWT="tu_jwt_aqui"

# 2. Guardar script
cat > test_suite.sh << 'EOF'
# ... (copiar el script de arriba)
EOF

# 3. Ejecutar
bash test_suite.sh
```

---

**Generado**: 19 de octubre, 2024  
**Para**: Testing rápido del sistema  
**Tiempo estimado**: 15-30 minutos para suite completa
