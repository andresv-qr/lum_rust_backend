#!/usr/bin/env python3
"""
Export QReader YOLOv8 models to ONNX format for RustQReader integration.

This script exports QReader's pre-trained YOLOv8 detection models to ONNX format
with maximum precision settings, enabling high-performance QR detection in Rust.
"""

import os
import sys
import torch
import onnx
import onnxruntime as ort
from pathlib import Path
import numpy as np
from PIL import Image

def find_qreader_models():
    """Find QReader model files in the Python environment."""
    try:
        import qreader
        
        # Initialize QReader to trigger model download if needed
        print("🔄 Initializing QReader to download models if needed...")
        try:
            # Create a dummy QReader instance to trigger model download
            qr_reader = qreader.QReader()
            print("✅ QReader initialized successfully")
        except Exception as e:
            print(f"⚠️ QReader initialization warning: {e}")
        
        # QReader models are actually stored in the qrdet package
        try:
            import qrdet
            qrdet_path = Path(qrdet.__file__).parent
            model_dir = qrdet_path / ".model"
            print(f"🎯 Using qrdet package models at: {model_dir}")
        except ImportError:
            # Fallback to qreader path
            qreader_path = Path(qreader.__file__).parent
            model_dir = qreader_path / ".model"
        
        print(f"🔍 Searching for QReader models in: {model_dir}")
        
        if not model_dir.exists():
            print(f"❌ QReader model directory not found: {model_dir}")
            # Try alternative locations
            alt_locations = [
                qreader_path / "models",
                qreader_path / "qrdet" / ".model",
                Path.home() / ".cache" / "qreader",
                Path.home() / ".qreader"
            ]
            
            for alt_dir in alt_locations:
                print(f"🔍 Trying alternative location: {alt_dir}")
                if alt_dir.exists():
                    model_dir = alt_dir
                    print(f"✅ Found models directory at: {model_dir}")
                    break
            else:
                return None, None
            
        # Look for all model files
        nano_model = model_dir / "qrdet-n.pt"
        small_model = model_dir / "qrdet-s.pt"
        medium_model = model_dir / "qrdet-m.pt"
        large_model = model_dir / "qrdet-l.pt"
        
        # Check for alternative names if not found
        models = {
            'nano': nano_model,
            'small': small_model,
            'medium': medium_model,
            'large': large_model
        }
        
        alternatives = {
            'nano': ["qrdet_n.pt", "nano.pt", "qrdet-nano.pt"],
            'small': ["qrdet_s.pt", "small.pt", "qrdet-small.pt"],
            'medium': ["qrdet_m.pt", "medium.pt", "qrdet-medium.pt"],
            'large': ["qrdet_l.pt", "large.pt", "qrdet-large.pt"]
        }
        
        for size_name, model_path in models.items():
            if not model_path.exists():
                for alt_name in alternatives[size_name]:
                    alt_path = model_dir / alt_name
                    if alt_path.exists():
                        models[size_name] = alt_path
                        break
        
        print(f"📁 Nano model: {models['nano']} ({'✅ Found' if models['nano'].exists() else '❌ Missing'})")
        print(f"📁 Small model: {models['small']} ({'✅ Found' if models['small'].exists() else '❌ Missing'})")
        print(f"📁 Medium model: {models['medium']} ({'✅ Found' if models['medium'].exists() else '❌ Missing'})")
        print(f"📁 Large model: {models['large']} ({'✅ Found' if models['large'].exists() else '❌ Missing'})")
        
        return models
        
    except ImportError:
        print("❌ QReader not installed. Please install with: pip install qreader")
        return None, None

def export_model_to_onnx(model_path, output_path, model_size="large"):
    """Export a PyTorch model to ONNX format."""
    print(f"\n🚀 Exporting {model_size} model: {model_path}")
    
    try:
        # Load the PyTorch model with compatibility handling
        print("📦 Loading PyTorch model...")
        
        # Try loading with compatibility for older ultralytics versions
        try:
            model = torch.load(model_path, map_location='cpu')
        except Exception as e:
            if "ultralytics.yolo" in str(e):
                print("🔧 Attempting compatibility fix for ultralytics.yolo...")
                # Try to create a compatibility alias
                import sys
                import ultralytics
                
                # Create alias for old module structure
                if not hasattr(sys.modules, 'ultralytics.yolo'):
                    sys.modules['ultralytics.yolo'] = ultralytics
                    sys.modules['ultralytics.yolo.utils'] = getattr(ultralytics, 'utils', None)
                    sys.modules['ultralytics.yolo.v8'] = getattr(ultralytics, 'models', None)
                
                # Try loading again with the alias
                model = torch.load(model_path, map_location='cpu')
                print("✅ Compatibility fix successful!")
            else:
                raise e
        
        # Extract the actual model if it's wrapped in a dict
        if isinstance(model, dict):
            if 'model' in model:
                model = model['model']
            elif 'ema' in model:
                model = model['ema']
        
        # Convert model to float32 to avoid mixed precision issues
        print("🔧 Converting model to float32...")
        model = model.float()
        
        # Set model to evaluation mode
        model.eval()
        
        # Disable gradient computation
        torch.set_grad_enabled(False)
        
        # Create dummy input (batch_size=1, channels=3, height=640, width=640)
        dummy_input = torch.randn(1, 3, 640, 640, dtype=torch.float32)
        
        print("🔄 Converting to ONNX...")
        
        # Export to ONNX with maximum precision settings
        with torch.no_grad():
            torch.onnx.export(
                model,
                dummy_input,
                output_path,
                export_params=True,
                opset_version=11,  # Use more stable opset version
                do_constant_folding=True,
                input_names=['images'],
                output_names=['output0'],
                dynamic_axes={
                    'images': {0: 'batch_size'},
                    'output0': {0: 'batch_size'}
                },
                verbose=False,
                training=torch.onnx.TrainingMode.EVAL
            )
        
        print(f"✅ ONNX export completed: {output_path}")
        
        # Verify the exported model
        print("🔍 Verifying ONNX model...")
        onnx_model = onnx.load(output_path)
        onnx.checker.check_model(onnx_model)
        print("✅ ONNX model verification passed")
        
        # Test with ONNX Runtime
        print("🧪 Testing ONNX Runtime inference...")
        session = ort.InferenceSession(output_path)
        
        # Get input/output info
        input_info = session.get_inputs()[0]
        output_info = session.get_outputs()[0]
        
        print(f"📊 Input shape: {input_info.shape}")
        print(f"📊 Output shape: {output_info.shape}")
        
        # Run a test inference
        test_input = np.random.randn(1, 3, 640, 640).astype(np.float32)
        outputs = session.run(None, {input_info.name: test_input})
        
        print(f"✅ Test inference successful - Output shape: {outputs[0].shape}")
        
        # Get model size
        file_size = os.path.getsize(output_path) / (1024 * 1024)  # MB
        print(f"📏 Model size: {file_size:.1f} MB")
        
        return True
        
    except Exception as e:
        print(f"❌ Export failed: {e}")
        return False

def create_test_image():
    """Create a simple test image for validation."""
    # Create a simple test image (640x640 RGB)
    test_image = Image.new('RGB', (640, 640), color='white')
    return test_image

def main():
    """Main export function."""
    print("🎯 QReader to ONNX Export Tool")
    print("=" * 50)
    
    # Create models directory
    models_dir = Path("models")
    models_dir.mkdir(exist_ok=True)
    print(f"📁 Models directory: {models_dir.absolute()}")
    
    # Find QReader models
    models = find_qreader_models()
    
    if not any(model.exists() for model in models.values()):
        print("\n❌ No QReader models found!")
        print("Please ensure QReader is properly installed:")
        print("  pip install qreader")
        sys.exit(1)
    
    success_count = 0
    total_count = 0
    
    # Export models in order of preference (largest to smallest)
    model_exports = [
        ('large', models['large'], "qreader_detector_large.onnx"),
        ('medium', models['medium'], "qreader_detector_medium.onnx"),
        ('small', models['small'], "qreader_detector_small.onnx"),
        ('nano', models['nano'], "qreader_detector_nano.onnx")
    ]
    
    for model_name, model_path, output_filename in model_exports:
        if model_path.exists():
            total_count += 1
            output_path = models_dir / output_filename
            print(f"\n🚀 Attempting to export {model_name} model...")
            if export_model_to_onnx(model_path, output_path, model_name):
                success_count += 1
            else:
                print(f"⚠️ {model_name.capitalize()} model export failed, but continuing...")
        else:
            print(f"⏭️ Skipping {model_name} model (not found)")
    
    print("\n" + "=" * 50)
    print(f"🎉 Export Summary: {success_count}/{total_count} models exported successfully")
    
    if success_count > 0:
        print("\n✅ RustQReader is now ready to use!")
        print("🚀 Run 'cargo build --release' to compile with ONNX support")
        print("📝 Check logs for 'RustQReader ONNX initialized successfully' message")
    else:
        print("\n❌ No models were exported successfully")
        print("🔧 Please check the error messages above and try again")
    
    return success_count > 0

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
