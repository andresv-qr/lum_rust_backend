#!/bin/bash

# 🧪 Test Script for QR Detection API - Phase 1 & 2 Optimizations
# Tests the POST /api/v4/qr/detect endpoint with real QR images

set -e

BASE_URL="http://localhost:3000"
ENDPOINT="/api/v4/qr/detect"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=================================${NC}"
echo -e "${BLUE}🧪 QR Detection API Test Suite${NC}"
echo -e "${BLUE}   Phase 1 & 2 Optimizations${NC}"
echo -e "${BLUE}=================================${NC}\n"

# Function to test QR detection
test_qr_detection() {
    local test_name="$1"
    local image_file="$2"
    local expected_level="$3"
    local expected_decoder="$4"
    
    echo -e "\n${YELLOW}📋 Test: $test_name${NC}"
    echo -e "   Image: $image_file"
    echo -e "   Expected Level: $expected_level"
    echo -e "   Expected Decoder: $expected_decoder"
    
    if [ ! -f "$image_file" ]; then
        echo -e "   ${RED}❌ SKIP: Image file not found${NC}"
        return
    fi
    
    # Make request
    response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL$ENDPOINT" \
        -H "x-request-id: test-$(date +%s)" \
        -F "image=@$image_file")
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" -eq 200 ]; then
        echo -e "   ${GREEN}✅ HTTP 200 OK${NC}"
        
        # Parse JSON response
        success=$(echo "$body" | jq -r '.data.success')
        qr_data=$(echo "$body" | jq -r '.data.qr_data // "null"')
        detection_level=$(echo "$body" | jq -r '.data.detection_level')
        processing_time=$(echo "$body" | jq -r '.data.processing_time_ms')
        
        if [ "$success" = "true" ]; then
            echo -e "   ${GREEN}✅ QR Detected${NC}"
            echo -e "      QR Content: ${qr_data:0:60}..."
            echo -e "      Decoder: $detection_level"
            echo -e "      Processing Time: ${processing_time}ms"
            
            # Validate expectations
            if [ -n "$expected_decoder" ] && [ "$detection_level" = "$expected_decoder" ]; then
                echo -e "      ${GREEN}✅ Expected decoder matched${NC}"
            elif [ -n "$expected_decoder" ]; then
                echo -e "      ${YELLOW}⚠️  Expected $expected_decoder, got $detection_level${NC}"
            fi
            
        else
            echo -e "   ${RED}❌ QR Not Detected${NC}"
            message=$(echo "$body" | jq -r '.data.message')
            echo -e "      Message: $message"
        fi
        
    else
        echo -e "   ${RED}❌ HTTP $http_code${NC}"
        echo -e "      Response: $body"
    fi
}

# Function to test health endpoint
test_health() {
    echo -e "\n${YELLOW}📋 Test: Health Check${NC}"
    
    response=$(curl -s -w "\n%{http_code}" -X GET "$BASE_URL/api/v4/qr/health" \
        -H "x-request-id: health-test-$(date +%s)")
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" -eq 200 ]; then
        echo -e "   ${GREEN}✅ HTTP 200 OK${NC}"
        
        status=$(echo "$body" | jq -r '.data.status')
        total_requests=$(echo "$body" | jq -r '.data.total_requests')
        success_rate=$(echo "$body" | jq -r '.data.success_rate')
        
        echo -e "      Status: $status"
        echo -e "      Total Requests: $total_requests"
        echo -e "      Success Rate: ${success_rate}%"
        
        echo -e "\n      Decoders:"
        echo "$body" | jq -r '.data.decoders[] | "        - \(.name): \(.status) (success: \(.success_count), errors: \(.error_count))"'
        
    else
        echo -e "   ${RED}❌ HTTP $http_code${NC}"
    fi
}

# Run tests
echo -e "\n${BLUE}🏥 Health Check${NC}"
test_health

echo -e "\n${BLUE}🧪 QR Detection Tests${NC}"

# Test 1: Perfect QR code (should use Level 1 - rqrr/quircs)
test_qr_detection \
    "Perfect QR Code" \
    "factura_prueba.jpg" \
    "1" \
    ""

# Test 2: Another QR image
test_qr_detection \
    "QR Code PNG" \
    "factura_prueba.png" \
    "1" \
    ""

# Test 3: Third QR image
test_qr_detection \
    "QR Code Test" \
    "factura_test.jpg" \
    "1" \
    ""

# Test 4: Invalid file (should fail)
echo -e "\n${YELLOW}📋 Test: Invalid Image${NC}"
test_qr_detection \
    "Invalid File" \
    "archivo_invalido.txt" \
    "" \
    ""

echo -e "\n${BLUE}=================================${NC}"
echo -e "${BLUE}📊 Test Suite Complete${NC}"
echo -e "${BLUE}=================================${NC}\n"

echo -e "${YELLOW}💡 Tips:${NC}"
echo -e "   1. Check logs for detailed processing info (preprocessing, rotation, etc.)"
echo -e "   2. Most cases should use Level 1 (preprocessed decoders) in 5-15ms"
echo -e "   3. Level 2 (rotation) should be rare (~5% of cases)"
echo -e "   4. Level 3 (Python fallback) should be very rare (~3% of cases)"
echo -e ""
echo -e "${YELLOW}📈 Monitor:${NC}"
echo -e "   - Processing time distribution"
echo -e "   - Level usage (1 vs 2 vs 3)"
echo -e "   - Decoder success rate (rqrr, quircs, rxing, python_opencv)"
echo -e ""
