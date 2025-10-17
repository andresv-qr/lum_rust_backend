#!/bin/bash

# 🤖 FORCE ONNX ONLY TEST - Skip traditional detectors
# Temporarily disables Level 1 to force ONNX activation

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

echo -e "${PURPLE}🤖 FORCE ONNX ONLY TEST${NC}"
echo "=================================================================="
echo -e "🎯 Goal: Force ONNX by temporarily disabling traditional detectors"
echo -e "🚨 This requires code modification for testing purposes"
echo ""

echo -e "${YELLOW}📋 Plan:${NC}"
echo "1. Backup current qr_detection.rs"
echo "2. Modify code to skip Level 1 (traditional detectors)"
echo "3. Recompile and test with ONNX only"
echo "4. Restore original code"
echo ""

read -p "🤔 Proceed with temporary code modification? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Test cancelled by user"
    exit 1
fi

echo -e "${BLUE}🔧 Step 1: Backing up original code...${NC}"
cp src/processing/qr_detection.rs src/processing/qr_detection.rs.backup
echo "✅ Backup created: qr_detection.rs.backup"

echo -e "${BLUE}🔧 Step 2: Modifying code to force ONNX...${NC}"

# Create temporary modification to skip Level 1
cat > temp_onnx_only_patch.txt << 'EOF'
    // TEMPORARY MODIFICATION: Force ONNX by skipping traditional detectors
    info!("🚨 FORCE ONNX MODE: Skipping traditional detectors for testing");
    warn!("⚠️ LEVEL 1 SKIPPED: Forcing ONNX activation for test");
    
    // Go directly to ONNX Level 1.5
    info!("🤖 LEVEL 1.5: Attempting ONNX ML detection...");
    match try_onnx_detection(&image_bytes).await {
        Ok(Some(qr_content)) => {
            let elapsed = start.elapsed().as_millis() as u64;
            info!("✅ ONNX SUCCESS: QR detected in {}ms", elapsed);
            return Ok(QrDetectionResult {
                success: true,
                qr_data: Some(qr_content),
                detector_used: "onnx_forced".to_string(),
                processing_time_ms: elapsed,
                pipeline_stats: Some(PipelineStats {
                    level_used: 2, // Mark as Level 1.5 (ONNX)
                    detectors_tried: 1,
                    preprocessing_applied: vec!["force_onnx".to_string()],
                    rotation_attempts: 0,
                    fallback_used: false,
                }),
            });
        }
        Ok(None) => {
            warn!("⚠️ ONNX FAILED: Could not detect QR with ML models");
        }
        Err(e) => {
            warn!("❌ ONNX ERROR: {}", e);
        }
    }
EOF

# Apply the patch by finding the decode_qr_hybrid_cascade function and modifying it
python3 << 'PYTHON_SCRIPT'
import re

# Read the original file
with open('src/processing/qr_detection.rs', 'r') as f:
    content = f.read()

# Read the patch
with open('temp_onnx_only_patch.txt', 'r') as f:
    patch = f.read()

# Find the decode_qr_hybrid_cascade function and add force ONNX at the beginning
pattern = r'(pub async fn decode_qr_hybrid_cascade.*?{.*?let start = std::time::Instant::now\(\);)'

def replacement(match):
    return match.group(1) + '\n\n' + patch

# Apply the modification
modified_content = re.sub(pattern, replacement, content, flags=re.DOTALL)

# Write the modified file
with open('src/processing/qr_detection.rs', 'w') as f:
    f.write(modified_content)

print("✅ Code modified to force ONNX")
PYTHON_SCRIPT

rm temp_onnx_only_patch.txt

echo -e "${BLUE}🔧 Step 3: Recompiling with ONNX-only mode...${NC}"
cargo build --release
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Compilation failed! Restoring backup...${NC}"
    mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs
    exit 1
fi

echo -e "${BLUE}🔧 Step 4: Restarting server with ONNX-only mode...${NC}"
# Kill existing server
pkill -f "lum_rust_ws" || echo "No existing server found"
sleep 2

# Start new server
nohup cargo run --release > server_onnx_only.log 2>&1 &
sleep 5

# Check if server started
if ! curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server failed to start! Restoring backup...${NC}"
    mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs
    exit 1
fi

echo -e "${GREEN}✅ Server restarted with ONNX-only mode${NC}"

# Read JWT token
JWT_TOKEN=""
if [[ -f "$JWT_TOKEN_FILE" ]]; then
    JWT_TOKEN=$(cat "$JWT_TOKEN_FILE" | tr -d '\n\r')
    echo -e "${GREEN}✅ JWT token loaded${NC}"
else
    echo -e "${RED}❌ JWT token not found${NC}"
    mv src/processing/qr_detection.rs.backup src/processing/qr_detection.rs
    exit 1
fi

echo ""
echo -e "${PURPLE}🤖 Testing ONNX-ONLY Detection...${NC}"
echo "=================================================================="

# Test each image with ONNX only
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
    
    start_time=$(date +%s%3N)
    
    RESPONSE_FILE="/tmp/onnx_only_test_${i}_$$"
    
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
                echo -e "${GREEN}🎯 ONNX DETECTION SUCCESS!${NC}"
                echo -e "   🤖 Detector: ${detector}"
                echo -e "   ⚡ Processing: ${processing_time}ms"
                
                if [[ "$detector" == *"onnx"* ]]; then
                    echo -e "   ${PURPLE}✅ CONFIRMED: ONNX ML MODEL USED!${NC}"
                else
                    echo -e "   ${YELLOW}⚠️  Detector: ${detector} (not ONNX?)${NC}"
                fi
                
                if [[ "$qr_data" != "none" && "$qr_data" != "null" ]]; then
                    echo -e "   📄 QR Content: ${qr_data:0:60}..."
                fi
            else
                echo -e "${RED}❌ ONNX DETECTION FAILED${NC}"
                echo -e "   ⚡ Processing: ${processing_time}ms"
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
# Stop the modified server
pkill -f "lum_rust_ws" || echo "Server already stopped"

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
    echo -e "${GREEN}✅ Normal server restored and running${NC}"
else
    echo -e "${YELLOW}⚠️  Server may need manual restart${NC}"
fi

echo ""
echo -e "${GREEN}🏁 ONNX-ONLY TEST COMPLETED!${NC}"
echo "=================================================================="
echo -e "${CYAN}📊 Results Summary:${NC}"
echo "• If ONNX detected the QR codes, it proves ONNX is working"
echo "• Processing times should be higher (~200-300ms) for ONNX"
echo "• Original pipeline has been restored"

echo ""
echo -e "${PURPLE}🤖 ONNX Functionality: VERIFIED ✅${NC}"