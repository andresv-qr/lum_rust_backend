#!/bin/bash
# Script de prueba para PUT /api/v4/userdata
# Uso: ./test_put_userdata.sh

# Configuración
API_URL="http://localhost:3000/api/v4/userdata"
JWT_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJlbWFpbCI6ImFuZHJlc2ZlbGlwZXZhbGVuY2lhZ0BnbWFpbC5jb20ifQ.Kv1OdRjKlFI4e6u3NZmNyh_Mf8BQ9zNiQxPD0RmM8cE"

echo "🧪 Testing PUT /api/v4/userdata"
echo "================================"
echo ""

# Test 1: Actualizar solo el nombre
echo "📝 Test 1: Actualizar solo el nombre"
echo "------------------------------------"
curl -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Juan Pérez Actualizado"}' \
  | jq .

echo ""
echo ""

# Test 2: Actualizar múltiples campos
echo "📝 Test 2: Actualizar múltiples campos"
echo "---------------------------------------"
curl -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "María García",
    "country_origin": "Colombia",
    "country_residence": "Panama",
    "segment_activity": "Technology",
    "genre": "F"
  }' \
  | jq .

echo ""
echo ""

# Test 3: Actualizar ws_id
echo "📝 Test 3: Actualizar WhatsApp ID"
echo "----------------------------------"
curl -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"ws_id": "507-1234-5678"}' \
  | jq .

echo ""
echo ""

# Test 4: Request vacío (debe fallar con 400)
echo "📝 Test 4: Request vacío (esperado: 400 BAD REQUEST)"
echo "----------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'

echo ""
echo ""

# Test 5: Sin JWT token (debe fallar con 401)
echo "📝 Test 5: Sin JWT token (esperado: 401 UNAUTHORIZED)"
echo "------------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Content-Type: application/json" \
  -d '{"name": "Test Sin Auth"}'

echo ""
echo ""

# Test 6: Verificar GET después de PUT
echo "📝 Test 6: Verificar datos con GET después de UPDATE"
echo "----------------------------------------------------"
curl -X GET "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  | jq .

echo ""
echo ""
echo "✅ Tests completados!"
