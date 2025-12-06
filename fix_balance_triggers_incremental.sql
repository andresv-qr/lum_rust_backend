-- ============================================================================
-- FIX: Sistema de Balance Incremental para fact_balance_points
-- ============================================================================
-- Fecha: 2025-10-19
-- Problema: El trigger actual recalcula TODO el balance en cada INSERT
--           causando que se "borre" el balance al subir facturas
-- Solución: Triggers incrementales + validación nocturna
-- ============================================================================

BEGIN;

-- ============================================================================
-- 1. BACKUP del trigger actual (por si necesitas revertir)
-- ============================================================================

-- Ver el trigger actual
SELECT 'BACKUP del trigger actual:' as step;
SELECT proname, prosrc 
FROM pg_proc 
WHERE proname = 'fun_update_balance_points';

-- ============================================================================
-- 2. ELIMINAR triggers y funciones antiguas
-- ============================================================================

SELECT 'Eliminando triggers antiguos...' as step;

-- Eliminar trigger de fact_accumulations
DROP TRIGGER IF EXISTS trigger_accumulations_points_updatebalance 
ON rewards.fact_accumulations;

-- Eliminar función antigua
DROP FUNCTION IF EXISTS rewards.fun_update_balance_points();

SELECT '✅ Triggers antiguos eliminados' as result;

-- ============================================================================
-- 3. CREAR NUEVA FUNCIÓN INCREMENTAL para fact_accumulations
-- ============================================================================

SELECT 'Creando trigger incremental para acumulaciones...' as step;

CREATE OR REPLACE FUNCTION rewards.fun_update_balance_points_incremental()
RETURNS TRIGGER AS $$
BEGIN
  -- INCREMENTAL: Solo sumar la cantidad del nuevo registro
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
'Trigger incremental: suma quantity al balance existente en cada INSERT a fact_accumulations';

-- Crear el trigger
DROP TRIGGER IF EXISTS trigger_accumulations_incremental 
ON rewards.fact_accumulations;

CREATE TRIGGER trigger_accumulations_incremental
AFTER INSERT ON rewards.fact_accumulations
FOR EACH ROW
EXECUTE FUNCTION rewards.fun_update_balance_points_incremental();

SELECT '✅ Trigger incremental para acumulaciones creado' as result;

-- ============================================================================
-- 4. CREAR TRIGGER para user_redemptions (restar al confirmar/pending)
-- ============================================================================

SELECT 'Creando trigger para redenciones...' as step;

CREATE OR REPLACE FUNCTION rewards.fun_update_balance_on_redemption()
RETURNS TRIGGER AS $$
BEGIN
  -- CASO 1: INSERT con status 'pending' o 'confirmed' - restar inmediatamente
  IF TG_OP = 'INSERT' AND NEW.redemption_status IN ('pending', 'confirmed') THEN
    
    UPDATE rewards.fact_balance_points
    SET balance = balance - NEW.lumis_spent,
        latest_update = NOW()
    WHERE user_id = NEW.user_id;
    
    IF NOT FOUND THEN
      RAISE EXCEPTION 'Usuario % no tiene balance en fact_balance_points', NEW.user_id;
    END IF;
  
  -- CASO 2: UPDATE de pending a confirmed - ya se restó en el INSERT, no hacer nada
  ELSIF TG_OP = 'UPDATE' 
        AND OLD.redemption_status = 'pending' 
        AND NEW.redemption_status = 'confirmed' THEN
    -- No hacer nada, ya se restó cuando se creó como 'pending'
    NULL;
  
  -- CASO 3: UPDATE a 'cancelled' - devolver los lümis
  ELSIF TG_OP = 'UPDATE' 
        AND OLD.redemption_status IN ('pending', 'confirmed')
        AND NEW.redemption_status = 'cancelled' THEN
    
    UPDATE rewards.fact_balance_points
    SET balance = balance + OLD.lumis_spent,
        latest_update = NOW()
    WHERE user_id = OLD.user_id;
  
  -- CASO 4: UPDATE a 'expired' - devolver los lümis
  ELSIF TG_OP = 'UPDATE' 
        AND OLD.redemption_status = 'pending'
        AND NEW.redemption_status = 'expired' THEN
    
    UPDATE rewards.fact_balance_points
    SET balance = balance + OLD.lumis_spent,
        latest_update = NOW()
    WHERE user_id = OLD.user_id;
  
  END IF;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION rewards.fun_update_balance_on_redemption() IS 
'Trigger incremental: resta lumis_spent al crear redención, devuelve al cancelar/expirar';

-- Reemplazar el trigger existente
DROP TRIGGER IF EXISTS trigger_update_balance_on_redemption 
ON rewards.user_redemptions;

CREATE TRIGGER trigger_update_balance_on_redemption
AFTER INSERT OR UPDATE ON rewards.user_redemptions
FOR EACH ROW
EXECUTE FUNCTION rewards.fun_update_balance_on_redemption();

SELECT '✅ Trigger para redenciones creado' as result;

-- ============================================================================
-- 5. FUNCIÓN DE VALIDACIÓN NOCTURNA (detectar discrepancias)
-- ============================================================================

SELECT 'Creando función de validación nocturna...' as step;

CREATE OR REPLACE FUNCTION rewards.validate_balance_integrity()
RETURNS TABLE(
  uid INTEGER,
  balance_actual BIGINT,
  balance_calculado BIGINT,
  diferencia BIGINT,
  acumulaciones_total BIGINT,
  redenciones_total BIGINT
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    fbp.user_id as uid,
    fbp.balance as balance_actual,
    (COALESCE(acum.total, 0) - COALESCE(redem.total, 0))::BIGINT as balance_calculado,
    (fbp.balance - (COALESCE(acum.total, 0) - COALESCE(redem.total, 0)))::BIGINT as diferencia,
    COALESCE(acum.total, 0)::BIGINT as acumulaciones_total,
    COALESCE(redem.total, 0)::BIGINT as redenciones_total
  FROM rewards.fact_balance_points fbp
  LEFT JOIN (
    SELECT fa.user_id, SUM(fa.quantity)::BIGINT as total
    FROM rewards.fact_accumulations fa
    GROUP BY fa.user_id
  ) acum ON fbp.user_id = acum.user_id
  LEFT JOIN (
    SELECT ur.user_id, SUM(ur.lumis_spent)::BIGINT as total
    FROM rewards.user_redemptions ur
    WHERE ur.redemption_status IN ('pending', 'confirmed')
    GROUP BY ur.user_id
  ) redem ON fbp.user_id = redem.user_id
  WHERE ABS(fbp.balance - (COALESCE(acum.total, 0) - COALESCE(redem.total, 0))) > 0
  ORDER BY ABS(fbp.balance - (COALESCE(acum.total, 0) - COALESCE(redem.total, 0))) DESC;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION rewards.validate_balance_integrity() IS 
'Valida integridad del balance comparando fact_balance_points vs suma real de acumulaciones - redenciones';

SELECT '✅ Función de validación creada' as result;

-- ============================================================================
-- 6. FUNCIÓN DE AUTO-CORRECCIÓN (reparar discrepancias)
-- ============================================================================

SELECT 'Creando función de auto-corrección...' as step;

CREATE OR REPLACE FUNCTION rewards.fix_balance_discrepancies()
RETURNS TABLE(
  uid INTEGER,
  balance_anterior BIGINT,
  balance_nuevo BIGINT,
  diferencia_corregida BIGINT
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

COMMENT ON FUNCTION rewards.fix_balance_discrepancies() IS 
'Corrige automáticamente las discrepancias encontradas recalculando el balance correcto';

SELECT '✅ Función de auto-corrección creada' as result;

-- ============================================================================
-- 7. RECALCULAR TODOS LOS BALANCES ACTUALES (una sola vez)
-- ============================================================================

SELECT 'Recalculando balances actuales...' as step;

-- Primero, validar cuántas discrepancias hay
SELECT 'Discrepancias encontradas ANTES de corregir:' as info;
SELECT COUNT(*) as total_usuarios_con_error
FROM rewards.validate_balance_integrity();

-- Mostrar las primeras 10 discrepancias
SELECT 'Primeras discrepancias:' as info;
SELECT * FROM rewards.validate_balance_integrity() LIMIT 10;

-- Preguntar antes de corregir (comentar esta línea si quieres auto-corregir)
DO $$
BEGIN
  RAISE NOTICE '=================================================================';
  RAISE NOTICE 'IMPORTANTE: Revisa las discrepancias arriba';
  RAISE NOTICE 'Si todo se ve bien, ejecuta: SELECT * FROM rewards.fix_balance_discrepancies();';
  RAISE NOTICE '=================================================================';
END $$;

-- Descomentar la siguiente línea para auto-corregir inmediatamente
-- SELECT * FROM rewards.fix_balance_discrepancies();

SELECT '⚠️  Revisa las discrepancias y ejecuta fix_balance_discrepancies() si es necesario' as result;

-- ============================================================================
-- 8. CREAR VISTA DE MONITOREO (para dashboard)
-- ============================================================================

SELECT 'Creando vista de monitoreo...' as step;

CREATE OR REPLACE VIEW rewards.vw_balance_health AS
SELECT 
  COUNT(*) as total_usuarios,
  COUNT(*) FILTER (WHERE diferencia = 0) as usuarios_correctos,
  COUNT(*) FILTER (WHERE diferencia != 0) as usuarios_con_error,
  ROUND(100.0 * COUNT(*) FILTER (WHERE diferencia = 0) / COUNT(*), 2) as porcentaje_salud,
  SUM(ABS(diferencia)) as suma_total_diferencias
FROM (
  SELECT 
    fbp.user_id,
    (fbp.balance - (COALESCE(acum.total, 0) - COALESCE(redem.total, 0))) as diferencia
  FROM rewards.fact_balance_points fbp
  LEFT JOIN (
    SELECT user_id, SUM(quantity)::BIGINT as total
    FROM rewards.fact_accumulations
    GROUP BY user_id
  ) acum ON fbp.user_id = acum.user_id
  LEFT JOIN (
    SELECT user_id, SUM(lumis_spent)::BIGINT as total
    FROM rewards.user_redemptions
    WHERE redemption_status IN ('pending', 'confirmed')
    GROUP BY user_id
  ) redem ON fbp.user_id = redem.user_id
) subq;

COMMENT ON VIEW rewards.vw_balance_health IS 
'Vista de salud del sistema de balance - muestra % de usuarios con balance correcto';

SELECT '✅ Vista de monitoreo creada' as result;

-- ============================================================================
-- 9. VERIFICACIÓN FINAL
-- ============================================================================

SELECT 'Verificando triggers instalados...' as step;

SELECT 
  t.tgname as trigger_name,
  c.relname as table_name,
  p.proname as function_name,
  CASE 
    WHEN t.tgenabled = 'O' THEN 'ENABLED'
    ELSE 'DISABLED'
  END as status
FROM pg_trigger t
JOIN pg_class c ON t.tgrelid = c.oid
JOIN pg_proc p ON t.tgfoid = p.oid
WHERE c.relnamespace = 'rewards'::regnamespace
  AND c.relname IN ('fact_accumulations', 'user_redemptions')
  AND t.tgname NOT LIKE 'RI_%'
ORDER BY c.relname, t.tgname;

SELECT 'Estado del sistema de balance:' as step;
SELECT * FROM rewards.vw_balance_health;

COMMIT;

-- ============================================================================
-- RESULTADO ESPERADO
-- ============================================================================
SELECT '
╔═══════════════════════════════════════════════════════════════════╗
║                    ✅ MIGRACIÓN COMPLETADA                        ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                   ║
║  ✅ Triggers antiguos eliminados                                  ║
║  ✅ Trigger incremental en fact_accumulations instalado           ║
║  ✅ Trigger incremental en user_redemptions instalado             ║
║  ✅ Función de validación nocturna creada                         ║
║  ✅ Función de auto-corrección creada                             ║
║  ✅ Vista de monitoreo creada                                     ║
║                                                                   ║
╠═══════════════════════════════════════════════════════════════════╣
║  PRÓXIMOS PASOS:                                                  ║
║                                                                   ║
║  1. Revisar discrepancias:                                        ║
║     SELECT * FROM rewards.validate_balance_integrity();           ║
║                                                                   ║
║  2. Corregir balances (si es necesario):                          ║
║     SELECT * FROM rewards.fix_balance_discrepancies();            ║
║                                                                   ║
║  3. Monitorear salud del sistema:                                 ║
║     SELECT * FROM rewards.vw_balance_health;                      ║
║                                                                   ║
║  4. Programar validación nocturna (cron):                         ║
║     0 3 * * * psql -c "SELECT * FROM rewards.fix_balance_discrepancies()" ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
' as resultado;
