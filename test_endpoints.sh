#!/bin/bash

BASE_URL="http://localhost:8000/api/v4"
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwiZW1haWwiOiJ1c2VyMUBleGFtcGxlLmNvbSIsInNvdXJjZSI6ImFwaSIsInJvbGVzIjpbInVzZXIiXSwiaWF0IjoxNzY3NzkxODMxLCJleHAiOjE3Njc3OTU0MzEsImp0aSI6ImM4MGUyMjQ0LTBhNjUtNDVmMS1hNmM4LWQ1MjZhOTJlYjE5ZiIsInRva2VuX3R5cGUiOiJhY2Nlc3MifQ.Y59EywLU6DjD0FHh4ASDnnh-hOE50pa6g-AzrczuelY"

echo "=========================================="
echo "TEST 1: ASK AI (Generate SQL)"
echo "=========================================="
curl -s -X POST "$BASE_URL/ask-ai" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "question": "Muestrame las ventas totales por mes del 2024, agrupado por producto"
  }' | jq .

echo ""
echo "=========================================="
echo "TEST 2: INTERPRET RESULTS"
echo "=========================================="
# Simulating a result that might come from the SQL generated above
# Let's say the SQL was SELECT STRFTIME('%Y-%m', issued_at) as month, SUM(total_amount) as total FROM invoices WHERE ...
curl -s -X POST "$BASE_URL/interpret-results" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "question": "Muestrame las ventas totales por mes del 2024",
    "chart_type": "bar",
    "data": [
      {"month": "2024-01", "total": 1500.50},
      {"month": "2024-02", "total": 2300.00},
      {"month": "2024-03", "total": 1800.25},
      {"month": "2024-04", "total": 2100.00}
    ]
  }' | jq .
