#!/bin/bash
# Test QR detection with multiple images

echo "=============================================="
echo "🔍 QR DETECTION TEST SUITE"
echo "=============================================="
echo ""

JWT=$(cat jwt_token_fresh.txt | grep "eyJ" | head -1)

if [ -z "$JWT" ]; then
    echo "❌ Error: No JWT token found"
    exit 1
fi

IMAGES=("qrimage.jpg" "qrimage2.jpg" "qrimage3.jpg" "qrimage4.jpg" "qrimage5.jpg")
SUCCESS_COUNT=0
FAIL_COUNT=0
TOTAL_TIME=0

for img in "${IMAGES[@]}"; do
    if [ ! -f "$img" ]; then
        echo "⚠️  Image not found: $img"
        continue
    fi
    
    FILE_SIZE=$(ls -lh "$img" | awk '{print $5}')
    DIMENSIONS=$(identify "$img" 2>/dev/null | awk '{print $3}')
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📷 Testing: $img"
    echo "   Size: $FILE_SIZE | Dimensions: $DIMENSIONS"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    RESPONSE=$(curl -s -X POST "http://localhost:8000/api/v4/qr/detect" \
        -H "Authorization: Bearer $JWT" \
        -F "image=@$img" 2>&1)
    
    SUCCESS=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print('true' if data.get('success') and data.get('data', {}).get('success') else 'false')" 2>/dev/null)
    
    if [ "$SUCCESS" = "true" ]; then
        QR_DATA=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['data']['qr_data'][:100])" 2>/dev/null)
        DETECTOR=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['data']['detection_level'])" 2>/dev/null)
        TIME_MS=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['data']['processing_time_ms'])" 2>/dev/null)
        
        echo "✅ SUCCESS"
        echo "   🔗 QR Data: ${QR_DATA}..."
        echo "   🔧 Detector: $DETECTOR"
        echo "   ⏱️  Time: ${TIME_MS}ms"
        
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        TOTAL_TIME=$((TOTAL_TIME + TIME_MS))
    else
        ERROR_MSG=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data.get('error', 'Unknown error'))" 2>/dev/null)
        echo "❌ FAILED"
        echo "   Error: $ERROR_MSG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    
    echo ""
done

echo "=============================================="
echo "📊 RESULTS SUMMARY"
echo "=============================================="
echo "✅ Successful: $SUCCESS_COUNT / ${#IMAGES[@]}"
echo "❌ Failed: $FAIL_COUNT / ${#IMAGES[@]}"

if [ $SUCCESS_COUNT -gt 0 ]; then
    AVG_TIME=$((TOTAL_TIME / SUCCESS_COUNT))
    echo "⏱️  Average time: ${AVG_TIME}ms"
fi

SUCCESS_RATE=$((SUCCESS_COUNT * 100 / ${#IMAGES[@]}))
echo "📈 Success rate: ${SUCCESS_RATE}%"
echo "=============================================="

if [ $SUCCESS_COUNT -eq ${#IMAGES[@]} ]; then
    echo "🎉 ALL TESTS PASSED!"
else
    echo "⚠️  Some tests failed"
fi
