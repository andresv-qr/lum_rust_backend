# 🎮 Daily Game API - Endpoints Documentation

**Versión**: v4  
**Estado**: ✅ Production Ready  
**Fecha**: 2025-10-13  
**Base URL**: `https://api.2factu.com/api/v4/daily-game`

---

## 📋 Tabla de Contenido

1. [Introducción](#introducción)
2. [Autenticación](#autenticación)
3. [Endpoints](#endpoints)
   - [GET /status](#get-status)
   - [POST /claim](#post-claim)
4. [Modelos de Datos](#modelos-de-datos)
5. [Códigos de Error](#códigos-de-error)
6. [Lógica de Negocio](#lógica-de-negocio)
7. [Ejemplos de Integración](#ejemplos-de-integración)

---

## 🎯 Introducción

El **Daily Game** (Constelación Diaria) es un mini-juego diario donde los usuarios pueden ganar Lümis al seleccionar estrellas. Cada usuario puede jugar **una vez por día** y recibir recompensas de **0, 1, o 5 Lümis** dependiendo del tipo de estrella que elijan.

### Características Principales

- ✅ **Una jugada por día**: Garantizado por constraint UNIQUE en base de datos
- ✅ **3 tipos de recompensas**: 0 (vacía), 1 (normal), 5 (dorada)
- ✅ **9 estrellas para elegir**: star_0 a star_8
- ✅ **Zona horaria de Panamá**: UTC-5 para cálculo de "hoy"
- ✅ **Integración con sistema de rewards**: Actualiza balance automáticamente
- ✅ **Estadísticas**: Total de jugadas, Lümis ganados, estrellas doradas capturadas
- ✅ **Transacciones atómicas**: Jugada + acumulación se registran juntas
- ✅ **Auditoría completa**: Cada jugada queda registrada en BD

### Arquitectura

```
Cliente Flutter
    ↓
JWT Auth Middleware
    ↓
Daily Game Endpoints
    ↓
PostgreSQL Database
    ├─ rewards.fact_daily_game_plays (jugadas)
    ├─ rewards.fact_accumulations (acumulaciones)
    └─ rewards.fact_balance_points (balance actualizado por trigger)
```

---

## 🔐 Autenticación

Todos los endpoints requieren autenticación mediante **JWT Bearer Token**.

### Header Requerido

```http
Authorization: Bearer <JWT_TOKEN>
```

### Estructura del JWT

```json
{
  "sub": "1",                           // user_id
  "email": "user@example.com",
  "name": "User Name",
  "iat": 1760318347,                    // Issued at
  "exp": 1760321947                     // Expiration
}
```

### Obtener Token

Ver documentación de autenticación en `API_ENDPOINTS.md` - sección `/api/v4/auth/login` o `/api/v4/auth/unified`.

---

## 📡 Endpoints

### GET /status

**Descripción**: Obtiene el estado actual del juego diario para el usuario autenticado.

**Endpoint**: `/api/v4/daily-game/status`

**Método**: `GET`

**Autenticación**: ✅ Requerida (Bearer Token)

#### Request

```http
GET /api/v4/daily-game/status HTTP/1.1
Host: api.2factu.com
Authorization: Bearer eyJhbGc...
Content-Type: application/json
```

#### Response Success (200 OK)

**Caso 1: Usuario puede jugar hoy (primera vez)**

```json
{
  "success": true,
  "data": {
    "can_play_today": true,
    "has_played_today": false
  }
}
```

**Caso 2: Usuario puede jugar hoy (jugó días anteriores)**

```json
{
  "success": true,
  "data": {
    "can_play_today": true,
    "has_played_today": false,
    "last_played_date": "2025-10-12",
    "stats": {
      "total_plays": 5,
      "total_lumis_won": 12,
      "golden_stars_captured": 1
    }
  }
}
```

**Caso 3: Usuario ya jugó hoy**

```json
{
  "success": true,
  "data": {
    "can_play_today": false,
    "has_played_today": true,
    "todays_reward": 5,
    "last_played_date": "2025-10-13",
    "stats": {
      "total_plays": 6,
      "total_lumis_won": 17,
      "golden_stars_captured": 2
    }
  }
}
```

#### Response Error

**401 Unauthorized** - Token inválido o expirado

```json
{
  "error": "Invalid token",
  "message": "Could not validate credentials. Please log in again.",
  "details": "JWT error: ExpiredSignature"
}
```

**500 Internal Server Error** - Error de base de datos

```json
{
  "success": false,
  "error": {
    "code": "DATABASE_ERROR",
    "message": "Error al obtener el estado del juego"
  }
}
```

#### Campos de Respuesta

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `success` | boolean | Indica si la petición fue exitosa |
| `data.can_play_today` | boolean | **true** si el usuario puede jugar hoy, **false** si ya jugó |
| `data.has_played_today` | boolean | **true** si el usuario ya jugó hoy |
| `data.todays_reward` | integer? | Lümis ganados hoy (0, 1, o 5). Solo presente si ya jugó hoy |
| `data.last_played_date` | string? | Última fecha de juego (formato: YYYY-MM-DD) |
| `data.stats` | object? | Estadísticas históricas del usuario |
| `data.stats.total_plays` | integer | Total de jugadas históricas |
| `data.stats.total_lumis_won` | integer | Total de Lümis ganados en el juego |
| `data.stats.golden_stars_captured` | integer | Total de estrellas doradas (5 Lümis) capturadas |

#### Lógica de Negocio

1. Obtiene fecha actual en zona horaria de **Panamá** (UTC-5)
2. Verifica si existe una jugada para `(user_id, today)`
3. Si existe jugada hoy:
   - `can_play_today = false`
   - `has_played_today = true`
   - Retorna `todays_reward`
4. Si NO existe jugada hoy:
   - `can_play_today = true`
   - `has_played_today = false`
5. Calcula estadísticas históricas del usuario
6. Retorna última fecha de juego si existe

#### Ejemplo cURL

```bash
curl -X GET "https://api.2factu.com/api/v4/daily-game/status" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json"
```

---

### POST /claim

**Descripción**: Reclama la recompensa diaria después de que el usuario seleccione una estrella.

**Endpoint**: `/api/v4/daily-game/claim`

**Método**: `POST`

**Autenticación**: ✅ Requerida (Bearer Token)

#### Request

```http
POST /api/v4/daily-game/claim HTTP/1.1
Host: api.2factu.com
Authorization: Bearer eyJhbGc...
Content-Type: application/json

{
  "star_id": "star_3",
  "lumis_won": 5
}
```

#### Request Body

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `star_id` | string | ✅ Sí | ID de la estrella elegida. Valores válidos: `"star_0"` a `"star_8"` |
| `lumis_won` | integer | ✅ Sí | Lümis ganados. Valores válidos: `0`, `1`, o `5` |

**Validaciones**:
- `star_id` debe coincidir con el patrón regex: `^star_[0-8]$`
- `lumis_won` debe ser exactamente: 0, 1, o 5
- Usuario no debe haber jugado hoy (validado por UNIQUE constraint en BD)

#### Response Success (200 OK)

**Estrella Dorada (5 Lümis)**

```json
{
  "success": true,
  "data": {
    "lumis_added": 5,
    "new_balance": 308,
    "play_id": 1
  },
  "message": "¡Increíble! 🌟✨ ¡Encontraste la estrella dorada! +5 Lümis"
}
```

**Estrella Normal (1 Lümi)**

```json
{
  "success": true,
  "data": {
    "lumis_added": 1,
    "new_balance": 304,
    "play_id": 2
  },
  "message": "¡Genial! ⭐ Has ganado +1 Lümi"
}
```

**Estrella Vacía (0 Lümis)**

```json
{
  "success": true,
  "data": {
    "lumis_added": 0,
    "new_balance": 303,
    "play_id": 3
  },
  "message": "¡Ups! 💫 Estrella vacía, pero mañana tendrás otra oportunidad."
}
```

#### Response Error

**400 Bad Request** - Validación fallida (`lumis_won` inválido)

```json
{
  "success": false,
  "error": {
    "code": "ERROR",
    "message": "Invalid lumis_won value: 10. Must be 0, 1, or 5"
  }
}
```

**400 Bad Request** - Validación fallida (`star_id` inválido)

```json
{
  "success": false,
  "error": {
    "code": "ERROR",
    "message": "Invalid star_id: star_99. Must be star_0 to star_8"
  }
}
```

**409 Conflict** - Usuario ya jugó hoy

```json
{
  "success": false,
  "error": {
    "code": "ALREADY_PLAYED_TODAY",
    "message": "Ya jugaste hoy. Vuelve mañana a las 00:00."
  }
}
```

**401 Unauthorized** - Token inválido

```json
{
  "error": "Invalid token",
  "message": "Could not validate credentials. Please log in again."
}
```

**500 Internal Server Error** - Error de base de datos

```json
{
  "success": false,
  "error": {
    "code": "DATABASE_ERROR",
    "message": "Error al procesar la jugada"
  }
}
```

#### Campos de Respuesta

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `success` | boolean | Indica si la petición fue exitosa |
| `data.lumis_added` | integer | Cantidad de Lümis ganados (0, 1, o 5) |
| `data.new_balance` | integer | Balance total actualizado del usuario |
| `data.play_id` | integer | ID único de la jugada registrada |
| `message` | string | Mensaje personalizado según la recompensa obtenida |

#### Lógica de Negocio

1. **Validación del request**:
   - Valida que `lumis_won` ∈ {0, 1, 5}
   - Valida que `star_id` cumpla formato `star_[0-8]`

2. **Obtener fecha/hora actual**:
   - Usa zona horaria de Panamá (UTC-5)
   - Extrae `play_date` (YYYY-MM-DD) y `play_time` (HH:MM:SS)

3. **Transacción atómica**:
   ```sql
   BEGIN TRANSACTION;
   
   -- Insertar jugada
   INSERT INTO rewards.fact_daily_game_plays 
   (user_id, play_date, play_time, star_id, lumis_won)
   VALUES ($1, $2, $3, $4, $5);
   -- UNIQUE (user_id, play_date) previene duplicados
   
   -- Si lumis_won > 0, registrar acumulación
   IF lumis_won > 0 THEN
     INSERT INTO rewards.fact_accumulations
     (user_id, accum_id, accum_type, quantity, date)
     VALUES ($1, 10, 'daily_game', $lumis_won, NOW());
   END IF;
   
   COMMIT;
   -- Trigger automático actualiza fact_balance_points
   ```

4. **Manejo de errores**:
   - Si ya jugó hoy → UNIQUE constraint violation → 409 Conflict
   - Si error de BD → Rollback → 500 Internal Server Error

5. **Obtener balance actualizado**:
   - Consulta `rewards.fact_balance_points` para `new_balance`

6. **Generar mensaje personalizado**:
   - 5 Lümis: "¡Increíble! 🌟✨ ¡Encontraste la estrella dorada! +5 Lümis"
   - 1 Lümi: "¡Genial! ⭐ Has ganado +1 Lümi"
   - 0 Lümis: "¡Ups! 💫 Estrella vacía, pero mañana tendrás otra oportunidad."

#### Diagrama de Flujo

```
Cliente envía claim request
    ↓
Middleware autentica JWT
    ↓
Validar lumis_won ∈ {0,1,5} ✓
    ↓
Validar star_id = star_[0-8] ✓
    ↓
Obtener fecha actual (Panamá UTC-5)
    ↓
Iniciar transacción BD
    ↓
INSERT fact_daily_game_plays
    ├─ SUCCESS → Continuar
    └─ UNIQUE violation → 409 "Ya jugaste hoy"
    ↓
IF lumis_won > 0:
    INSERT fact_accumulations
    ↓
COMMIT transacción
    ↓
Trigger actualiza fact_balance_points
    ↓
Consultar nuevo balance
    ↓
Generar mensaje personalizado
    ↓
Retornar response 200 OK
```

#### Ejemplo cURL

**Reclamar estrella dorada (5 Lümis)**

```bash
curl -X POST "https://api.2factu.com/api/v4/daily-game/claim" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_7",
    "lumis_won": 5
  }'
```

**Reclamar estrella normal (1 Lümi)**

```bash
curl -X POST "https://api.2factu.com/api/v4/daily-game/claim" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_3",
    "lumis_won": 1
  }'
```

**Reclamar estrella vacía (0 Lümis)**

```bash
curl -X POST "https://api.2factu.com/api/v4/daily-game/claim" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "star_id": "star_0",
    "lumis_won": 0
  }'
```

---

## 📊 Modelos de Datos

### DailyGameStatusResponse

```typescript
interface DailyGameStatusResponse {
  success: boolean;
  data: {
    can_play_today: boolean;
    has_played_today: boolean;
    todays_reward?: number;           // 0, 1, o 5
    last_played_date?: string;        // "YYYY-MM-DD"
    stats?: {
      total_plays: number;
      total_lumis_won: number;
      golden_stars_captured: number;
    };
  };
}
```

### DailyGameClaimRequest

```typescript
interface DailyGameClaimRequest {
  star_id: string;    // "star_0" a "star_8"
  lumis_won: number;  // 0, 1, o 5
}
```

### DailyGameClaimResponse

```typescript
interface DailyGameClaimResponse {
  success: boolean;
  data: {
    lumis_added: number;      // 0, 1, o 5
    new_balance: number;      // Balance total actualizado
    play_id: number;          // ID de la jugada
  };
  message: string;            // Mensaje personalizado
}
```

### ErrorResponse

```typescript
interface ErrorResponse {
  success: boolean;           // false
  error: {
    code: string;
    message: string;
  };
}
```

---

## ⚠️ Códigos de Error

### HTTP Status Codes

| Código | Descripción | Causa |
|--------|-------------|-------|
| 200 | OK | Operación exitosa |
| 400 | Bad Request | Validación fallida (lumis_won o star_id inválidos) |
| 401 | Unauthorized | Token JWT inválido, expirado, o faltante |
| 409 | Conflict | Usuario ya jugó hoy (UNIQUE constraint violation) |
| 500 | Internal Server Error | Error de base de datos o error interno |

### Error Codes

| Code | HTTP Status | Descripción |
|------|-------------|-------------|
| `ERROR` | 400 | Error de validación genérico |
| `ALREADY_PLAYED_TODAY` | 409 | Usuario ya jugó hoy |
| `DATABASE_ERROR` | 500 | Error al acceder a la base de datos |

### Manejo de Errores en Cliente

```typescript
async function claimReward(starId: string, lumisWon: number) {
  try {
    const response = await fetch('/api/v4/daily-game/claim', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ star_id: starId, lumis_won: lumisWon }),
    });

    const data = await response.json();

    if (response.ok) {
      // Success (200)
      showSuccess(data.message);
      updateBalance(data.data.new_balance);
    } else if (response.status === 409) {
      // Already played today
      showWarning('Ya jugaste hoy. Vuelve mañana.');
    } else if (response.status === 400) {
      // Validation error
      showError(data.error.message);
    } else if (response.status === 401) {
      // Unauthorized
      redirectToLogin();
    } else {
      // Other errors
      showError('Error al procesar la jugada');
    }
  } catch (error) {
    showError('Error de conexión');
  }
}
```

---

## 🎲 Lógica de Negocio

### Reglas del Juego

1. **Una jugada por día**:
   - Cada usuario puede jugar exactamente **1 vez por día**
   - El "día" se calcula en zona horaria de **Panamá (UTC-5)**
   - La restricción está garantizada por UNIQUE constraint en BD: `(user_id, play_date)`

2. **9 estrellas para elegir**:
   - El usuario puede seleccionar entre 9 estrellas: `star_0` a `star_8`
   - El cliente decide qué estrella tiene qué premio (lógica en frontend)
   - El backend valida y registra el resultado

3. **3 tipos de recompensas**:
   - **Estrella vacía**: 0 Lümis (40% probabilidad recomendada)
   - **Estrella normal**: 1 Lümi (50% probabilidad recomendada)
   - **Estrella dorada**: 5 Lümis (10% probabilidad recomendada)

4. **Integración con sistema de rewards**:
   - Cada jugada se registra en `rewards.fact_daily_game_plays`
   - Si `lumis_won > 0`, se crea un registro en `rewards.fact_accumulations`
   - Un trigger automático actualiza `rewards.fact_balance_points`

### Zona Horaria

El sistema usa la zona horaria de **Panamá (UTC-5)** para calcular "hoy":

```rust
use chrono::Utc;
use chrono_tz::America::Panama;

let now_panama = Utc::now().with_timezone(&Panama);
let today = now_panama.date_naive(); // YYYY-MM-DD
```

**Ejemplo**:
- UTC: `2025-10-14 04:30:00` (4:30 AM)
- Panamá: `2025-10-13 23:30:00` (11:30 PM)
- `today` = `2025-10-13` ✓

Esto evita problemas donde un usuario cerca de medianoche podría jugar dos veces en diferentes "días" según UTC.

### Transacciones Atómicas

Todas las operaciones de claim se ejecutan en una **transacción atómica**:

```rust
let mut tx = pool.begin().await?;

// 1. Insertar jugada
sqlx::query!(/* INSERT fact_daily_game_plays */)
    .execute(&mut *tx)
    .await?;

// 2. Si lumis_won > 0, insertar acumulación
if request.lumis_won > 0 {
    sqlx::query!(/* INSERT fact_accumulations */)
        .execute(&mut *tx)
        .await?;
}

// 3. Commit (o Rollback si hay error)
tx.commit().await?;
```

**Ventajas**:
- ✅ Integridad de datos: jugada y acumulación se registran juntas
- ✅ No quedan registros huérfanos si falla la acumulación
- ✅ Rollback automático si hay cualquier error

### Estadísticas

Las estadísticas se calculan con una query optimizada usando CTEs:

```sql
WITH stats AS (
  SELECT 
    COUNT(*) as total_plays,
    SUM(lumis_won) as total_lumis_won,
    SUM(CASE WHEN lumis_won = 5 THEN 1 ELSE 0 END) as golden_stars
  FROM rewards.fact_daily_game_plays
  WHERE user_id = $1
)
SELECT * FROM stats;
```

### Probabilidades (Frontend)

El **cliente decide** el resultado antes de enviar el request. Probabilidades recomendadas:

```typescript
function calculateReward(): number {
  const roll = Math.random();
  
  if (roll < 0.10) {
    return 5; // 10% estrella dorada
  } else if (roll < 0.60) {
    return 1; // 50% estrella normal
  } else {
    return 0; // 40% estrella vacía
  }
}
```

**Nota**: El backend **acepta** el valor enviado pero **valida** que sea 0, 1, o 5. El UNIQUE constraint previene múltiples intentos.

---

## 🔒 Seguridad

### Validaciones del Backend

| Validación | Implementación | Resultado |
|------------|----------------|-----------|
| Autenticación JWT | Middleware `extract_current_user` | 401 si inválido |
| `lumis_won` ∈ {0,1,5} | Validación en `DailyGameClaimRequest::validate()` | 400 si inválido |
| `star_id` = `star_[0-8]` | Regex check | 400 si inválido |
| Una jugada por día | UNIQUE constraint `(user_id, play_date)` | 409 si duplicado |
| Transacciones atómicas | `pool.begin()`, `tx.commit()` | Rollback si error |

### Consideraciones de Seguridad

**❓ ¿El cliente puede hacer trampa?**

Sí, técnicamente un cliente modificado podría:
1. Siempre enviar `lumis_won = 5` (estrella dorada)
2. Intentar jugar múltiples veces

**🛡️ Mitigaciones implementadas (MVP)**:

1. **UNIQUE constraint**: Impide múltiples jugadas por día (garantizado por BD)
2. **Validación de valores**: Solo acepta 0, 1, o 5
3. **JWT obligatorio**: Requiere autenticación
4. **Transacciones**: Garantizan integridad

**🔮 Mitigaciones futuras (Fase 2)**:

1. **Análisis de patrones**: Detectar usuarios con tasa anormal de estrellas doradas
2. **Rate limiting**: Limitar requests por IP/usuario
3. **Backend decide resultado**: Endpoint `/reveal` que calcula el premio en servidor
4. **Auditoría**: Dashboard para revisar jugadores sospechosos

**Recomendación para producción**:

- **MVP (actual)**: Confiar en el cliente, monitorear patrones
- **Fase 2**: Backend decide el resultado para mayor seguridad

---

## 📱 Ejemplos de Integración

### Flutter / Dart

```dart
import 'package:http/http.dart' as http;
import 'dart:convert';

class DailyGameService {
  final String baseUrl = 'https://api.2factu.com/api/v4/daily-game';
  final String token;

  DailyGameService(this.token);

  // GET /status
  Future<DailyGameStatus> getStatus() async {
    final response = await http.get(
      Uri.parse('$baseUrl/status'),
      headers: {
        'Authorization': 'Bearer $token',
        'Content-Type': 'application/json',
      },
    );

    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      return DailyGameStatus.fromJson(data['data']);
    } else {
      throw Exception('Failed to load status');
    }
  }

  // POST /claim
  Future<DailyGameClaimResponse> claim(String starId, int lumisWon) async {
    final response = await http.post(
      Uri.parse('$baseUrl/claim'),
      headers: {
        'Authorization': 'Bearer $token',
        'Content-Type': 'application/json',
      },
      body: jsonEncode({
        'star_id': starId,
        'lumis_won': lumisWon,
      }),
    );

    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      return DailyGameClaimResponse.fromJson(data);
    } else if (response.statusCode == 409) {
      throw AlreadyPlayedException('Ya jugaste hoy');
    } else {
      throw Exception('Failed to claim reward');
    }
  }
}

// Models
class DailyGameStatus {
  final bool canPlayToday;
  final bool hasPlayedToday;
  final int? todaysReward;
  final DailyGameStats? stats;

  DailyGameStatus({
    required this.canPlayToday,
    required this.hasPlayedToday,
    this.todaysReward,
    this.stats,
  });

  factory DailyGameStatus.fromJson(Map<String, dynamic> json) {
    return DailyGameStatus(
      canPlayToday: json['can_play_today'],
      hasPlayedToday: json['has_played_today'],
      todaysReward: json['todays_reward'],
      stats: json['stats'] != null
          ? DailyGameStats.fromJson(json['stats'])
          : null,
    );
  }
}

class DailyGameStats {
  final int totalPlays;
  final int totalLumisWon;
  final int goldenStarsCaptured;

  DailyGameStats({
    required this.totalPlays,
    required this.totalLumisWon,
    required this.goldenStarsCaptured,
  });

  factory DailyGameStats.fromJson(Map<String, dynamic> json) {
    return DailyGameStats(
      totalPlays: json['total_plays'],
      totalLumisWon: json['total_lumis_won'],
      goldenStarsCaptured: json['golden_stars_captured'],
    );
  }
}

class DailyGameClaimResponse {
  final int lumisAdded;
  final int newBalance;
  final int playId;
  final String message;

  DailyGameClaimResponse({
    required this.lumisAdded,
    required this.newBalance,
    required this.playId,
    required this.message,
  });

  factory DailyGameClaimResponse.fromJson(Map<String, dynamic> json) {
    return DailyGameClaimResponse(
      lumisAdded: json['data']['lumis_added'],
      newBalance: json['data']['new_balance'],
      playId: json['data']['play_id'],
      message: json['message'] ?? '',
    );
  }
}

class AlreadyPlayedException implements Exception {
  final String message;
  AlreadyPlayedException(this.message);
}
```

### JavaScript / TypeScript

```typescript
interface DailyGameStatus {
  can_play_today: boolean;
  has_played_today: boolean;
  todays_reward?: number;
  last_played_date?: string;
  stats?: {
    total_plays: number;
    total_lumis_won: number;
    golden_stars_captured: number;
  };
}

interface DailyGameClaimRequest {
  star_id: string;
  lumis_won: number;
}

interface DailyGameClaimResponse {
  success: boolean;
  data: {
    lumis_added: number;
    new_balance: number;
    play_id: number;
  };
  message: string;
}

class DailyGameAPI {
  private baseUrl = 'https://api.2factu.com/api/v4/daily-game';
  private token: string;

  constructor(token: string) {
    this.token = token;
  }

  async getStatus(): Promise<DailyGameStatus> {
    const response = await fetch(`${this.baseUrl}/status`, {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();
    return result.data;
  }

  async claim(starId: string, lumisWon: number): Promise<DailyGameClaimResponse> {
    const response = await fetch(`${this.baseUrl}/claim`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        star_id: starId,
        lumis_won: lumisWon,
      }),
    });

    const result = await response.json();

    if (!response.ok) {
      if (response.status === 409) {
        throw new Error('Ya jugaste hoy. Vuelve mañana.');
      }
      throw new Error(result.error?.message || 'Error al reclamar recompensa');
    }

    return result;
  }

  // Helper: Calculate reward probability (client-side)
  calculateReward(): number {
    const roll = Math.random();
    
    if (roll < 0.10) {
      return 5; // 10% golden star
    } else if (roll < 0.60) {
      return 1; // 50% normal star
    } else {
      return 0; // 40% empty star
    }
  }
}

// Usage example
async function playDailyGame() {
  const api = new DailyGameAPI('your-jwt-token');

  try {
    // Check status
    const status = await api.getStatus();
    
    if (!status.can_play_today) {
      console.log('Ya jugaste hoy. Vuelve mañana.');
      return;
    }

    // User selects star (0-8)
    const selectedStar = 3;
    
    // Calculate reward (client-side)
    const lumisWon = api.calculateReward();
    
    // Claim reward
    const result = await api.claim(`star_${selectedStar}`, lumisWon);
    
    console.log(result.message);
    console.log(`Nuevo balance: ${result.data.new_balance} Lümis`);
    
  } catch (error) {
    console.error('Error:', error.message);
  }
}
```

### Python

```python
import requests
from typing import Optional, Dict, Any
from dataclasses import dataclass
import random

@dataclass
class DailyGameStats:
    total_plays: int
    total_lumis_won: int
    golden_stars_captured: int

@dataclass
class DailyGameStatus:
    can_play_today: bool
    has_played_today: bool
    todays_reward: Optional[int] = None
    last_played_date: Optional[str] = None
    stats: Optional[DailyGameStats] = None

@dataclass
class DailyGameClaimResponse:
    lumis_added: int
    new_balance: int
    play_id: int
    message: str

class DailyGameAPI:
    def __init__(self, token: str, base_url: str = 'https://api.2factu.com/api/v4/daily-game'):
        self.base_url = base_url
        self.headers = {
            'Authorization': f'Bearer {token}',
            'Content-Type': 'application/json'
        }
    
    def get_status(self) -> DailyGameStatus:
        """Get daily game status for authenticated user."""
        response = requests.get(
            f'{self.base_url}/status',
            headers=self.headers
        )
        response.raise_for_status()
        
        data = response.json()['data']
        
        stats = None
        if 'stats' in data and data['stats']:
            stats = DailyGameStats(**data['stats'])
        
        return DailyGameStatus(
            can_play_today=data['can_play_today'],
            has_played_today=data['has_played_today'],
            todays_reward=data.get('todays_reward'),
            last_played_date=data.get('last_played_date'),
            stats=stats
        )
    
    def claim(self, star_id: str, lumis_won: int) -> DailyGameClaimResponse:
        """Claim daily game reward."""
        payload = {
            'star_id': star_id,
            'lumis_won': lumis_won
        }
        
        response = requests.post(
            f'{self.base_url}/claim',
            headers=self.headers,
            json=payload
        )
        
        if response.status_code == 409:
            raise Exception('Ya jugaste hoy. Vuelve mañana.')
        
        response.raise_for_status()
        
        result = response.json()
        return DailyGameClaimResponse(
            lumis_added=result['data']['lumis_added'],
            new_balance=result['data']['new_balance'],
            play_id=result['data']['play_id'],
            message=result['message']
        )
    
    @staticmethod
    def calculate_reward() -> int:
        """Calculate reward based on probability (client-side)."""
        roll = random.random()
        
        if roll < 0.10:
            return 5  # 10% golden star
        elif roll < 0.60:
            return 1  # 50% normal star
        else:
            return 0  # 40% empty star

# Usage example
if __name__ == '__main__':
    api = DailyGameAPI(token='your-jwt-token')
    
    try:
        # Check status
        status = api.get_status()
        
        if not status.can_play_today:
            print('Ya jugaste hoy. Vuelve mañana.')
            exit()
        
        # User selects star
        selected_star = 3
        
        # Calculate reward
        lumis_won = api.calculate_reward()
        
        # Claim reward
        result = api.claim(f'star_{selected_star}', lumis_won)
        
        print(result.message)
        print(f'Nuevo balance: {result.new_balance} Lümis')
        
    except Exception as e:
        print(f'Error: {e}')
```

---

## 🗄️ Estructura de Base de Datos

### Tabla: rewards.fact_daily_game_plays

```sql
CREATE TABLE rewards.fact_daily_game_plays (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES dim_users(id),
    play_date DATE NOT NULL,
    play_time TIME NOT NULL,
    star_id VARCHAR(10) NOT NULL,
    lumis_won SMALLINT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT unique_user_play_date UNIQUE (user_id, play_date),
    CONSTRAINT check_lumis_won CHECK (lumis_won IN (0, 1, 5)),
    CONSTRAINT check_star_id CHECK (star_id ~ '^star_[0-8]$')
);

-- Indexes
CREATE INDEX idx_daily_game_user_date 
    ON rewards.fact_daily_game_plays(user_id, play_date DESC);
    
CREATE INDEX idx_daily_game_play_date 
    ON rewards.fact_daily_game_plays(play_date DESC);
```

### Tabla: rewards.dim_accumulations

```sql
-- Regla genérica para Daily Game
INSERT INTO rewards.dim_accumulations (id, name, points, valid_from, valid_to)
VALUES (10, 'daily_game', 0, '2025-01-01', '2099-12-31');
```

**Nota**: `points = 0` porque el valor real viene en el campo `quantity` de `fact_accumulations`.

### Queries Útiles

**Jugadas de hoy**:
```sql
SELECT * FROM rewards.fact_daily_game_plays
WHERE play_date = CURRENT_DATE
ORDER BY created_at DESC;
```

**Estadísticas de un usuario**:
```sql
SELECT 
    COUNT(*) as total_plays,
    SUM(lumis_won) as total_lumis_won,
    SUM(CASE WHEN lumis_won = 5 THEN 1 ELSE 0 END) as golden_stars
FROM rewards.fact_daily_game_plays
WHERE user_id = $1;
```

**Top jugadores**:
```sql
SELECT 
    user_id,
    COUNT(*) as total_plays,
    SUM(lumis_won) as total_lumis,
    SUM(CASE WHEN lumis_won = 5 THEN 1 ELSE 0 END) as golden_stars
FROM rewards.fact_daily_game_plays
GROUP BY user_id
ORDER BY total_lumis DESC
LIMIT 10;
```

**Distribución de recompensas**:
```sql
SELECT 
    lumis_won,
    COUNT(*) as count,
    COUNT(*) * 100.0 / SUM(COUNT(*)) OVER () as percentage
FROM rewards.fact_daily_game_plays
GROUP BY lumis_won
ORDER BY lumis_won;
```

---

## 🧪 Testing

### Casos de Prueba

#### Test 1: Status - Primera vez
```bash
curl -X GET "https://api.2factu.com/api/v4/daily-game/status" \
  -H "Authorization: Bearer $TOKEN"

# Expected: can_play_today=true, has_played_today=false
```

#### Test 2: Claim - Estrella dorada
```bash
curl -X POST "https://api.2factu.com/api/v4/daily-game/claim" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"star_id": "star_7", "lumis_won": 5}'

# Expected: 200 OK, lumis_added=5, message="¡Increíble! 🌟✨..."
```

#### Test 3: Status - Después de jugar
```bash
curl -X GET "https://api.2factu.com/api/v4/daily-game/status" \
  -H "Authorization: Bearer $TOKEN"

# Expected: can_play_today=false, has_played_today=true, todays_reward=5
```

#### Test 4: Claim - Intento duplicado
```bash
curl -X POST "https://api.2factu.com/api/v4/daily-game/claim" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"star_id": "star_3", "lumis_won": 1}'

# Expected: 409 Conflict, "Ya jugaste hoy..."
```

#### Test 5: Validación - lumis_won inválido
```bash
curl -X POST "https://api.2factu.com/api/v4/daily-game/claim" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"star_id": "star_2", "lumis_won": 10}'

# Expected: 400 Bad Request, "Invalid lumis_won value: 10..."
```

#### Test 6: Validación - star_id inválido
```bash
curl -X POST "https://api.2factu.com/api/v4/daily-game/claim" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"star_id": "star_99", "lumis_won": 1}'

# Expected: 400 Bad Request, "Invalid star_id: star_99..."
```

### Resultados de Testing

Ver `DAILY_GAME_TESTING_RESULTS.md` para resultados completos de las pruebas ejecutadas.

---

## 📊 Métricas y Monitoreo

### Métricas Recomendadas

1. **Jugadores diarios**: `COUNT(DISTINCT user_id) WHERE play_date = CURRENT_DATE`
2. **Tasa de estrellas doradas**: `(golden_stars / total_plays) * 100`
3. **Lümis distribuidos por día**: `SUM(lumis_won) GROUP BY play_date`
4. **Usuarios activos**: Usuarios que jugaron en últimos 7 días
5. **Tasa de retención**: Usuarios que jugaron hoy y ayer

### Queries de Métricas

```sql
-- Jugadores diarios
SELECT 
    play_date,
    COUNT(DISTINCT user_id) as daily_players
FROM rewards.fact_daily_game_plays
WHERE play_date >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY play_date
ORDER BY play_date DESC;

-- Tasa de estrellas doradas
SELECT 
    COUNT(*) FILTER (WHERE lumis_won = 5) * 100.0 / COUNT(*) as golden_rate,
    COUNT(*) FILTER (WHERE lumis_won = 1) * 100.0 / COUNT(*) as normal_rate,
    COUNT(*) FILTER (WHERE lumis_won = 0) * 100.0 / COUNT(*) as empty_rate
FROM rewards.fact_daily_game_plays;

-- Lümis distribuidos por día
SELECT 
    play_date,
    SUM(lumis_won) as lumis_distributed,
    COUNT(*) as plays
FROM rewards.fact_daily_game_plays
WHERE play_date >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY play_date
ORDER BY play_date DESC;

-- Top 10 jugadores
SELECT 
    u.id,
    u.email,
    COUNT(*) as total_plays,
    SUM(d.lumis_won) as total_lumis,
    SUM(CASE WHEN d.lumis_won = 5 THEN 1 ELSE 0 END) as golden_stars
FROM rewards.fact_daily_game_plays d
JOIN dim_users u ON d.user_id = u.id
GROUP BY u.id, u.email
ORDER BY total_lumis DESC
LIMIT 10;
```

---

## 🚀 Roadmap Futuro

### Fase 2: Rachas Consecutivas

- [ ] Campo `streak` en tabla (días consecutivos)
- [ ] Calcular racha al consultar status
- [ ] Bonus por rachas (7 días → x2, 14 días → x3, 30 días → x5)
- [ ] Notificación push si racha en riesgo

### Fase 3: Estadísticas Avanzadas

- [ ] Endpoint `/history` (últimos 30 días)
- [ ] Gráficos de jugadas
- [ ] Leaderboard global
- [ ] Badges por logros

### Fase 4: Seguridad Mejorada

- [ ] Backend decide resultado (endpoint `/reveal`)
- [ ] Análisis de patrones sospechosos
- [ ] Rate limiting por IP
- [ ] Dashboard de auditoría

### Fase 5: Gamificación

- [ ] Misiones diarias adicionales
- [ ] Eventos especiales (x2 Lümis los domingos)
- [ ] Constelaciones especiales por temporada

---

## 📝 Changelog

### v1.0.0 (2025-10-13)

**✅ MVP Completado**

- ✅ GET `/status` - Obtener estado del juego
- ✅ POST `/claim` - Reclamar recompensa
- ✅ Autenticación JWT obligatoria
- ✅ Validaciones robustas
- ✅ UNIQUE constraint previene duplicados
- ✅ Integración con sistema de rewards
- ✅ Zona horaria de Panamá
- ✅ Transacciones atómicas
- ✅ Estadísticas básicas
- ✅ Mensajes personalizados
- ✅ Testing completo
- ✅ Documentación completa

---

## 🆘 Soporte

### Issues Conocidos

Ninguno

### Documentación Adicional

- `DAILY_GAME_IMPLEMENTATION_SUMMARY.md` - Resumen de implementación
- `DAILY_GAME_TESTING_RESULTS.md` - Resultados de testing
- `DAILY_GAME_FLUTTER_INTEGRATION.md` - Guía de integración Flutter
- `daily_game_setup.sql` - Script SQL de instalación
- `verify_daily_game_data.sql` - Script de verificación

### Contacto

Para preguntas o issues, contactar al equipo de desarrollo.

---

**Última actualización**: 2025-10-13  
**Versión API**: v4  
**Estado**: ✅ Production Ready  
**Autor**: AI Assistant + Team
