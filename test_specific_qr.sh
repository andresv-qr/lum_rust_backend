#!/bin/bash

# 🎯 Specific ONNX Test - Testing successful QR images
# Tests qrimage.jpg, qrimage3.jpg, and qrimage4.jpg to analyze pipeline behavior

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

# Specific images to test
declare -a TEST_IMAGES=(
    "qrimage.jpg"
    "qrimage3.jpg"
    "qrimage4.jpg"
)

echo -e "${BLUE}🎯 SPECIFIC QR IMAGES TEST - Pipeline Analysis${NC}"
echo "=================================================================="
echo -e "🌐 Server: ${SERVER_URL}"
echo -e "📊 Testing 3 successful QR images"
echo -e "🎯 Goal: Analyze pipeline behavior with working QR codes"
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

# Test function with detailed analysis
test_qr_image_detailed() {
    local image_name="$1"
    local test_number="$2"
    local image_path="/home/client_1099_1/scripts/lum_rust_ws/$image_name"
    
    echo -e "${CYAN}🔍 Test ${test_number}/3: ${image_name}${NC}"
    echo "=================================================="
    
    if [[ ! -f "$image_path" ]]; then
        echo -e "${RED}❌ Image not found: $image_path${NC}"
        return 1
    fi
    
    # Get image info
    local size_bytes=$(stat --format=%s "$image_path")
    local size_kb=$((size_bytes / 1024))
    
    # Get image dimensions using identify if available
    local dimensions="unknown"
    if command -v identify &> /dev/null; then
        dimensions=$(identify -format "%wx%h" "$image_path" 2>/dev/null || echo "unknown")
    fi
    
    echo -e "📁 File: ${image_name}"
    echo -e "📊 Size: ${size_kb} KB (${size_bytes} bytes)"
    echo -e "📐 Dimensions: ${dimensions}"
    
    # Clear server logs marker
    echo -e "${YELLOW}🚀 Starting QR detection...${NC}"
    local start_timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Test with timing
    local start_time=$(date +%s%3N)
    
    RESPONSE_FILE="/tmp/specific_qr_test_${test_number}_$$"
    
    HTTP_STATUS=$(curl -s -w "%{http_code}" -o "$RESPONSE_FILE" \
        -X POST "$SERVER_URL/api/v4/qr/detect" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -F "image=@$image_path")
    
    local end_time=$(date +%s%3N)
    local total_request_time=$((end_time - start_time))
    
    echo -e "📊 HTTP Status: ${HTTP_STATUS}"
    echo -e "⏱️  Total Request Time: ${total_request_time}ms"
    
    if [[ "$HTTP_STATUS" == "200" ]]; then
        echo -e "${GREEN}✅ Request successful!${NC}"
        
        if command -v jq &> /dev/null; then
            local success=$(cat "$RESPONSE_FILE" | jq -r '.data.success // false')
            local qr_data=$(cat "$RESPONSE_FILE" | jq -r '.data.qr_data // "none"')
            local detector=$(cat "$RESPONSE_FILE" | jq -r '.data.detection_level // "unknown"')
            local processing_time=$(cat "$RESPONSE_FILE" | jq -r '.data.processing_time_ms // 0')
            local request_id=$(cat "$RESPONSE_FILE" | jq -r '.request_id // "unknown"')
            
            if [[ "$success" == "true" ]]; then
                echo -e "${GREEN}🎯 QR DETECTED SUCCESSFULLY!${NC}"
                echo -e "   📱 Detector Used: ${detector}"
                echo -e "   ⚡ Processing Time: ${processing_time}ms"
                echo -e "   🆔 Request ID: ${request_id}"
                
                # Analyze detector type
                case "$detector" in
                    *"onnx"*|*"ONNX"*)
                        echo -e "   ${PURPLE}🤖 ONNX ML MODEL USED! 🚀${NC}"
                        echo -e "   ${PURPLE}✨ Machine Learning Detection Success${NC}"
                        ;;
                    "rxing")
                        echo -e "   📝 Traditional Detector: rxing (Rust QR library)"
                        echo -e "   ⚡ Fast native detection (Level 1)"
                        ;;
                    "rqrr")
                        echo -e "   📝 Traditional Detector: rqrr (Rust QR library)" 
                        echo -e "   ⚡ Fast native detection (Level 1)"
                        ;;
                    "quircs")
                        echo -e "   📝 Traditional Detector: quircs (C library)"
                        echo -e "   ⚡ Fast native detection (Level 1)"
                        ;;
                    *)
                        echo -e "   📝 Detector: ${detector}"
                        ;;
                esac
                
                # Performance analysis
                if [[ $processing_time -lt 100 ]]; then
                    echo -e "   🚀 Performance: Excellent (<100ms)"
                elif [[ $processing_time -lt 300 ]]; then
                    echo -e "   ⚡ Performance: Good (100-300ms)"
                else
                    echo -e "   🔄 Performance: Complex (>300ms - likely used multiple levels)"
                fi
                
                if [[ "$qr_data" != "none" && "$qr_data" != "null" ]]; then
                    echo -e "   📄 QR Content Preview: ${qr_data:0:80}..."
                    
                    # Analyze QR content type
                    if [[ "$qr_data" == *"dgi-fep.mef.gob.pa"* ]]; then
                        echo -e "   🏛️  Type: Panamanian Government Invoice (DGI)"
                    elif [[ "$qr_data" == *"panamapac.com.pa"* ]]; then
                        echo -e "   🏢 Type: Corporate Panama Invoice"
                    elif [[ "$qr_data" == *"http"* ]]; then
                        echo -e "   🌐 Type: URL/Web Link"
                    else
                        echo -e "   📄 Type: Text/Data"
                    fi
                fi
                
            else
                echo -e "${RED}❌ QR NOT DETECTED${NC}"
                echo -e "   ⚡ Processing Time: ${processing_time}ms"
                echo -e "   🆔 Request ID: ${request_id}"
            fi
            
            # Show processing breakdown
            echo -e "   📊 Processing Breakdown:"
            echo -e "      • Server Processing: ${processing_time}ms"
            echo -e "      • Network + Overhead: $((total_request_time - processing_time))ms"
            echo -e "      • Total Request: ${total_request_time}ms"
            
        else
            echo -e "${YELLOW}⚠️  jq not available - showing raw response${NC}"
            cat "$RESPONSE_FILE"
        fi
        
    else
        echo -e "${RED}❌ Request failed with status $HTTP_STATUS${NC}"
        echo "Response:"
        cat "$RESPONSE_FILE"
    fi
    
    # Show relevant server logs for this request
    echo -e "   📋 Server Logs (last 5 lines):"
    tail -5 server_onnx.log | grep -E "rxing|rqrr|quircs|ONNX|LEVEL" | tail -2 || echo "      No detector logs found"
    
    rm -f "$RESPONSE_FILE"
    echo ""
}

# Test each image
for i in "${!TEST_IMAGES[@]}"; do
    test_qr_image_detailed "${TEST_IMAGES[$i]}" "$((i+1))"
done

# Final analysis
echo "=================================================================="
echo -e "${BLUE}📊 PIPELINE ANALYSIS SUMMARY${NC}"
echo "=================================================================="

echo -e "${CYAN}🎯 Key Observations:${NC}"
echo -e "   • All 3 images should be detected successfully"
echo -e "   • Processing times indicate pipeline efficiency"
echo -e "   • Detector choice shows pipeline optimization"

echo -e "${CYAN}📋 Expected Results:${NC}"
echo -e "   • qrimage.jpg: rxing detector (~70-90ms)"
echo -e "   • qrimage3.jpg: rxing detector (~60-80ms)" 
echo -e "   • qrimage4.jpg: rqrr detector (~70-90ms)"

echo -e "${CYAN}🤖 ONNX Activation Conditions:${NC}"
echo -e "   • ONNX activates when traditional detectors fail (Level 1)"
echo -e "   • Processing time >200ms typically indicates ONNX usage"
echo -e "   • These clear images likely won't need ONNX (good efficiency!)"

echo ""
echo -e "${GREEN}🏁 SPECIFIC QR IMAGES TEST COMPLETED!${NC}"
echo "=================================================================="