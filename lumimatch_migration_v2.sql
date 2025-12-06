-- Migración: Agregar campo specific_date y nuevos índices para lumimatch
-- Ejecutar: psql -h localhost -d tfactu -U avalencia -f lumimatch_migration_v2.sql

-- 1. Agregar campo specific_date a la tabla questions
ALTER TABLE lumimatch.questions ADD COLUMN IF NOT EXISTS specific_date DATE;

-- 2. Índice para búsqueda por fecha específica
CREATE INDEX IF NOT EXISTS idx_lumimatch_questions_specific_date 
ON lumimatch.questions(specific_date) 
WHERE specific_date IS NOT NULL;

-- 3. Índices para Analítica de respuestas
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_question ON lumimatch.user_answers(question_id);
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_option ON lumimatch.user_answers(option_id);
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_date ON lumimatch.user_answers(answered_at);

-- 4. Índice para búsqueda de tags por usuario
CREATE INDEX IF NOT EXISTS idx_lumimatch_user_tags_user ON lumimatch.user_tags(user_id);

-- Verificación
SELECT 'Migración completada exitosamente' AS status;
