-- ============================================================================
-- FIX: Sistema de Balance Incremental - PARTE 1 (Triggers)
-- ============================================================================
-- Ejecutar primero para crear los triggers críticos

-- ============================================================================
-- 1. ELIMINAR triggers y funciones antiguas
-- ============================================================================

SELECT 'Eliminando triggers antiguos...' as step;

-- Eliminar trigger de fact_accumulations
DROP TRIGGER IF EXISTS trigger_accumulations_points_updatebalance 
ON rewards.fact_accumulations;

-- Eliminar función antigua
DROP FUNCTION IF EXISTS rewards.fun_update_balance_points();

SELECT '✅ Triggers antiguos eliminados' as result;

-- ============================================================================
-- 2. CREAR NUEVA FUNCIÓN INCREMENTAL para fact_accumulations
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
-- 3. CREAR TRIGGER para user_redemptions (restar al confirmar/pending)
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
-- VERIFICACIÓN
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

SELECT '
╔═══════════════════════════════════════════════════════════════════╗
║                ✅ TRIGGERS INCREMENTALES INSTALADOS                ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                   ║
║  ✅ Trigger en fact_accumulations: SUMA al balance                ║
║  ✅ Trigger en user_redemptions: RESTA al balance                 ║
║                                                                   ║
║  AHORA puedes subir facturas sin que se borre el balance         ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
' as resultado;
