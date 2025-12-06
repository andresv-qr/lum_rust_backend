-- ============================================================================
-- MIGRACIÓN: Corrección de Foreign Keys - Gamificación
-- Archivo: db/migrations/20251129_fix_gamification_fk_constraints.sql
-- Fecha: 29 Noviembre 2025
-- ============================================================================
--
-- PROPÓSITO:
-- Agregar ON DELETE CASCADE a todas las FKs de tablas gamification que 
-- referencian dim_users para evitar registros huérfanos.
--
-- IMPACTO:
-- - Cada ALTER TABLE toma ACCESS EXCLUSIVE lock en la tabla
-- - Ejecutar en ventana de bajo tráfico
-- - Tiempo estimado: ~30 segundos por tabla (depende del tamaño)
--
-- ROLLBACK:
-- Remover CASCADE y volver a NO ACTION (default)
--
-- ============================================================================

-- ============================================================================
-- PRE-CHECKS: Verificar estado actual
-- ============================================================================

-- Ver constraints actuales (informativo, no modifica nada)
DO $$
BEGIN
    RAISE NOTICE '=== Verificando constraints existentes ===';
END $$;

SELECT 
    tc.table_schema,
    tc.table_name, 
    tc.constraint_name, 
    rc.delete_rule,
    kcu.column_name
FROM information_schema.table_constraints tc
JOIN information_schema.referential_constraints rc 
    ON tc.constraint_name = rc.constraint_name
JOIN information_schema.key_column_usage kcu 
    ON tc.constraint_name = kcu.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY' 
    AND tc.table_schema = 'gamification'
ORDER BY tc.table_name;

-- ============================================================================
-- MIGRACIÓN PRINCIPAL
-- ============================================================================

BEGIN;

-- ============================================================================
-- 1. fact_user_streaks
-- ============================================================================
DO $$ 
BEGIN
    RAISE NOTICE 'Migrando fact_user_streaks...';
    
    -- Eliminar constraint existente si existe
    ALTER TABLE gamification.fact_user_streaks
    DROP CONSTRAINT IF EXISTS fact_user_streaks_user_id_fkey;
    
    ALTER TABLE gamification.fact_user_streaks
    DROP CONSTRAINT IF EXISTS fk_fact_user_streaks_user;
    
    -- Crear nuevo constraint con CASCADE
    ALTER TABLE gamification.fact_user_streaks
    ADD CONSTRAINT fact_user_streaks_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
    ON DELETE CASCADE
    ON UPDATE CASCADE;
    
    RAISE NOTICE '✅ fact_user_streaks migrada';
END $$;

-- ============================================================================
-- 2. fact_engagement_transactions
-- ============================================================================
DO $$ 
BEGIN
    RAISE NOTICE 'Migrando fact_engagement_transactions...';
    
    ALTER TABLE gamification.fact_engagement_transactions
    DROP CONSTRAINT IF EXISTS fact_engagement_transactions_user_id_fkey;
    
    ALTER TABLE gamification.fact_engagement_transactions
    DROP CONSTRAINT IF EXISTS fk_fact_engagement_transactions_user;
    
    ALTER TABLE gamification.fact_engagement_transactions
    ADD CONSTRAINT fact_engagement_transactions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
    ON DELETE CASCADE
    ON UPDATE CASCADE;
    
    RAISE NOTICE '✅ fact_engagement_transactions migrada';
END $$;

-- ============================================================================
-- 3. fact_user_progression
-- ============================================================================
DO $$ 
BEGIN
    RAISE NOTICE 'Migrando fact_user_progression...';
    
    ALTER TABLE gamification.fact_user_progression
    DROP CONSTRAINT IF EXISTS fact_user_progression_user_id_fkey;
    
    ALTER TABLE gamification.fact_user_progression
    DROP CONSTRAINT IF EXISTS fk_fact_user_progression_user;
    
    ALTER TABLE gamification.fact_user_progression
    ADD CONSTRAINT fact_user_progression_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
    ON DELETE CASCADE
    ON UPDATE CASCADE;
    
    RAISE NOTICE '✅ fact_user_progression migrada';
END $$;

-- ============================================================================
-- 4. fact_user_achievements
-- ============================================================================
DO $$ 
BEGIN
    RAISE NOTICE 'Migrando fact_user_achievements...';
    
    ALTER TABLE gamification.fact_user_achievements
    DROP CONSTRAINT IF EXISTS fact_user_achievements_user_id_fkey;
    
    ALTER TABLE gamification.fact_user_achievements
    DROP CONSTRAINT IF EXISTS fk_fact_user_achievements_user;
    
    ALTER TABLE gamification.fact_user_achievements
    ADD CONSTRAINT fact_user_achievements_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
    ON DELETE CASCADE
    ON UPDATE CASCADE;
    
    RAISE NOTICE '✅ fact_user_achievements migrada';
END $$;

-- ============================================================================
-- 5. fact_user_missions
-- ============================================================================
DO $$ 
BEGIN
    RAISE NOTICE 'Migrando fact_user_missions...';
    
    ALTER TABLE gamification.fact_user_missions
    DROP CONSTRAINT IF EXISTS fact_user_missions_user_id_fkey;
    
    ALTER TABLE gamification.fact_user_missions
    DROP CONSTRAINT IF EXISTS fk_fact_user_missions_user;
    
    ALTER TABLE gamification.fact_user_missions
    ADD CONSTRAINT fact_user_missions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
    ON DELETE CASCADE
    ON UPDATE CASCADE;
    
    RAISE NOTICE '✅ fact_user_missions migrada';
END $$;

-- ============================================================================
-- 6. fact_user_activity_log (PARTITIONED TABLE)
-- ============================================================================
-- NOTA: En PostgreSQL 11+, los constraints en tablas particionadas 
-- se heredan automáticamente a las particiones.
-- Sin embargo, el constraint debe definirse en la tabla padre.

DO $$ 
BEGIN
    RAISE NOTICE 'Migrando fact_user_activity_log (tabla particionada)...';
    
    -- Primero verificar si ya existe un constraint
    -- Las tablas particionadas pueden tener comportamiento diferente
    
    -- Intentar eliminar constraint existente
    ALTER TABLE gamification.fact_user_activity_log
    DROP CONSTRAINT IF EXISTS fact_user_activity_log_user_id_fkey;
    
    ALTER TABLE gamification.fact_user_activity_log
    DROP CONSTRAINT IF EXISTS fk_fact_user_activity_log_user;
    
    -- Crear nuevo constraint
    -- NOTA: En tablas particionadas, esto puede requerir que todas las
    -- particiones tengan índices adecuados
    ALTER TABLE gamification.fact_user_activity_log
    ADD CONSTRAINT fact_user_activity_log_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
    ON DELETE CASCADE
    ON UPDATE CASCADE;
    
    RAISE NOTICE '✅ fact_user_activity_log migrada';
EXCEPTION 
    WHEN OTHERS THEN
        RAISE WARNING '⚠️ Error en fact_user_activity_log: %. Puede requerir migración manual.', SQLERRM;
END $$;

COMMIT;

-- ============================================================================
-- POST-CHECKS: Verificar migración exitosa
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== VERIFICACIÓN POST-MIGRACIÓN ===';
    RAISE NOTICE '';
END $$;

SELECT 
    tc.table_name, 
    tc.constraint_name, 
    rc.delete_rule as "ON DELETE",
    rc.update_rule as "ON UPDATE"
FROM information_schema.table_constraints tc
JOIN information_schema.referential_constraints rc 
    ON tc.constraint_name = rc.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY' 
    AND tc.table_schema = 'gamification'
    AND tc.constraint_name LIKE '%user%'
ORDER BY tc.table_name;

-- ============================================================================
-- ROLLBACK SCRIPT (ejecutar solo si es necesario revertir)
-- ============================================================================

/*
-- ROLLBACK: Volver a NO ACTION

BEGIN;

ALTER TABLE gamification.fact_user_streaks
DROP CONSTRAINT fact_user_streaks_user_id_fkey,
ADD CONSTRAINT fact_user_streaks_user_id_fkey
FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
ON DELETE NO ACTION;

ALTER TABLE gamification.fact_engagement_transactions
DROP CONSTRAINT fact_engagement_transactions_user_id_fkey,
ADD CONSTRAINT fact_engagement_transactions_user_id_fkey
FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
ON DELETE NO ACTION;

ALTER TABLE gamification.fact_user_progression
DROP CONSTRAINT fact_user_progression_user_id_fkey,
ADD CONSTRAINT fact_user_progression_user_id_fkey
FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
ON DELETE NO ACTION;

ALTER TABLE gamification.fact_user_achievements
DROP CONSTRAINT fact_user_achievements_user_id_fkey,
ADD CONSTRAINT fact_user_achievements_user_id_fkey
FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
ON DELETE NO ACTION;

ALTER TABLE gamification.fact_user_missions
DROP CONSTRAINT fact_user_missions_user_id_fkey,
ADD CONSTRAINT fact_user_missions_user_id_fkey
FOREIGN KEY (user_id) REFERENCES public.dim_users(id)
ON DELETE NO ACTION;

-- fact_user_activity_log requiere manejo especial por ser particionada

COMMIT;
*/
