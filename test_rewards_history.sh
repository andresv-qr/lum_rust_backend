#!/bin/bash

BASE_URL="http://localhost:8000"
EMAIL="andresfelipevalenciag@gmail.com"
PASSWORD="Andres123+"

echo "========================================="
echo "Test 1: Login con auth_v4"
echo "========================================="
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v4/auth/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$EMAIL\",
    \"password\": \"$PASSWORD\"
  }")

echo "$LOGIN_RESPONSE" | jq '.'

# Extract token
TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.token // .access_token // empty')

if [ -z "$TOKEN" ] || [ "$TOKEN" = "null" ]; then
    echo "❌ Error: No se pudo obtener el token"
    exit 1
fi

echo ""
echo "✅ Token obtenido: ${TOKEN:0:50}..."

echo ""
echo "========================================="
echo "Test 2: GET /api/v1/rewards/history (sin parámetros)"
echo "========================================="
HISTORY_RESPONSE=$(curl -s -X GET "$BASE_URL/api/v1/rewards/history" \
  -H "Authorization: Bearer $TOKEN")

echo "$HISTORY_RESPONSE" | jq '.'

echo ""
echo "========================================="
echo "Test 3: GET /api/v1/rewards/history con limit"
echo "========================================="
HISTORY_LIMIT=$(curl -s -X GET "$BASE_URL/api/v1/rewards/history?limit=5" \
  -H "Authorization: Bearer $TOKEN")

echo "$HISTORY_LIMIT" | jq '.'

echo ""
echo "========================================="
echo "Test 4: GET /api/v1/rewards/history con limit y offset"
echo "========================================="
HISTORY_OFFSET=$(curl -s -X GET "$BASE_URL/api/v1/rewards/history?limit=5&offset=0" \
  -H "Authorization: Bearer $TOKEN")

echo "$HISTORY_OFFSET" | jq '.'

echo ""
echo "========================================="
echo "Test 5: GET /api/v1/rewards/history filtrando por status"
echo "========================================="
HISTORY_STATUS=$(curl -s -X GET "$BASE_URL/api/v1/rewards/history?status=completed" \
  -H "Authorization: Bearer $TOKEN")

echo "$HISTORY_STATUS" | jq '.'

echo ""
echo "========================================="
echo "Test 6: Verificar estructura de la respuesta"
echo "========================================="
echo "Campos esperados en cada redención:"
echo "- redemption_id"
echo "- offer_id"
echo "- offer_name"
echo "- cost"
echo "- status"
echo "- created_at"
echo "- merchant_name (opcional)"

# Verificar que tenga al menos estos campos
FIRST_ITEM=$(echo "$HISTORY_RESPONSE" | jq '.[0] // empty')
if [ ! -z "$FIRST_ITEM" ]; then
    echo ""
    echo "Primer registro completo:"
    echo "$FIRST_ITEM" | jq '.'
    
    # Validar campos
    HAS_ID=$(echo "$FIRST_ITEM" | jq 'has("redemption_id")')
    HAS_OFFER=$(echo "$FIRST_ITEM" | jq 'has("offer_id")')
    HAS_STATUS=$(echo "$FIRST_ITEM" | jq 'has("status")')
    HAS_CREATED=$(echo "$FIRST_ITEM" | jq 'has("created_at")')
    HAS_COST=$(echo "$FIRST_ITEM" | jq 'has("cost")')
    
    echo ""
    echo "Validación de campos:"
    echo "- redemption_id: $HAS_ID"
    echo "- offer_id: $HAS_OFFER"
    echo "- status: $HAS_STATUS"
    echo "- created_at: $HAS_CREATED"
    echo "- cost: $HAS_COST"
else
    echo "⚠️ No hay registros en el historial"
fi

echo ""
echo "========================================="
echo "Test 7: GET /api/v1/rewards/stats - Estadísticas"
echo "========================================="
STATS_RESPONSE=$(curl -s -X GET "$BASE_URL/api/v1/rewards/stats" \
  -H "Authorization: Bearer $TOKEN")

echo "$STATS_RESPONSE" | jq '.'

echo ""
echo "========================================="
echo "Test 8: Sin autenticación (debe fallar)"
echo "========================================="
NO_AUTH=$(curl -s -w "\nHTTP Status: %{http_code}\n" -X GET "$BASE_URL/api/v1/rewards/history")
echo "$NO_AUTH"

echo ""
echo "========================================="
echo "RESUMEN DE PRUEBAS"
echo "========================================="
echo "✅ Tests completados"
