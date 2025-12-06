# 🎮 Sistema de Gamificación - Documentación de Tablas

## Índice
1. [Configuración Core](#1-configuración-core)
2. [Seguimiento de Usuario](#2-seguimiento-de-usuario)
3. [Sistema de Logros](#3-sistema-de-logros)
4. [Niveles y Progresión](#4-niveles-y-progresión)
5. [Social y Competencia](#5-social-y-competencia)
6. [Anti-Fraude](#6-anti-fraude)
7. [Localización](#7-localización)
8. [Notificaciones](#8-notificaciones)
9. [Transacciones y Logs](#9-transacciones-y-logs)
10. [Performance y Cache](#10-performance-y-cache)

---

## 1. Configuración Core

### `dim_engagement_mechanics`
**Propósito:** Define las mecánicas base de engagement (rachas, misiones, eventos).

**Casos de Uso:**
- Configurar mecánicas como "daily streak", "survey bonus", "happy hour"
- Activar/desactivar mecánicas dinámicamente
- Almacenar configuración flexible en JSON

**Ejemplo:**
```sql
INSERT INTO gamification.dim_engagement_mechanics VALUES
(1, 'daily_streak', 'Racha Diaria', 'streak', 'Racha de logins consecutivos', true, 
'{"min_days": 1, "max_multiplier": 5}', NOW(), NOW());
```

### `dim_rewards_config`
**Propósito:** Configuración de recompensas estáticas y sus requisitos.

**Casos de Uso:**
- Definir recompensas por login diario
- Configurar bonus por completar encuestas
- Establecer multiplicadores por nivel de usuario

**Ejemplo:**
```sql
INSERT INTO gamification.dim_rewards_config VALUES
(1, 'daily_login_bonus', 'Bonus Login Diario', 'lumis', 10, 1.00, 'common',
'{"requires_streak": false}', true, NOW());
```

### `dim_events`
**Propósito:** Eventos temporales con multiplicadores y bonificaciones.

**Casos de Uso:**
- Happy Hours con 2x Lümis
- Eventos estacionales (Navidad, Verano)
- Flash events de duración limitada

**Ejemplo:**
```sql
INSERT INTO gamification.dim_events VALUES
(1, 'happy_hour_evening', 'Happy Hour Nocturno', 'daily',
'2025-08-27 18:00:00-05:00', '2025-08-27 20:00:00-05:00',
2.00, 0, '["invoice_upload", "survey_complete"]', NULL, true, NOW());
```

### `dim_targeting_criteria`
**Propósito:** Criterios para segmentar usuarios y personalizar eventos.

**Casos de Uso:**
- Eventos solo para usuarios premium
- Bonus para usuarios inactivos
- Segmentación por edad o ubicación

**Ejemplo:**
```sql
INSERT INTO gamification.dim_targeting_criteria VALUES
(1, 'premium_users', 'Usuarios Premium', 'segment',
'SELECT user_id FROM public.users WHERE subscription_type = ''premium''',
3600, true, NOW());
```

### `fact_event_targeting`
**Propósito:** Relaciona eventos con criterios de targeting.

**Casos de Uso:**
- Aplicar eventos solo a usuarios específicos
- Combinar múltiples criterios con lógica AND/OR

### `dim_dynamic_rewards`
**Propósito:** Recompensas que cambian dinámicamente (jackpots, ruletas).

**Casos de Uso:**
- Jackpot progresivo que aumenta diariamente
- Ruleta con probabilidades variables
- Mystery boxes con contenido aleatorio

**Ejemplo:**
```sql
INSERT INTO gamification.dim_dynamic_rewards VALUES
(1, 'daily_jackpot', 'Jackpot Diario', 'progressive',
'{"base_amount": 100, "increment_per_day": 50, "max_amount": 1000}',
'{"current_amount": 250, "days_without_winner": 3}',
NOW(), 'daily', NOW() + INTERVAL '1 day', true, NOW());
```

### `dim_combo_chains`
**Propósito:** Define secuencias de acciones para bonus especiales.

**Casos de Uso:**
- Combo: 3 facturas + 2 encuestas = 3x bonus
- Secuencia de acciones en orden específico
- Chains con ventana de tiempo limitada

---

## 2. Seguimiento de Usuario

### `fact_user_streaks`
**Propósito:** Rastrea rachas activas de usuarios por tipo de actividad.

**Casos de Uso:**
- Racha de login diario (días consecutivos)
- Racha de subida de facturas
- Racha de completar encuestas

**Ejemplo de Query:**
```sql
-- Obtener rachas activas del usuario
SELECT streak_type, current_count, last_activity_date 
FROM gamification.fact_user_streaks 
WHERE user_id = 123 AND is_active = true;
```

### `fact_user_missions`
**Propósito:** Misiones asignadas a usuarios con progreso y estado.

**Casos de Uso:**
- Misiones diarias: "Sube 2 facturas"
- Desafíos semanales: "Completa 5 encuestas"
- Misiones especiales con recompensas únicas

**Ejemplo de Query:**
```sql
-- Obtener misiones activas del usuario
SELECT mission_name, current_progress, target_count, due_date, reward_lumis
FROM gamification.fact_user_missions 
WHERE user_id = 123 AND status = 'active';
```

### `fact_user_events`
**Propósito:** Participación de usuarios en eventos temporales.

**Casos de Uso:**
- Registro de participación en Happy Hour
- Tracking de progreso en eventos
- Posición en leaderboards de eventos

### `fact_user_combo_progress`
**Propósito:** Progreso de usuarios en combos/chains activos.

**Casos de Uso:**
- Seguimiento de secuencias de acciones
- Validación de combos completados
- Expiración de combos por tiempo

---

## 3. Sistema de Logros

### `dim_achievements`
**Propósito:** Catálogo de logros disponibles en el sistema.

**Casos de Uso:**
- Logros por primera vez: "Primera Factura"
- Logros por volumen: "100 Encuestas Completadas"
- Logros secretos: "Medianoche Activo"

**Ejemplo:**
```sql
INSERT INTO gamification.dim_achievements VALUES
(1, 'first_invoice', 'Primera Factura', 'Sube tu primera factura', 'invoices',
'bronze', '/icons/first_invoice.png', '{"min_invoices": 1}', 50, false, 1, true, NOW());
```

### `fact_user_achievements`
**Propósito:** Logros desbloqueados por cada usuario.

**Casos de Uso:**
- Registro de cuándo se desbloqueó el logro
- Estado de reclamación de recompensa
- Datos de progreso al momento del desbloqueo

---

## 4. Niveles y Progresión

### `dim_user_levels`
**Propósito:** Configuración de niveles de usuario y sus beneficios.

**Casos de Uso:**
- Niveles: Bronze Explorer, Silver Hunter, Gold Master
- Beneficios por nivel: más Lümis por acción
- Colores e iconos personalizados por nivel

**Ejemplo:**
```sql
INSERT INTO gamification.dim_user_levels VALUES
(1, 1, 'Bronze Explorer', 0, 999, '#CD7F32', '/icons/bronze.png',
'{"lumis_multiplier": 1.0, "daily_missions": 3}', NOW());
```

### `fact_user_progression`
**Propósito:** Estado actual de progresión de cada usuario.

**Casos de Uso:**
- XP actual y total del usuario
- Nivel actual y progreso al siguiente
- Sistema de "prestige" para reinicios

---

## 5. Social y Competencia

### `fact_leaderboards`
**Propósito:** Rankings y clasificaciones por períodos.

**Casos de Uso:**
- Leaderboard semanal de facturas subidas
- Ranking mensual de encuestas completadas
- Competencias temporales con premios

### `fact_user_social`
**Propósito:** Conexiones sociales entre usuarios.

**Casos de Uso:**
- Sistema de amigos
- Programa de referidos
- Invitaciones a equipos

### `dim_teams` / `fact_team_members`
**Propósito:** Sistema de equipos para competencia grupal.

**Casos de Uso:**
- Equipos de 5-10 usuarios
- Competencias inter-equipos
- Objetivos grupales compartidos

### `fact_team_competitions`
**Propósito:** Torneos y competencias entre equipos.

**Casos de Uso:**
- Torneos de eliminación
- Ligas estacionales
- Eventos especiales de equipos

---

## 6. Anti-Fraude

### `dim_fraud_rules`
**Propósito:** Reglas para detectar comportamiento sospechoso.

**Casos de Uso:**
- Detección de facturas duplicadas
- Patrones de velocidad anómala
- Comportamiento tipo bot

**Ejemplo:**
```sql
INSERT INTO gamification.dim_fraud_rules VALUES
(1, 'duplicate_invoice', 'Factura Duplicada', 'duplicate',
'SELECT COUNT(*) FROM invoices WHERE qr_code = $1 AND user_id = $2',
'{"max_duplicates": 3, "time_window_hours": 24}', 'warning', true, NOW());
```

### `fact_fraud_signals`
**Propósito:** Registro de señales de fraude detectadas.

**Casos de Uso:**
- Log de actividad sospechosa
- Escalamiento automático de casos
- Resolución manual de falsos positivos

---

## 7. Localización

### `dim_localized_events`
**Propósito:** Eventos adaptados por región y zona horaria.

**Casos de Uso:**
- Happy Hour local para cada país
- Eventos que respetan feriados locales
- Horarios adaptados a timezone del usuario

---

## 8. Notificaciones

### `dim_notification_templates`
**Propósito:** Plantillas de notificaciones multi-idioma.

**Casos de Uso:**
- Push notifications personalizadas
- Emails de recordatorio
- Mensajes in-app contextuales

### `fact_user_notification_preferences`
**Propósito:** Preferencias de notificación por usuario.

**Casos de Uso:**
- Horarios de "no molestar"
- Frecuencia de notificaciones
- Canales preferidos (push, email, SMS)

### `fact_notification_queue`
**Propósito:** Cola de notificaciones programadas.

**Casos de Uso:**
- Envío diferido de notificaciones
- Retry en caso de fallo
- Priorización de mensajes urgentes

---

## 9. Transacciones y Logs

### `fact_engagement_transactions`
**Propósito:** Log de todas las transacciones de gamificación.

**Casos de Uso:**
- Auditoría de Lümis otorgados
- Análisis de efectividad de eventos
- Debugging de problemas de recompensas

### `fact_user_activity_log`
**Propósito:** Log detallado de actividad de usuarios (particionado).

**Casos de Uso:**
- Analytics de comportamiento
- Detección de patrones
- Optimización de engagement

---

## 10. Performance y Cache

### `cache_leaderboards`
**Propósito:** Cache pre-computado de rankings para performance.

**Casos de Uso:**
- Leaderboards de alta frecuencia
- Reducción de queries complejos
- Actualización periódica automática

### `vw_user_dashboard` (Vista Materializada)
**Propósito:** Dashboard consolidado del usuario actualizado cada hora.

**Casos de Uso:**
- API endpoint de dashboard rápido
- Datos pre-agregados de usuario
- Reducción de joins complejos

---

## 📊 Flujos de Datos Principales

### 1. **Usuario Sube Factura**
```
fact_user_activity_log → check dim_events → apply multipliers → 
fact_engagement_transactions → update fact_user_streaks → 
check fact_user_missions progress → trigger notifications
```

### 2. **Evento Happy Hour**
```
dim_events (active) → get_active_events_for_user() → 
apply multiplier → fact_engagement_transactions → 
update leaderboards → send notifications
```

### 3. **Sistema Anti-Fraude**
```
fact_user_activity_log → check dim_fraud_rules → 
detect patterns → fact_fraud_signals → 
auto-resolution or manual review
```

### 4. **Dashboard de Usuario**
```
vw_user_dashboard (materialized view) → 
refresh hourly → fast API response
```

Este sistema está diseñado para ser **escalable**, **flexible** y **auditable**, permitiendo implementar cualquier mecánica de gamificación mientras mantiene performance óptimo.
