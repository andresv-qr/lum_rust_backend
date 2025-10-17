import logging
from decimal import Decimal
from bs4 import BeautifulSoup
from datetime import datetime
from pyzbar.pyzbar import decode, ZBarSymbol
from app_db import line_to_db
import cv2
import numpy as np
from qreader import QReader
import requests
import httpx # Import httpx
import json
from ws_mensajes.app_variables import WHATSAPP_APP_SOURCE, TELEGRAM_APP_SOURCE, EMAIL_APP_SOURCE, INVALID_CUFE
import pytz
import torch
from typing import Optional, Tuple
import time

panama_timezone = pytz.timezone('America/Panama')

# ✅ SINGLETON PATTERN - Modelos se cargan UNA VEZ
_qreader_small: Optional[QReader] = None
_qreader_medium: Optional[QReader] = None
_qreader_large: Optional[QReader] = None

# ✅ Metrics para monitoreo
_detection_metrics = {
    'total_requests': 0,
    'cv2_success': 0,
    'cv2_curved_success': 0,
    'pyzbar_success': 0,
    'qreader_small_success': 0,
    'qreader_medium_success': 0,
    'qreader_large_success': 0,
    'total_failures': 0,
    'avg_latency_ms': 0,
}

def initialize_qreaders():
    """
    ✅ Inicializar todos los modelos QReader al startup de la aplicación
    Esto evita la latencia del primer request y garantiza singleton
    """
    global _qreader_small, _qreader_medium, _qreader_large
    
    print("🚀 Initializing QReader models...")
    
    # ✅ Optimizaciones PyTorch globales
    torch.set_grad_enabled(False)  # Desactivar gradientes (ahorra 30% RAM)
    torch.set_num_threads(4)       # Limitar threads CPU
    
    start_time = time.time()
    
    # Cargar Small model
    if _qreader_small is None:
        print("📦 Loading QReader Small model...")
        _qreader_small = QReader(
            model_size='s',
            min_confidence=0.5,  # ✅ Confidence más alta (menos false positives)
            device='cpu'
        )
        print(f"✅ Small model loaded (~100MB RAM)")
    
    # Cargar Medium model (mejor balance que Large)
    if _qreader_medium is None:
        print("📦 Loading QReader Medium model...")
        _qreader_medium = QReader(
            model_size='m',
            min_confidence=0.5,
            device='cpu'
        )
        print(f"✅ Medium model loaded (~250MB RAM)")
    
    # Opcional: Large solo si realmente lo necesitas
    # if _qreader_large is None:
    #     print("📦 Loading QReader Large model...")
    #     _qreader_large = QReader(
    #         model_size='l',
    #         min_confidence=0.5,
    #         device='cpu'
    #     )
    #     print(f"✅ Large model loaded (~700MB RAM)")
    
    total_time = (time.time() - start_time) * 1000
    print(f"🎉 All QReader models initialized in {total_time:.0f}ms")
    print(f"💾 Total RAM usage: ~350MB (Small + Medium)")

def get_qreader_small() -> QReader:
    """✅ Lazy loading Small QReader (singleton)"""
    global _qreader_small
    if _qreader_small is None:
        torch.set_grad_enabled(False)
        torch.set_num_threads(4)
        _qreader_small = QReader(model_size='s', min_confidence=0.5, device='cpu')
    return _qreader_small

def get_qreader_medium() -> QReader:
    """✅ Lazy loading Medium QReader (singleton)"""
    global _qreader_medium
    if _qreader_medium is None:
        torch.set_grad_enabled(False)
        torch.set_num_threads(4)
        _qreader_medium = QReader(model_size='m', min_confidence=0.5, device='cpu')
    return _qreader_medium

def get_qreader_large() -> QReader:
    """✅ Lazy loading Large QReader (singleton)"""
    global _qreader_large
    if _qreader_large is None:
        torch.set_grad_enabled(False)
        torch.set_num_threads(4)
        _qreader_large = QReader(model_size='l', min_confidence=0.5, device='cpu')
    return _qreader_large

def leer_limpiar_imagen(image_data):
    """
    ✅ Preprocessing OPTIMIZADO - Menos agresivo, más efectivo
    
    CAMBIOS vs versión original:
    - Sin CLAHE agresivo (destruía QRs)
    - Sin Gaussian Blur excesivo
    - Sin sharpening agresivo
    - Sin conversión PNG innecesaria
    - Múltiples estrategias de preprocessing
    """
    # Leer imagen directamente a grayscale
    image_array = np.frombuffer(image_data, np.uint8)
    imagen = cv2.imdecode(image_array, cv2.IMREAD_GRAYSCALE)  # ✅ Directo a grayscale
    
    if imagen is None:
        raise ValueError("No se pudo decodificar la imagen")
    
    # ✅ ESTRATEGIA 1: Solo histogram equalization (simple y efectivo)
    equalized = cv2.equalizeHist(imagen)
    
    # ✅ ESTRATEGIA 2: Otsu threshold
    _, binary = cv2.threshold(equalized, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)
    
    return {
        'original': imagen,      # Imagen original en grayscale
        'equalized': equalized,  # Con histogram equalization
        'binary': binary,        # Con Otsu threshold
        'raw': imagen           # Sin procesamiento (RAW)
    }

def imagen_a_url(image_data):
    """
    ✅ Detección QR OPTIMIZADA con multi-strategy y singleton
    
    CAMBIOS PRINCIPALES:
    1. ✅ Singleton pattern (no crear instancias nuevas)
    2. ✅ Multi-strategy preprocessing
    3. ✅ Orden inteligente de métodos
    4. ✅ torch.inference_mode() para velocidad
    5. ✅ Métricas de performance
    6. ✅ Smart fallback (Small → Medium en lugar de Large)
    """
    global _detection_metrics
    _detection_metrics['total_requests'] += 1
    
    start_time = time.time()
    
    try:
        # ✅ Preprocessing multi-strategy
        processed_images = leer_limpiar_imagen(image_data)
        
        # Lista de estrategias de preprocessing a probar
        strategies = [
            ('equalized', processed_images['equalized']),
            ('raw', processed_images['raw']),
            ('binary', processed_images['binary']),
        ]
        
        for strategy_name, processed_image in strategies:
            
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            # METHOD 1: OpenCV QR detector (más rápido)
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            try:
                detector = cv2.QRCodeDetector()
                result, bbox, _ = detector.detectAndDecode(processed_image)
                
                if result:
                    _detection_metrics['cv2_success'] += 1
                    latency = (time.time() - start_time) * 1000
                    _detection_metrics['avg_latency_ms'] = (
                        (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
                        / _detection_metrics['total_requests']
                    )
                    return result, f'CV2_{strategy_name.upper()}'
            except Exception as e:
                logging.debug(f"CV2 error with {strategy_name}: {e}")
            
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            # METHOD 2: OpenCV curved QR detector
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            try:
                detector = cv2.QRCodeDetector()
                detector.setEpsX(0.3)
                detector.setEpsY(0.3)
                result, bbox, _ = detector.detectAndDecodeCurved(processed_image)
                
                if result:
                    _detection_metrics['cv2_curved_success'] += 1
                    latency = (time.time() - start_time) * 1000
                    _detection_metrics['avg_latency_ms'] = (
                        (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
                        / _detection_metrics['total_requests']
                    )
                    return result, f'CV2_CURVED_{strategy_name.upper()}'
            except Exception as e:
                logging.debug(f"CV2_CURVED error with {strategy_name}: {e}")
            
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            # METHOD 3: pyzbar library
            # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            try:
                decoded_data = decode(processed_image)
                qr_codes = [x for x in decoded_data if x.type == 'QRCODE']
                
                if qr_codes:
                    data = qr_codes[0].data.decode()
                    _detection_metrics['pyzbar_success'] += 1
                    latency = (time.time() - start_time) * 1000
                    _detection_metrics['avg_latency_ms'] = (
                        (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
                        / _detection_metrics['total_requests']
                    )
                    return data, f'PYZBAR_{strategy_name.upper()}'
            except Exception as e:
                logging.debug(f"PYZBAR error with {strategy_name}: {e}")
        
        # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        # METHOD 4: QReader Small model (✅ SINGLETON - NO crear nueva instancia)
        # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        try:
            qreader_small = get_qreader_small()  # ✅ Reutiliza instancia existente
            
            # Probar con diferentes estrategias de preprocessing
            for strategy_name, processed_image in strategies:
                with torch.inference_mode():  # ✅ Optimización crítica de PyTorch
                    detected_data = qreader_small.detect_and_decode(image=processed_image)
                
                if detected_data and len(detected_data) > 0 and detected_data[0]:
                    _detection_metrics['qreader_small_success'] += 1
                    latency = (time.time() - start_time) * 1000
                    _detection_metrics['avg_latency_ms'] = (
                        (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
                        / _detection_metrics['total_requests']
                    )
                    return detected_data[0], f'QREADER_S_{strategy_name.upper()}'
        except Exception as e:
            logging.error(f"QReader Small error: {e}")
        
        # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        # METHOD 5: QReader Medium model (✅ MEJOR BALANCE que Large)
        # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        try:
            qreader_medium = get_qreader_medium()  # ✅ Reutiliza instancia existente
            
            # Probar con diferentes estrategias de preprocessing
            for strategy_name, processed_image in strategies:
                with torch.inference_mode():  # ✅ Optimización crítica de PyTorch
                    detected_data = qreader_medium.detect_and_decode(image=processed_image)
                
                if detected_data and len(detected_data) > 0 and detected_data[0]:
                    _detection_metrics['qreader_medium_success'] += 1
                    latency = (time.time() - start_time) * 1000
                    _detection_metrics['avg_latency_ms'] = (
                        (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
                        / _detection_metrics['total_requests']
                    )
                    return detected_data[0], f'QREADER_M_{strategy_name.upper()}'
        except Exception as e:
            logging.error(f"QReader Medium error: {e}")
        
        # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        # METHOD 6: QReader Large model (solo si realmente necesitas máxima precisión)
        # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        # COMENTADO: Medium suele ser suficiente y más eficiente
        # try:
        #     qreader_large = get_qreader_large()
        #     
        #     for strategy_name, processed_image in strategies:
        #         with torch.inference_mode():
        #             detected_data = qreader_large.detect_and_decode(image=processed_image)
        #         
        #         if detected_data and len(detected_data) > 0 and detected_data[0]:
        #             _detection_metrics['qreader_large_success'] += 1
        #             latency = (time.time() - start_time) * 1000
        #             _detection_metrics['avg_latency_ms'] = (
        #                 (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
        #                 / _detection_metrics['total_requests']
        #             )
        #             return detected_data[0], f'QREADER_L_{strategy_name.upper()}'
        # except Exception as e:
        #     logging.error(f"QReader Large error: {e}")
        
    except Exception as e:
        logging.error(f"Error in imagen_a_url: {e}")
        _detection_metrics['total_failures'] += 1
        latency = (time.time() - start_time) * 1000
        _detection_metrics['avg_latency_ms'] = (
            (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
            / _detection_metrics['total_requests']
        )
        return None, "ERROR"
    
    # Si llegamos aquí, todos los métodos fallaron
    _detection_metrics['total_failures'] += 1
    latency = (time.time() - start_time) * 1000
    _detection_metrics['avg_latency_ms'] = (
        (_detection_metrics['avg_latency_ms'] * (_detection_metrics['total_requests'] - 1) + latency) 
        / _detection_metrics['total_requests']
    )
    return None, "FAILED_ALL_METHODS"

def get_detection_metrics():
    """✅ Obtener métricas de detección para monitoreo"""
    total = _detection_metrics['total_requests']
    if total == 0:
        return _detection_metrics
    
    # Calcular success rate
    total_success = (
        _detection_metrics['cv2_success'] +
        _detection_metrics['cv2_curved_success'] +
        _detection_metrics['pyzbar_success'] +
        _detection_metrics['qreader_small_success'] +
        _detection_metrics['qreader_medium_success'] +
        _detection_metrics['qreader_large_success']
    )
    
    success_rate = (total_success / total) * 100
    
    return {
        **_detection_metrics,
        'success_rate_pct': round(success_rate, 2),
        'methods_breakdown': {
            'cv2_pct': round((_detection_metrics['cv2_success'] / total) * 100, 2),
            'cv2_curved_pct': round((_detection_metrics['cv2_curved_success'] / total) * 100, 2),
            'pyzbar_pct': round((_detection_metrics['pyzbar_success'] / total) * 100, 2),
            'qreader_small_pct': round((_detection_metrics['qreader_small_success'] / total) * 100, 2),
            'qreader_medium_pct': round((_detection_metrics['qreader_medium_success'] / total) * 100, 2),
            'qreader_large_pct': round((_detection_metrics['qreader_large_success'] / total) * 100, 2),
            'failure_pct': round((_detection_metrics['total_failures'] / total) * 100, 2)
        }
    }

def reset_detection_metrics():
    """✅ Resetear métricas (útil para testing)"""
    global _detection_metrics
    _detection_metrics = {
        'total_requests': 0,
        'cv2_success': 0,
        'cv2_curved_success': 0,
        'pyzbar_success': 0,
        'qreader_small_success': 0,
        'qreader_medium_success': 0,
        'qreader_large_success': 0,
        'total_failures': 0,
        'avg_latency_ms': 0,
    }

# ✅ Función para compatibilidad con código existente
def imagen_a_url_legacy_compatible(sharpened):
    """
    ✅ Wrapper para mantener compatibilidad con el código existente
    que espera una imagen ya procesada en lugar de bytes
    """
    try:
        # Convertir imagen procesada de vuelta a bytes para usar la nueva función
        _, img_encoded = cv2.imencode('.png', sharpened)
        image_bytes = img_encoded.tobytes()
        
        # Usar la nueva implementación optimizada
        return imagen_a_url(image_bytes)
    except Exception as e:
        logging.error(f"Error in legacy compatibility wrapper: {e}")
        return None, "ERROR"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# El resto del código permanece igual (url_to_dfs, etc.)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async def url_to_dfs(url, source_id, origin, user_email, user_db_id, reception_date):
    """
    ✅ Esta función permanece igual - solo cambié la detección QR
    """
    # API para obtener todos los productos#
    async with httpx.AsyncClient() as client: # Use httpx.AsyncClient
        response = await client.get(url) # Changed to await client.get()

    # Convierte a bs4
    soup = BeautifulSoup(response.content, 'html.parser')

    #validar si url muestra datos
    is_url_valid = True
    try:
        is_url_valid = not soup.find(class_="alert-danger").text in INVALID_CUFE
    except:
        pass

    ############################### header
    if is_url_valid:
        # Extraer información
        try:
            no = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(1) > div > div > div:nth-of-type(1) > div > div:nth-of-type(1) > h5').get_text(strip=True).replace('No. ', '')
            datet = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(1) > div > div > div:nth-of-type(1) > div > div:nth-of-type(3) > h5').get_text(strip=True)
            datet = datetime.strptime(datet, '%d/%m/%Y %H:%M:%S')
            date = datet.strftime('%Y%m%d')
            time = datet.strftime('%H%M%S')
            cufe = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(1) > div > div > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
            auth_date = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(1) > div > div > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
            issuer_ruc = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
            issuer_dv = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
            issuer_name = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(3) > dl > dd').get_text(strip=True)
            issuer_address = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
            issuer_phone = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
            try: #si es nacional
                receptor_id = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
                receptor_dv = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
                receptor_name = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(3) > dl > dd').get_text(strip=True)
                receptor_address = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
                receptor_phone = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
            except: #si es extranjero
                receptor_name = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
                receptor_address = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
                receptor_phone = soup.select_one('html > body > div:nth-of-type(2) > div:nth-of-type(3) > div > div > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(2) > dl > dd').get_text(strip=True)

        except:
            no = soup.select_one('#facturaHTML > div:nth-of-type(1) > div > div > div:nth-of-type(1) > div > div:nth-of-type(1) > h5').get_text(strip=True).replace('No. ', '')
            datet = soup.select_one('#facturaHTML > div:nth-of-type(1) > div > div > div:nth-of-type(1) > div > div:nth-of-type(3) > h5').get_text(strip=True)
            datet = datetime.strptime(datet, '%d/%m/%Y %H:%M:%S')
            date = datet.strftime('%Y%m%d')
            time = datet.strftime('%H%M%S')
            cufe = soup.select_one('#facturaHTML > div:nth-of-type(1) > div > div > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
            auth_date = soup.select_one('#facturaHTML > div:nth-of-type(1) > div > div > div:nth-of-type(2) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
            issuer_ruc = soup.select_one('#facturaHTML > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(1) > dl > dd').get_text(strip=True)    
            issuer_dv = soup.select_one('#facturaHTML > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
            issuer_name = soup.select_one('#facturaHTML > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(1) > div:nth-of-type(3) > dl > dd').get_text(strip=True)
            issuer_address = soup.select_one('#facturaHTML > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(1) > dl > dd').get_text(strip=True)
            issuer_phone = soup.select_one('#facturaHTML > div:nth-of-type(2) > div:nth-of-type(1) > div > div:nth-of-type(2) > div:nth-of-type(2) > div:nth-of-type(2) > dl > dd').get_text(strip=True)
            receptor_id = None
            receptor_dv = None
            receptor_name = None
            receptor_address = None
            receptor_phone = None

        process_date = datetime.now(panama_timezone)
        user_phone_number = ''

        items = {
            'no': no,
            'date': datet,
            'time': time,
            'cufe': cufe,
            'auth_date':auth_date,
            'issuer_ruc':issuer_ruc,
            'issuer_dv':issuer_dv,
            'issuer_name':issuer_name,
            'issuer_address':issuer_address,
            'issuer_phone':issuer_phone,
            'receptor_id':receptor_id,
            'receptor_dv':receptor_dv,
            'receptor_name':receptor_name,
            'receptor_address':receptor_address,
            'receptor_phone':receptor_phone,
            'user_phone_number':user_phone_number,
            'user_telegram_id':None if origin != TELEGRAM_APP_SOURCE else source_id,
            'user_email':user_email,
            'type':'QR',
            'origin':origin,
            'user_ws': None if origin != WHATSAPP_APP_SOURCE else source_id,
            'user_id': user_db_id,
            'url': url,
            'process_date': process_date,
            'reception_date': reception_date
        }

        # Line to DB
        try:
            line_to_db(items)
        except Exception as e:
            print(f"Error inserting to DB: {e}")

        # Return items for further processing if needed
        return items

    else:
        print(f"Invalid URL: {url}")
        return None

# ✅ Para compatibilidad, mantener las funciones existentes pero optimizadas
# Si tu código llama directamente a leer_limpiar_imagen con la imagen ya cargada:
def leer_limpiar_imagen_legacy(image_array_or_cv2_image):
    """
    ✅ Wrapper para código legacy que pasa imagen ya cargada
    """
    try:
        # Si recibe imagen ya en formato OpenCV
        if isinstance(image_array_or_cv2_image, np.ndarray):
            if len(image_array_or_cv2_image.shape) == 3:  # BGR
                imagen = cv2.cvtColor(image_array_or_cv2_image, cv2.COLOR_BGR2GRAY)
            else:  # Ya en grayscale
                imagen = image_array_or_cv2_image
        else:
            # Asumir que son bytes
            imagen = cv2.imdecode(image_array_or_cv2_image, cv2.IMREAD_GRAYSCALE)
        
        # ✅ Usar el preprocessing optimizado
        equalized = cv2.equalizeHist(imagen)
        return equalized
        
    except Exception as e:
        logging.error(f"Error in legacy preprocessing: {e}")
        return image_array_or_cv2_image  # Devolver original como fallback

if __name__ == "__main__":
    # ✅ Testing básico
    print("🧪 Testing QReader optimizations...")
    
    # Inicializar modelos
    initialize_qreaders()
    
    # Mostrar métricas
    print("📊 Initial metrics:", get_detection_metrics())
    
    print("✅ QReader optimization ready!")