-- ============================================================================
-- DAILY GAME - Database Setup
-- ============================================================================
-- Versión: 1.0 MVP
-- Fecha: 2025-10-13
-- Descripción: Sistema minimalista de juego diario sin rachas/multiplicadores
-- ============================================================================

-- 1. Crear tabla para registrar jugadas diarias
CREATE TABLE IF NOT EXISTS rewards.fact_daily_game_plays (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES public.dim_users(id) ON DELETE CASCADE,
  play_date DATE NOT NULL,
  play_time TIME NOT NULL,
  star_id VARCHAR(10) NOT NULL,
  lumis_won SMALLINT NOT NULL CHECK (lumis_won IN (0, 1, 5)),
  created_at TIMESTAMP DEFAULT NOW(),
  
  -- Constraint: Solo 1 jugada por día por usuario
  CONSTRAINT unique_user_daily_play UNIQUE (user_id, play_date),
  
  -- Validación adicional de formato star_id
  CONSTRAINT valid_star_id CHECK (star_id ~ '^star_[0-8]$')
);

-- 2. Crear índices para queries rápidas
CREATE INDEX IF NOT EXISTS idx_daily_game_user_date 
ON rewards.fact_daily_game_plays (user_id, play_date DESC);

CREATE INDEX IF NOT EXISTS idx_daily_game_play_date 
ON rewards.fact_daily_game_plays (play_date DESC);

-- 3. Comentarios de documentación
COMMENT ON TABLE rewards.fact_daily_game_plays IS 
'Registro de jugadas del juego diario de constelación. Una jugada por usuario por día.';

COMMENT ON COLUMN rewards.fact_daily_game_plays.play_date IS 
'Fecha de la jugada (sin hora) - usado para constraint UNIQUE';

COMMENT ON COLUMN rewards.fact_daily_game_plays.play_time IS 
'Hora exacta de la jugada - solo para trazabilidad/auditoría';

COMMENT ON COLUMN rewards.fact_daily_game_plays.lumis_won IS 
'Lümis ganados: 0 (estrella vacía), 1 (estrella normal), 5 (estrella dorada)';

-- 4. Insertar regla genérica en dim_accumulations
INSERT INTO rewards.dim_accumulations 
(id, name, points, valid_from, valid_to) 
VALUES 
(10, 'daily_game', 0, '2025-01-01'::DATE, '2099-12-31'::DATE)
ON CONFLICT (id) DO UPDATE 
SET name = EXCLUDED.name,
    points = EXCLUDED.points,
    valid_from = EXCLUDED.valid_from,
    valid_to = EXCLUDED.valid_to;

COMMENT ON TABLE rewards.dim_accumulations IS 
'Regla genérica para daily game. points=0 es placeholder, el valor real viene en quantity de fact_accumulations.';

-- 5. Verificar tablas existentes necesarias
DO $$ 
BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.tables 
                 WHERE table_schema = 'rewards' 
                 AND table_name = 'fact_accumulations') THEN
    RAISE EXCEPTION 'Table rewards.fact_accumulations does not exist. Run rewards setup first.';
  END IF;
  
  IF NOT EXISTS (SELECT 1 FROM information_schema.tables 
                 WHERE table_schema = 'rewards' 
                 AND table_name = 'fact_balance_points') THEN
    RAISE EXCEPTION 'Table rewards.fact_balance_points does not exist. Run rewards setup first.';
  END IF;
END $$;

-- 6. Grant permissions (ajustar según tu usuario de DB)
-- GRANT SELECT, INSERT ON rewards.fact_daily_game_plays TO your_app_user;
-- GRANT USAGE, SELECT ON SEQUENCE rewards.fact_daily_game_plays_id_seq TO your_app_user;

-- ============================================================================
-- QUERIES DE TESTING
-- ============================================================================

-- Verificar instalación
SELECT 
  'fact_daily_game_plays' as table_name,
  COUNT(*) as row_count
FROM rewards.fact_daily_game_plays
UNION ALL
SELECT 
  'dim_accumulations (daily_game)',
  COUNT(*)
FROM rewards.dim_accumulations
WHERE name = 'daily_game';

-- Ver constraints
SELECT
  conname as constraint_name,
  contype as constraint_type,
  pg_get_constraintdef(oid) as definition
FROM pg_constraint
WHERE conrelid = 'rewards.fact_daily_game_plays'::regclass;

-- ============================================================================
-- CLEANUP (Solo para desarrollo/testing)
-- ============================================================================
/*
-- Eliminar todo (usar con cuidado)
DROP TABLE IF EXISTS rewards.fact_daily_game_plays CASCADE;
DELETE FROM rewards.dim_accumulations WHERE id = 10;
*/
