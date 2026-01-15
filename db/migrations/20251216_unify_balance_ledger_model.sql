-- ============================================================================
-- MIGRACIÓN: Unificación del Modelo de Balance (Ledger Único)
-- ============================================================================
-- Fecha: 2025-12-16
-- Autor: Sistema
-- 
-- MODELO CONCEPTUAL:
-- ==================
-- FUENTE DE VERDAD: rewards.fact_accumulations (libro mayor / ledger)
--   - Acumulaciones (earn): quantity > 0
--   - Gastos (spend):       quantity < 0
--   - Reembolsos (refund):  quantity > 0
--
-- BALANCE MATERIALIZADO: rewards.fact_balance_points
--   - Se actualiza SOLO via trigger desde fact_accumulations
--   - balance = SUM(fact_accumulations.quantity)
--
-- TABLA OPERACIONAL: rewards.user_redemptions
--   - Solo para gestión de QR, estados, validaciones
--   - NO afecta el balance directamente
--
-- ============================================================================

BEGIN;

-- ============================================================================
-- 1. ELIMINAR TRIGGER DUPLICADO EN user_redemptions
-- ============================================================================
-- Este trigger restaba del balance cuando se insertaba una redención,
-- causando DOBLE DESCUENTO (una vez por fact_accumulations, otra por aquí)

SELECT '🔧 Eliminando trigger duplicado en user_redemptions...' as step;

DROP TRIGGER IF EXISTS trigger_update_balance_on_redemption 
ON rewards.user_redemptions;

DROP FUNCTION IF EXISTS rewards.fun_update_balance_on_redemption() CASCADE;

SELECT '✅ Trigger duplicado eliminado' as result;

-- ============================================================================
-- 2. ASEGURAR TRIGGER INCREMENTAL EN fact_accumulations
-- ============================================================================
-- Este es el ÚNICO trigger que debe modificar fact_balance_points

SELECT '🔧 Verificando/creando trigger incremental en fact_accumulations...' as step;

CREATE OR REPLACE FUNCTION rewards.fun_update_balance_points_incremental()
RETURNS TRIGGER AS $$
BEGIN
  -- INCREMENTAL: Solo sumar la cantidad del nuevo registro
  -- Si quantity es negativo (spend), restará; si es positivo (earn), sumará
  UPDATE rewards.fact_balance_points
  SET balance = balance + NEW.quantity,
      latest_update = NOW()
  WHERE user_id = NEW.user_id;
  
  -- Si no existe el usuario en fact_balance_points, crearlo
  IF NOT FOUND THEN
    INSERT INTO rewards.fact_balance_points (user_id, balance, latest_update)
    VALUES (NEW.user_id, NEW.quantity, NOW());
  END IF;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION rewards.fun_update_balance_points_incremental() IS 
'Trigger incremental (ÚNICO): suma quantity al balance. Spends son negativos, earns positivos.';

-- Recrear el trigger (idempotente)
DROP TRIGGER IF EXISTS trigger_accumulations_incremental 
ON rewards.fact_accumulations;

CREATE TRIGGER trigger_accumulations_incremental
AFTER INSERT ON rewards.fact_accumulations
FOR EACH ROW
EXECUTE FUNCTION rewards.fun_update_balance_points_incremental();

SELECT '✅ Trigger incremental único configurado' as result;

-- ============================================================================
-- 3. ACTUALIZAR FUNCIÓN DE VALIDACIÓN (solo usa fact_accumulations)
-- ============================================================================
-- El balance ahora es simplemente SUM(fact_accumulations.quantity)

SELECT '🔧 Actualizando función de validación de integridad...' as step;

CREATE OR REPLACE FUNCTION rewards.validate_balance_integrity()
RETURNS TABLE(
  uid INTEGER,
  balance_actual NUMERIC,
  balance_calculado NUMERIC,
  diferencia NUMERIC,
  total_earns NUMERIC,
  total_spends NUMERIC
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    fbp.user_id as uid,
    fbp.balance as balance_actual,
    COALESCE(ledger.total, 0) as balance_calculado,
    (fbp.balance - COALESCE(ledger.total, 0)) as diferencia,
    COALESCE(ledger.earns, 0) as total_earns,
    ABS(COALESCE(ledger.spends, 0)) as total_spends
  FROM rewards.fact_balance_points fbp
  LEFT JOIN (
    SELECT 
      fa.user_id,
      SUM(fa.quantity) as total,
      SUM(CASE WHEN fa.quantity > 0 THEN fa.quantity ELSE 0 END) as earns,
      SUM(CASE WHEN fa.quantity < 0 THEN fa.quantity ELSE 0 END) as spends
    FROM rewards.fact_accumulations fa
    GROUP BY fa.user_id
  ) ledger ON fbp.user_id = ledger.user_id
  WHERE ABS(fbp.balance - COALESCE(ledger.total, 0)) > 0
  ORDER BY ABS(fbp.balance - COALESCE(ledger.total, 0)) DESC;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION rewards.validate_balance_integrity() IS 
'Valida integridad: balance debe ser igual a SUM(fact_accumulations.quantity). 
Earns son positivos, spends son negativos en el ledger.';

SELECT '✅ Función de validación actualizada' as result;

-- ============================================================================
-- 4. ACTUALIZAR FUNCIÓN DE AUTO-CORRECCIÓN
-- ============================================================================

SELECT '🔧 Actualizando función de auto-corrección...' as step;

CREATE OR REPLACE FUNCTION rewards.fix_balance_discrepancies()
RETURNS TABLE(
  uid INTEGER,
  balance_anterior NUMERIC,
  balance_nuevo NUMERIC,
  diferencia_corregida NUMERIC
) AS $$
BEGIN
  RETURN QUERY
  WITH discrepancias AS (
    SELECT * FROM rewards.validate_balance_integrity()
  ),
  correcciones AS (
    UPDATE rewards.fact_balance_points fbp
    SET balance = d.balance_calculado,
        latest_update = NOW()
    FROM discrepancias d
    WHERE fbp.user_id = d.uid
    RETURNING fbp.user_id, d.balance_actual, d.balance_calculado, d.diferencia
  )
  SELECT 
    c.user_id as uid,
    c.balance_actual as balance_anterior,
    c.balance_calculado as balance_nuevo,
    c.diferencia as diferencia_corregida
  FROM correcciones c;
END;
$$ LANGUAGE plpgsql;
  FROM correcciones c;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION rewards.fix_balance_discrepancies() IS 
'Corrige discrepancias recalculando balance desde el ledger (fact_accumulations).';

SELECT '✅ Función de auto-corrección actualizada' as result;

-- ============================================================================
-- 5. CREAR VISTA RESUMEN DEL LEDGER
-- ============================================================================

SELECT '🔧 Creando vista resumen del ledger...' as step;

CREATE OR REPLACE VIEW rewards.v_ledger_summary AS
SELECT 
  fa.user_id,
  COUNT(*) as total_transactions,
  SUM(fa.quantity) as balance_from_ledger,
  SUM(CASE WHEN fa.quantity > 0 THEN fa.quantity ELSE 0 END) as total_earned,
  ABS(SUM(CASE WHEN fa.quantity < 0 THEN fa.quantity ELSE 0 END)) as total_spent,
  COUNT(CASE WHEN fa.accum_type = 'earn' THEN 1 END) as earn_count,
  COUNT(CASE WHEN fa.accum_type = 'spend' THEN 1 END) as spend_count,
  MAX(fa.date) as last_transaction,
  fbp.balance as materialized_balance,
  CASE 
    WHEN fbp.balance = SUM(fa.quantity) THEN 'OK'
    ELSE 'MISMATCH'
  END as integrity_status
FROM rewards.fact_accumulations fa
LEFT JOIN rewards.fact_balance_points fbp ON fa.user_id = fbp.user_id
GROUP BY fa.user_id, fbp.balance;

COMMENT ON VIEW rewards.v_ledger_summary IS 
'Vista resumen del ledger mostrando balance calculado vs materializado por usuario.';

SELECT '✅ Vista de resumen creada' as result;

-- ============================================================================
-- 6. DOCUMENTACIÓN INLINE
-- ============================================================================

COMMENT ON TABLE rewards.fact_accumulations IS 
'LEDGER ÚNICO (Fuente de Verdad). Todas las transacciones de puntos:
- Acumulaciones (earn): quantity > 0, dtype indica origen (invoice, daily_game, streak, etc.)
- Gastos (spend): quantity < 0, dtype indica destino (points, ocr, legacy_reward)
- Reembolsos: quantity > 0, dtype = refund, redemption_id vincula al canje original
El trigger actualiza automáticamente fact_balance_points.';

COMMENT ON TABLE rewards.fact_balance_points IS 
'BALANCE MATERIALIZADO. Actualizado ÚNICAMENTE por trigger desde fact_accumulations.
balance = SUM(fact_accumulations.quantity) para el usuario.
NO modificar directamente desde código de aplicación.';

COMMENT ON TABLE rewards.user_redemptions IS 
'TABLA OPERACIONAL de redenciones. Gestiona QR codes, estados, validaciones.
NO afecta el balance directamente. El gasto se registra en fact_accumulations.';

SELECT '✅ Documentación agregada' as result;

-- ============================================================================
-- 7. VERIFICACIÓN FINAL
-- ============================================================================

SELECT '🔍 Verificando triggers activos...' as step;

SELECT 
  tgname as trigger_name,
  tgrelid::regclass as table_name,
  tgenabled as enabled
FROM pg_trigger 
WHERE tgrelid IN (
  'rewards.fact_accumulations'::regclass,
  'rewards.user_redemptions'::regclass
)
AND NOT tgisinternal
ORDER BY tgrelid::regclass::text, tgname;

SELECT '✅ Migración completada exitosamente' as final_result;

COMMIT;

-- ============================================================================
-- POST-MIGRACIÓN: Verificar integridad (ejecutar manualmente)
-- ============================================================================
-- SELECT * FROM rewards.validate_balance_integrity();
-- SELECT * FROM rewards.v_ledger_summary WHERE integrity_status = 'MISMATCH';
-- SELECT * FROM rewards.fix_balance_discrepancies(); -- Solo si hay discrepancias
-- ============================================================================
