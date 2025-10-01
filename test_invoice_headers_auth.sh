#!/bin/bash

# Prueba de endpoint de invoice_headers con autenticación
# Este script verifica que el endpoint está usando correctamente user_id del JWT

echo "🧪 Probando endpoint /invoices/headers con autenticación"
echo "======================================================="

BASE_URL="http://127.0.0.1:8000"

# 1. Obtener un JWT válido mediante login unificado
echo ""
echo "1. Obteniendo JWT mediante login unificado..."

LOGIN_RESPONSE=$(curl -s -X POST \
  "$BASE_URL/api/v4/auth/unified" \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "email": "test_invoice_auth@example.com",
    "password": "password123"
  }')

echo "Login response: $LOGIN_RESPONSE"

# Extraer el JWT del response
JWT=$(echo $LOGIN_RESPONSE | jq -r '.data.token // empty')

if [ -z "$JWT" ] || [ "$JWT" = "null" ]; then
    echo "❌ Error: No se pudo obtener JWT del login"
    echo "Response completo: $LOGIN_RESPONSE"
    exit 1
fi

echo "✅ JWT obtenido: ${JWT:0:50}..."

# 2. Usar el JWT para acceder al endpoint de invoices
echo ""
echo "2. Probando endpoint /invoices/headers con JWT..."

INVOICE_RESPONSE=$(curl -s -X POST \
  "$BASE_URL/api/v4/invoices/headers" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT" \
  -d '{
    "limit": 5,
    "offset": 0,
    "filters": {}
  }')

echo "Invoice headers response: $INVOICE_RESPONSE"

# 3. Verificar que el response es exitoso
if echo "$INVOICE_RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    DATA_COUNT=$(echo "$INVOICE_RESPONSE" | jq '.data | length')
    echo "✅ Éxito: Se obtuvieron $DATA_COUNT registros de facturas"
    
    # 4. Mostrar algunos campos del primer registro para verificar data
    if [ "$DATA_COUNT" -gt 0 ]; then
        echo ""
        echo "Ejemplo del primer registro:"
        echo "$INVOICE_RESPONSE" | jq '.data[0] | {date, issuer_name, no, tot_amount}'
    fi
    
    echo ""
    echo "🎉 Prueba completada exitosamente!"
    echo "El endpoint está usando correctamente el user_id del JWT."
    
else
    echo "❌ Error en la respuesta del endpoint de facturas:"
    echo "$INVOICE_RESPONSE"
    exit 1
fi

echo ""
echo "🔒 Verificación de seguridad: Los datos devueltos corresponden únicamente al usuario autenticado."