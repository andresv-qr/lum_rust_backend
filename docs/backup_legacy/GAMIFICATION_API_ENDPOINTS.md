# 🚀 API Endpoints - Sistema de Gamificación v4

> **Última actualización:** 2025-11-27
> **Estado:** Documentación sincronizada con implementación real

## Índice
1. [Dashboard y Overview](#1-dashboard-y-overview)
2. [Sistema de Rachas](#2-sistema-de-rachas)
3. [Misiones y Desafíos](#3-misiones-y-desafíos)
4. [Eventos Temporales](#4-eventos-temporales)
5. [Logros y Achievements](#5-logros-y-achievements)
6. [Niveles y Progresión](#6-niveles-y-progresión)
7. [Leaderboards y Competencia](#7-leaderboards-y-competencia)
8. [Track Actions](#8-track-actions)

---

## 1. Dashboard y Overview

### `GET /api/v4/gamification/dashboard`
**Descripción:** Dashboard principal de gamificación del usuario. Devuelve toda la información de niveles, rachas activas y progreso.

**Headers:**
```
Authorization: Bearer {jwt_token}
Content-Type: application/json
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user_id": 1,
    "email": "usuario@ejemplo.com",
    "total_lumis": 708,
    "current_level": 3,
    "level_name": "Explorador",
    "level_description": "Explorador - Basado en 45 facturas",
    "level_color": "#4CAF50",
    "level_benefits": {
      "multiplier": 1.1,
      "perks": ["descuento_5%", "acceso_ofertas_exclusivas"]
    },
    "next_level_hint": "Faltan 15 facturas para Aventurero",
    "lumis_to_next_level": 15,
    "next_level_name": "Aventurero",
    "active_streaks": [
      {
        "type": "daily_login",
        "current": 5,
        "max": 12,
        "last_date": "2025-11-26"
      },
      {
        "type": "consistent_month",
        "current": 2,
        "max": 4,
        "last_date": "2025-11-26"
      }
    ],
    "active_missions_count": 0,
    "completed_missions_count": 0,
    "total_achievements": 0,
    "recent_activity": []
  },
  "request_id": "uuid-here",
  "timestamp": "2025-11-27T10:00:00Z",
  "execution_time_ms": 5,
  "cached": false
}
```

**Campos de Respuesta:**

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `user_id` | int | ID del usuario |
| `email` | string | Email del usuario |
| `total_lumis` | int | Balance total de Lümis (facturas registradas) |
| `current_level` | int | Nivel actual (1-10) |
| `level_name` | string | Nombre del nivel actual |
| `level_description` | string | Descripción contextual del nivel |
| `level_color` | string | Color hex para UI |
| `level_benefits` | object | Beneficios del nivel (multiplicadores, perks) |
| `next_level_hint` | string | Mensaje sobre próximo nivel |
| `lumis_to_next_level` | int | Facturas restantes para subir de nivel |
| `next_level_name` | string | Nombre del próximo nivel |
| `active_streaks` | array | Rachas activas del usuario |
| `active_missions_count` | int | Cantidad de misiones activas |
| `completed_missions_count` | int | Misiones completadas |
| `total_achievements` | int | Total de logros desbloqueados |
| `recent_activity` | array | Actividad reciente |

**Tipos de Rachas (`active_streaks`):**

| Tipo | Descripción | Recompensa |
|------|-------------|------------|
| `daily_login` | Login 7 días consecutivos | 1 Lümi al día 7, resetea a 1 |
| `consistent_month` | 4 semanas consecutivas con facturas | Achievement + recompensa |

---

## 2. Sistema de Rachas

Las rachas se obtienen a través del endpoint `/dashboard` en el campo `active_streaks`.

### Racha de Login Diario (`daily_login`)
- **Ciclo:** 7 días
- **Días 1-6:** Solo progreso, 0 Lümis
- **Día 7:** 1 Lümi + Achievement `week_perfect`
- **Día 8:** Contador resetea automáticamente a 1 (ciclo infinito)

### Racha de Facturas Semanales (`consistent_month`)
- **Requisito:** Subir al menos 1 factura por semana
- **Meta:** 4 semanas consecutivas
- **Al completar:** Achievement `consistent_month` + recompensa
- **Se actualiza automáticamente** al subir facturas

### Estructura de cada streak:
```json
{
  "type": "daily_login",
  "current": 5,
  "max": 12,
  "last_date": "2025-11-26"
}
```

| Campo | Descripción |
|-------|-------------|
| `type` | Tipo de racha |
| `current` | Días/semanas consecutivas actuales |
| `max` | Máximo histórico alcanzado |
| `last_date` | Última fecha de actividad |

---

## 3. Track Actions (Registrar Acciones)

### `POST /api/v4/gamification/track`
**Descripción:** Endpoint principal para registrar acciones del usuario y obtener recompensas.

**Headers:**
```
Authorization: Bearer {jwt_token}
Content-Type: application/json
```

**Request:**
```json
{
  "action": "daily_login",
  "channel": "mobile_app",
  "metadata": {}
}
```

**Acciones válidas:**
| Action | Descripción | Recompensa |
|--------|-------------|------------|
| `daily_login` | Login diario | 0-1 Lümi (1 al día 7) |
| `invoice_upload` | Subir factura | Variable según categoría |
| `survey_complete` | Completar encuesta | Variable (en metadata) |

**Response exitosa:**
```json
{
  "success": true,
  "data": {
    "lumis_earned": 1,
    "total_lumis": 709,
    "xp_earned": 1,
    "current_level": 3,
    "level_name": "Explorador",
    "streaks": {
      "current_streak": 7,
      "lumis_earned": 1,
      "max_streak": 12,
      "next_milestone": 7,
      "achievement_unlocked": "week_perfect",
      "message": "🏆 ¡Semana perfecta! +1 Lümi. Contador resetea para nueva semana",
      "days_until_reward": 0,
      "weekly_cycle": true
    },
    "achievements_unlocked": [],
    "active_events": [],
    "message": "Ganaste 1 Lümis"
  },
  "request_id": "uuid-here",
  "timestamp": "2025-11-27T10:00:00Z",
  "execution_time_ms": 15,
  "cached": false
}
```

**Response cuando ya se registró hoy:**
```json
{
  "success": true,
  "data": {
    "lumis_earned": 0,
    "streaks": {
      "current_streak": 5,
      "already_claimed": true,
      "next_reward_day": 7,
      "lumis_at_day_7": 1,
      "message": "Ya registraste tu ingreso hoy"
    }
  }
}
```

---

## 4. Misiones y Desafíos

### `GET /api/v4/gamification/missions`
**Descripción:** Obtiene misiones del usuario.

**Headers:**
```
Authorization: Bearer {jwt_token}
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "mission_code": "daily_invoices_3",
      "mission_name": "Subir 3 facturas hoy",
      "mission_type": "daily",
      "description": "Sube 3 facturas para completar esta misión",
      "current_progress": 1,
      "target_count": 3,
      "reward_lumis": 50,
      "due_date": "2025-11-27",
      "status": "active",
      "progress_percentage": 33.33
    }
  ],
  "request_id": "uuid-here",
  "timestamp": "2025-11-27T10:00:00Z",
  "execution_time_ms": 3,
  "cached": false
}
```

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `mission_code` | string | Código único de la misión |
| `mission_name` | string | Nombre para mostrar |
| `mission_type` | string | `daily`, `weekly`, `monthly`, `special` |
| `description` | string | Descripción detallada |
| `current_progress` | int | Progreso actual |
| `target_count` | int | Meta a alcanzar |
| `reward_lumis` | int | Recompensa en Lümis |
| `due_date` | date | Fecha límite |
| `status` | string | `active`, `completed`, `expired` |
| `progress_percentage` | float | Porcentaje de progreso (0-100) |

---

## 5. Eventos Temporales

### `GET /api/v4/gamification/events`
**Descripción:** Obtiene eventos activos y próximos (Happy Hours, eventos especiales).

**Headers:**
```
Authorization: Bearer {jwt_token}
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "event_code": "happy_hour_evening",
      "event_name": "Happy Hour Nocturno",
      "event_type": "happy_hour",
      "starts_in_minutes": -30,
      "ends_in_minutes": 90,
      "multiplier": "2.0",
      "description": "¡Doble Lümis en todas las acciones!",
      "is_active_now": true
    }
  ],
  "request_id": "uuid-here",
  "timestamp": "2025-11-27T18:30:00Z",
  "execution_time_ms": 2,
  "cached": false
}
```

| Campo | Descripción |
|-------|-------------|
| `starts_in_minutes` | Negativo = ya comenzó hace X minutos |
| `ends_in_minutes` | Positivo = termina en X minutos |
| `is_active_now` | Si el evento está activo ahora |
| `multiplier` | Multiplicador de Lümis durante el evento |

---

## 6. Logros y Achievements

### `GET /api/v4/gamification/achievements`
**Descripción:** Obtiene logros disponibles y desbloqueados del usuario.

**Headers:**
```
Authorization: Bearer {jwt_token}
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "achievement_code": "week_perfect",
      "achievement_name": "Semana Perfecta",
      "description": "Loguéate 7 días consecutivos",
      "category": "streak",
      "rarity": "common",
      "points_reward": 1,
      "unlocked_at": "2025-11-20T15:30:00Z",
      "progress_current": 7,
      "progress_target": 7,
      "is_unlocked": true
    },
    {
      "achievement_code": "consistent_month",
      "achievement_name": "Mes Consistente",
      "description": "Sube facturas 4 semanas seguidas",
      "category": "invoice",
      "rarity": "rare",
      "points_reward": 10,
      "unlocked_at": null,
      "progress_current": 2,
      "progress_target": 4,
      "is_unlocked": false
    }
  ]
}
```

---

## 7. Leaderboard

### `GET /api/v4/gamification/leaderboard`
**Descripción:** Ranking de usuarios.

**Headers:**
```
Authorization: Bearer {jwt_token}
```

**Query Parameters:**
- `period` (optional): `daily`, `weekly`, `monthly`, `all_time` (default: `weekly`)

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "user_id": 1,
      "display_name": "Usuario***",
      "total_lumis": 1500,
      "current_level": 5,
      "level_name": "Aventurero",
      "rank": 1
    },
    {
      "user_id": 2,
      "display_name": "Otro***",
      "total_lumis": 1200,
      "current_level": 4,
      "level_name": "Explorador",
      "rank": 2
    }
  ]
}
```

---

## 8. Mecánicas del Sistema

### `GET /api/v4/gamification/mechanics`
**Descripción:** Información sobre las mecánicas de gamificación (para mostrar en UI de ayuda).

**Headers:**
```
Authorization: Bearer {jwt_token}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "levels": [
      {"level": 1, "name": "Chispa Lüm", "min_invoices": 0},
      {"level": 2, "name": "Novato", "min_invoices": 10},
      {"level": 3, "name": "Explorador", "min_invoices": 30}
    ],
    "streaks": {
      "daily_login": {
        "description": "Ingresa 7 días seguidos para ganar 1 Lümi",
        "cycle": 7,
        "reward_day": 7
      },
      "consistent_month": {
        "description": "Sube facturas 4 semanas seguidas",
        "cycle": 4,
        "reward_week": 4
      }
    }
  }
}
```

---

## Resumen de Endpoints Implementados

| Endpoint | Método | Descripción |
|----------|--------|-------------|
| `/api/v4/gamification/dashboard` | GET | Dashboard completo con streaks |
| `/api/v4/gamification/track` | POST | Registrar acciones (login, factura, encuesta) |
| `/api/v4/gamification/missions` | GET | Misiones del usuario |
| `/api/v4/gamification/events` | GET | Eventos activos (Happy Hours) |
| `/api/v4/gamification/achievements` | GET | Logros disponibles y desbloqueados |
| `/api/v4/gamification/leaderboard` | GET | Ranking de usuarios |
| `/api/v4/gamification/mechanics` | GET | Info de mecánicas para UI |

---

## Notas de Implementación

### Autenticación
Todos los endpoints requieren JWT válido en header `Authorization: Bearer {token}`.

### Rachas
- **No hay endpoint `/streaks` separado** - usar `/dashboard` que incluye `active_streaks`
- El login diario se registra con `POST /track` action `"daily_login"`
- Las facturas actualizan `consistent_month` automáticamente via trigger

### Errores Comunes
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid action type"
  }
}
```

Códigos: `VALIDATION_ERROR` (400), `UNAUTHORIZED` (401), `NOT_FOUND` (404), `DATABASE_ERROR` (500)
  "mission_code": "weekly_surveys_5"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "mission_accepted": true,
    "mission_id": 789,
    "due_date": "2025-09-03T23:59:59Z",
    "message": "Misión aceptada. ¡Tienes 7 días para completarla!"
  }
}
```

### `POST /api/v4/gamification/missions/claim`
**Descripción:** Reclama recompensa de misión completada.

**Request:**
```json
{
  "mission_id": 456
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "reward_claimed": true,
    "lumis_earned": 50,
    "bonus_earned": 0,
    "new_lumis_balance": 5480,
    "xp_earned": 25,
    "achievement_unlocked": null
  }
}
```

---

## 4. Eventos Temporales

### `GET /api/v4/gamification/events`
**Descripción:** Eventos activos y próximos.

**Query Parameters:**
- `status` (optional): `active`, `upcoming`, `ended`
- `type` (optional): `daily`, `flash`, `seasonal`, `tournament`

**Response:**
```json
{
  "success": true,
  "data": {
    "active_events": [
      {
        "event_id": 123,
        "event_code": "happy_hour_evening",
        "event_name": "Happy Hour Nocturno",
        "event_type": "daily",
        "multiplier": 2.0,
        "bonus_lumis": 0,
        "start_date": "2025-08-27T18:00:00Z",
        "end_date": "2025-08-27T20:00:00Z",
        "time_remaining": "1h 23m",
        "applicable_actions": ["invoice_upload", "survey_complete"],
        "user_participations": 3,
        "lumis_earned_in_event": 150
      }
    ],
    "upcoming_events": [
      {
        "event_code": "weekend_bonus",
        "event_name": "Bonus Fin de Semana",
        "starts_in": "2 days",
        "start_date": "2025-08-30T00:00:00Z",
        "multiplier": 1.5,
        "description": "50% bonus en todas las acciones durante el fin de semana"
      }
    ],
    "user_qualifies_for": ["premium_events", "loyal_user_events"]
  }
}
```

### `POST /api/v4/gamification/events/join`
**Descripción:** Unirse a un evento específico (si requiere registro).

**Request:**
```json
{
  "event_id": 456
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "joined_event": true,
    "event_name": "Torneo Semanal",
    "participation_id": 789,
    "message": "¡Te has unido al torneo! Comienza en 2 horas."
  }
}
```

### `GET /api/v4/gamification/events/{event_id}/leaderboard`
**Descripción:** Leaderboard específico de un evento.

**Response:**
```json
{
  "success": true,
  "data": {
    "event_name": "Happy Hour Nocturno",
    "user_rank": 15,
    "user_score": 235,
    "total_participants": 1247,
    "leaderboard": [
      {
        "rank": 1,
        "user_id": 456,
        "username": "PowerUser2025",
        "score": 850,
        "actions_completed": 17
      }
    ],
    "rewards": {
      "top_1": "500 Lümis + Badge Oro",
      "top_10": "200 Lümis + Badge Plata",
      "top_100": "50 Lümis + Badge Bronce"
    }
  }
}
```

---

## 5. Logros y Achievements

### `GET /api/v4/gamification/achievements`
**Descripción:** Logros desbloqueados y disponibles.

**Query Parameters:**
- `category` (optional): `invoices`, `surveys`, `social`, `streaks`
- `status` (optional): `unlocked`, `locked`, `claimed`, `unclaimed`

**Response:**
```json
{
  "success": true,
  "data": {
    "unlocked_achievements": [
      {
        "achievement_id": 12,
        "achievement_code": "survey_master",
        "achievement_name": "Survey Master",
        "description": "Completa 100 encuestas",
        "category": "surveys",
        "difficulty": "gold",
        "icon_url": "/icons/survey_master.png",
        "unlocked_at": "2025-08-26T15:30:00Z",
        "reward_lumis": 200,
        "is_claimed": false,
        "progress_at_unlock": {
          "surveys_completed": 100,
          "accuracy_rate": 95
        }
      }
    ],
    "locked_achievements": [
      {
        "achievement_code": "platinum_collector",
        "achievement_name": "Platinum Collector",
        "description": "Alcanza nivel Platinum",
        "category": "progression",
        "difficulty": "platinum",
        "current_progress": 75,
        "required_progress": 100,
        "is_hidden": false
      }
    ],
    "categories": {
      "invoices": { "unlocked": 5, "total": 12 },
      "surveys": { "unlocked": 8, "total": 15 },
      "social": { "unlocked": 3, "total": 8 },
      "streaks": { "unlocked": 4, "total": 10 }
    },
    "completion_percentage": 67,
    "unclaimed_rewards": 3
  }
}
```

### `POST /api/v4/gamification/achievements/claim`
**Descripción:** Reclama recompensa de achievement desbloqueado.

**Request:**
```json
{
  "achievement_id": 12
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "achievement_claimed": true,
    "achievement_name": "Survey Master",
    "lumis_earned": 200,
    "xp_earned": 100,
    "new_lumis_balance": 5680,
    "special_unlock": "Golden Survey Badge"
  }
}
```

---

## 6. Niveles y Progresión

### `GET /api/v4/gamification/progression`
**Descripción:** Información de nivel y progresión del usuario.

**Response:**
```json
{
  "success": true,
  "data": {
    "current_level": {
      "level_number": 5,
      "level_name": "Silver Hunter",
      "level_color": "#C0C0C0",
      "icon_url": "/icons/silver_hunter.png",
      "current_xp": 1250,
      "min_xp": 1000,
      "max_xp": 1999,
      "progress_percentage": 25
    },
    "next_level": {
      "level_number": 6,
      "level_name": "Silver Elite",
      "xp_required": 749,
      "new_benefits": [
        "15% bonus en Lümis por factura",
        "Acceso a misiones VIP",
        "Multiplicador de racha x1.3"
      ]
    },
    "xp_sources": {
      "invoices": 450,
      "surveys": 320,
      "achievements": 280,
      "streaks": 200
    },
    "prestige": {
      "available": false,
      "required_level": 20,
      "current_max_level": 15,
      "benefits": "Reinicio con bonus permanente +10%"
    },
    "level_benefits": {
      "lumis_multiplier": 1.15,
      "daily_missions_limit": 5,
      "freeze_tokens_per_week": 2,
      "special_events": true
    }
  }
}
```

### `POST /api/v4/gamification/progression/prestige`
**Descripción:** Realizar prestige (reiniciar progresión con bonus permanente).

**Response:**
```json
{
  "success": true,
  "data": {
    "prestige_completed": true,
    "new_prestige_level": 1,
    "permanent_bonus": 0.10,
    "starting_level": 1,
    "starting_xp": 0,
    "prestige_rewards": {
      "lumis": 1000,
      "special_badge": "Prestige Star",
      "permanent_multiplier": 1.10
    }
  }
}
```

---

## 7. Leaderboards y Competencia

### `GET /api/v4/gamification/leaderboards`
**Descripción:** Rankings y leaderboards disponibles.

**Query Parameters:**
- `type`: `weekly_invoices`, `monthly_surveys`, `daily_activity`, `streak_leaders`
- `period` (optional): `current`, `previous`, `all_time`
- `limit` (optional): número de posiciones a mostrar (default: 50)

**Response:**
```json
{
  "success": true,
  "data": {
    "leaderboard_type": "weekly_invoices",
    "period": "current",
    "period_start": "2025-08-25",
    "period_end": "2025-08-31",
    "user_position": {
      "rank": 23,
      "score": 45,
      "user_id": 123,
      "username": "CurrentUser"
    },
    "top_rankings": [
      {
        "rank": 1,
        "user_id": 456,
        "username": "InvoiceKing",
        "score": 127,
        "avatar_url": "/avatars/user456.png",
        "level_name": "Gold Master",
        "streak_bonus": "🔥 15 días"
      }
    ],
    "nearby_rankings": [
      {
        "rank": 21,
        "user_id": 789,
        "username": "NearbyUser1",
        "score": 47
      },
      {
        "rank": 22,
        "user_id": 101,
        "username": "NearbyUser2", 
        "score": 46
      }
    ],
    "rewards": {
      "rank_1_3": "1000 Lümis + Corona Dorada",
      "rank_4_10": "500 Lümis + Medalla Plata",
      "rank_11_50": "200 Lümis + Medalla Bronce",
      "participation": "50 Lümis"
    },
    "time_remaining": "3 days 14 hours"
  }
}
```

### `GET /api/v4/gamification/leaderboards/history`
**Descripción:** Historial de posiciones en leaderboards.

**Response:**
```json
{
  "success": true,
  "data": {
    "user_history": [
      {
        "period": "2025-08-18 to 2025-08-24",
        "leaderboard_type": "weekly_invoices",
        "final_rank": 15,
        "final_score": 67,
        "reward_earned": "200 Lümis + Medalla Bronce"
      }
    ],
    "best_performances": {
      "highest_rank": 8,
      "best_leaderboard": "monthly_surveys",
      "total_rewards_earned": "3400 Lümis"
    }
  }
}
```

---

## 8. Sistema Social

### `GET /api/v4/gamification/social/friends`
**Descripción:** Lista de amigos y sus estadísticas.

**Response:**
```json
{
  "success": true,
  "data": {
    "friends": [
      {
        "user_id": 456,
        "username": "BestFriend",
        "avatar_url": "/avatars/user456.png",
        "level_name": "Gold Master",
        "current_streak": 12,
        "last_active": "2025-08-27T10:30:00Z",
        "weekly_score": 89,
        "status": "online"
      }
    ],
    "friend_requests": {
      "pending_received": 2,
      "pending_sent": 1
    },
    "social_stats": {
      "total_friends": 15,
      "active_friends_today": 8,
      "friends_ahead_in_leaderboard": 3,
      "lumis_from_referrals": 500
    }
  }
}
```

### `POST /api/v4/gamification/social/friend-request`
**Descripción:** Enviar solicitud de amistad.

**Request:**
```json
{
  "target_user_id": 789
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "request_sent": true,
    "target_username": "NewFriend",
    "message": "Solicitud de amistad enviada"
  }
}
```

### `POST /api/v4/gamification/social/refer`
**Descripción:** Referir nuevo usuario.

**Request:**
```json
{
  "email": "friend@example.com",
  "message": "¡Únete a la app y gana Lümis!"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "referral_sent": true,
    "referral_code": "REF_USER123_2025",
    "referral_bonus": {
      "for_referrer": "100 Lümis cuando se registre",
      "for_referred": "50 Lümis bonus de bienvenida"
    }
  }
}
```

### `GET /api/v4/gamification/social/activity-feed`
**Descripción:** Feed de actividad de amigos.

**Response:**
```json
{
  "success": true,
  "data": {
    "activities": [
      {
        "activity_id": 123,
        "user_id": 456,
        "username": "BestFriend",
        "activity_type": "achievement_unlocked",
        "activity_data": {
          "achievement_name": "Survey Master",
          "difficulty": "gold"
        },
        "timestamp": "2025-08-27T14:30:00Z",
        "can_congratulate": true
      }
    ]
  }
}
```

---

## 9. Equipos y Torneos

### `GET /api/v4/gamification/teams/discover`
**Descripción:** Descubrir equipos disponibles para unirse.

**Query Parameters:**
- `team_type` (optional): `casual`, `competitive`, `corporate`
- `has_space` (optional): `true` para equipos con cupo disponible

**Response:**
```json
{
  "success": true,
  "data": {
    "available_teams": [
      {
        "team_id": 123,
        "team_name": "Lumis Hunters",
        "team_type": "competitive",
        "current_members": 8,
        "max_members": 10,
        "team_level": 15,
        "avg_member_level": 12,
        "weekly_score": 2340,
        "captain": {
          "username": "TeamCaptain",
          "level_name": "Platinum Elite"
        },
        "requirements": "Nivel mínimo: Gold",
        "can_join": true
      }
    ],
    "user_team": {
      "team_id": 456,
      "team_name": "My Current Team",
      "role": "member"
    }
  }
}
```

### `POST /api/v4/gamification/teams/join`
**Descripción:** Unirse a un equipo.

**Request:**
```json
{
  "team_id": 123
}
```

### `GET /api/v4/gamification/teams/competitions`
**Descripción:** Competencias de equipos activas y próximas.

**Response:**
```json
{
  "success": true,
  "data": {
    "active_competitions": [
      {
        "competition_id": 789,
        "competition_name": "Torneo Semanal de Equipos",
        "competition_type": "tournament",
        "start_date": "2025-08-26T00:00:00Z",
        "end_date": "2025-09-02T23:59:59Z",
        "participating_teams": 64,
        "user_team_rank": 12,
        "user_team_score": 1850,
        "prize_pool": "10,000 Lümis totales"
      }
    ]
  }
}
```

---

## 10. Notificaciones

### `GET /api/v4/gamification/notifications/preferences`
**Descripción:** Obtener preferencias de notificación del usuario.

**Response:**
```json
{
  "success": true,
  "data": {
    "push_enabled": true,
    "email_enabled": true,
    "sms_enabled": false,
    "quiet_hours": {
      "start": "22:00",
      "end": "08:00"
    },
    "frequency_preference": "normal",
    "timezone": "America/Panama",
    "language_preference": "es",
    "notification_types": {
      "streak_reminders": true,
      "mission_updates": true,
      "event_notifications": true,
      "achievement_unlocks": true,
      "friend_activity": false,
      "leaderboard_updates": true
    }
  }
}
```

### `PATCH /api/v4/gamification/notifications/preferences`
**Descripción:** Actualizar preferencias de notificación.

**Request:**
```json
{
  "push_enabled": true,
  "quiet_hours": {
    "start": "23:00",
    "end": "07:00"
  },
  "notification_types": {
    "friend_activity": true
  }
}
```

### `GET /api/v4/gamification/notifications/history`
**Descripción:** Historial de notificaciones del usuario.

**Query Parameters:**
- `limit` (optional): número de notificaciones (default: 20)
- `type` (optional): filtrar por tipo de notificación

**Response:**
```json
{
  "success": true,
  "data": {
    "notifications": [
      {
        "notification_id": 123,
        "notification_type": "streak_reminder",
        "title": "¡No pierdas tu racha!",
        "message": "Tienes 2 horas para mantener tu racha de 7 días",
        "sent_at": "2025-08-27T20:00:00Z",
        "was_opened": true,
        "action_taken": "opened_app"
      }
    ],
    "unread_count": 3
  }
}
```

---

## 11. Combos y Chains

### `GET /api/v4/gamification/combos/active`
**Descripción:** Combos activos del usuario.

**Response:**
```json
{
  "success": true,
  "data": {
    "active_combos": [
      {
        "combo_id": 123,
        "combo_name": "Triple Threat",
        "combo_type": "simultaneous",
        "current_step": 2,
        "steps_required": 3,
        "time_remaining": "4h 30m",
        "steps_completed": [
          {
            "step": 1,
            "action": "invoice_upload",
            "completed_at": "2025-08-27T15:30:00Z"
          },
          {
            "step": 2,
            "action": "survey_complete",
            "completed_at": "2025-08-27T16:45:00Z"
          }
        ],
        "next_step": {
          "action": "daily_login",
          "description": "Inicia sesión mañana para completar el combo"
        },
        "reward": {
          "multiplier": 3.0,
          "bonus_lumis": 200,
          "special_unlock": "Combo Master Badge"
        }
      }
    ],
    "available_combos": [
      {
        "combo_code": "weekend_warrior",
        "combo_name": "Weekend Warrior",
        "description": "5 acciones diferentes en fin de semana",
        "reward_preview": "5x multiplier + 500 Lümis",
        "can_start": true
      }
    ]
  }
}
```

### `POST /api/v4/gamification/combos/start`
**Descripción:** Iniciar un nuevo combo.

**Request:**
```json
{
  "combo_code": "weekend_warrior"
}
```

---

## 12. Anti-Fraude y Seguridad

### `GET /api/v4/gamification/security/status`
**Descripción:** Estado de seguridad de la cuenta del usuario.

**Response:**
```json
{
  "success": true,
  "data": {
    "account_status": "good_standing",
    "trust_score": 95,
    "recent_flags": 0,
    "verification_level": "verified",
    "last_security_check": "2025-08-27T12:00:00Z",
    "active_restrictions": [],
    "security_tips": [
      "Tu cuenta está en perfecto estado",
      "Continúa con tu actividad normal"
    ]
  }
}
```

### `POST /api/v4/gamification/security/report`
**Descripción:** Reportar actividad sospechosa.

**Request:**
```json
{
  "report_type": "suspicious_user",
  "target_user_id": 456,
  "description": "Usuario subiendo facturas duplicadas",
  "evidence": {
    "screenshot_urls": ["/evidence/screenshot1.png"]
  }
}
```

---

## 13. Administración

### `GET /api/v4/admin/gamification/analytics`
**Descripción:** Analytics del sistema de gamificación (solo admin).

**Headers:**
```
Authorization: Bearer {admin_jwt_token}
X-Admin-Role: super_admin
```

**Response:**
```json
{
  "success": true,
  "data": {
    "engagement_metrics": {
      "daily_active_users": 2547,
      "weekly_retention": 78.5,
      "avg_session_duration": "12m 34s",
      "gamification_adoption": 89.2
    },
    "event_performance": [
      {
        "event_code": "happy_hour_evening",
        "participations_today": 1247,
        "lumis_distributed": 15630,
        "engagement_lift": 34.5
      }
    ],
    "fraud_detection": {
      "signals_detected_today": 23,
      "auto_resolved": 18,
      "pending_review": 5,
      "false_positive_rate": 2.1
    }
  }
}
```

### `POST /api/v4/admin/gamification/events/bulk-create`
**Descripción:** Crear eventos masivamente (solo admin).

**Request:**
```json
{
  "events": [
    {
      "event_code": "holiday_bonus_2025",
      "event_name": "Bonus Navideño 2025",
      "event_type": "seasonal",
      "start_date": "2025-12-20T00:00:00Z",
      "end_date": "2025-12-26T23:59:59Z",
      "multiplier": 2.5,
      "target_actions": ["invoice_upload", "survey_complete"]
    }
  ]
}
```

### `PATCH /api/v4/admin/gamification/users/{user_id}/adjust`
**Descripción:** Ajustar manualmente stats de usuario (solo admin).

**Request:**
```json
{
  "adjustment_type": "add_lumis",
  "amount": 500,
  "reason": "Compensación por error en el sistema",
  "notify_user": true
}
```

---

## 🔧 Códigos de Error Comunes

```json
{
  "success": false,
  "error": {
    "code": "STREAK_ALREADY_FROZEN",
    "message": "La racha ya está congelada",
    "details": {
      "current_freeze_expires": "2025-08-28T23:59:59Z"
    }
  }
}
```

**Códigos de Error:**
- `INSUFFICIENT_FREEZE_TOKENS` - No tienes tokens de freeze disponibles
- `MISSION_ALREADY_COMPLETED` - Misión ya completada
- `EVENT_NOT_ACTIVE` - Evento no está activo
- `ACHIEVEMENT_ALREADY_CLAIMED` - Logro ya reclamado
- `TEAM_FULL` - Equipo lleno
- `INSUFFICIENT_LEVEL` - Nivel insuficiente
- `COMBO_EXPIRED` - Combo expirado
- `FRAUD_DETECTED` - Actividad sospechosa detectada

---

## 📊 Rate Limiting

Todos los endpoints tienen rate limiting:
- **Dashboard/Stats:** 60 requests/minute
- **Actions (claim, join, etc.):** 30 requests/minute  
- **Admin endpoints:** 100 requests/minute
- **Bulk operations:** 10 requests/minute

## 🔐 Autenticación

Todos los endpoints requieren JWT token válido en el header:
```
Authorization: Bearer {jwt_token}
```

Los endpoints de admin requieren rol adicional:
```
X-Admin-Role: super_admin | admin | moderator
```
