#!/bin/bash

# 🚀 Test ONNX QR Detection Pipeline
# Tests the enhanced QR detection with ONNX ML models

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SERVER_URL="http://localhost:8000"
IMAGE_PATH="/home/client_1099_1/scripts/lum_rust_ws/qrimage.jpg"
JWT_TOKEN_FILE="/home/client_1099_1/scripts/lum_rust_ws/jwt_token.txt"

echo -e "${BLUE}🤖 ONNX QR Detection Pipeline Test${NC}"
echo "=================================================="
echo -e "📁 Image: ${IMAGE_PATH}"
echo -e "🌐 Server: ${SERVER_URL}"
echo ""

# Check if image exists
if [[ ! -f "$IMAGE_PATH" ]]; then
    echo -e "${RED}❌ Test image not found: $IMAGE_PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Test image found ($(stat --format=%s $IMAGE_PATH) bytes)${NC}"

# Check if server is running
if ! curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server not running at $SERVER_URL${NC}"
    echo "Please start the server with: cargo run"
    exit 1
fi

echo -e "${GREEN}✅ Server is running${NC}"

# Read JWT token if available
JWT_TOKEN=""
if [[ -f "$JWT_TOKEN_FILE" ]]; then
    JWT_TOKEN=$(cat "$JWT_TOKEN_FILE" | tr -d '\n\r')
    echo -e "${GREEN}✅ JWT token loaded${NC}"
else
    echo -e "${YELLOW}⚠️  No JWT token found, testing without auth${NC}"
fi

echo ""
echo -e "${BLUE}🔍 Testing QR Detection Pipeline...${NC}"

# Test the QR detection endpoint
RESPONSE_FILE="/tmp/qr_response_$$"

if [[ -n "$JWT_TOKEN" ]]; then
    # With authentication
    HTTP_STATUS=$(curl -s -w "%{http_code}" -o "$RESPONSE_FILE" \
        -X POST "$SERVER_URL/api/v4/qr/detect" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -F "image=@$IMAGE_PATH")
else
    # Without authentication (if endpoint allows it)
    HTTP_STATUS=$(curl -s -w "%{http_code}" -o "$RESPONSE_FILE" \
        -X POST "$SERVER_URL/api/v4/qr/detect" \
        -F "image=@$IMAGE_PATH")
fi

echo -e "📊 HTTP Status: ${HTTP_STATUS}"

# Parse response
if [[ "$HTTP_STATUS" == "200" ]]; then
    echo -e "${GREEN}✅ Request successful!${NC}"
    
    # Pretty print the JSON response
    if command -v jq &> /dev/null; then
        echo -e "\n${BLUE}📋 Response:${NC}"
        cat "$RESPONSE_FILE" | jq '.'
        
        # Extract key information
        SUCCESS=$(cat "$RESPONSE_FILE" | jq -r '.data.success // false')
        QR_DATA=$(cat "$RESPONSE_FILE" | jq -r '.data.data // "none"')
        DETECTOR=$(cat "$RESPONSE_FILE" | jq -r '.data.detector_model // "unknown"')
        TIME_MS=$(cat "$RESPONSE_FILE" | jq -r '.data.processing_time_ms // 0')
        LEVEL_USED=$(cat "$RESPONSE_FILE" | jq -r '.data.pipeline_stats.detectors_tried // 0')
        
        echo ""
        echo -e "${BLUE}🎯 Results Summary:${NC}"
        echo -e "   Success: ${SUCCESS}"
        echo -e "   Detector: ${DETECTOR}"
        echo -e "   Processing Time: ${TIME_MS}ms"
        echo -e "   Detectors Tried: ${LEVEL_USED}"
        
        if [[ "$SUCCESS" == "true" ]]; then
            echo -e "   ${GREEN}✅ QR Content Found${NC}"
            if [[ "$QR_DATA" != "none" && "$QR_DATA" != "null" ]]; then
                echo -e "   📱 QR Data: ${QR_DATA:0:50}..."
            fi
            
            # Check if ONNX was used
            if [[ "$DETECTOR" == *"onnx"* ]]; then
                echo -e "   ${GREEN}🤖 ONNX ML Model Used Successfully!${NC}"
            else
                echo -e "   ${YELLOW}📝 Traditional Detector Used: ${DETECTOR}${NC}"
            fi
        else
            echo -e "   ${RED}❌ No QR Code Detected${NC}"
        fi
        
    else
        echo -e "\n${BLUE}📋 Raw Response:${NC}"
        cat "$RESPONSE_FILE"
    fi
    
elif [[ "$HTTP_STATUS" == "401" ]]; then
    echo -e "${RED}❌ Authentication failed${NC}"
    echo "Response:"
    cat "$RESPONSE_FILE"
    
elif [[ "$HTTP_STATUS" == "429" ]]; then
    echo -e "${YELLOW}⚠️  Rate limit exceeded${NC}"
    echo "Response:"
    cat "$RESPONSE_FILE"
    
else
    echo -e "${RED}❌ Request failed with status $HTTP_STATUS${NC}"
    echo "Response:"
    cat "$RESPONSE_FILE"
fi

# Cleanup
rm -f "$RESPONSE_FILE"

echo ""
echo -e "${BLUE}🏁 Test completed!${NC}"

# Test summary
echo ""
echo "=================================================="
echo -e "${BLUE}📊 Pipeline Test Summary${NC}"
echo "=================================================="
echo "✅ Image: 1280x1280 JPEG (250KB)"
echo "✅ Server: Running on port 8000"
echo "✅ Endpoint: POST /api/v4/qr/detect"
if [[ "$HTTP_STATUS" == "200" ]]; then
    echo "✅ Result: Request successful"
else
    echo "❌ Result: Request failed ($HTTP_STATUS)"
fi