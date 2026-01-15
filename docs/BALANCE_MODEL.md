# 🏦 Modelo de Balance de Lümis (Ledger Único)

## Resumen Ejecutivo

El sistema de balance de Lümis utiliza un **modelo de ledger único** (libro mayor) donde todas las transacciones se registran en una sola tabla (`rewards.fact_accumulations`) y el balance se materializa automáticamente via trigger en `rewards.fact_balance_points`.

---

## 📊 Arquitectura

```
┌─────────────────────────────────────────────────────────────────────┐
│                    FUENTE DE VERDAD (LEDGER)                        │
│                 rewards.fact_accumulations                          │
│  ┌───────────────┬──────────────┬─────────────────────────────────┐ │
│  │   accum_type  │   quantity   │         dtype                   │ │
│  ├───────────────┼──────────────┼─────────────────────────────────┤ │
│  │   'earn'      │   +100       │ invoice, daily_game, streak...  │ │
│  │   'spend'     │   -50        │ points, ocr, legacy_reward      │ │
│  │   'earn'      │   +50        │ refund, ocr_refund              │ │
│  └───────────────┴──────────────┴─────────────────────────────────┘ │
│                              │                                      │
│                              ▼ TRIGGER (automático)                 │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │              rewards.fact_balance_points                        ││
│  │              balance = SUM(quantity)                            ││
│  │              (Actualizado SOLO por trigger)                     ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                    TABLA OPERACIONAL                                │
│                 rewards.user_redemptions                            │
│         (QR codes, estados, validaciones - NO afecta balance)       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 📋 Tablas Principales

### 1. `rewards.fact_accumulations` (Ledger / Fuente de Verdad)

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `user_id` | INTEGER | ID del usuario |
| `accum_type` | VARCHAR | `'earn'` o `'spend'` |
| `dtype` | VARCHAR | Tipo específico de transacción |
| `quantity` | DECIMAL | **Positivo** para ganar, **Negativo** para gastar |
| `balance` | DECIMAL | Balance snapshot al momento (auditoría) |
| `date` | TIMESTAMP | Fecha de la transacción |
| `redemption_id` | UUID | Opcional, vincula con redención específica |

### 2. `rewards.fact_balance_points` (Balance Materializado)

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `user_id` | INTEGER | ID del usuario (PK) |
| `balance` | BIGINT | Balance actual = SUM(fact_accumulations.quantity) |
| `latest_update` | TIMESTAMP | Última actualización |

> ⚠️ **IMPORTANTE**: Esta tabla es actualizada **ÚNICAMENTE** por el trigger. No modificar directamente desde código.

### 3. `rewards.user_redemptions` (Tabla Operacional)

Gestiona el ciclo de vida de redenciones (QR codes, estados, validaciones). **NO afecta el balance directamente**.

---

## 🔄 Tipos de Transacciones

### Acumulaciones (quantity > 0)

| dtype | Origen | Descripción |
|-------|--------|-------------|
| `invoice` | Triggers SQL | Puntos por factura procesada |
| `daily_game` | `daily_game/claim.rs` | Premio del juego diario |
| `streak` | Triggers SQL | Bonus por racha |
| `achievement` | `gamification_service.rs` | Logros completados |
| `refund` | `redemption_service.rs` | Devolución por cancelación |
| `ocr_refund` | `service.rs` | Reembolso servicio OCR |

### Gastos (quantity < 0)

| dtype | Origen | Descripción |
|-------|--------|-------------|
| `points` | `redemption_service.rs` | Canje de ofertas |
| `ocr` | `service.rs` | Costo servicio OCR |
| `legacy_reward` | `service.rs` | Canje via WhatsApp bot |

---

## 🛠️ Código Rust: Dónde se modifica el balance

### Lectura de Balance

```rust
// src/domains/rewards/offer_service.rs
pub async fn get_user_balance(&self, user_id: i32) -> Result<i64, RedemptionError>

// src/api/gamification_service.rs  
pub async fn get_user_balance(pool: &PgPool, user_id: i64) -> Result<i32>

// src/domains/rewards/service.rs
pub async fn get_user_balance(pool: &PgPool, user_id: i64) -> Result<i32>
```

### Escritura al Ledger (Acumulaciones)

```rust
// src/api/gamification_service.rs - award_gamification_lumis()
INSERT INTO rewards.fact_accumulations (user_id, accum_type, dtype, quantity, ...)
VALUES ($1, 'earn', 'points', $2, ...)  // quantity POSITIVO

// src/api/daily_game/claim.rs - claim handler
INSERT INTO rewards.fact_accumulations (user_id, accum_type, dtype, quantity, ...)
VALUES ($1, 'daily_game', $2, 'points', $3, ...)  // quantity POSITIVO
```

### Escritura al Ledger (Gastos)

```rust
// src/domains/rewards/redemption_service.rs - create_redemption()
INSERT INTO rewards.fact_accumulations (user_id, accum_type, dtype, quantity, ...)
VALUES ($1, 'spend', 'points', -$2, ...)  // quantity NEGATIVO

// src/domains/rewards/service.rs - deduct_lumis_for_ocr()
INSERT INTO rewards.fact_accumulations (user_id, accum_type, dtype, quantity, ...)
VALUES ($1, 'spend', 'ocr', -$2, ...)  // quantity NEGATIVO
```

---

## 🔍 Validación de Integridad

### Función de Validación

```sql
SELECT * FROM rewards.validate_balance_integrity();
```

Retorna usuarios donde `fact_balance_points.balance ≠ SUM(fact_accumulations.quantity)`.

### Función de Auto-Corrección

```sql
SELECT * FROM rewards.fix_balance_discrepancies();
```

Corrige automáticamente discrepancias recalculando desde el ledger.

### Vista de Monitoreo

```sql
SELECT * FROM rewards.v_ledger_summary 
WHERE integrity_status = 'MISMATCH';
```

---

## ✅ Reglas de Oro

1. **NUNCA** hacer `UPDATE rewards.fact_balance_points SET balance = ...` desde código Rust
2. **SIEMPRE** insertar en `rewards.fact_accumulations` - el trigger actualiza el balance
3. **Gastos** deben tener `quantity` **NEGATIVA** (`-lumis_cost`)
4. **Ganancias/Reembolsos** deben tener `quantity` **POSITIVA** (`+lumis`)
5. Para auditoría, usar `redemption_id` para vincular transacciones con canjes específicos

---

## 📅 Historial de Cambios

| Fecha | Cambio |
|-------|--------|
| 2025-12-16 | Unificación a modelo ledger único |
| 2025-12-16 | Eliminación de trigger duplicado en user_redemptions |
| 2025-12-16 | Actualización de validate_balance_integrity() |
| 2025-12-16 | Eliminación de service_new.rs y service_backup.rs |
