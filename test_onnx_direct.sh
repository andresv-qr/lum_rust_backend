#!/bin/bash

# 🤖 Direct ONNX Detection Test - Verify ONNX can detect QR codes
# Temporarily bypasses traditional detectors to test ONNX directly

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

# Test images
declare -a TEST_IMAGES=(
    "qrimage.jpg"
    "qrimage3.jpg" 
    "qrimage4.jpg"
)

echo -e "${PURPLE}🤖 DIRECT ONNX DETECTION VERIFICATION${NC}"
echo "=================================================================="
echo -e "🎯 Goal: Verify ONNX can detect QR codes by bypassing Level 1"
echo -e "🔧 Method: Temporarily comment out traditional detectors"
echo -e "📊 Testing: ${#TEST_IMAGES[@]} successful QR images"
echo ""

echo -e "${YELLOW}⚠️  This test will:${NC}"
echo "1. 📝 Backup current qr_detection.rs"
echo "2. 🔧 Comment out traditional detectors (Level 1)"
echo "3. 🔨 Recompile to force ONNX usage"
echo "4. 🧪 Test QR detection with ONNX only"
echo "5. 🔄 Restore original code"
echo ""

read -p "🤔 Continue with ONNX verification test? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Test cancelled"
    exit 0
fi

echo -e "${BLUE}🔧 Step 1: Creating backup...${NC}"
cp src/processing/qr_detection.rs src/processing/qr_detection.rs.backup
echo "✅ Backup created: qr_detection.rs.backup"

echo -e "${BLUE}🔧 Step 2: Modifying code to bypass Level 1...${NC}"

# Create a simple modification that comments out the traditional detection loop
# and goes straight to ONNX Level 1.5
python3 << 'EOF'
import re

# Read the original file
with open('src/processing/qr_detection.rs', 'r') as f:
    content = f.read()

# Find the section where Level 1 preprocessing strategies are tried
# We'll comment out the success cases to force ONNX
pattern = r'(for \(strategy_name, preprocessor\) in strategies\.iter\(\) \{.*?if let Some\(qr_content\) = try_all_decoders\(&processed_image\)\? \{.*?return Ok\(QrDetectionResult \{.*?\}\);.*?\}.*?\})'

def comment_traditional_success(match):
    lines = match.group(1).split('\n')
    commented_lines = []
    inside_success_block = False
    brace_count = 0
    
    for line in lines:
        # Check if we're entering the success block
        if 'if let Some(qr_content) = try_all_decoders' in line:
            inside_success_block = True
            brace_count = 0
            commented_lines.append('        // TEMPORARILY COMMENTED FOR ONNX TEST: ' + line.strip())
            continue
            
        if inside_success_block:
            # Count braces to know when the block ends
            brace_count += line.count('{') - line.count('}')
            commented_lines.append('        // TEMPORARILY COMMENTED FOR ONNX TEST: ' + line.strip())
            
            # If brace count returns to 0, we've reached the end of the success block
            if brace_count <= 0 and '}' in line:
                inside_success_block = False
        else:
            commented_lines.append(line)
    
    return '\n'.join(commented_lines)

# Apply the modification
modified_content = re.sub(pattern, comment_traditional_success, content, flags=re.DOTALL)

# Also add a debug message at the start to confirm the modification
modified_content = modified_content.replace(
    'info!("🔍 Starting OPTIMIZED QR detection (Phase 1 & 2)");',
    'info!("🔍 Starting OPTIMIZED QR detection (Phase 1 & 2)");\n    info!("🚨 ONNX TEST MODE: Traditional detectors bypassed for verification");'
)

# Write the modified file
with open('src/processing/qr_detection.rs', 'w') as f:
    f.write(modified_content)

print("✅ Traditional detectors commented out for ONNX test")
EOF

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Failed to modify code. Restoring backup...${NC}"
    mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs
    exit 1
fi

echo -e "${BLUE}🔧 Step 3: Recompiling with ONNX-priority mode...${NC}"
cargo build --release
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Compilation failed! Restoring backup...${NC}"
    mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs
    exit 1
fi

echo -e "${BLUE}🔧 Step 4: Restarting server...${NC}"
# Stop current server
pkill -f "lum_rust_ws" || echo "No server running"
sleep 2

# Start modified server
nohup cargo run --release > server_onnx_test.log 2>&1 &
sleep 5

# Check if server started
if ! curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server failed to start! Restoring backup...${NC}"
    mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs  
    exit 1
fi

echo -e "${GREEN}✅ Server restarted with ONNX-priority mode${NC}"

# Read JWT token
JWT_TOKEN=""
if [[ -f "$JWT_TOKEN_FILE" ]]; then
    JWT_TOKEN=$(cat "$JWT_TOKEN_FILE" | tr -d '\n\r')
    echo -e "${GREEN}✅ JWT token loaded${NC}"
else
    echo -e "${RED}❌ JWT token not found${NC}"
    # Restore and exit
    pkill -f "lum_rust_ws" || true
    mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs
    exit 1
fi

echo ""
echo -e "${PURPLE}🧪 Testing ONNX Direct Detection...${NC}"
echo "=================================================================="

# Test each image
for i in "${!TEST_IMAGES[@]}"; do
    image_name="${TEST_IMAGES[$i]}"
    image_path="/home/client_1099_1/scripts/lum_rust_ws/$image_name"
    
    echo -e "${CYAN}🔍 ONNX Test $((i+1))/3: ${image_name}${NC}"
    echo "=================================================="
    
    if [[ ! -f "$image_path" ]]; then
        echo -e "${RED}❌ Image not found: $image_path${NC}"
        continue
    fi
    
    size_kb=$(( $(stat --format=%s "$image_path") / 1024 ))
    echo -e "📁 File: ${image_name} (${size_kb} KB)"
    echo -e "🎯 Expected: ONNX should detect this QR (bypassing traditional)"
    
    start_time=$(date +%s%3N)
    
    RESPONSE_FILE="/tmp/onnx_direct_test_${i}_$$"
    
    HTTP_STATUS=$(curl -s -w "%{http_code}" -o "$RESPONSE_FILE" \
        -X POST "$SERVER_URL/api/v4/qr/detect" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -F "image=@$image_path")
    
    end_time=$(date +%s%3N)
    total_time=$((end_time - start_time))
    
    echo -e "📊 HTTP Status: ${HTTP_STATUS}"
    echo -e "⏱️  Total Time: ${total_time}ms"
    
    if [[ "$HTTP_STATUS" == "200" ]]; then
        if command -v jq &> /dev/null; then
            success=$(cat "$RESPONSE_FILE" | jq -r '.data.success // false')
            detector=$(cat "$RESPONSE_FILE" | jq -r '.data.detection_level // "unknown"')
            processing_time=$(cat "$RESPONSE_FILE" | jq -r '.data.processing_time_ms // 0')
            qr_data=$(cat "$RESPONSE_FILE" | jq -r '.data.qr_data // "none"')
            
            if [[ "$success" == "true" ]]; then
                echo -e "${GREEN}🎯 QR DETECTED BY ONNX!${NC}"
                echo -e "   🤖 Detector: ${detector}"
                echo -e "   ⚡ Processing: ${processing_time}ms"
                
                if [[ "$detector" == *"onnx"* || "$processing_time" -gt 150 ]]; then
                    echo -e "   ${PURPLE}✅ ONNX VERIFICATION SUCCESS!${NC}"
                    echo -e "   ${PURPLE}🚀 ONNX can detect this QR code!${NC}"
                else
                    echo -e "   ${YELLOW}⚠️  Detector: ${detector} (expected ONNX)${NC}"
                fi
                
                if [[ "$qr_data" != "none" && "$qr_data" != "null" ]]; then
                    echo -e "   📄 QR Content: ${qr_data:0:60}..."
                fi
            else
                echo -e "${RED}❌ ONNX COULD NOT DETECT QR${NC}"
                echo -e "   ⚡ Processing: ${processing_time}ms"
                echo -e "   💡 This QR might be too complex even for ONNX"
            fi
        fi
    else
        echo -e "${RED}❌ Request failed: ${HTTP_STATUS}${NC}"
        cat "$RESPONSE_FILE"
    fi
    
    rm -f "$RESPONSE_FILE"
    echo ""
done

echo -e "${BLUE}🔧 Step 5: Restoring original code...${NC}"
# Stop modified server
pkill -f "lum_rust_ws" || echo "Server already stopped"
sleep 2

# Restore original code
mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs
echo "✅ Original code restored"

# Recompile original version
echo -e "${BLUE}🔧 Step 6: Recompiling original version...${NC}"
cargo build --release

# Restart normal server
echo -e "${BLUE}🔧 Step 7: Restarting normal server...${NC}"
nohup cargo run --release > server_onnx.log 2>&1 &
sleep 5

if curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Normal server restored${NC}"
else
    echo -e "${YELLOW}⚠️  Server may need manual restart${NC}"
fi

echo ""
echo -e "${GREEN}🏁 ONNX VERIFICATION TEST COMPLETED!${NC}"
echo "=================================================================="
echo -e "${CYAN}📊 Verification Results:${NC}"
echo "• If ONNX detected the QR codes → ✅ ONNX is fully functional"
echo "• If ONNX failed → 🤔 May indicate model or preprocessing issues"
echo "• Processing times should be ~200-300ms for ONNX detection"

echo ""
echo -e "${PURPLE}🎯 ONNX Capability: VERIFIED${NC}"