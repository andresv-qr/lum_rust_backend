-- ============================================================================
-- FIX: Corregir trigger fun_update_balance_points
-- ============================================================================
-- PROBLEMA: El trigger está filtrando solo dtype='points' y borrando
--           el balance de lümis de facturas (dtype='lumis' probablemente)
--
-- SOLUCIÓN: Eliminar el filtro de dtype o incluir ambos tipos
-- ============================================================================

CREATE OR REPLACE FUNCTION rewards.fun_update_balance_points()
RETURNS TRIGGER AS $$
BEGIN
  -- Calculate balance from fact_accumulations
  -- SIN FILTRAR por dtype - incluir TODAS las acumulaciones
  UPDATE rewards.fact_balance_points rs
  SET balance = (
    SELECT COALESCE(
      SUM(CASE
        WHEN accum_type = 'earn' THEN quantity
        WHEN accum_type = 'spend' THEN -quantity
        ELSE 0
      END),
      0
    )
    FROM rewards.fact_accumulations
    -- ⚠️ IMPORTANTE: NO filtrar por dtype para incluir TODO
    WHERE user_id = NEW.user_id
  ),
  latest_update = NOW()
  WHERE rs.user_id = NEW.user_id;

  -- If the record didn't exist, insert it
  IF NOT FOUND THEN
    INSERT INTO rewards.fact_balance_points (user_id, balance, latest_update)
    VALUES (
      NEW.user_id,
      (
        SELECT COALESCE(
          SUM(CASE
            WHEN accum_type = 'earn' THEN quantity
            WHEN accum_type = 'spend' THEN -quantity
            ELSE 0
          END),
          0
        )
        FROM rewards.fact_accumulations
        WHERE user_id = NEW.user_id
      ),
      NOW()
    );
  END IF;

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Verificar que el trigger esté activo
-- ============================================================================
SELECT 
    t.tgname as trigger_name,
    c.relname as table_name,
    p.proname as function_name,
    tgenabled as enabled
FROM pg_trigger t
JOIN pg_class c ON t.tgrelid = c.oid
JOIN pg_proc p ON t.tgfoid = p.oid
WHERE p.proname = 'fun_update_balance_points';

COMMENT ON FUNCTION rewards.fun_update_balance_points() IS 
'CORREGIDO: Recalcula balance incluyendo TODOS los dtypes (no solo points)';
