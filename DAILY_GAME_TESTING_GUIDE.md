# 🎮 Daily Game API - Testing Guide

## ✅ Implementación Completada

### 📊 Base de Datos
- ✅ Tabla `rewards.fact_daily_game_plays` creada
- ✅ Regla en `rewards.dim_accumulations` (id=10, name='daily_game')
- ✅ Índices optimizados

### 💻 Código Rust
- ✅ Módulo `daily_game` implementado
- ✅ Endpoint POST `/v4/daily-game/claim`
- ✅ Endpoint GET `/v4/daily-game/status`
- ✅ Integración con sistema de rewards existente

---

## 🔧 Testing Manual

### 1. Verificar que el servidor esté corriendo

```bash
# Iniciar servidor
cd /home/client_1099_1/scripts/lum_rust_ws
cargo run --bin lum_rust_ws

# Verificar logs
tail -f nohup.out
```

### 2. GET /v4/daily-game/status

**Request:**
```bash
curl -X GET http://localhost:8000/api/v4/daily-game/status \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json"
```

**Response esperado (primera vez):**
```json
{
  "success": true,
  "data": {
    "can_play_today": true,
    "has_played_today": false,
    "last_played_date": null,
    "todays_reward": null,
    "stats": null
  }
}
```

---

### 3. POST /v4/daily-game/claim (Primera jugada - Estrella vacía)

**Request:**
```bash
curl -X POST http://localhost:8000/api/v4/daily-game/claim \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_2",
    "lumis_won": 0
  }'
```

**Response esperado:**
```json
{
  "success": true,
  "data": {
    "lumis_added": 0,
    "new_balance": 0,
    "play_id": 1
  },
  "message": "¡Ups! Estrella vacía. Mejor suerte mañana. 🌟"
}
```

---

### 4. POST /v4/daily-game/claim (Estrella normal - 1 Lümi)

**Request:**
```bash
curl -X POST http://localhost:8000/api/v4/daily-game/claim \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_5",
    "lumis_won": 1
  }'
```

**Response esperado:**
```json
{
  "success": true,
  "data": {
    "lumis_added": 1,
    "new_balance": 1,
    "play_id": 2
  },
  "message": "¡Genial! +1 Lümi ganado. 🌟"
}
```

---

### 5. POST /v4/daily-game/claim (Estrella dorada - 5 Lümis)

**Request:**
```bash
curl -X POST http://localhost:8000/api/v4/daily-game/claim \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_7",
    "lumis_won": 5
  }'
```

**Response esperado:**
```json
{
  "success": true,
  "data": {
    "lumis_added": 5,
    "new_balance": 6,
    "play_id": 3
  },
  "message": "¡Increíble! 🌟✨ ¡Encontraste la estrella dorada! +5 Lümis"
}
```

---

### 6. POST /v4/daily-game/claim (Ya jugó hoy - Error 409)

**Request:**
```bash
curl -X POST http://localhost:8000/api/v4/daily-game/claim \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_1",
    "lumis_won": 1
  }'
```

**Response esperado:**
```json
{
  "success": false,
  "error": {
    "code": "ALREADY_PLAYED_TODAY",
    "message": "Ya jugaste hoy. Vuelve mañana a las 00:00."
  }
}
```

**HTTP Status:** 409 Conflict

---

### 7. GET /v4/daily-game/status (Después de jugar)

**Request:**
```bash
curl -X GET http://localhost:8000/api/v4/daily-game/status \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json"
```

**Response esperado:**
```json
{
  "success": true,
  "data": {
    "can_play_today": false,
    "has_played_today": true,
    "last_played_date": "2025-10-13",
    "todays_reward": 5,
    "stats": {
      "total_plays": 3,
      "total_lumis_won": 6,
      "golden_stars_captured": 1
    }
  }
}
```

---

### 8. Validación - Valor inválido (Error 400)

**Request:**
```bash
curl -X POST http://localhost:8000/api/v4/daily-game/claim \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_3",
    "lumis_won": 10
  }'
```

**Response esperado:**
```json
{
  "success": false,
  "error": {
    "code": "ERROR",
    "message": "Invalid lumis_won value: 10. Must be 0, 1, or 5"
  }
}
```

**HTTP Status:** 400 Bad Request

---

### 9. Validación - star_id inválido (Error 400)

**Request:**
```bash
curl -X POST http://localhost:8000/api/v4/daily-game/claim \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_99",
    "lumis_won": 1
  }'
```

**Response esperado:**
```json
{
  "success": false,
  "error": {
    "code": "ERROR",
    "message": "Invalid star_id: star_99. Must be star_0 to star_8"
  }
}
```

**HTTP Status:** 400 Bad Request

---

## 🔍 Verificación en Base de Datos

### Ver jugadas registradas
```sql
SELECT 
  id,
  user_id,
  play_date,
  play_time,
  star_id,
  lumis_won,
  created_at
FROM rewards.fact_daily_game_plays
ORDER BY created_at DESC
LIMIT 10;
```

### Ver acumulaciones registradas
```sql
SELECT 
  id,
  user_id,
  accum_type,
  accum_key,
  quantity,
  date,
  accum_id
FROM rewards.fact_accumulations
WHERE accum_type = 'daily_game'
ORDER BY date DESC
LIMIT 10;
```

### Ver balance actualizado
```sql
SELECT 
  user_id,
  balance,
  latest_update
FROM rewards.fact_balance_points
WHERE user_id = 1;
```

### Verificar constraint UNIQUE funciona
```sql
-- Intento duplicado (debe fallar)
INSERT INTO rewards.fact_daily_game_plays
(user_id, play_date, play_time, star_id, lumis_won)
VALUES (1, CURRENT_DATE, CURRENT_TIME, 'star_0', 1);

-- ERROR: duplicate key value violates unique constraint "unique_user_daily_play"
```

---

## 📊 Métricas Básicas

### Tasa de juego diario
```sql
SELECT 
  COUNT(DISTINCT user_id) as daily_players,
  CURRENT_DATE as date
FROM rewards.fact_daily_game_plays
WHERE play_date = CURRENT_DATE;
```

### Tasa de estrellas doradas
```sql
SELECT 
  COUNT(*) FILTER (WHERE lumis_won = 5) * 100.0 / COUNT(*) as golden_rate_percent,
  COUNT(*) FILTER (WHERE lumis_won = 5) as golden_count,
  COUNT(*) as total_plays
FROM rewards.fact_daily_game_plays;
```

### Promedio de Lümis por jugada
```sql
SELECT 
  AVG(lumis_won) as avg_lumis,
  MIN(lumis_won) as min_lumis,
  MAX(lumis_won) as max_lumis
FROM rewards.fact_daily_game_plays;
```

### Lümis distribuidos por día
```sql
SELECT 
  play_date,
  COUNT(*) as total_plays,
  SUM(lumis_won) as total_lumis_distributed
FROM rewards.fact_daily_game_plays
GROUP BY play_date
ORDER BY play_date DESC;
```

---

## 🧪 Testing con JWT

### Generar token de prueba
```bash
# Usar el token existente del sistema
TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VyX2lkIjoxLCJleHAiOjE4OTM0NTYwMDB9.dG1yUh8FLqOdB4qAOaL7Gk7mHLPTWTEHJ5zSEBOi-fA"

# Testing completo
curl -X GET "http://localhost:8000/api/v4/daily-game/status" \
  -H "Authorization: Bearer $TOKEN"

curl -X POST "http://localhost:8000/api/v4/daily-game/claim" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"star_id": "star_4", "lumis_won": 5}'
```

---

## ✅ Checklist de Validación

### Base de Datos
- [ ] Tabla `fact_daily_game_plays` existe
- [ ] Regla `daily_game` en `dim_accumulations` existe (id=10)
- [ ] Constraint UNIQUE funciona (no permite duplicados)
- [ ] Índices creados correctamente

### Endpoints
- [ ] `/status` retorna correctamente (primera vez)
- [ ] `/claim` con `lumis_won=0` funciona
- [ ] `/claim` con `lumis_won=1` funciona
- [ ] `/claim` con `lumis_won=5` funciona
- [ ] `/claim` segundo intento retorna 409 Conflict
- [ ] Validación de `lumis_won` inválido (400)
- [ ] Validación de `star_id` inválido (400)
- [ ] `/status` retorna stats después de jugar

### Integración
- [ ] Lumis se registran en `fact_accumulations`
- [ ] Balance se actualiza en `fact_balance_points`
- [ ] Trigger funciona correctamente
- [ ] Zona horaria de Panamá se usa correctamente
- [ ] Logs informativos en consola

---

## 🎯 Próximos Pasos (Fase 2 - Opcional)

### Rachas Consecutivas
- [ ] Agregar campo `streak` a `fact_daily_game_plays`
- [ ] Calcular días consecutivos
- [ ] Mostrar racha en `/status`

### Multiplicadores
- [ ] Aplicar multiplicador x2 si `streak >= 7`
- [ ] Aplicar multiplicador x3 si `streak >= 14`
- [ ] Aplicar multiplicador x5 si `streak >= 30`

### Estadísticas Avanzadas
- [ ] Endpoint `/history` con últimos 30 días
- [ ] Gráficos de jugadas por día
- [ ] Análisis de patrones de juego

---

## 📝 Notas Técnicas

### Zona Horaria
- **Importante**: Todos los cálculos de "hoy" usan `America/Panama` (UTC-5)
- Evita problemas con jugadores que juegan cerca de medianoche

### Transacciones Atómicas
- `fact_daily_game_plays` + `fact_accumulations` en misma transacción
- Si falla uno, se hace rollback de ambos

### Performance
- Query única en `/status` con CTEs (eficiente)
- Índices en `(user_id, play_date)` y `play_date`

---

**Implementación completada: 2025-10-13** ✅
