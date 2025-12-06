# 🗄️ Migración de Base de Datos - Sistema de Redenciones

**Archivo**: `migrations/2025_10_17_redemption_system.sql`  
**Estado**: ⚠️ **PENDIENTE DE EJECUTAR**  
**Base de Datos**: `tfactu` en `dbmain.lumapp.org`

---

## 📋 ¿Qué hace esta migración?

La migración crea el schema completo para el sistema de redenciones de Lümis con QR codes y validación por comercios.

### 🔧 Cambios Principales:

#### 1. **Extiende tabla existente** (`fact_accumulations`)
- Agrega columna `redemption_id` para vincular transacciones negativas con redenciones

#### 2. **Renombra y adapta** `dim_redemptions` → `redemption_offers`
- Agrega campos nuevos: `stock_quantity`, `merchant_id`, `lumis_cost`, etc.
- Migra datos legacy: `points` → `lumis_cost`
- Crea índices optimizados para queries frecuentes

#### 3. **Reemplaza** `fact_redemptions` → `user_redemptions`
- **BACKUP AUTOMÁTICO**: Crea `fact_redemptions_legacy` con datos antiguos
- Nueva tabla con campos para QR codes, validación, expiración
- Campos: `redemption_code`, `qr_image_url`, `validated_by_merchant_id`, etc.

#### 4. **Crea tabla nueva** `merchants`
- Gestión de comercios que validan redenciones
- Autenticación con API key (bcrypt hash)
- Estadísticas denormalizadas

#### 5. **Crea tabla de auditoría** `redemption_audit_log`
- Registro de todas las acciones (created, validated, confirmed, cancelled)
- Timestamp + IP + metadata JSON

#### 6. **Crea schema separado** `rewards`
- Todas las tablas nuevas van en schema `rewards.*`
- Aislamiento lógico del resto del sistema

---

## 🚀 Cómo Ejecutar la Migración

### Opción 1: Desde línea de comandos (psql)

```bash
# 1. Conectar a la base de datos
psql -h dbmain.lumapp.org -U tfactu_user -d tfactu

# 2. Ejecutar migración
\i /home/client_1099_1/scripts/lum_rust_ws/migrations/2025_10_17_redemption_system.sql

# 3. Verificar que se crearon las tablas
\dt rewards.*

# 4. Verificar índices
\di rewards.*
```

### Opción 2: Desde script remoto

```bash
# Ejecutar directamente (requiere password)
psql -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -f /home/client_1099_1/scripts/lum_rust_ws/migrations/2025_10_17_redemption_system.sql
```

### Opción 3: Con PGPASSWORD (automatizado)

```bash
# Exportar password (temporal)
export PGPASSWORD='tu_password_aqui'

# Ejecutar migración
psql -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -f /home/client_1099_1/scripts/lum_rust_ws/migrations/2025_10_17_redemption_system.sql

# Limpiar password
unset PGPASSWORD
```

---

## 📊 Tablas que se Crearán

### En schema `rewards`:

#### 1. `rewards.redemption_offers`
```sql
-- Catálogo de ofertas
Columns:
- offer_id (UUID, PK)
- name_friendly (VARCHAR)
- description (TEXT)
- lumis_cost (INTEGER)
- stock_quantity (INTEGER)
- merchant_id (UUID) -- FK opcional
- is_active (BOOLEAN)
- valid_from, valid_to (TIMESTAMP)
```

**Índices**:
- `idx_offers_active_valid` - Ofertas activas y vigentes
- `idx_offers_category` - Búsqueda por categoría
- `idx_offers_cost` - Filtrado por precio
- `idx_offers_merchant` - Ofertas de comercio específico

---

#### 2. `rewards.user_redemptions`
```sql
-- Redenciones de usuarios (con QR)
Columns:
- redemption_id (UUID, PK)
- user_id (INTEGER) -- FK a tabla users
- offer_id (UUID) -- FK a redemption_offers
- lumis_spent (INTEGER)
- redemption_code (VARCHAR, UNIQUE) -- Código del QR
- qr_image_url (TEXT)
- qr_landing_url (TEXT)
- redemption_status (VARCHAR) -- pending|confirmed|cancelled|expired
- validated_by_merchant_id (UUID)
- validated_at (TIMESTAMP)
- code_expires_at (TIMESTAMP)
- created_at (TIMESTAMP)
```

**Índices**:
- `idx_redemptions_user` - Redenciones de usuario específico
- `idx_redemptions_code` - Búsqueda por código QR (CRÍTICO para validación)
- `idx_redemptions_status` - Filtrado por estado
- `idx_redemptions_offer` - Redenciones de oferta específica
- `idx_redemptions_merchant` - Redenciones validadas por comercio
- `idx_redemptions_created` - Ordenamiento temporal

---

#### 3. `rewards.merchants`
```sql
-- Comercios que validan redenciones
Columns:
- merchant_id (UUID, PK)
- merchant_name (VARCHAR, UNIQUE)
- merchant_type (VARCHAR) -- restaurant, cinema, bookstore, etc.
- contact_email (VARCHAR)
- contact_phone (VARCHAR)
- api_key_hash (VARCHAR) -- bcrypt hash
- webhook_url (TEXT)
- is_active (BOOLEAN)
- total_redemptions (INTEGER) -- denormalizado
- total_lumis_redeemed (BIGINT) -- denormalizado
```

**Índices**:
- `idx_merchants_active` - Comercios activos
- `idx_merchants_name` - Búsqueda por nombre

---

#### 4. `rewards.redemption_audit_log`
```sql
-- Auditoría de todas las acciones
Columns:
- log_id (BIGSERIAL, PK)
- redemption_id (UUID)
- action (VARCHAR) -- created, validated, confirmed, cancelled, expired
- actor_type (VARCHAR) -- user, merchant, system
- actor_id (VARCHAR)
- ip_address (INET)
- user_agent (TEXT)
- metadata (JSONB)
- created_at (TIMESTAMP)
```

**Índices**:
- `idx_audit_redemption` - Logs de redención específica
- `idx_audit_action` - Filtrado por tipo de acción
- `idx_audit_created` - Ordenamiento temporal

---

## 🔐 Funciones y Triggers Creados

### 1. **Trigger de Auditoría Automática**
```sql
CREATE TRIGGER trg_audit_redemption_changes
AFTER INSERT OR UPDATE OR DELETE ON rewards.user_redemptions
FOR EACH ROW EXECUTE FUNCTION rewards.audit_redemption_change();
```
- Registra automáticamente todos los cambios en `user_redemptions`
- Detecta tipo de cambio (INSERT, UPDATE status, DELETE)

### 2. **Función de Cleanup Automático**
```sql
CREATE FUNCTION rewards.expire_old_redemptions()
```
- Marca como `expired` redenciones que pasaron `code_expires_at`
- Debe ejecutarse periódicamente (ver sección Cron Jobs)

### 3. **Trigger de Estadísticas de Merchant**
```sql
CREATE TRIGGER trg_update_merchant_stats
AFTER UPDATE ON rewards.user_redemptions
FOR EACH ROW EXECUTE FUNCTION rewards.update_merchant_stats();
```
- Actualiza automáticamente `total_redemptions` y `total_lumis_redeemed`
- Solo cuando cambia de `pending` → `confirmed`

---

## ✅ Verificación Post-Migración

### 1. Verificar Tablas Creadas
```sql
\c tfactu
\dt rewards.*
```

**Esperado**:
```
                   List of relations
 Schema  |            Name            | Type  |   Owner    
---------+----------------------------+-------+------------
 rewards | merchants                  | table | tfactu_user
 rewards | redemption_audit_log       | table | tfactu_user
 rewards | redemption_offers          | table | tfactu_user
 rewards | user_redemptions           | table | tfactu_user
(4 rows)
```

---

### 2. Verificar Índices
```sql
\di rewards.*
```

Deberías ver ~15 índices creados.

---

### 3. Verificar Funciones
```sql
\df rewards.*
```

**Esperado**:
```
- audit_redemption_change()
- expire_old_redemptions()
- update_merchant_stats()
```

---

### 4. Verificar Triggers
```sql
SELECT trigger_name, event_manipulation, event_object_table 
FROM information_schema.triggers 
WHERE trigger_schema = 'rewards';
```

**Esperado**: 2 triggers activos.

---

### 5. Test Query Simple
```sql
-- Debe retornar 0 rows (tabla vacía)
SELECT COUNT(*) FROM rewards.redemption_offers;
SELECT COUNT(*) FROM rewards.user_redemptions;
SELECT COUNT(*) FROM rewards.merchants;
```

---

## 📝 Datos de Prueba (Opcional)

Después de la migración, puedes insertar datos de prueba:

### 1. Crear Merchant de Prueba
```sql
-- Generar API key hash con bcrypt (cost 12)
-- Password: testkey123
-- Hash: $2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TT.KW0ckT8UrYKl1D6y7t8ZPqAum

INSERT INTO rewards.merchants (
    merchant_name, 
    merchant_type,
    contact_email,
    api_key_hash, 
    is_active
) VALUES (
    'Café Test',
    'restaurant',
    'test@cafe.com',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TT.KW0ckT8UrYKl1D6y7t8ZPqAum',
    true
) RETURNING merchant_id, merchant_name;
```

---

### 2. Crear Ofertas de Prueba
```sql
INSERT INTO rewards.redemption_offers (
    name_friendly,
    description,
    lumis_cost,
    stock_quantity,
    is_active,
    valid_from,
    valid_to,
    offer_category
) VALUES 
(
    'Café Gratis',
    'Un café americano gratis de cualquier tamaño',
    50,
    100,
    true,
    NOW(),
    NOW() + INTERVAL '30 days',
    'food_drink'
),
(
    'Descuento 20%',
    '20% de descuento en tu compra total',
    100,
    50,
    true,
    NOW(),
    NOW() + INTERVAL '30 days',
    'discount'
),
(
    'Postre Gratis',
    'Postre del día gratis con cualquier orden',
    75,
    30,
    true,
    NOW(),
    NOW() + INTERVAL '15 days',
    'food_drink'
)
RETURNING offer_id, name_friendly, lumis_cost;
```

---

### 3. Verificar Ofertas Activas
```sql
SELECT 
    offer_id,
    name_friendly,
    lumis_cost,
    stock_quantity,
    valid_to
FROM rewards.redemption_offers
WHERE is_active = true 
  AND valid_to > NOW()
ORDER BY lumis_cost;
```

---

## 🔄 Rollback (si algo sale mal)

Si necesitas revertir la migración:

```sql
BEGIN;

-- 1. Eliminar schema completo
DROP SCHEMA IF EXISTS rewards CASCADE;

-- 2. Restaurar fact_redemptions original
DROP TABLE IF EXISTS fact_redemptions;
ALTER TABLE fact_redemptions_legacy RENAME TO fact_redemptions;

-- 3. Renombrar redemption_offers de vuelta
ALTER TABLE redemption_offers RENAME TO dim_redemptions;

-- 4. Remover columna de fact_accumulations
ALTER TABLE fact_accumulations DROP COLUMN IF EXISTS redemption_id;

COMMIT;
```

**⚠️ ADVERTENCIA**: Esto eliminará TODOS los datos del sistema de redenciones.

---

## 📊 Performance Esperado

Después de la migración, con índices optimizados:

### Queries Críticos:
- **Buscar oferta por ID**: < 1ms (PK lookup)
- **Listar ofertas activas**: < 10ms (índice compuesto)
- **Validar código QR**: < 5ms (índice UNIQUE en redemption_code)
- **Listar redenciones de usuario**: < 15ms (índice en user_id)
- **Confirmar redención**: < 20ms (transacción con lock)

### Capacidad:
- **Ofertas**: ~10,000 ofertas sin degradación
- **Redenciones**: Millones de rows con índices eficientes
- **Merchants**: ~1,000 comercios activos

---

## 🔧 Configuración Post-Migración

### 1. Variables de Entorno (Rust)
Asegúrate de que estén configuradas:

```bash
DATABASE_URL=postgresql://tfactu_user:PASSWORD@dbmain.lumapp.org/tfactu
REDIS_URL=redis://localhost:6379
JWT_SECRET=lumis_jwt_secret_super_seguro_production_2024_rust_server_key
```

### 2. Permisos de Usuario
```sql
-- Verificar que tfactu_user tenga permisos
GRANT ALL PRIVILEGES ON SCHEMA rewards TO tfactu_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA rewards TO tfactu_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA rewards TO tfactu_user;
```

### 3. Connection Pool (Rust)
Ya configurado en el código:
- Max connections: 50
- Timeout: 30 segundos
- Pool optimizado para high throughput

---

## 📅 Cron Jobs Recomendados

### 1. Expirar Redenciones Viejas (cada hora)
```bash
# Crontab entry
0 * * * * psql -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -c "SELECT rewards.expire_old_redemptions();" >> /var/log/redemption_expire.log 2>&1
```

### 2. Limpiar Audit Log Viejo (semanal)
```bash
# Mantener solo últimos 90 días
0 2 * * 0 psql -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -c "DELETE FROM rewards.redemption_audit_log WHERE created_at < NOW() - INTERVAL '90 days';" \
  >> /var/log/audit_cleanup.log 2>&1
```

### 3. Actualizar Estadísticas de PostgreSQL (diario)
```bash
# Actualizar statistics para optimizador
0 3 * * * psql -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -c "ANALYZE rewards.user_redemptions; ANALYZE rewards.redemption_offers;" \
  >> /var/log/pg_analyze.log 2>&1
```

---

## 🐛 Troubleshooting

### Error: "schema rewards does not exist"
**Solución**: La migración no se ejecutó completamente. Verifica que el script llegó hasta el final.

```sql
-- Verificar si existe el schema
SELECT schema_name FROM information_schema.schemata WHERE schema_name = 'rewards';
```

---

### Error: "relation redemption_offers does not exist"
**Causa**: La tabla anterior `dim_redemptions` no existía o el rename falló.

**Solución**:
```sql
-- Verificar tabla original
\dt dim_redemptions

-- Si existe, hacer rename manual
ALTER TABLE dim_redemptions RENAME TO redemption_offers;
```

---

### Error: "duplicate key value violates unique constraint"
**Causa**: Intentando crear datos de prueba con IDs duplicados.

**Solución**: Usar `ON CONFLICT DO NOTHING` o verificar existencia primero.

---

### Performance Lento en Queries
**Causa**: Índices no creados o estadísticas desactualizadas.

**Solución**:
```sql
-- Recrear índices
REINDEX SCHEMA rewards;

-- Actualizar estadísticas
ANALYZE rewards.user_redemptions;
ANALYZE rewards.redemption_offers;
```

---

## 📞 Comandos Útiles

### Ver Tamaño de Tablas
```sql
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'rewards'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### Ver Queries Lentos
```sql
SELECT 
    query,
    calls,
    total_exec_time / 1000 as total_sec,
    mean_exec_time / 1000 as mean_sec
FROM pg_stat_statements
WHERE query LIKE '%rewards%'
ORDER BY mean_exec_time DESC
LIMIT 10;
```

### Backup Manual
```bash
# Backup solo schema rewards
pg_dump -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -n rewards \
  -f backup_rewards_$(date +%Y%m%d).sql

# Backup con datos
pg_dump -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -n rewards --data-only \
  -f backup_rewards_data_$(date +%Y%m%d).sql
```

---

## ✅ Checklist de Migración

- [ ] Backup de base de datos completo
- [ ] Verificar credenciales de acceso
- [ ] Ejecutar migración: `\i migrations/2025_10_17_redemption_system.sql`
- [ ] Verificar tablas creadas: `\dt rewards.*`
- [ ] Verificar índices: `\di rewards.*`
- [ ] Verificar funciones: `\df rewards.*`
- [ ] Insertar merchant de prueba
- [ ] Insertar ofertas de prueba
- [ ] Test query SELECT simple
- [ ] Reiniciar servidor Rust
- [ ] Probar endpoint GET /offers
- [ ] Probar flujo completo end-to-end

---

## 🎯 Siguiente Paso

**Ejecutar la migración**:

```bash
psql -h dbmain.lumapp.org -U tfactu_user -d tfactu \
  -f /home/client_1099_1/scripts/lum_rust_ws/migrations/2025_10_17_redemption_system.sql
```

Luego reiniciar el servidor y probar:
```bash
curl http://localhost:8000/api/v1/rewards/offers \
  -H "Authorization: Bearer $TOKEN"
```

🚀 **¡Listo para ejecutar!**
