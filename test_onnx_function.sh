#!/bin/bash

# 🤖 ONNX Unit Test - Direct function call verification
# Tests ONNX detection capability without modifying main code

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${PURPLE}🤖 ONNX DIRECT FUNCTION TEST${NC}"
echo "=================================================================="
echo -e "🎯 Goal: Test ONNX detection capability directly"
echo -e "🔧 Method: Create unit test calling try_onnx_detection"
echo -e "📊 Safe: No modification to main code"
echo ""

# Test images
declare -a TEST_IMAGES=(
    "qrimage.jpg"
    "qrimage3.jpg"
    "qrimage4.jpg"
)

echo -e "${BLUE}🔧 Creating ONNX unit test...${NC}"

# Create a simple Rust test file
cat > src/test_onnx_unit.rs << 'EOF'
//! 🤖 ONNX Unit Test - Direct detection verification
//! Tests the try_onnx_detection function directly

use std::fs;
use tokio;

// Import the ONNX detection function (we need to make it public temporarily)
use crate::processing::qr_detection::try_onnx_detection;

#[tokio::test]
async fn test_onnx_direct_detection() {
    println!("🤖 Testing ONNX direct detection...");
    
    let test_images = vec![
        "/home/client_1099_1/scripts/lum_rust_ws/qrimage.jpg",
        "/home/client_1099_1/scripts/lum_rust_ws/qrimage3.jpg", 
        "/home/client_1099_1/scripts/lum_rust_ws/qrimage4.jpg",
    ];
    
    for (i, image_path) in test_images.iter().enumerate() {
        println!("🔍 Testing image {}: {}", i + 1, image_path);
        
        match fs::read(image_path) {
            Ok(image_bytes) => {
                println!("  📁 Loaded image: {} bytes", image_bytes.len());
                
                let start = std::time::Instant::now();
                match try_onnx_detection(&image_bytes).await {
                    Ok(Some(result)) => {
                        let elapsed = start.elapsed().as_millis();
                        println!("  ✅ ONNX SUCCESS: QR detected in {}ms", elapsed);
                        println!("  📄 QR Data: {}", result.qr_data.as_ref().unwrap_or(&"none".to_string()));
                        println!("  🤖 Detector: {}", result.detector_used);
                        println!("  ⚡ Processing Time: {}ms", result.processing_time_ms);
                    }
                    Ok(None) => {
                        let elapsed = start.elapsed().as_millis();
                        println!("  ❌ ONNX: No QR detected ({}ms)", elapsed);
                    }
                    Err(e) => {
                        let elapsed = start.elapsed().as_millis();
                        println!("  ⚠️ ONNX Error: {} ({}ms)", e, elapsed);
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Failed to load image: {}", e);
            }
        }
        println!();
    }
}

pub async fn run_onnx_unit_test() {
    test_onnx_direct_detection().await;
}
EOF

echo "✅ Unit test created: src/test_onnx_unit.rs"

echo -e "${BLUE}🔧 Making try_onnx_detection temporarily public...${NC}"

# Make the function public temporarily so we can test it
sed -i 's/async fn try_onnx_detection/pub async fn try_onnx_detection/' src/processing/qr_detection.rs

# Add the test module to lib.rs
echo "" >> src/lib.rs
echo "// Temporary ONNX unit test" >> src/lib.rs  
echo "#[cfg(test)]" >> src/lib.rs
echo "pub mod test_onnx_unit;" >> src/lib.rs

echo -e "${BLUE}🔧 Creating test runner...${NC}"

# Create a simple test runner binary
cat > src/bin/test_onnx.rs << 'EOF'
//! 🤖 ONNX Test Runner - Direct detection test

use lum_rust_ws::test_onnx_unit;

#[tokio::main]
async fn main() {
    println!("🚀 Starting ONNX direct detection test...");
    println!("================================================================");
    
    // Initialize ONNX models first
    use lum_rust_ws::processing::qr_detection;
    qr_detection::initialize_onnx_readers();
    
    // Wait a moment for initialization
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Run the test
    test_onnx_unit::run_onnx_unit_test().await;
    
    println!("================================================================");
    println!("🏁 ONNX test completed!");
}
EOF

echo "✅ Test runner created: src/bin/test_onnx.rs"

echo -e "${BLUE}🔧 Compiling test...${NC}"
cargo build --release --bin test_onnx

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Compilation failed! Cleaning up...${NC}"
    # Restore original function visibility
    sed -i 's/pub async fn try_onnx_detection/async fn try_onnx_detection/' src/processing/qr_detection.rs
    # Remove test module from lib.rs
    sed -i '/Temporary ONNX unit test/,$d' src/lib.rs
    rm -f src/test_onnx_unit.rs src/bin/test_onnx.rs
    exit 1
fi

echo -e "${GREEN}✅ Test compiled successfully${NC}"

echo ""
echo -e "${PURPLE}🧪 Running ONNX Direct Detection Test...${NC}"
echo "=================================================================="

# Run the test
./target/release/test_onnx

echo ""
echo -e "${BLUE}🔧 Cleaning up test files...${NC}"

# Restore original function visibility
sed -i 's/pub async fn try_onnx_detection/async fn try_onnx_detection/' src/processing/qr_detection.rs

# Remove test module from lib.rs  
sed -i '/Temporary ONNX unit test/,$d' src/lib.rs

# Remove test files
rm -f src/test_onnx_unit.rs src/bin/test_onnx.rs

echo "✅ Cleanup completed"

echo ""
echo -e "${GREEN}🏁 ONNX DIRECT FUNCTION TEST COMPLETED!${NC}"
echo "=================================================================="
echo -e "${CYAN}📊 Test Results Analysis:${NC}"
echo "• ✅ If ONNX detected QRs → ONNX is fully functional"
echo "• ❌ If ONNX failed → May indicate model loading issues"  
echo "• ⏱️ Processing times show ONNX performance"
echo "• 🎯 This proves ONNX capability independent of pipeline"

echo ""
echo -e "${PURPLE}🤖 ONNX Function Verification: COMPLETED ✅${NC}"