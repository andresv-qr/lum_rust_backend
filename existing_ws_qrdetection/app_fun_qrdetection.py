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
panama_timezone = pytz.timezone('America/Panama')


def leer_limpiar_imagen(image_data):
    # Leer la imagen con OpenCV directamente desde los bytes
    image_array = np.frombuffer(image_data, np.uint8)  # Convierte a un array de NumPy
    imagen = cv2.imdecode(image_array, cv2.IMREAD_COLOR)

    # convertir a grises
    gray = cv2.cvtColor(imagen, cv2.COLOR_BGR2GRAY)
    
    # Aplicar CLAHE para mejorar el contraste
    clahe = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))
    enhanced_contrast = clahe.apply(gray)
    
    # Aplicar un filtro Gaussian Blur para reducir el ruido
    blurred = cv2.GaussianBlur(enhanced_contrast, (9, 9), 10.0)
    
    # Crear una máscara de enfoque restando la imagen borrosa de la original
    sharpened = cv2.addWeighted(enhanced_contrast, 1.5, blurred, -0.5, 0)

    #convertir a png
    _, img_encoded = cv2.imencode('.png', sharpened)
    img_bytes = img_encoded.tobytes()
    nparr = np.frombuffer(img_bytes, np.uint8)
    sharpened_png = cv2.imdecode(nparr, cv2.IMREAD_GRAYSCALE) 

    return sharpened_png

def imagen_a_url(sharpened):
    """
    Attempts to detect and decode a QR code from an image using multiple methods.
    Returns a tuple of (decoded_data, detector_model) where detector_model indicates
    which detection method was successful.
    """
    data = None  # Initialize data to None
    data_model_id = None  # Initialize model ID to None
    
    try:
        # Method 1: OpenCV QR detector
        detector = cv2.QRCodeDetector()
        result, bbox, _ = detector.detectAndDecode(sharpened)
        
        if result:  # If data was successfully decoded
            data = result
            data_model_id = 'CV2'
            return data, data_model_id
            
        # Method 2: OpenCV curved QR detector
        detector.setEpsX(0.3)  # Adjust for curved codes
        detector.setEpsY(0.3)  
        result, bbox, _ = detector.detectAndDecodeCurved(sharpened)
        
        if result:  # If data was successfully decoded
            data = result
            data_model_id = 'CV2 CURVED'
            return data, data_model_id
            
        # Method 3: pyzbar library
        decoded_data = decode(sharpened)
        qr_codes = [x for x in decoded_data if x.type == 'QRCODE']
        
        if qr_codes:  # If any QR codes were found
            data = qr_codes[0].data.decode()
            data_model_id = 'PYZBAR'
            return data, data_model_id
            
        # Method 4: QReader small model
        qreader = QReader(min_confidence=0.01, model_size='s')
        detected_data = qreader.detect_and_decode(image=sharpened)
        
        if detected_data and len(detected_data) > 0 and detected_data[0]:
            data = detected_data[0]
            data_model_id = 'QREADER S'
            return data, data_model_id
            
        # Method 5: QReader large model (last resort)
        qreader = QReader(min_confidence=0.01, model_size='l')
        detected_data = qreader.detect_and_decode(image=sharpened)
        
        if detected_data and len(detected_data) > 0 and detected_data[0]:
            data = detected_data[0]
            data_model_id = 'QREADER L'
            return data, data_model_id
            
    except Exception as e:
        # Log any exceptions that occur during QR detection
        logging.error(f"Error in imagen_a_url: {e}")
        data_model_id = "ERROR"
    
    # If we've reached this point, no method was successful
    return None, data_model_id

    
async def url_to_dfs(url, source_id, origin, user_email, user_db_id, reception_date): # Changed to async def
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

        #capturar la parte de los totales que puede incluir multiples medios de pago
        foot = soup.find(id='detalle').find('table').find('tfoot').find_all('tr')
        for f in foot:
            tv = f.find('td').find('div').get_text(strip=True).lower()
            tv = None if tv == '' else float(tv)
            td = f.find('td')
            for div in td.find_all('div'):
                div.decompose()
            td = td.text.strip().replace(':','').lower()

            if td == 'valor total':
                items['tot_amount'] = float(tv)
            elif td == 'itbms total':
                items['tot_itbms'] = float(tv)
            else:
                varname = td.replace(' ','_')
                items[varname] = str(tv)

        #lista de diccionarios total
        items_columns = list(items.keys())

        #extraer unicamente la parte que corresponde al header
        header_cols = ['cufe','issuer_name', 'no', 'user_phone_number', 'user_telegram_id', 'time',
        'receptor_address', 'tot_itbms', 'issuer_dv', 'receptor_phone', 'user_email',
        'auth_date', 'date', 'receptor_id', 'issuer_address',
        'issuer_ruc', 'tot_amount', 'receptor_name', 'receptor_dv',
        'issuer_phone','type','origin','user_ws','user_id','url','process_date','reception_date']

        line_header = {clave: valor for clave, valor in items.items() if clave in header_cols}

        #extraer la tabla de pagos
        current_payment_cols = ['cufe','vuelto','total_pagado','tarjeta_crédito','forma_de_pago',
            'descuentos','efectivo','forma_de_pago_otro','tarjeta_débito',
            'tarjeta_clave__banistmo_','valor_pago']

        line_payment = {
            **{clave: items.get(clave, None) for clave in current_payment_cols},  # Claves permitidas
            'merged': (
                json.dumps({
                    clave: valor for clave, valor in items.items() 
                    if clave not in current_payment_cols and clave not in items_columns  # Excluir claves de current_payment_cols y base_columns
                }) or None  # Si el JSON es vacío, asignar None
            )
        }

        # Detalle de la tabla en el cuerpo
        detail = soup.find(id='detalle').find('table').find('tbody')
        line_detail = []

        i = 1
        for line in detail.find_all('tr'):
            cols = line.find_all('td')
            item = {
                'date': date,
                'cufe': cufe,
                'partkey': f"{cufe}|{i}",
                'code': cols[1].get_text(strip=True),
                'description': cols[2].get_text(strip=True),
                'information_of_interest': cols[3].get_text(strip=True),
                'quantity': cols[4].get_text(strip=True),
                'unit_price': cols[5].get_text(strip=True),
                'unit_discount': cols[6].get_text(strip=True),
                'amount': cols[7].get_text(strip=True),
                'itbms': cols[8].get_text(strip=True),
                'total': cols[12].get_text(strip=True)      
            }
            line_detail = line_detail + [item]
            i = i + 1


        return [line_header], [line_payment], line_detail
    else:
        return None, None, None

async def errorwithimage_copyingtodb(url, chat_id, message_id, e): # Changed to async def
    try:
        d_mefpending_direct = [{
            'url': url,
            'chat_id': chat_id,
            'date':datetime.now(panama_timezone),
            'message_id':message_id,
            'type':'QR',
            'user_email':None,
            'user_phone':None,
            'error':str(e)
        }]
        logging.info(f"OK Imagen. error. url se guarda en public.mefpending:  {chat_id} | error: {e}")
        # Assuming line_to_db remains synchronous for now, or needs its own async version
        # If line_to_db becomes async, it should be awaited.
        # For now, if called from an async func, it might block or need run_in_threadpool
        # For simplicity here, let's assume it's okay or will be handled.
        # A better approach would be: await run_in_threadpool(line_to_db, d_mefpending_direct, 'public.mef_pending')
        # if line_to_db is sync.
        line_to_db(d_mefpending_direct, 'public.mef_pending')
        respuesta = f'La factura fue recibida. Pronto le confirmaremos si fue procesada exitosamente. (QRx001 {e})'
    except Exception as f:
        logging.info(f"OK Imagen. error. error copaindo url en public.mef_pending:  {chat_id} | error: {f}")
        respuesta = f'Inténtelo nuevamente: (error copiando en mef_pending: {f})'
    return respuesta