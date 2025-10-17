# app_image_processing.py
from ws_mensajes.app_variables import *
from app_db import *
from ws_qrdetection.app_fun_qrdetection import (
    url_to_dfs,
    imagen_a_url,
    leer_limpiar_imagen
)
from tg_fun.app_telegram_messaging import *
from ws_fun.app_wha_messaging import *
from ws_mensajes.app_mensajes_text import *
from ws_redis.app_fun_redis import *
from datetime import datetime
import fitz
import random
import requests
import logging
import pytz
import base64
from ws_rewards.app_rewards_engine import RewardsEngine

panama_timezone = pytz.timezone('America/Panama')

def validate_and_extract_qr_code(image_data, db_log):
    """Processes image data to extract a QR code URL."""
    try:
        sharpened_image = leer_limpiar_imagen(image_data)
        url, model = imagen_a_url(sharpened_image)
        db_log['qr_detection_model'] = model
        if url:
            return url, 'QR'
    except Exception as e:
        logging.error(f"Error processing image for QR code: {e}")
    return None, None

def validate_and_extract_cufe(message, db_log):
    """Validates and extracts a CUFE from a text message to form a URL."""
    cufe_match = re.search(r'([a-fA-F0-9]{64})', message)
    if cufe_match:
        cufe = cufe_match.group(1)
        # Asumimos que la URL base es la de consulta por CUFE
        url = f"https://mef.gob.pa/dgi/Consultas/FacturasPorCUFE/Informacion?cufe={cufe}"
        return url, 'CUFE'
    return None, None

async def flow_process_image(message, chat_id, message_id, source, db_log, message_type='image', email=None, user_db_id=None, header_type="public.invoice_header"):
    """
    Processes an image received via Telegram or WhatsApp.

    Args:
        message: The message object from Telegram or WhatsApp.
        chat_id: The chat ID (Telegram or WhatsApp ID).
        source: The source of the message ('telegram' or 'whatsapp').
    """

    db_log['image_processed_status'] = False
    process_type = None
    # Initialize respuesta with a default value to avoid "not set" errors
    respuesta = "Lo sentimos, hubo un problema al procesar tu imagen. Por favor, intenta de nuevo."

    # DESCARGAR ARCHIVO
    step_0 = None
    st = datetime.now(panama_timezone)
    image_data = None
    try:
        if source == WHATSAPP_APP_SOURCE:
            img_id = message["image"]["id"]
            image_data = await descargar_imagen_whatsapp(img_id)
        elif source == TELEGRAM_APP_SOURCE:
            file_id = message
            image_data = await descargar_imagen_telegram(file_id)
        db_log['t_download_image'] = datetime.now(panama_timezone) - st
        step_0 = True
    except Exception as e:
        db_log['error_download_image'] = str(e)
        respuesta = "Error al descargar la imagen. Por favor, intenta enviarla nuevamente."

    # SI ES PDF CONVERTIR A IMAGEN
    if message_type == 'document' and step_0:
        try:
            if image_data:
                pdf_document = fitz.open(stream=image_data, filetype="pdf")
                if pdf_document.page_count > 0:
                    page_to_load = pdf_document.load_page(0)
                    pix = page_to_load.get_pixmap()
                    image_data = pix.tobytes("png")
                else:
                    step_0 = False
                    db_log['pdf_processing_error'] = "PDF has no pages"
                    respuesta = "El documento PDF no contiene páginas. Por favor, envía un PDF válido."
            else:
                step_0 = False
                db_log['pdf_processing_error'] = "Image data is None, cannot process PDF"
                respuesta = "No se pudo procesar el documento. Por favor, intenta nuevamente."
        except Exception as e:
            step_0 = False
            db_log['pdf_processing_error'] = str(e)
            respuesta = "Error al procesar el documento PDF. Por favor, intenta con otro archivo."

    # LIMPIAR IMAGEN
    step_1 = None
    img = None
    if step_0:
        try:
            if image_data:
                db_log['image_data_exists'] = True
                st = datetime.now(panama_timezone)
                img = leer_limpiar_imagen(image_data)
                db_log['t_clean_image'] = datetime.now(panama_timezone) - st
                step_1 = True
            else:
                db_log['image_data_exists'] = False
                respuesta = "No se recibió ninguna imagen. Por favor, envía la imagen nuevamente."
        except Exception as e:
            db_log['clean_image_error'] = str(e)
            respuesta = "Error al procesar la imagen. Por favor, intenta con una imagen más clara."

    # DETECTOR DE QR
    step_2 = None
    url = None
    if step_1:
        try:
            st = datetime.now(panama_timezone)
            url, qr_detector_model = imagen_a_url(img)
            db_log['t_qr_detection'] = datetime.now(panama_timezone) - st
            db_log['qr_model'] = qr_detector_model
            db_log['url'] = url
            step_2 = True
        except Exception as e:
            logging.error(f"error detector qr: {e}")
            db_log['qr_detection_error'] = str(e)
            respuesta = "Tuvimos un problema al leer el QR de tu imagen. Por favor, intenta con una imagen más clara o asegúrate que el QR sea legible."

    # VALIDAR URL
    if step_2:
        try:
            if url is None:
                db_log['failed_image'] = base64.b64encode(img).decode('utf-8')
                respuesta = "No encontramos el QR esta vez 📷. Intenta con una imagen menos borrosa, ¡tú puedes! ¡Casi lo logramos! 😅 Subamos otra foto con mejor enfoque."
                step_2 = False
            elif 'Consultas/FacturasPorCUFE' in url:
                process_type = 'CUFE'
            elif any(allowed_url in url for allowed_url in allowed_urls):
                process_type = 'QR'
            else:
                db_log['failed_image'] = base64.b64encode(img).decode('utf-8')
                step_2 = False
                respuesta = "La URL del QR no es válida. Por favor, verifica que sea una factura electrónica de Panamá."
        except Exception as e:
            logging.error(f"error validando url: {e}")
            db_log['data_extraction_error'] = str(e)
            step_2 = False
            respuesta = "Hubo un error al validar la URL de la factura."

    # DELEGATE TO CORE PROCESSING LOGIC
    if url:
        respuesta = await _process_invoice_data_from_url(
            url=url,
            chat_id=chat_id,
            message_id=message_id,
            source=source,
            db_log=db_log,
            process_type=process_type,
            email=email,
            user_db_id=user_db_id,
            header_type=header_type
        )
    elif process_type == 'TEXT':
        # Si el mensaje es de tipo texto y contiene un CUFE, procesarlo directamente.
        # La URL en este caso es el propio CUFE.
        url = message  # Asumimos que el mensaje es el CUFE/URL
        respuesta = await _process_invoice_data_from_url(
            url=url,
            chat_id=chat_id,
            message_id=message_id,
            source=source,
            db_log=db_log,
            process_type='CUFE',  # Se marca como tipo CUFE
            email=email,
            user_db_id=user_db_id,
            header_type=header_type
        )
    else:
        # Si no hay URL válida de una imagen, se devuelve una respuesta genérica.
        respuesta = "No se pudo encontrar un código QR o CUFE válido. Por favor, asegúrate de que sea legible y vuelve a intentarlo."

    # Si después de todo el proceso no hay respuesta, se asigna un mensaje de error.
    if not respuesta:
        respuesta = "Hubo un problema al procesar tu solicitud. Por favor, intenta más tarde."

    return respuesta

async def _process_invoice_data_from_url(url, chat_id, message_id, source, db_log, process_type, email, user_db_id, header_type):
    step_nd = False
    step_4_qr_1 = False
    respuesta = None

    # FLUJO QR: EXTRAER DATA
    if process_type == 'QR' or process_type == 'CUFE':
        try:
            st = datetime.now(panama_timezone)
            d_header, d_payment, d_detail = await url_to_dfs(url, chat_id, source, email, user_db_id, st)
            db_log['t_data_extraction'] = datetime.now(panama_timezone) - st

            # ✅ Validación explícita: el MEF aún no tiene esta factura
            if not d_header or not isinstance(d_header, list) or not d_header[0].get('cufe'):
                db_log['data_extraction_error'] = "Factura no encontrada o sin CUFE. MEF no ha publicado aún."
                respuesta = """¡Factura recibida!
                    Tu factura ya está en manos de la magia LÜM ✨
                    Ahora solo debemos esperar a que se suba al sistema donde capturamos esta info.
                    Apenas tengamos acceso a ese dato, te avisaremos si pudo ser procesada correctamente."""
                step_nd = True
            else:
                step_4_qr_1 = True
        except Exception as e:
            logging.warning(f"extraer data: {e}")
            step_nd = True
            db_log['data_extraction_error'] = str(e)
            respuesta = "Error al extraer datos de la factura. Por favor, intenta nuevamente más tarde."

    # FLUJO QR: VALIDAR SI CUFE EXISTE
    step_4_qr_2 = None
    if step_4_qr_1:
        try:
            if d_header:
                st = datetime.now(panama_timezone)
                validation = await dict_to_check_if_exists(d_header[0]['cufe'])
                db_log['t_data_validation'] = datetime.now(panama_timezone) - st
                if validation:
                    db_log['cufe_ya_existe'] = True
                    respuesta = '¡Estos Lümis ya están en tu cuenta! 🔍 ¿Probamos con otra factura para ganar más Lümis? 💰'
                else:
                    db_log['cufe_ya_existe'] = False
                    step_4_qr_2 = True
            else:
                step_nd = True
                respuesta = "No pudimos validar la factura. Por favor, intenta nuevamente."
        except Exception as e:
            step_nd = True
            db_log['data_validation_error'] = str(e)
            respuesta = "Error al validar el CUFE de la factura. Por favor, intenta nuevamente."

    # FLUJO QR: CARGAR DATOS EN BBDD
    step_5 = None
    if step_4_qr_2:
        try:
            st = datetime.now(panama_timezone)
            await line_to_db(d_header, header_type)
            await line_to_db(d_payment, 'public.invoice_payment')
            await line_to_db(d_detail, 'public.invoice_detail')
            db_log['t_data_upload'] = datetime.now(panama_timezone) - st
            step_5 = True
        except Exception as e:
            step_nd = True
            db_log['data_upload_error'] = str(e)
            respuesta = "Error al guardar los datos de la factura. Por favor, intenta nuevamente más tarde."

    # IMAGEN PROCESADA
    if step_5:
        db_log['image_processed_status'] = True

        cierre = random.choice(mensajes_cierre)

        try:
            respuesta = f'Tu factura de {d_header[0]["issuer_name"]} por un valor de ${d_header[0]["tot_amount"]} fue procesada exitosamente. {cierre}'
        except (IndexError, TypeError, AttributeError, KeyError) as e:
            logging.error(f"Error al formatear respuesta: {e}")
            respuesta = f'Tu factura fue procesada exitosamente. {cierre}'

        # Procesar acumulación de puntos usando el motor de recompensas
        if user_db_id:
            try:
                # Obtener la configuración de la promoción de acumulación con ID 0
                columns = "id, name, points"
                current_time = datetime.now(panama_timezone).astimezone(pytz.utc).replace(tzinfo=None)

                promo_result = await fun_scan_table(
                    "rewards.dim_accumulations",
                    columns,
                    "WHERE id = $1 AND valid_from <= $2 AND valid_to >= $2",
                    (0, current_time)
                )

                if promo_result and promo_result[0]:
                    promo = promo_result[0]
                    promo_id = promo[0]  # ID de la promoción
                    promo_name = promo[1]  # accum_type column
                    promo_points = promo[2]  # points column

                    # Registrar la acumulación directamente
                    accumulation_data = {
                        'user_id': user_db_id,
                        'accum_type': promo_name,
                        'accum_key': d_header[0]['cufe'] if d_header and len(d_header) > 0 else f'invoice-{chat_id}-{datetime.now().timestamp()}',
                        'dtype': 'points',
                        'quantity': promo_points,
                        'date': datetime.now(panama_timezone),
                        'accum_id': promo_id,
                    }

                    await line_to_db([accumulation_data], 'rewards.fact_accumulations')

                    # El balance se actualiza automáticamente por trigger
                    new_balance = await RewardsEngine.get_user_balance(user_db_id)

                    respuesta += f"\n\n¡Has ganado {promo_points} Lümis! 🌟 Tu nuevo balance es {new_balance} Lümis."
                else:
                    logging.warning(f"Promoción de acumulación ID 0 no encontrada o no activa")

            except Exception as e:
                logging.error(f'Error procesando acumulación de puntos: {str(e)}')
                # No interrumpir el flujo principal si falla la acumulación

    # FLUJO PENDING
    if step_nd:
        db_log['image_processed_status'] = False
        try:
            st = datetime.now(panama_timezone)
            d_mefpending_indirect = [{
                'url': url,
                'chat_id': chat_id if source == TELEGRAM_APP_SOURCE else None,
                'date': datetime.now(panama_timezone),
                'message_id': message_id,
                'type': 'QR' if process_type == 'QR' else 'CUFE' if process_type == 'CUFE' else None,
                'user_email': email,
                'user_id': user_db_id,
                'error': None,
                'origin': source,
                'ws_id': chat_id if source == WHATSAPP_APP_SOURCE else None
            }]
            await line_to_db(d_mefpending_indirect, 'public.mef_pending')
            db_log['t_upload_mefpending'] = datetime.now(panama_timezone) - st
            if not respuesta:
                 respuesta = "Hemos recibido tu factura. Pronto te confirmaremos si pudo ser procesada."
        except Exception as e:
            respuesta = "¡Hubo un destello inesperado en el universo! 🌌 Nuestros magos lo revisarán. ¿Mientras tanto subimos otra factura? o puedes intentarlo más tarde 🕔"
            logging.error(f'Error copiando a mef_pending: {str(e)}')

    return respuesta
