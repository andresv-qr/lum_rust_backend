#!/usr/bin/env python3

"""
🤖 Create Test QR Image for ONNX Detection
Generates a QR code image with controlled difficulty to test ONNX pipeline
"""

import qrcode
from PIL import Image, ImageFilter, ImageEnhance
import numpy as np
import sys
import os

def create_test_qr():
    """Create a test QR code image"""
    
    # Create QR code with test data
    qr_data = "https://test-onnx.example.com/qr-detection-test?id=12345&model=onnx&timestamp=1696681200"
    
    qr = qrcode.QRCode(
        version=1,  # Small version
        error_correction=qrcode.constants.ERROR_CORRECT_L,  # Low error correction (more challenging)
        box_size=8,  # Smaller boxes
        border=2,  # Smaller border
    )
    
    qr.add_data(qr_data)
    qr.make(fit=True)
    
    # Create QR image
    qr_img = qr.make_image(fill_color="black", back_color="white")
    
    return qr_img, qr_data

def create_challenging_variants():
    """Create challenging variants for ONNX testing"""
    
    base_qr, qr_data = create_test_qr()
    
    # Convert to RGB for manipulations
    base_qr = base_qr.convert('RGB')
    
    variants = []
    
    # Variant 1: Slightly blurred (should be detectable by ONNX)
    blurred = base_qr.filter(ImageFilter.GaussianBlur(radius=0.5))
    variants.append(("qr_onnx_test1.jpg", blurred, "Slightly blurred"))
    
    # Variant 2: Low contrast (challenging for traditional, good for ML)
    low_contrast = ImageEnhance.Contrast(base_qr).enhance(0.6)
    variants.append(("qr_onnx_test2.jpg", low_contrast, "Low contrast"))
    
    # Variant 3: Slightly rotated and smaller
    small_rotated = base_qr.resize((150, 150)).rotate(5)
    variants.append(("qr_onnx_test3.jpg", small_rotated, "Small and rotated"))
    
    # Variant 4: Add some noise but keep detectable
    noisy = np.array(base_qr)
    noise = np.random.normal(0, 10, noisy.shape).astype(np.int16)
    noisy = np.clip(noisy + noise, 0, 255).astype(np.uint8)
    noisy_img = Image.fromarray(noisy)
    variants.append(("qr_onnx_test4.jpg", noisy_img, "With noise"))
    
    return variants, qr_data

def main():
    print("🤖 Creating ONNX Test QR Images...")
    print("=" * 50)
    
    try:
        variants, qr_data = create_challenging_variants()
        
        print(f"📋 QR Data: {qr_data}")
        print()
        
        for filename, image, description in variants:
            filepath = f"/home/client_1099_1/scripts/lum_rust_ws/{filename}"
            image.save(filepath, "JPEG", quality=85)
            file_size = os.path.getsize(filepath) // 1024
            print(f"✅ Created: {filename} ({file_size} KB) - {description}")
        
        print()
        print("🎯 Test Images Created Successfully!")
        print("💡 These images are designed to challenge traditional detectors")
        print("   while being detectable by ONNX ML models")
        
    except ImportError as e:
        print(f"❌ Missing dependency: {e}")
        print("💡 Install with: pip install qrcode[pil] pillow numpy")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Error creating test images: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()