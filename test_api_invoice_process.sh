#!/bin/bash

# Script para probar el API POST /api/invoices/process
# Fecha: $(date)

echo "🧪 TESTING API POST /api/invoices/process"
echo "========================================"

# URL de la factura de prueba (la que ya probamos en el scraper)
INVOICE_URL="https://dgi-fep.mef.gob.pa/FacturasPorQR/ConsultasFactura/Consultar?FacturaConsultar=FE01200002679372-1-844914-7300002025051500311570140020317481978892"

# Puerto del servidor (por defecto 8000)
PORT=${1:-8000}
BASE_URL="http://localhost:${PORT}"

echo "🌐 Base URL: ${BASE_URL}"
echo "📄 Invoice URL: ${INVOICE_URL}"
echo ""

# Test 1: Probar con datos válidos
echo "📋 TEST 1: Datos válidos (nueva factura)"
echo "----------------------------------------"

RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d "{
    \"url\": \"${INVOICE_URL}\",
    \"user_id\": \"test_user_001\",
    \"user_email\": \"test@example.com\",
    \"origin\": \"whatsapp\"
  }" \
  "${BASE_URL}/api/invoices/process")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1 | cut -d: -f2)
BODY=$(echo "$RESPONSE" | head -n -1)

echo "HTTP Status: ${HTTP_CODE}"
echo "Response Body:"
echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
echo ""

# Test 2: Probar la misma URL (debería retornar 409 Duplicate)
echo "📋 TEST 2: Misma URL (debería ser duplicado)"
echo "--------------------------------------------"

RESPONSE2=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d "{
    \"url\": \"${INVOICE_URL}\",
    \"user_id\": \"test_user_002\",
    \"user_email\": \"test2@example.com\",
    \"origin\": \"aplicacion\"
  }" \
  "${BASE_URL}/api/invoices/process")

HTTP_CODE2=$(echo "$RESPONSE2" | tail -n1 | cut -d: -f2)
BODY2=$(echo "$RESPONSE2" | head -n -1)

echo "HTTP Status: ${HTTP_CODE2}"
echo "Response Body:"
echo "$BODY2" | jq '.' 2>/dev/null || echo "$BODY2"
echo ""

# Test 3: Probar con URL inválida
echo "📋 TEST 3: URL inválida (debería retornar 400)"
echo "----------------------------------------------"

RESPONSE3=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d "{
    \"url\": \"https://invalid-url.com/factura\",
    \"user_id\": \"test_user_003\",
    \"user_email\": \"test3@example.com\",
    \"origin\": \"telegram\"
  }" \
  "${BASE_URL}/api/invoices/process")

HTTP_CODE3=$(echo "$RESPONSE3" | tail -n1 | cut -d: -f2)
BODY3=$(echo "$RESPONSE3" | head -n -1)

echo "HTTP Status: ${HTTP_CODE3}"
echo "Response Body:"
echo "$BODY3" | jq '.' 2>/dev/null || echo "$BODY3"
echo ""

# Test 4: Probar con email inválido
echo "📋 TEST 4: Email inválido (debería retornar 400)"
echo "------------------------------------------------"

RESPONSE4=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d "{
    \"url\": \"${INVOICE_URL}\",
    \"user_id\": \"test_user_004\",
    \"user_email\": \"invalid-email\",
    \"origin\": \"whatsapp\"
  }" \
  "${BASE_URL}/api/invoices/process")

HTTP_CODE4=$(echo "$RESPONSE4" | tail -n1 | cut -d: -f2)
BODY4=$(echo "$RESPONSE4" | head -n -1)

echo "HTTP Status: ${HTTP_CODE4}"
echo "Response Body:"
echo "$BODY4" | jq '.' 2>/dev/null || echo "$BODY4"
echo ""

# Resumen
echo "📊 RESUMEN DE PRUEBAS"
echo "===================="
echo "Test 1 (Datos válidos):    HTTP ${HTTP_CODE} (esperado: 200)"
echo "Test 2 (Duplicado):        HTTP ${HTTP_CODE2} (esperado: 409)"
echo "Test 3 (URL inválida):     HTTP ${HTTP_CODE3} (esperado: 400)"
echo "Test 4 (Email inválido):   HTTP ${HTTP_CODE4} (esperado: 400)"
echo ""

# Verificar servidor
echo "🔍 Estado del servidor:"
ps aux | grep -E "(rust|lum_rust)" | grep -v grep || echo "❌ Servidor no encontrado"
echo ""

echo "✅ Pruebas completadas"
