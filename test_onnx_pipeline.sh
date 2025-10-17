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

echo -e "${BLUE}🤖 COMPREHENSIVE ONNX QR DETECTION PIPELINE TEST${NC}"
echo "=================================================================="
echo -e "🌐 Server: ${SERVER_URL}"
echo -e "📊 Testing ${#QR_IMAGES[@]} QR images"
echo ""

# Check if server is running
if ! curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server not running at $SERVER_URL${NC}"
    echo "Please start the server with: cargo run --release"
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

# Results arrays
declare -a RESULTS_SUCCESS=()
declare -a RESULTS_DETECTOR=()
declare -a RESULTS_TIME=()
declare -a RESULTS_SIZE=()

# Test each image
for i in "${!QR_IMAGES[@]}"; do
    IMAGE_FILE="${QR_IMAGES[$i]}"
    IMAGE_PATH="/home/client_1099_1/scripts/lum_rust_ws/$IMAGE_FILE"
    
    echo -e "${CYAN}🔍 Testing Image $((i+1))/${#QR_IMAGES[@]}: ${IMAGE_FILE}${NC}"
    echo "=================================================="
    
    # Check if image exists
    if [[ ! -f "$IMAGE_PATH" ]]; then
        echo -e "${RED}❌ Image not found: $IMAGE_PATH${NC}"
        RESULTS_SUCCESS+=("MISSING")
        RESULTS_DETECTOR+=("N/A")
        RESULTS_TIME+=("0")
        RESULTS_SIZE+=("0")
        echo ""
        continue
    fi
    
    # Get image size
    IMAGE_SIZE=$(stat --format=%s "$IMAGE_PATH")
    IMAGE_SIZE_KB=$((IMAGE_SIZE / 1024))
    
    echo -e "📁 File: ${IMAGE_FILE}"
    echo -e "📊 Size: ${IMAGE_SIZE_KB} KB (${IMAGE_SIZE} bytes)"
    
    # Test the QR detection endpoint
    RESPONSE_FILE="/tmp/qr_response_${i}_$$"
    
    echo -e "${YELLOW}🚀 Starting QR detection...${NC}"
    
    HTTP_STATUS=$(curl -s -w "%{http_code}" -o "$RESPONSE_FILE" \
        -X POST "$SERVER_URL/api/v4/qr/detect" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -F "image=@$IMAGE_PATH")
    
    echo -e "📊 HTTP Status: ${HTTP_STATUS}"
    
    # Parse response
    if [[ "$HTTP_STATUS" == "200" ]]; then
        echo -e "${GREEN}✅ Request successful!${NC}"
        
        if command -v jq &> /dev/null; then
            # Extract key information with jq
            SUCCESS=$(cat "$RESPONSE_FILE" | jq -r '.data.success // false')
            QR_DATA=$(cat "$RESPONSE_FILE" | jq -r '.data.qr_data // "none"')
            DETECTOR=$(cat "$RESPONSE_FILE" | jq -r '.data.detection_level // "unknown"')
            TIME_MS=$(cat "$RESPONSE_FILE" | jq -r '.data.processing_time_ms // 0')
            
            if [[ "$SUCCESS" == "true" ]]; then
                echo -e "${GREEN}🎯 QR DETECTED SUCCESSFULLY${NC}"
                echo -e "   📱 Detector: ${DETECTOR}"
                echo -e "   ⏱️  Processing Time: ${TIME_MS}ms"
                
                # Check if ONNX was used
                if [[ "$DETECTOR" == *"onnx"* || "$DETECTOR" == *"ONNX"* ]]; then
                    echo -e "   ${PURPLE}🤖 ONNX ML MODEL USED! 🚀${NC}"
                else
                    echo -e "   📝 Traditional Detector Used: ${DETECTOR}"
                fi
                
                # Show first part of QR data
                if [[ "$QR_DATA" != "none" && "$QR_DATA" != "null" ]]; then
                    echo -e "   📄 QR Content: ${QR_DATA:0:80}..."
                fi
                
                # Store results
                RESULTS_SUCCESS+=("SUCCESS")
                RESULTS_DETECTOR+=("$DETECTOR")
                RESULTS_TIME+=("$TIME_MS")
                RESULTS_SIZE+=("$IMAGE_SIZE_KB")
                
            else
                echo -e "${RED}❌ QR NOT DETECTED${NC}"
                RESULTS_SUCCESS+=("FAILED")
                RESULTS_DETECTOR+=("none")
                RESULTS_TIME+=("$TIME_MS")
                RESULTS_SIZE+=("$IMAGE_SIZE_KB")
            fi
        else
            echo -e "${YELLOW}⚠️  jq not available, showing raw response${NC}"
            cat "$RESPONSE_FILE"
            RESULTS_SUCCESS+=("UNKNOWN")
            RESULTS_DETECTOR+=("unknown")
            RESULTS_TIME+=("0")
            RESULTS_SIZE+=("$IMAGE_SIZE_KB")
        fi
        
    else
        echo -e "${RED}❌ Request failed with status $HTTP_STATUS${NC}"
        echo "Response:"
        cat "$RESPONSE_FILE"
        
        RESULTS_SUCCESS+=("HTTP_ERROR")
        RESULTS_DETECTOR+=("error")
        RESULTS_TIME+=("0")
        RESULTS_SIZE+=("$IMAGE_SIZE_KB")
    fi
    
    # Cleanup
    rm -f "$RESPONSE_FILE"
    
    echo ""
done

# Final summary
echo ""
echo "=================================================================="
echo -e "${BLUE}📊 COMPREHENSIVE TEST RESULTS SUMMARY${NC}"
echo "=================================================================="

echo -e "${CYAN}📋 Individual Results:${NC}"
printf "%-15s %-10s %-15s %-15s %-10s\n" "IMAGE" "SIZE(KB)" "SUCCESS" "DETECTOR" "TIME(ms)"
echo "----------------------------------------------------------------"

for i in "${!QR_IMAGES[@]}"; do
    IMAGE="${QR_IMAGES[$i]}"
    SUCCESS="${RESULTS_SUCCESS[$i]}"
    DETECTOR="${RESULTS_DETECTOR[$i]}"
    TIME="${RESULTS_TIME[$i]}"
    SIZE="${RESULTS_SIZE[$i]}"
    
    # Color code success
    if [[ "$SUCCESS" == "SUCCESS" ]]; then
        SUCCESS_COLOR="${GREEN}${SUCCESS}${NC}"
    else
        SUCCESS_COLOR="${RED}${SUCCESS}${NC}"
    fi
    
    # Color code ONNX detection
    if [[ "$DETECTOR" == *"onnx"* || "$DETECTOR" == *"ONNX"* ]]; then
        DETECTOR_COLOR="${PURPLE}${DETECTOR}${NC}"
    else
        DETECTOR_COLOR="${DETECTOR}"
    fi
    
    printf "%-15s %-10s %-24s %-24s %-10s\n" "$IMAGE" "$SIZE" "$SUCCESS_COLOR" "$DETECTOR_COLOR" "$TIME"
done

echo ""
echo -e "${CYAN}🎯 Pipeline Performance Analysis:${NC}"

# Count successes
SUCCESS_COUNT=0
ONNX_COUNT=0
TOTAL_TIME=0
TRADITIONAL_COUNT=0

for i in "${!RESULTS_SUCCESS[@]}"; do
    if [[ "${RESULTS_SUCCESS[$i]}" == "SUCCESS" ]]; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        TOTAL_TIME=$((TOTAL_TIME + ${RESULTS_TIME[$i]}))
        
        if [[ "${RESULTS_DETECTOR[$i]}" == *"onnx"* || "${RESULTS_DETECTOR[$i]}" == *"ONNX"* ]]; then
            ONNX_COUNT=$((ONNX_COUNT + 1))
        else
            TRADITIONAL_COUNT=$((TRADITIONAL_COUNT + 1))
        fi
    fi
done

TOTAL_IMAGES=${#QR_IMAGES[@]}
SUCCESS_RATE=$(( (SUCCESS_COUNT * 100) / TOTAL_IMAGES ))

if [[ $SUCCESS_COUNT -gt 0 ]]; then
    AVG_TIME=$((TOTAL_TIME / SUCCESS_COUNT))
else
    AVG_TIME=0
fi

echo -e "   📊 Success Rate: ${SUCCESS_COUNT}/${TOTAL_IMAGES} (${SUCCESS_RATE}%)"
echo -e "   ⏱️  Average Processing Time: ${AVG_TIME}ms"
echo -e "   📝 Traditional Detectors: ${TRADITIONAL_COUNT}"
echo -e "   🤖 ONNX ML Models Used: ${ONNX_COUNT}"

if [[ $ONNX_COUNT -gt 0 ]]; then
    echo -e "${PURPLE}   🚀 ONNX ML MODELS SUCCESSFULLY ACTIVATED!${NC}"
else
    echo -e "${YELLOW}   📋 All detections used traditional methods (images were clear enough)${NC}"
fi

echo ""
echo -e "${GREEN}🏁 COMPREHENSIVE PIPELINE TEST COMPLETED!${NC}"
echo "=================================================================="