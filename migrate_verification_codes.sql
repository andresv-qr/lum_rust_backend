-- MIGRACIÓN SISTEMA UNIFICADO DE VERIFICACIÓN
-- Fecha: 26 de Septiembre, 2025
-- Propósito: Unificar códigos de verificación en PostgreSQL

-- ============================================================================
-- 1. VERIFICAR TABLA EXISTENTE
-- ============================================================================

-- Verificar estructura de password_verification_codes
SELECT column_name, data_type, is_nullable, column_default 
FROM information_schema.columns 
WHERE table_name = 'password_verification_codes' 
ORDER BY ordinal_position;

-- ============================================================================
-- 2. AGREGAR NUEVO PURPOSE SI NO EXISTE
-- ============================================================================

-- Verificar si ya existe constraint del enum purpose
SELECT constraint_name, check_clause 
FROM information_schema.check_constraints 
WHERE constraint_name LIKE '%purpose%';

-- Si necesitas recrear el constraint con el nuevo valor:
-- ALTER TABLE password_verification_codes DROP CONSTRAINT IF EXISTS password_verification_codes_purpose_check;
-- ALTER TABLE password_verification_codes ADD CONSTRAINT password_verification_codes_purpose_check 
--   CHECK (purpose IN ('first_time_setup', 'reset_password', 'change_password', 'email_verification'));

-- ============================================================================
-- 3. CLEANUP DE CÓDIGOS EXPIRADOS
-- ============================================================================

-- Limpiar códigos expirados antes de la migración
DELETE FROM password_verification_codes 
WHERE expires_at < NOW();

-- ============================================================================
-- 4. ESTADÍSTICAS PRE-MIGRACIÓN
-- ============================================================================

-- Ver códigos existentes por purpose
SELECT 
    purpose,
    COUNT(*) as count,
    COUNT(CASE WHEN used_at IS NULL THEN 1 END) as unused_count,
    COUNT(CASE WHEN expires_at > NOW() THEN 1 END) as valid_count
FROM password_verification_codes 
GROUP BY purpose;

-- Ver códigos por usuario
SELECT 
    email,
    COUNT(*) as total_codes,
    COUNT(CASE WHEN used_at IS NULL AND expires_at > NOW() THEN 1 END) as active_codes
FROM password_verification_codes 
GROUP BY email
HAVING COUNT(*) > 1
ORDER BY total_codes DESC;

-- ============================================================================
-- 5. NOTAS IMPORTANTES
-- ============================================================================

/*
IMPORTANTE: MIGRACIÓN DE REDIS A POSTGRESQL

1. Los códigos en Redis (send-verification) ya NO se usarán
2. Todos los nuevos códigos van a PostgreSQL
3. Los endpoints existentes siguen funcionando pero usan PostgreSQL
4. Redis keys como 'verification:{email}:verify_account' se pueden limpiar

FLUJOS DESPUÉS DE LA MIGRACIÓN:
- send-verification → request-code (purpose: email_verification)
- verify-account → usa PostgreSQL en lugar de Redis
- set-with-code → sigue igual (ya usaba PostgreSQL)

CLEANUP REDIS (opcional):
- Limpiar keys: verification:*:verify_account
- Limpiar keys: verification:*:set_password  
- Limpiar keys: verification:*:reset_password

TESTING:
1. Probar send-verification + verify-account (flujo email)
2. Probar request-code + set-with-code (flujo contraseña)
3. Probar request-code(email_verification) + verify-account (flujo híbrido)
*/

-- ============================================================================
-- 6. VERIFICACIÓN POST-MIGRACIÓN
-- ============================================================================

-- Query para monitorear el nuevo sistema
SELECT 
    'Sistema Unificado - Estadísticas' as info,
    COUNT(*) as total_codes,
    COUNT(CASE WHEN purpose = 'email_verification' THEN 1 END) as email_verification_codes,
    COUNT(CASE WHEN purpose = 'first_time_setup' THEN 1 END) as first_time_setup_codes,
    COUNT(CASE WHEN purpose = 'reset_password' THEN 1 END) as reset_password_codes,
    COUNT(CASE WHEN purpose = 'change_password' THEN 1 END) as change_password_codes,
    COUNT(CASE WHEN used_at IS NULL AND expires_at > NOW() THEN 1 END) as active_codes
FROM password_verification_codes;