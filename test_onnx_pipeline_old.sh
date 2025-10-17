#!/bin/bash

# 🚀 Comprehensive ONNX QR Detection Pipeline Test
# Tests multiple QR images to validate hybrid detection system

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

# QR Images to test
declare -a QR_IMAGES=(
    "qrimage.jpg"
    "qrimage2.jpg"
    "qrimage3.jpg"
    "qrimage4.jpg"
    "qrimage5.jpg"
)

# Function to test QR detection with ONNX
test_onnx_qr_detection() {
    local test_name="$1"
    local image_file="$2"
    
    echo -e "${BLUE}🔍 Testing: $test_name${NC}"
    echo "   Image: $image_file"
    echo "   Pipeline: Rust → ONNX → Python Fallback"
    
    if [[ ! -f "$image_file" ]]; then
        echo -e "   ${YELLOW}⚠️  Image not found: $image_file${NC}"
        return 1
    fi
    
    local start_time=$(date +%s%3N)
    
    # Get JWT token
    local jwt_token=$(cat jwt_token.txt 2>/dev/null || echo "")
    
    # Make the request
    local response=$(curl -s -w "HTTPSTATUS:%{http_code};TIME:%{time_total}" \
        -X POST "$BASE_URL$ENDPOINT" \
        -F "file=@$image_file" \
        -H "Content-Type: multipart/form-data" \
        -H "Authorization: Bearer $jwt_token")
    
    local end_time=$(date +%s%3N)
    local total_time=$((end_time - start_time))
    
    # Extract HTTP status and response body
    local http_status=$(echo "$response" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    local time_total=$(echo "$response" | grep -o "TIME:[0-9.]*" | cut -d: -f2)
    local body=$(echo "$response" | sed -E 's/HTTPSTATUS:[0-9]*;TIME:[0-9.]*$//')
    
    echo "   📊 Response Time: ${time_total}s (${total_time}ms total)"
    echo "   🔗 HTTP Status: $http_status"
    
    if [[ "$http_status" == "200" ]]; then
        echo -e "   ${GREEN}✅ Request Successful${NC}"
        
        # Parse JSON response
        local success=$(echo "$body" | jq -r '.data.success // false' 2>/dev/null)
        local qr_data=$(echo "$body" | jq -r '.data.qr_data // empty' 2>/dev/null)
        local detector=$(echo "$body" | jq -r '.data.detector_model // empty' 2>/dev/null)
        local processing_time=$(echo "$body" | jq -r '.data.processing_time_ms // 0' 2>/dev/null)
        local level_used=$(echo "$body" | jq -r '.data.level_used // 0' 2>/dev/null)
        
        if [[ "$success" == "true" && -n "$qr_data" ]]; then
            echo -e "   ${GREEN}✅ QR Detected${NC}"
            echo "   📄 Content: ${qr_data:0:80}..."
            echo "   🤖 Detector: $detector"
            echo "   ⚡ Processing: ${processing_time}ms"
            echo "   🎯 Pipeline Level: $level_used"
            
            # Identify which level was used
            case "$level_used" in
                1) echo "   📊 Level 1: Fast Rust decoders (rqrr/quircs/rxing)" ;;
                2) echo "   🤖 Level 1.5: ONNX ML Detection" ;;
                3) echo "   🔄 Level 2: Rotation correction" ;;
                4) echo "   🐍 Level 3: Python fallback" ;;
                *) echo "   ❓ Unknown level: $level_used" ;;
            esac
            
        else
            echo -e "   ${RED}❌ QR Not Detected${NC}"
            echo "   📊 Processing: ${processing_time}ms"
            echo "   🤖 Final Detector: $detector"
        fi
        
        # Show raw response for debugging
        echo "   🔍 Raw Response:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        
    else
        echo -e "   ${RED}❌ Request Failed${NC}"
        echo "   📄 Response: $body"
    fi
    
    echo ""
}

echo -e "${BLUE}🧪 Enhanced QR Detection Pipeline Test Suite${NC}"
echo "🎯 Testing: Rust → ONNX → Python Fallback Pipeline"
echo ""

# Check if server is running
if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server not running at $BASE_URL${NC}"
    echo "Please start the server first: cargo run"
    exit 1
fi

echo -e "${GREEN}✅ Server is running${NC}"
echo ""

# Test with various images
echo -e "${BLUE}🧪 QR Detection Tests with ONNX Pipeline${NC}"
echo ""

# Test 1: Main test image
test_onnx_qr_detection \
    "QR Test Image 1" \
    "$TEST_IMAGE"

# Test 2: Alternative test image  
test_onnx_qr_detection \
    "QR Test Image 2 (PNG)" \
    "factura_test.png"

# Test 3: Factura prueba
test_onnx_qr_detection \
    "Factura Prueba (JPG)" \
    "factura_prueba.jpg"

# Test 4: Small image stress test
if [[ -f "qrimage4.jpg" ]]; then
    test_onnx_qr_detection \
        "Small Image Stress Test" \
        "qrimage4.jpg"
fi

echo -e "${BLUE}📊 Pipeline Enhancement Summary${NC}"
echo "🎯 Pipeline Order:"
echo "   1. Fast Rust Decoders (Level 1) - rqrr, quircs, rxing"
echo "   2. 🤖 ONNX ML Models (Level 1.5) - Small & Medium YOLOv8"
echo "   3. Rotation Correction (Level 2) - 90°/180°/270°" 
echo "   4. Python QReader Fallback (Level 3) - Advanced PyTorch"
echo ""
echo "🚀 Expected Performance:"
echo "   • Level 1: 80% success, 5-15ms"
echo "   • Level 1.5: +10% success, 100-150ms (ONNX ML)"
echo "   • Level 2: +5% success, 50ms" 
echo "   • Level 3: +3% success, 255ms"
echo "   • Total: 98% success rate"
echo ""
echo -e "${GREEN}✅ Enhanced QR Pipeline Testing Complete!${NC}"