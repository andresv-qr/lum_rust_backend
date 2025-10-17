#!/usr/bin/env python3
"""
QR Fallback Service - Small + Medium Hybrid (Smart Fallback)
Architecture: Small (fast, 80% cases) → Medium (slower, 15% cases) → Fail (5%)
RAM: 100MB base, 350MB after first Medium use | Latency: 40-120ms
"""

import io
import sys
import time
import json
import torch
from qreader import QReader
from PIL import Image
from http.server import HTTPServer, BaseHTTPRequestHandler

# Global singletons (lazy loading)
qr_reader_small = None
qr_reader_medium = None

# Metrics
metrics = {
    'total_requests': 0,
    'small_success': 0,
    'medium_success': 0,
    'total_failures': 0,
    'small_time_ms': 0,
    'medium_time_ms': 0,
}

def get_small_reader():
    """Lazy load Small model (always loaded first)"""
    global qr_reader_small
    if qr_reader_small is None:
        print("📦 Loading Small model (s)...")
        start = time.time()
        
        torch.set_grad_enabled(False)
        torch.set_num_threads(4)
        qr_reader_small = QReader(model_size='s', device='cpu')
        
        load_time = (time.time() - start) * 1000
        print(f"✅ Small model loaded in {load_time:.0f}ms (~100MB RAM)")
    return qr_reader_small

def get_medium_reader():
    """Lazy load Medium model (only when needed)"""
    global qr_reader_medium
    if qr_reader_medium is None:
        print("⚠️  First fallback to Medium - Loading model (m)...")
        start = time.time()
        
        torch.set_grad_enabled(False)
        torch.set_num_threads(6)  # Medium benefits from more threads
        qr_reader_medium = QReader(model_size='m', device='cpu')
        
        load_time = (time.time() - start) * 1000
        print(f"✅ Medium model loaded in {load_time:.0f}ms (~250MB RAM)")
        print(f"💾 Total RAM now: ~350MB (Small + Medium)")
    return qr_reader_medium

def preprocess_image(image_data):
    """Preprocess image with smart resizing"""
    img = Image.open(io.BytesIO(image_data)).convert('RGB')
    original_size = img.size
    
    # Smart resize based on image size
    # Small can handle larger images, Medium even more
    max_dim = 2048
    if max(img.size) > max_dim:
        ratio = max_dim / max(img.size)
        new_size = tuple(int(dim * ratio) for dim in img.size)
        img = img.resize(new_size, Image.Resampling.LANCZOS)
    
    return img, original_size

def detect_with_confidence(reader, img, model_name):
    """Detect QR with confidence scoring"""
    with torch.inference_mode():
        start = time.time()
        result = reader.detect_and_decode(img)
        latency_ms = int((time.time() - start) * 1000)
    
    if result and len(result) > 0:
        # QReader puede retornar string directo o tupla (data, bbox, confidence)
        if isinstance(result[0], tuple):
            data, bbox, confidence = result[0]
        else:
            data = result[0]
            confidence = 1.0  # Assume high confidence if not provided
        
        return {
            'success': True,
            'data': data,
            'confidence': confidence,
            'latency_ms': latency_ms,
            'model': model_name
        }
    
    return {
        'success': False,
        'data': None,
        'confidence': 0.0,
        'latency_ms': latency_ms,
        'model': model_name
    }

class QRHybridHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        """Silenciar logs HTTP por defecto"""
        pass
    
    def do_GET(self):
        """Health check y metrics endpoint"""
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'status': 'healthy'}).encode())
        elif self.path == '/metrics':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            
            # Calculate percentages
            total = metrics['total_requests']
            if total > 0:
                small_pct = (metrics['small_success'] / total) * 100
                medium_pct = (metrics['medium_success'] / total) * 100
                fail_pct = (metrics['total_failures'] / total) * 100
                avg_small = metrics['small_time_ms'] / total
                avg_medium = metrics['medium_time_ms'] / metrics['medium_success'] if metrics['medium_success'] > 0 else 0
            else:
                small_pct = medium_pct = fail_pct = avg_small = avg_medium = 0
            
            response = {
                'total_requests': total,
                'small_success': metrics['small_success'],
                'medium_success': metrics['medium_success'],
                'total_failures': metrics['total_failures'],
                'small_success_pct': round(small_pct, 2),
                'medium_success_pct': round(medium_pct, 2),
                'failure_pct': round(fail_pct, 2),
                'avg_small_latency_ms': round(avg_small, 2),
                'avg_medium_latency_ms': round(avg_medium, 2),
                'small_loaded': qr_reader_small is not None,
                'medium_loaded': qr_reader_medium is not None,
            }
            self.wfile.write(json.dumps(response, indent=2).encode())
        else:
            self.send_error(404)
    
    def do_POST(self):
        """QR detection endpoint"""
        if self.path != '/detect':
            self.send_error(404)
            return
        
        request_start = time.time()
        metrics['total_requests'] += 1
        
        content_length = int(self.headers['Content-Length'])
        image_data = self.rfile.read(content_length)
        
        try:
            # Preprocess image
            img, original_size = preprocess_image(image_data)
            
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            # LEVEL 1: Try Small Model (Fast - 80% of cases)
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            small_result = detect_with_confidence(
                get_small_reader(),
                img,
                'small'
            )
            
            metrics['small_time_ms'] += small_result['latency_ms']
            
            # Success criteria for Small:
            # 1. Detection successful
            # 2. Confidence > 0.65 (good enough to trust)
            if small_result['success'] and small_result['confidence'] > 0.65:
                metrics['small_success'] += 1
                
                total_latency = int((time.time() - request_start) * 1000)
                response = {
                    'success': True,
                    'data': small_result['data'],
                    'confidence': small_result['confidence'],
                    'model_used': 'small',
                    'latency_ms': total_latency,
                    'small_latency_ms': small_result['latency_ms'],
                    'fallback_used': False,
                    'image_size': f"{img.size[0]}x{img.size[1]}",
                    'original_size': f"{original_size[0]}x{original_size[1]}"
                }
                
                # Log success
                print(f"✅ SMALL | {total_latency}ms | conf={small_result['confidence']:.2f} | {img.size[0]}×{img.size[1]}")
                
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(response).encode())
                return
            
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            # LEVEL 2: Fallback to Medium Model (Slower - 15% of cases)
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            print(f"⚠️  Small failed/low confidence ({small_result['confidence']:.2f}) - trying Medium fallback...")
            
            medium_result = detect_with_confidence(
                get_medium_reader(),
                img,
                'medium'
            )
            
            metrics['medium_time_ms'] += medium_result['latency_ms']
            
            if medium_result['success']:
                metrics['medium_success'] += 1
                
                total_latency = int((time.time() - request_start) * 1000)
                response = {
                    'success': True,
                    'data': medium_result['data'],
                    'confidence': medium_result['confidence'],
                    'model_used': 'medium',
                    'latency_ms': total_latency,
                    'small_latency_ms': small_result['latency_ms'],
                    'medium_latency_ms': medium_result['latency_ms'],
                    'fallback_used': True,
                    'small_confidence': small_result['confidence'],
                    'image_size': f"{img.size[0]}x{img.size[1]}",
                    'original_size': f"{original_size[0]}x{original_size[1]}"
                }
                
                # Log fallback success
                print(f"✅ MEDIUM | {total_latency}ms | small={small_result['latency_ms']}ms + medium={medium_result['latency_ms']}ms | conf={medium_result['confidence']:.2f}")
                
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(response).encode())
                return
            
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            # LEVEL 3: Both Failed (5% of cases - genuinely unreadable)
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            metrics['total_failures'] += 1
            
            total_latency = int((time.time() - request_start) * 1000)
            response = {
                'success': False,
                'data': None,
                'confidence': 0.0,
                'model_used': 'none',
                'latency_ms': total_latency,
                'small_latency_ms': small_result['latency_ms'],
                'medium_latency_ms': medium_result['latency_ms'],
                'fallback_used': True,
                'error': 'QR not detected by any model (likely unreadable)',
                'image_size': f"{img.size[0]}x{img.size[1]}",
                'original_size': f"{original_size[0]}x{original_size[1]}"
            }
            
            # Log failure
            print(f"❌ FAILED | {total_latency}ms | Both Small and Medium failed | {img.size[0]}×{img.size[1]}")
            
            self.send_response(200)  # 200 with success=false (not 404)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode())
            
        except Exception as e:
            metrics['total_failures'] += 1
            print(f"❌ ERROR: {e}")
            self.send_error(500, str(e))

def print_banner():
    """Print startup banner"""
    print("=" * 70)
    print("🚀 QR Fallback Service - Small + Medium Hybrid")
    print("=" * 70)
    print("📍 Endpoint:      http://127.0.0.1:8008/detect")
    print("📊 Metrics:       http://127.0.0.1:8008/metrics")
    print("🏥 Health:        http://127.0.0.1:8008/health")
    print("-" * 70)
    print("🎯 Strategy:")
    print("   1. Try Small (s) first       → 80% cases (fast)")
    print("   2. Fallback to Medium (m)    → 15% cases (slower)")
    print("   3. Fail gracefully           → 5% cases (unreadable)")
    print("-" * 70)
    print("💾 RAM Usage:")
    print("   Initial:           ~100MB (Small only)")
    print("   After 1st fallback: ~350MB (Small + Medium)")
    print("   Steady state:       ~350MB")
    print("-" * 70)
    print("⚡ Expected Latency:")
    print("   Small success:     35-70ms   (80% of requests)")
    print("   Medium fallback:   100-180ms (15% of requests)")
    print("   Average:           ~55ms     (weighted)")
    print("   P95:               ~150ms")
    print("-" * 70)
    print("📈 Expected Success Rate:")
    print("   Small:   80-88%")
    print("   Medium:  +5-7% additional")
    print("   Total:   85-90% ✅")
    print("=" * 70)
    print()

if __name__ == '__main__':
    print_banner()
    
    # Pre-load Small model to avoid first-request latency
    print("🔄 Pre-loading Small model...")
    get_small_reader()
    print("✅ Small model ready")
    print("⏳ Medium model will load on first fallback (lazy loading)")
    print()
    print("✅ Server ready. Waiting for requests...")
    print()
    
    try:
        server = HTTPServer(('127.0.0.1', 8008), QRHybridHandler)
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n")
        print("=" * 70)
        print("👋 Shutting down...")
        print("=" * 70)
        print()
        print("📊 Final Statistics:")
        total = metrics['total_requests']
        if total > 0:
            print(f"   Total Requests:    {total}")
            print(f"   Small Success:     {metrics['small_success']} ({metrics['small_success']/total*100:.1f}%)")
            print(f"   Medium Success:    {metrics['medium_success']} ({metrics['medium_success']/total*100:.1f}%)")
            print(f"   Failures:          {metrics['total_failures']} ({metrics['total_failures']/total*100:.1f}%)")
            print(f"   Overall Success:   {metrics['small_success']+metrics['medium_success']} ({(metrics['small_success']+metrics['medium_success'])/total*100:.1f}%)")
        print()
        sys.exit(0)
