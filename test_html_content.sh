#!/usr/bin/env bash

echo "🧪 Testing Local HTML Extraction"
echo "================================="

# Verificar que los archivos HTML existen
if [ ! -f "webscrapy_htmlsample1.html" ]; then
    echo "❌ File webscrapy_htmlsample1.html not found"
    exit 1
fi

if [ ! -f "webscrapy_htmlsample2.html" ]; then
    echo "❌ File webscrapy_htmlsample2.html not found"
    exit 1
fi

echo "✅ HTML files found"
echo ""

# Test 1: Verificar contenido de webscrapy_htmlsample1.html
echo "🔍 Test 1: webscrapy_htmlsample1.html"
echo "-------------------------------------"

# Buscar número de factura
NUMERO_FOUND=$(grep -o "No\. [0-9]*" webscrapy_htmlsample1.html | head -1 | sed 's/No\. //')
echo "Número encontrado: '$NUMERO_FOUND'"
if [ "$NUMERO_FOUND" = "0031157014" ]; then
    echo "✅ Número de factura correcto: $NUMERO_FOUND"
else
    echo "❌ Número de factura incorrecto. Esperado: 0031157014, Encontrado: $NUMERO_FOUND"
fi

# Buscar fecha
FECHA_FOUND=$(grep -o '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]' webscrapy_htmlsample1.html | head -1)
echo "Fecha encontrada: '$FECHA_FOUND'"
if [ "$FECHA_FOUND" = "15/05/2025 09:50:04" ]; then
    echo "✅ Fecha correcta: $FECHA_FOUND"
else
    echo "❌ Fecha incorrecta. Esperado: 15/05/2025 09:50:04, Encontrado: $FECHA_FOUND"
fi

echo ""

# Test 2: Verificar contenido de webscrapy_htmlsample2.html
echo "🔍 Test 2: webscrapy_htmlsample2.html"
echo "-------------------------------------"

# Buscar número de factura
NUMERO_FOUND2=$(grep -o "No\. [0-9]*" webscrapy_htmlsample2.html | head -1 | sed 's/No\. //')
echo "Número encontrado: '$NUMERO_FOUND2'"
if [ "$NUMERO_FOUND2" = "0031157014" ]; then
    echo "✅ Número de factura correcto: $NUMERO_FOUND2"
else
    echo "❌ Número de factura incorrecto. Esperado: 0031157014, Encontrado: $NUMERO_FOUND2"
fi

# Buscar fecha
FECHA_FOUND2=$(grep -o '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]' webscrapy_htmlsample2.html | head -1)
echo "Fecha encontrada: '$FECHA_FOUND2'"
if [ "$FECHA_FOUND2" = "15/05/2025 09:50:04" ]; then
    echo "✅ Fecha correcta: $FECHA_FOUND2"
else
    echo "❌ Fecha incorrecta. Esperado: 15/05/2025 09:50:04, Encontrado: $FECHA_FOUND2"
fi

echo ""

# Test 3: Verificar estructura esperada
echo "🔍 Test 3: Verificar estructura HTML"
echo "------------------------------------"

# Verificar que existe el panel-heading con FACTURA
if grep -q "panel-heading" webscrapy_htmlsample1.html && grep -q "FACTURA" webscrapy_htmlsample1.html; then
    echo "✅ Estructura panel-heading con FACTURA encontrada"
else
    echo "❌ Estructura panel-heading con FACTURA no encontrada"
fi

# Verificar las clases CSS esperadas
if grep -q "col-sm-4 text-left" webscrapy_htmlsample1.html; then
    echo "✅ Clase CSS 'col-sm-4 text-left' encontrada"
else
    echo "❌ Clase CSS 'col-sm-4 text-left' no encontrada"
fi

if grep -q "col-sm-4 text-center" webscrapy_htmlsample1.html; then
    echo "✅ Clase CSS 'col-sm-4 text-center' encontrada"
else
    echo "❌ Clase CSS 'col-sm-4 text-center' no encontrada"
fi

if grep -q "col-sm-4 text-right" webscrapy_htmlsample1.html; then
    echo "✅ Clase CSS 'col-sm-4 text-right' encontrada"
else
    echo "❌ Clase CSS 'col-sm-4 text-right' no encontrada"
fi

echo ""
echo "🏁 Test completed!"
echo ""

# Resumen
echo "📋 RESUMEN:"
echo "==========="
if [ "$NUMERO_FOUND" = "0031157014" ] && [ "$NUMERO_FOUND2" = "0031157014" ]; then
    echo "✅ Números de factura: CORRECTOS"
else
    echo "❌ Números de factura: INCORRECTOS"
fi

if [ "$FECHA_FOUND" = "15/05/2025 09:50:04" ] && [ "$FECHA_FOUND2" = "15/05/2025 09:50:04" ]; then
    echo "✅ Fechas: CORRECTAS"
else
    echo "❌ Fechas: INCORRECTAS"
fi
