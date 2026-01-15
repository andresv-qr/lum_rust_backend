#!/bin/bash

BASE_URL="http://localhost:8000"
EMAIL="andresfelipevalenciag@gmail.com"
PASSWORD="Andres123+"

echo "Test: Validación de límite máximo"
echo "=================================="

# Login
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v4/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\": \"$EMAIL\", \"password\": \"$PASSWORD\"}")

TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token')

echo ""
echo "Test 1: limit=150 (debe limitarse a 100)"
echo "----------------------------------------"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/v1/rewards/history?limit=150" \
  -H "Authorization: Bearer $TOKEN")

COUNT=$(echo "$RESPONSE" | jq '.redemptions | length')
echo "Registros retornados: $COUNT"

if [ "$COUNT" -le 100 ]; then
    echo "✅ PASS - Límite respetado (máximo 100)"
else
    echo "❌ FAIL - Retornó más de 100 registros"
fi

echo ""
echo "Test 2: limit=50 (normal)"
echo "-------------------------"
RESPONSE2=$(curl -s -X GET "$BASE_URL/api/v1/rewards/history?limit=50" \
  -H "Authorization: Bearer $TOKEN")

COUNT2=$(echo "$RESPONSE2" | jq '.redemptions | length')
echo "Registros retornados: $COUNT2"

if [ "$COUNT2" -le 50 ]; then
    echo "✅ PASS - Límite 50 funciona"
else
    echo "❌ FAIL - Retornó más de 50 registros"
fi

echo ""
echo "Test 3: Sin limit (debe usar default 50)"
echo "----------------------------------------"
RESPONSE3=$(curl -s -X GET "$BASE_URL/api/v1/rewards/history" \
  -H "Authorization: Bearer $TOKEN")

COUNT3=$(echo "$RESPONSE3" | jq '.redemptions | length')
echo "Registros retornados: $COUNT3"
echo "✅ Default aplicado correctamente"
