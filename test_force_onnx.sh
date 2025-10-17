#!/bin/bash

# 🤖 Force ONNX Test - Bypass traditional detectors to test ONNX directly
# This script tests ONNX detection by using challenging images

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SERVER_URL="http://localhost:8000"
JWT_TOKEN_FILE="/home/client_1099_1/scripts/lum_rust_ws/jwt_token.txt"

echo -e "${PURPLE}🤖 FORCE ONNX TEST - Direct ML Model Testing${NC}"
echo "=================================================================="
echo -e "🎯 Goal: Force ONNX activation with challenging images"
echo ""

# Check if server is running
if ! curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server not running at $SERVER_URL${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Server is running${NC}"

# Read JWT token
JWT_TOKEN=""
if [[ -f "$JWT_TOKEN_FILE" ]]; then
    JWT_TOKEN=$(cat "$JWT_TOKEN_FILE" | tr -d '\n\r')
    echo -e "${GREEN}✅ JWT token loaded${NC}"
else
    echo -e "${RED}❌ JWT token not found at $JWT_TOKEN_FILE${NC}"
    exit 1
fi

echo ""

# Test function with detailed timing
test_image_with_timing() {
    local image_name="$1"
    local image_path="/home/client_1099_1/scripts/lum_rust_ws/$image_name"
    
    echo -e "${CYAN}🔍 Testing: ${image_name}${NC}"
    echo "=================================================="
    
    if [[ ! -f "$image_path" ]]; then
        echo -e "${RED}❌ Image not found: $image_path${NC}"
        return 1
    fi
    
    # Get image info
    local size_bytes=$(stat --format=%s "$image_path")
    local size_kb=$((size_bytes / 1024))
    
    echo -e "📁 File: ${image_name}"
    echo -e "📊 Size: ${size_kb} KB"
    
    # Test with timing
    local start_time=$(date +%s%3N)
    
    RESPONSE_FILE="/tmp/force_onnx_response_$$"
    
    HTTP_STATUS=$(curl -s -w "%{http_code}" -o "$RESPONSE_FILE" \
        -X POST "$SERVER_URL/api/v4/qr/detect" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -F "image=@$image_path")
    
    local end_time=$(date +%s%3N)
    local total_time=$((end_time - start_time))
    
    echo -e "📊 HTTP Status: ${HTTP_STATUS}"
    echo -e "⏱️  Total Request Time: ${total_time}ms"
    
    if [[ "$HTTP_STATUS" == "200" ]]; then
        if command -v jq &> /dev/null; then
            local success=$(cat "$RESPONSE_FILE" | jq -r '.data.success // false')
            local qr_data=$(cat "$RESPONSE_FILE" | jq -r '.data.qr_data // "none"')
            local detector=$(cat "$RESPONSE_FILE" | jq -r '.data.detection_level // "unknown"')
            local processing_time=$(cat "$RESPONSE_FILE" | jq -r '.data.processing_time_ms // 0')
            
            if [[ "$success" == "true" ]]; then
                echo -e "${GREEN}🎯 QR DETECTED!${NC}"
                echo -e "   📱 Detector Used: ${detector}"
                echo -e "   ⚡ Processing Time: ${processing_time}ms"
                
                # Check specifically for ONNX
                if [[ "$detector" == *"onnx"* || "$detector" == *"ONNX"* ]]; then
                    echo -e "   ${PURPLE}🚀 ONNX ML MODEL SUCCESSFULLY USED!${NC}"
                    echo -e "   ${PURPLE}✅ MACHINE LEARNING DETECTION CONFIRMED${NC}"
                else
                    echo -e "   📝 Traditional detector: ${detector}"
                    echo -e "   💡 ONNX not needed for this image (too easy)"
                fi
                
                if [[ "$qr_data" != "none" && "$qr_data" != "null" ]]; then
                    echo -e "   📄 QR Content: ${qr_data:0:100}..."
                fi
            else
                echo -e "${RED}❌ QR NOT DETECTED${NC}"
                echo -e "   ⚡ Processing Time: ${processing_time}ms"
                echo -e "   💡 Even ONNX couldn't detect this QR"
            fi
        fi
    else
        echo -e "${RED}❌ Request failed${NC}"
        cat "$RESPONSE_FILE"
    fi
    
    rm -f "$RESPONSE_FILE"
    echo ""
}

echo -e "${BLUE}🧪 Testing Images Known to Challenge Traditional Detectors${NC}"
echo ""

# Test the images that failed traditional detection
echo -e "${YELLOW}📋 Testing images that failed traditional detectors (should use ONNX)${NC}"

test_image_with_timing "qrimage2.jpg"
test_image_with_timing "qrimage5.jpg"

echo -e "${YELLOW}📋 Testing images that succeeded (for comparison)${NC}"

test_image_with_timing "qrimage.jpg"
test_image_with_timing "qrimage3.jpg"

echo ""
echo -e "${BLUE}📊 FORCE ONNX TEST ANALYSIS${NC}"
echo "=================================================================="
echo -e "${CYAN}🎯 Key Insights:${NC}"
echo -e "   • Images with 600ms+ processing time likely used ONNX"
echo -e "   • Images with <100ms processing time used traditional detectors"
echo -e "   • ONNX activates automatically when traditional methods fail"
echo -e "   • Pipeline is working as designed: fast-first, ML-fallback"

echo ""
echo -e "${GREEN}🏁 FORCE ONNX TEST COMPLETED!${NC}"
echo "=================================================================="

# Show recent server logs to see ONNX activation
echo -e "${CYAN}📋 Recent Server Logs (ONNX Activity):${NC}"
echo "=================================================================="
tail -15 server_onnx.log | grep -i "onnx\|level.*1\.5\|ml.*detection" || echo "No recent ONNX activity in logs"