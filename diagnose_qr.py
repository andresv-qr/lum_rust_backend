#!/usr/bin/env python3
"""
Script de diagnóstico para analizar la imagen con QR
y probar diferentes estrategias de preprocesamiento
"""

import cv2
import numpy as np
from pyzbar import pyzbar
import sys

def load_image(path):
    """Cargar imagen"""
    img = cv2.imread(path)
    if img is None:
        print(f"❌ Error: No se pudo cargar la imagen: {path}")
        sys.exit(1)
    print(f"✅ Imagen cargada: {img.shape[1]}x{img.shape[0]}")
    return img

def try_detect_qr(img, strategy_name, img_processed):
    """Intentar detectar QR con una estrategia específica"""
    print(f"\n🔍 Probando: {strategy_name}")
    
    # Detectar QR codes
    qr_codes = pyzbar.decode(img_processed)
    
    if qr_codes:
        print(f"   ✅ ÉXITO: {len(qr_codes)} QR detectado(s)")
        for qr in qr_codes:
            data = qr.data.decode('utf-8')
            print(f"   📊 Contenido: {data[:80]}...")
            print(f"   📍 Posición: {qr.rect}")
            print(f"   📐 Tipo: {qr.type}")
        return True
    else:
        print(f"   ❌ No se detectó QR")
        return False

def main():
    img_path = "/home/client_1099_1/scripts/lum_rust_ws/qrimage.jpg"
    
    print("=" * 70)
    print("🔬 DIAGNÓSTICO DE QR CODE")
    print("=" * 70)
    
    img = load_image(img_path)
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    
    strategies_tried = []
    
    # Estrategia 1: Imagen original
    success = try_detect_qr(img, "1. Imagen Original (Color)", img)
    strategies_tried.append(("Original Color", success))
    
    # Estrategia 2: Escala de grises
    success = try_detect_qr(gray, "2. Escala de Grises", gray)
    strategies_tried.append(("Escala Grises", success))
    
    # Estrategia 3: CLAHE
    clahe = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8,8))
    gray_clahe = clahe.apply(gray)
    success = try_detect_qr(gray_clahe, "3. CLAHE (clip=2.0, tiles=8x8)", gray_clahe)
    strategies_tried.append(("CLAHE", success))
    
    # Estrategia 4: CLAHE más agresivo
    clahe_aggressive = cv2.createCLAHE(clipLimit=4.0, tileGridSize=(8,8))
    gray_clahe_agg = clahe_aggressive.apply(gray)
    success = try_detect_qr(gray_clahe_agg, "4. CLAHE Agresivo (clip=4.0)", gray_clahe_agg)
    strategies_tried.append(("CLAHE Agresivo", success))
    
    # Estrategia 5: Binarización adaptativa
    binary = cv2.adaptiveThreshold(gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, 
                                   cv2.THRESH_BINARY, 11, 2)
    success = try_detect_qr(binary, "5. Binarización Adaptativa (Gaussian)", binary)
    strategies_tried.append(("Binarización Adaptativa", success))
    
    # Estrategia 6: Otsu
    _, binary_otsu = cv2.threshold(gray, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)
    success = try_detect_qr(binary_otsu, "6. Binarización Otsu", binary_otsu)
    strategies_tried.append(("Otsu", success))
    
    # Estrategia 7: Redimensionar (más pequeño)
    scale = 0.5
    img_small = cv2.resize(img, None, fx=scale, fy=scale, interpolation=cv2.INTER_AREA)
    success = try_detect_qr(img_small, f"7. Redimensionado ({scale}x)", img_small)
    strategies_tried.append((f"Redimensión {scale}x", success))
    
    # Estrategia 8: Redimensionar (más grande)
    scale = 1.5
    img_large = cv2.resize(img, None, fx=scale, fy=scale, interpolation=cv2.INTER_CUBIC)
    success = try_detect_qr(img_large, f"8. Redimensionado ({scale}x)", img_large)
    strategies_tried.append((f"Redimensión {scale}x", success))
    
    # Estrategia 9: Desenfoque + CLAHE
    blurred = cv2.GaussianBlur(gray, (5, 5), 0)
    clahe_blur = clahe.apply(blurred)
    success = try_detect_qr(clahe_blur, "9. Blur + CLAHE", clahe_blur)
    strategies_tried.append(("Blur + CLAHE", success))
    
    # Estrategia 10: Sharpening
    kernel = np.array([[-1,-1,-1], [-1,9,-1], [-1,-1,-1]])
    sharpened = cv2.filter2D(gray, -1, kernel)
    success = try_detect_qr(sharpened, "10. Sharpening", sharpened)
    strategies_tried.append(("Sharpening", success))
    
    # Estrategia 11: Ecualización de histograma simple
    equalized = cv2.equalizeHist(gray)
    success = try_detect_qr(equalized, "11. Ecualización de Histograma", equalized)
    strategies_tried.append(("Equalización", success))
    
    # Estrategia 12: Morfología (Opening)
    kernel_morph = np.ones((3,3), np.uint8)
    opening = cv2.morphologyEx(gray, cv2.MORPH_OPEN, kernel_morph)
    success = try_detect_qr(opening, "12. Morfología Opening", opening)
    strategies_tried.append(("Opening", success))
    
    # Estrategia 13: Morfología (Closing)
    closing = cv2.morphologyEx(gray, cv2.MORPH_CLOSE, kernel_morph)
    success = try_detect_qr(closing, "13. Morfología Closing", closing)
    strategies_tried.append(("Closing", success))
    
    # Estrategia 14: Inversión
    inverted = cv2.bitwise_not(gray)
    success = try_detect_qr(inverted, "14. Inversión de Colores", inverted)
    strategies_tried.append(("Inversión", success))
    
    # Resumen
    print("\n" + "=" * 70)
    print("📊 RESUMEN DE ESTRATEGIAS")
    print("=" * 70)
    
    successful = [s for s, success in strategies_tried if success]
    failed = [s for s, success in strategies_tried if not success]
    
    if successful:
        print(f"\n✅ ESTRATEGIAS EXITOSAS ({len(successful)}):")
        for s in successful:
            print(f"   - {s}")
    
    if failed:
        print(f"\n❌ ESTRATEGIAS FALLIDAS ({len(failed)}):")
        for s in failed:
            print(f"   - {s}")
    
    if not successful:
        print("\n⚠️  NINGUNA ESTRATEGIA DETECTÓ EL QR")
        print("📋 Posibles problemas:")
        print("   - QR muy pequeño en la imagen")
        print("   - QR distorsionado o dañado")
        print("   - QR con bajo contraste")
        print("   - Formato de QR no estándar")
        print("   - Librería pyzbar no soporta este tipo de QR")
        print("\n💡 Recomendación:")
        print("   - Usa una herramienta online para escanear: https://zxing.org/w/decode")
        print("   - O comparte una porción de la imagen con solo el QR")

if __name__ == "__main__":
    main()
