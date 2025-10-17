#!/bin/bash
# Script de prueba para PUT /api/v4/userdata/password
# Uso: ./test_change_password.sh

# Configuración
API_URL="http://localhost:3000/api/v4/userdata/password"
JWT_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJlbWFpbCI6ImFuZHJlc2ZlbGlwZXZhbGVuY2lhZ0BnbWFpbC5jb20ifQ.Kv1OdRjKlFI4e6u3NZmNyh_Mf8BQ9zNiQxPD0RmM8cE"

# Contraseñas de prueba (ajustar según tu usuario)
CURRENT_PASSWORD="Password123!"
NEW_PASSWORD="NewPassword456!"

echo "🔐 Testing PUT /api/v4/userdata/password"
echo "=========================================="
echo ""

# Test 1: Cambio exitoso de contraseña
echo "📝 Test 1: Cambio exitoso de contraseña"
echo "----------------------------------------"
curl -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"current_password\": \"$CURRENT_PASSWORD\",
    \"new_password\": \"$NEW_PASSWORD\",
    \"confirmation_password\": \"$NEW_PASSWORD\"
  }" \
  | jq .

echo ""
echo ""

# Test 2: Contraseñas de confirmación no coinciden
echo "📝 Test 2: Contraseñas de confirmación no coinciden (esperado: 400)"
echo "--------------------------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "'"$NEW_PASSWORD"'",
    "new_password": "AnotherPassword789!",
    "confirmation_password": "DifferentPassword789!"
  }' 2>&1 | head -20

echo ""
echo ""

# Test 3: Contraseña actual incorrecta
echo "📝 Test 3: Contraseña actual incorrecta (esperado: 401)"
echo "-------------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "WrongPassword123!",
    "new_password": "AnotherPassword789!",
    "confirmation_password": "AnotherPassword789!"
  }' 2>&1 | head -20

echo ""
echo ""

# Test 4: Nueva contraseña no cumple requisitos (sin mayúscula)
echo "📝 Test 4: Nueva contraseña sin mayúscula (esperado: 400)"
echo "----------------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "'"$NEW_PASSWORD"'",
    "new_password": "weakpassword123!",
    "confirmation_password": "weakpassword123!"
  }' 2>&1 | head -20

echo ""
echo ""

# Test 5: Nueva contraseña sin número
echo "📝 Test 5: Nueva contraseña sin número (esperado: 400)"
echo "-------------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "'"$NEW_PASSWORD"'",
    "new_password": "WeakPassword!",
    "confirmation_password": "WeakPassword!"
  }' 2>&1 | head -20

echo ""
echo ""

# Test 6: Nueva contraseña sin carácter especial
echo "📝 Test 6: Nueva contraseña sin carácter especial (esperado: 400)"
echo "------------------------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "'"$NEW_PASSWORD"'",
    "new_password": "WeakPassword123",
    "confirmation_password": "WeakPassword123"
  }' 2>&1 | head -20

echo ""
echo ""

# Test 7: Contraseña muy corta (menos de 8 caracteres)
echo "📝 Test 7: Contraseña muy corta (esperado: 400)"
echo "------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "'"$NEW_PASSWORD"'",
    "new_password": "Pass1!",
    "confirmation_password": "Pass1!"
  }' 2>&1 | head -20

echo ""
echo ""

# Test 8: Sin JWT token (esperado: 401)
echo "📝 Test 8: Sin JWT token (esperado: 401 UNAUTHORIZED)"
echo "------------------------------------------------------"
curl -i -X PUT "$API_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "SomePassword123!",
    "new_password": "NewPassword456!",
    "confirmation_password": "NewPassword456!"
  }' 2>&1 | head -20

echo ""
echo ""

# Test 9: Revertir cambio (volver a contraseña original)
echo "📝 Test 9: Revertir a contraseña original"
echo "-----------------------------------------"
curl -X PUT "$API_URL" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"current_password\": \"$NEW_PASSWORD\",
    \"new_password\": \"$CURRENT_PASSWORD\",
    \"confirmation_password\": \"$CURRENT_PASSWORD\"
  }" \
  | jq .

echo ""
echo ""
echo "✅ Tests completados!"
echo ""
echo "📊 Resumen:"
echo "  - Test 1: Cambio exitoso ✅"
echo "  - Test 2: Confirmación no coincide (400) ❌"
echo "  - Test 3: Contraseña incorrecta (401) ❌"
echo "  - Test 4-7: Validaciones de fortaleza (400) ❌"
echo "  - Test 8: Sin autenticación (401) ❌"
echo "  - Test 9: Revertir cambio ✅"
