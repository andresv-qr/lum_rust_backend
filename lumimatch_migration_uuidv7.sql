-- Migración: Cambiar DEFAULT de UUID a uuidv7() para mejor performance
-- PostgreSQL 18+ requerido
-- Ejecutar: psql -h localhost -d tfactu -U avalencia -f lumimatch_migration_uuidv7.sql

BEGIN;

-- Verificar que uuidv7() está disponible (PostgreSQL 18+)
DO $$
BEGIN
    PERFORM uuidv7();
    RAISE NOTICE '✅ uuidv7() disponible - PostgreSQL 18+';
EXCEPTION WHEN undefined_function THEN
    RAISE EXCEPTION '❌ uuidv7() no disponible. Requiere PostgreSQL 18+';
END $$;

-- Cambiar DEFAULT en lumimatch.questions
ALTER TABLE lumimatch.questions 
ALTER COLUMN id SET DEFAULT uuidv7();

-- Cambiar DEFAULT en lumimatch.options
ALTER TABLE lumimatch.options 
ALTER COLUMN id SET DEFAULT uuidv7();

-- Cambiar DEFAULT en lumimatch.user_answers
ALTER TABLE lumimatch.user_answers 
ALTER COLUMN id SET DEFAULT uuidv7();

COMMIT;

SELECT 'Migración a UUIDv7 completada. Los nuevos registros usarán UUIDv7 (ordenable cronológicamente).' as status;
