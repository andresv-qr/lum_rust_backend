#!/bin/bash

# ============================================================================
# SCRIPT DE PRUEBA PARA AUDIT LOGGING
# ============================================================================
# Fecha: 19 de Septiembre, 2025
# Propósito: Verificar que los logs de auditoría se guarden correctamente
# ============================================================================

set -e

BASE_URL="http://localhost:8000"
API_URL="$BASE_URL/api/v4/auth/unified"

echo "🧪 PRUEBAS DE AUDIT LOGGING - Sistema de Autenticación Unificado"
echo "=================================================================="
echo ""

# Función para verificar logs en la base de datos
check_logs() {
    echo "📋 Verificando logs en la base de datos..."
    psql -U avalencia -d tfactu -h localhost -c "
        SELECT 
            id, user_id, event_type, provider, success, 
            error_code, error_message, created_at 
        FROM public.auth_audit_log 
        ORDER BY created_at DESC 
        LIMIT 10;
    " || echo "❌ Error al consultar logs"
    echo ""
}

# Función para limpiar logs antiguos
clean_logs() {
    echo "🧹 Limpiando logs antiguos para prueba limpia..."
    psql -U avalencia -d tfactu -h localhost -c "
        DELETE FROM public.auth_audit_log 
        WHERE created_at < NOW() - INTERVAL '1 minute';
    " || echo "❌ Error al limpiar logs"
    echo ""
}

# 1. LIMPIAR LOGS ANTIGUOS
clean_logs

# 2. PRUEBA: LOGIN EXITOSO CON EMAIL
echo "🔑 Test 1: Login exitoso con email"
curl -s -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -H "User-Agent: TestScript/1.0" \
  -H "X-Forwarded-For: 192.168.1.100" \
  -d '{
    "provider": "email",
    "email": "avalencia@example.com",
    "password": "password123",
    "client_info": {
      "ip_address": "192.168.1.100",
      "user_agent": "TestScript/1.0",
      "device_id": "test-device-1"
    },
    "create_if_not_exists": false
  }' | jq '.' || echo "❌ Error en login exitoso"

echo ""
check_logs

# 3. PRUEBA: LOGIN FALLIDO - CREDENCIALES INCORRECTAS
echo "❌ Test 2: Login fallido - credenciales incorrectas"
curl -s -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -H "User-Agent: TestScript/1.0" \
  -H "X-Forwarded-For: 192.168.1.101" \
  -d '{
    "provider": "email",
    "email": "usuario@inexistente.com",
    "password": "password_incorrecto",
    "client_info": {
      "ip_address": "192.168.1.101",
      "user_agent": "TestScript/1.0",
      "device_id": "test-device-2"
    },
    "create_if_not_exists": false
  }' | jq '.' || echo "❌ Error en request"

echo ""
check_logs

# 4. PRUEBA: REGISTRO DE NUEVO USUARIO
echo "📝 Test 3: Registro de nuevo usuario"
RANDOM_EMAIL="test_$(date +%s)@example.com"
curl -s -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -H "User-Agent: TestScript/1.0" \
  -H "X-Forwarded-For: 192.168.1.102" \
  -d "{
    \"provider\": \"email\",
    \"email\": \"$RANDOM_EMAIL\",
    \"password\": \"password123\",
    \"name\": \"Usuario Test\",
    \"client_info\": {
      \"ip_address\": \"192.168.1.102\",
      \"user_agent\": \"TestScript/1.0\",
      \"device_id\": \"test-device-3\"
    },
    \"create_if_not_exists\": true
  }" | jq '.' || echo "❌ Error en registro"

echo ""
check_logs

# 5. VERIFICAR LOGS FINALES
echo "📊 RESUMEN FINAL DE LOGS DE AUDITORÍA"
echo "====================================="
psql -U avalencia -d tfactu -h localhost -c "
    SELECT 
        event_type,
        provider,
        success,
        COUNT(*) as count
    FROM public.auth_audit_log 
    WHERE created_at > NOW() - INTERVAL '5 minutes'
    GROUP BY event_type, provider, success
    ORDER BY event_type, provider, success;
" || echo "❌ Error al obtener resumen"

echo ""
echo "✅ Pruebas de audit logging completadas!"
echo ""
echo "💡 Para revisar logs detallados:"
echo "   psql -U avalencia -d tfactu -h localhost -c \"SELECT * FROM public.auth_audit_log ORDER BY created_at DESC LIMIT 20;\""