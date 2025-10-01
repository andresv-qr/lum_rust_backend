#!/bin/bash

# Script para hacer scraping directo de la URL y ver qué fecha extrae
URL="https://dgi-fep.mef.gob.pa/Consultas/FacturasPorCUFE?CUFE=FE0120000155606006-2-2015-9700022025060600000143090020110971620622"

echo "🔍 DEBUGGING EXTRACCIÓN DE FECHA"
echo "==============================="
echo "URL: ${URL}"
echo ""

# Descargar HTML y buscar elementos con fecha
echo "📄 Buscando elementos h5 con fechas..."
curl -s "${URL}" | grep -E '<h5.*[0-9]{2}/[0-9]{2}/[0-9]{4}.*</h5>' | head -5

echo ""
echo "📄 Buscando elementos en div.text-right..."
curl -s "${URL}" | grep -A3 -B3 'text-right' | grep -E 'h5|[0-9]{2}/[0-9]{2}/[0-9]{4}'

echo ""
echo "📄 Buscando cualquier elemento con patrón de fecha..."
curl -s "${URL}" | grep -E '[0-9]{2}/[0-9]{2}/[0-9]{4}' | head -10

echo ""
echo "✅ Análisis completado"
