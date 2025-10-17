# Main file for the FastAPI application

import os
import logging
import asyncio
import time
import random
import re
import secrets
from datetime import datetime, timedelta
from typing import Optional, List, Any, Dict
from collections import defaultdict

from fastapi import FastAPI, UploadFile, File, HTTPException, Depends, status, Request, Query
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field, validator, EmailStr, HttpUrl
import re
from fastapi.middleware.cors import CORSMiddleware
import jwt
from slowapi import Limiter, _rate_limit_exceeded_handler
from slowapi.util import get_remote_address
from slowapi.errors import RateLimitExceeded
import json

# Imports de nuestros módulos locales
from app_db import (
    get_password_hash, 
    verify_password, 
    fun_execute_update,
    fun_scan_table_any,
    fun_register_user,
    init_db_pool,
    close_db_pool,
    DatabaseOperations
)
from ws_ocr.app_flow_ocr import flow_process_ocr_invoice
from ws_redis.app_fun_redis import RedisManager
from flows.flow_email.app_fun_email import send_verification_email as send_verification_email_flow
from ws_mensajes.app_variables import allowed_urls
from ws_qrdetection.app_flow_image import _process_invoice_data_from_url

# Visual Dashboard imports
from ws_rewards.app_fun_rewards_ofertas_visual import crear_dashboard_ofertas
from celery import Celery
import uuid
import pytz

# Configuración de timezone
panama_timezone = pytz.timezone('America/Panama')

# Configuración de logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Configuración JWT
JWT_SECRET = os.getenv("JWT_SECRET", "your-super-secret-jwt-key-change-in-production")
JWT_ALGORITHM = "HS256"
JWT_EXPIRATION_HOURS = 24

# Rate limiter
limiter = Limiter(key_func=get_remote_address)

# Celery configuration for visual dashboard tasks
celery_app = Celery(
    "qreader_dashboard",
    broker=os.getenv("REDIS_URL", "redis://localhost:6379/1"),
    backend=os.getenv("REDIS_URL", "redis://localhost:6379/1")
)

# Configure Celery to avoid serialization issues
celery_app.conf.update(
    task_serializer='json',
    accept_content=['json'],
    result_serializer='json',
    timezone='UTC',
    enable_utc=True,
    result_expires=3600,  # Results expire after 1 hour
    task_track_started=True,
    task_always_eager=False,
)

# Task storage for tracking dashboard generation
task_storage = {}

# FastAPI app
app = FastAPI(title="QReader API", version="1.0.0")

# Static files for dashboard images
static_dir = "/home/client_1099_1/scripts/qreader_server/static"
os.makedirs(static_dir, exist_ok=True)
app.mount("/static", StaticFiles(directory=static_dir), name="static")

# Rate limiting error handler
app.state.limiter = limiter
app.add_exception_handler(RateLimitExceeded, _rate_limit_exceeded_handler)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # En producción, especifica los dominios permitidos
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Security
security = HTTPBearer()

# --- Email validation regex ---
EMAIL_REGEX = re.compile(r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$')

# --- Pydantic Models for Data Validation ---

class EmailCheckRequest(BaseModel):
    email: str

class EmailCheckResponse(BaseModel):
    exists: bool
    message: str

class UserRegistrationRequest(BaseModel):
    email: EmailStr
    password: str = Field(..., min_length=8)
    name: str = Field(..., min_length=1, max_length=100)
    
    @validator('password')
    def validate_password(cls, v):
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters long')
        if not any(c.isupper() for c in v):
            raise ValueError('Password must contain at least one uppercase letter')
        if not any(c.islower() for c in v):
            raise ValueError('Password must contain at least one lowercase letter')
        if not any(c.isdigit() for c in v):
            raise ValueError('Password must contain at least one digit')
        return v

class UserLoginRequest(BaseModel):
    email: EmailStr
    password: str

class Token(BaseModel):
    access_token: str
    token_type: str
    user_id: int
    email: str

class TokenData(BaseModel):
    user_id: Optional[int] = None
    email: Optional[str] = None

class UserLoginResponse(BaseModel):
    success: bool
    message: str
    # In a real app, you'd return a JWT token here
    # token: str | None = None 

# Model for the response of the OCR upload
class OCRResponse(BaseModel):
    success: bool
    message: str
    cufe: str | None = None
    error_details: str | None = None

# --- New models for existing users without password ---

class UserStatusRequest(BaseModel):
    email: str

class UserStatusResponse(BaseModel):
    exists: bool
    has_password: bool
    source: str | None = None  # whatsapp, telegram, email
    message: str

class SendVerificationRequest(BaseModel):
    email: str

class SendVerificationResponse(BaseModel):
    success: bool
    message: str
    method: str  # email, whatsapp

class VerifyAccountRequest(BaseModel):
    email: str
    verification_code: str

class VerifyAccountResponse(BaseModel):
    success: bool
    message: str

class SetPasswordRequest(BaseModel):
    email: str
    verification_code: str
    new_password: str = Field(..., min_length=8)
    
    @validator('new_password')
    def validate_password(cls, v):
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters long')
        if not any(c.isupper() for c in v):
            raise ValueError('Password must contain at least one uppercase letter')
        if not any(c.islower() for c in v):
            raise ValueError('Password must contain at least one lowercase letter')
        if not any(c.isdigit() for c in v):
            raise ValueError('Password must contain at least one digit')
        return v

class ResetPasswordRequest(BaseModel):
    email: str
    verification_code: str
    new_password: str = Field(..., min_length=8)
    
    @validator('new_password')
    def validate_password(cls, v):
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters long')
        if not any(c.isupper() for c in v):
            raise ValueError('Password must contain at least one uppercase letter')
        if not any(c.islower() for c in v):
            raise ValueError('Password must contain at least one lowercase letter')
        if not any(c.isdigit() for c in v):
            raise ValueError('Password must contain at least one digit')
        return v

class TokenResponse(BaseModel):
    access_token: str
    token_type: str = "bearer"
    expires_in: int

class MessageResponse(BaseModel):
    message: str

class ProcessUrlRequest(BaseModel):
    url: HttpUrl
    source: Optional[str] = "APP"  # Origen de la petición, por defecto 'APP'

class InvoiceDetail(BaseModel):
    cufe: Optional[str] = None
    quantity: Optional[float] = None
    code: Optional[str] = None
    date: Optional[datetime] = None
    total: Optional[float] = None
    unit_price: Optional[float] = None
    amount: Optional[float] = None
    unit_discount: Optional[str] = None
    description: Optional[str] = None
    user_id: Optional[int] = None

class InvoiceHeader(BaseModel):
    date: Optional[datetime] = None
    tot_itbms: Optional[float] = None
    issuer_name: Optional[str] = None
    no: Optional[str] = None
    tot_amount: Optional[float] = None
    url: Optional[str] = None
    process_date: Optional[datetime] = None
    reception_date: Optional[datetime] = None
    type: Optional[str] = None
    cufe: Optional[str] = None

class InvoiceDetailResponse(BaseModel):
    invoices: List[InvoiceDetail]
    count: int
    query_info: dict

class InvoiceHeaderResponse(BaseModel):
    invoices: List[InvoiceHeader]
    count: int
    query_info: dict

class ProcessUrlResponse(BaseModel):
    detail: str

# --- Visual Dashboard Models ---
class OfferData(BaseModel):
    category: Optional[str] = None
    offer_title: Optional[str] = None
    offer_description: Optional[str] = None
    original_price: Optional[float] = None
    discounted_price: Optional[float] = None
    discount_percentage: Optional[float] = None
    store_name: Optional[str] = None
    store_logo: Optional[str] = None
    offer_url: Optional[str] = None
    valid_until: Optional[datetime] = None

class DashboardRequest(BaseModel):
    offers: List[OfferData]
    category: str
    whatsapp_number: str
    price_range: Optional[str] = None

class TaskStatus(BaseModel):
    task_id: str
    status: str  # pending, processing, completed, failed
    progress: Optional[int] = None
    result: Optional[Dict] = None
    error: Optional[str] = None

class DashboardResponse(BaseModel):
    task_id: str
    status: str
    message: str

class UserRegistrationRequest(BaseModel):
    email: EmailStr
    password: str = Field(..., min_length=8, description="Password must be at least 8 characters long, contain an uppercase letter, a lowercase letter, and a number.")
    name: str | None = Field(None, max_length=50)

    @validator('password')
    def password_strength(cls, v):
        if not re.search(r'[A-Z]', v):
            raise ValueError('Password must contain at least one uppercase letter')
        if not re.search(r'[a-z]', v):
            raise ValueError('Password must contain at least one lowercase letter')
        if not re.search(r'[0-9]', v):
            raise ValueError('Password must contain at least one number')
        return v

    @validator('name')
    def name_santization(cls, v):
        if v and not re.match(r"^[a-zA-Z0-9\s\-\'\.]*$", v):
            raise ValueError('Name contains invalid characters.')
        return v


class RegistrationResponse(BaseModel):
    success: bool = True
    message: str = "User registered successfully"
    user_id: int

class UserProfile(BaseModel):
    id: int
    email: str
    name: str
    created_at: Optional[datetime] = None
    registration_source: Optional[str] = None

class InvoiceData(BaseModel):
    id: int
    user_id: int
    filename: str
    ocr_text: Optional[str] = None
    uploaded_at: datetime

# Funciones de utilidad para JWT
def create_jwt_token(user_id: int, email: str) -> str:
    payload = {
        "user_id": user_id,
        "email": email,
        "exp": datetime.utcnow() + timedelta(hours=JWT_EXPIRATION_HOURS),
        "iat": datetime.utcnow()
    }
    return jwt.encode(payload, JWT_SECRET, algorithm=JWT_ALGORITHM)

async def verify_jwt_token(credentials: HTTPAuthorizationCredentials = Depends(security)) -> dict:
    try:
        payload = jwt.decode(credentials.credentials, JWT_SECRET, algorithms=[JWT_ALGORITHM])
        return payload
    except jwt.ExpiredSignatureError:
        raise HTTPException(status_code=401, detail="Token has expired")
    except jwt.InvalidTokenError:
        raise HTTPException(status_code=401, detail="Invalid token")

# Funciones para códigos de verificación con Redis
def generate_verification_code() -> str:
    """Genera un código de verificación de 6 dígitos"""
    return str(secrets.randbelow(900000) + 100000)

async def store_verification_code(email: str, code: str, purpose: str = "general") -> None:
    """Almacena un código de verificación en Redis con expiración automática"""
    try:
        redis_client = await RedisManager.get_client(1)  # Usar DB 1 para códigos de verificación
        key = f"verification:{email}:{purpose}"
        data = {
            "code": code,
            "purpose": purpose,
            "attempts": 0,
            "created_at": datetime.utcnow().isoformat()
        }
        # Guardar con expiración de 15 minutos (900 segundos)
        await redis_client.setex(key, 900, json.dumps(data))
        logger.info(f"Verification code stored in Redis for {email} with purpose {purpose}")
    except Exception as e:
        logger.error(f"Error storing verification code in Redis: {e}")
        raise HTTPException(status_code=500, detail="Error al generar código de verificación")

async def verify_verification_code(email: str, code: str, purpose: str = "general") -> bool:
    """Verifica un código de verificación desde Redis"""
    try:
        redis_client = await RedisManager.get_client(1)
        key = f"verification:{email}:{purpose}"
        
        # Obtener datos
        data_str = await redis_client.get(key)
        if not data_str:
            logger.warning(f"No verification code found for {email} with purpose {purpose}")
            return False
        
        data = json.loads(data_str)
        
        # Incrementar intentos
        data["attempts"] += 1
        
        # Máximo 3 intentos
        if data["attempts"] > 3:
            await redis_client.delete(key)
            logger.warning(f"Max attempts exceeded for {email}")
            return False
        
        # Obtener TTL actual para mantenerlo
        ttl = await redis_client.ttl(key)
        if ttl > 0:
            # Actualizar con el mismo TTL
            await redis_client.setex(key, ttl, json.dumps(data))
        
        # Verificar código
        if data["code"] == code:
            await redis_client.delete(key)  # Eliminar después de uso exitoso
            logger.info(f"Verification code validated successfully for {email}")
            return True
        
        logger.warning(f"Invalid verification code attempt for {email}")
        return False
        
    except Exception as e:
        logger.error(f"Error verifying code from Redis: {e}")
        return False

async def get_verification_code_info(email: str, purpose: str = "general") -> Optional[Dict]:
    """Obtiene información sobre un código de verificación (para debugging)"""
    try:
        redis_client = await RedisManager.get_client(1)
        key = f"verification:{email}:{purpose}"
        data_str = await redis_client.get(key)
        
        if not data_str:
            return None
            
        data = json.loads(data_str)
        ttl = await redis_client.ttl(key)
        data["ttl_seconds"] = ttl
        
        return data
    except Exception as e:
        logger.error(f"Error getting verification code info: {e}")
        return None

async def send_email_verification_code(email: str, code: str, purpose: str) -> bool:
    """Envía código de verificación por email usando el sistema existente"""
    try:
        # Por defecto, la aplicación correrá en modo producción.
        # Para activar el modo desarrollo, se debe establecer la variable de entorno DEVELOPMENT_MODE="true"
        development_mode = os.getenv("DEVELOPMENT_MODE", "false").lower() == "true"
        
        if development_mode:
            # En modo desarrollo, solo loggear el código
            logger.info(f"🔧 DEVELOPMENT MODE - Email would be sent to: {email}")
            logger.info(f"📧 Subject: {'Establece tu contraseña - 2Factu' if purpose == 'set_password' else 'Restablecer contraseña - 2Factu'}")
            logger.info(f"🔑 Verification Code: {code}")
            logger.info(f"⏰ Purpose: {purpose}")
            logger.info(f"✅ Email simulation successful")
            return True
        
        # En producción, usar el sistema real de emails
        if purpose == "set_password":
            subject = "Establece tu contraseña - Lüm"
            message = f"""
                🎉 ¡Bienvenid@ a Lüm! 🎉

                Tu cuenta está a punto de brillar ✨
                Pero antes... necesitamos que crees tu contraseña secreta de acceso 🧠🔒

                🪄 Tu código de activación es: {code}
                (Sí, es tu pase mágico. Caduca en 15 minutos ⏳)

                Una vez entres, prepárate para:
                💸 Ganar premios por escanear tus facturas
                📲 Llevar el control de tus compras como un pro
                🎁 Y desbloquear beneficios que hacen lüminar tu bolsillo

                ¿No estabas esperando esto?
                ¡Ups! Alguien se adelantó.
                Contáctanos y lo resolvemos enseguida.

                Con energía brillante,
                El equipo Lüm ⚡💜
            """
        else:  # reset_password
            subject = "Restablecer contraseña - Lüm"
            message = f"""
                🌟 ¡Alerta Lüm-inosa! 🌟

                Sabemos que tu memoria es poderosa, pero por si acaso...
                ¡Aquí está tu código mágico para acceder nuevamente a tu cuenta!

                🔐 Tu código secreto es: {code}
                (Sí, es solo para tus ojos. Caduca en 15 minutos como cenicienta 🕒✨)

                Mientras tanto, recuerda:
                ✨ Cada factura escaneada es un paso más cerca de recompensas brillantes.
                📲 Menos papeles, más premios.
                🎁 Y lo mejor aún está por venir…

                ¿Tú no pediste esto?
                No pasa nada. Alguien probablemente te envidia. 😎
                Contacta a nuestro equipo de soporte y lo arreglamos rapidito.

                Con Lüm,
                Escanea, gana, sonríe. 💜
            """
        
        # Usar la función de envío de emails mejorada
        send_success = await asyncio.get_event_loop().run_in_executor(
            None,
            send_verification_email_flow,
            email,
            code
        )

        if not send_success:
            # Si la función de envío devuelve False, se lanza un error
            raise smtplib.SMTPException("Failed to send email via send_verification_email_flow")

        logger.info(f"📧 Production email dispatch attempted to {email}")
        return send_success
        
    except Exception as e:
        logger.error(f"Error sending verification email: {e}")
        
        # En caso de error, si estamos en desarrollo, simular éxito
        development_mode = os.getenv("DEVELOPMENT_MODE", "true").lower() == "true"
        if development_mode:
            logger.warning(f"📧 Email sending failed, but continuing in development mode")
            logger.info(f"🔑 DEVELOPMENT CODE for {email}: {code}")
            return True
        
        return False

# Funciones auxiliares para protección contra timing attacks
async def get_user_by_email_with_timing_protection(email: str) -> Optional[dict]:
    """
    Obtiene usuario con protección contra timing attacks.
    Siempre toma aproximadamente el mismo tiempo sin importar si el usuario existe.
    """
    start_time = time.time()
    
    # Usar la función existente de DatabaseOperations
    user_record = await DatabaseOperations.get_user_by_email(email)
    user = dict(user_record) if user_record else None
    
    # Agregar delay constante para prevenir timing attacks
    elapsed = time.time() - start_time
    min_time = 0.1  # 100ms mínimo
    if elapsed < min_time:
        await asyncio.sleep(min_time - elapsed)
    
    return user

# --- Dependency for Authentication ---
async def get_current_user(credentials: HTTPAuthorizationCredentials = Depends(security)) -> dict:
    """
    Decodes JWT token to get user data. Implements a cache layer with Redis.
    """
    token = credentials.credentials
    API_TOKEN_DB = 1 # Using DB 1 for API tokens
    redis_manager = RedisManager()

    # 1. Check cache first
    try:
        redis_client = await redis_manager.get_client(API_TOKEN_DB)
        cached_data_bytes = await redis_client.get(f"token:{token}")
        if cached_data_bytes:
            return json.loads(cached_data_bytes)
    except Exception as e:
        logging.warning(f"Redis cache read failed for get_current_user: {e}")

    # 2. If not in cache or Redis fails, decode token
    try:
        payload = jwt.decode(token, JWT_SECRET, algorithms=[JWT_ALGORITHM])
        user_id: int = payload.get("user_id")
        email: str = payload.get("email")
        if email is None or user_id is None:
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token payload")
        
        user_data = {"user_id": user_id, "email": email}

        # 3. Try to store in cache with expiration matching the token's lifetime
        try:
            token_data = jwt.decode(token, JWT_SECRET, algorithms=[JWT_ALGORITHM], options={"verify_signature": False})
            expiration_time = token_data.get("exp", int(time.time()) + JWT_EXPIRATION_HOURS * 3600)
            ttl = expiration_time - int(time.time())
            if ttl > 0:
                await redis_client.set(f"token:{token}", json.dumps(user_data), ex=ttl)
        except Exception as e:
            logging.warning(f"Redis cache write failed for get_current_user: {e}")

        return user_data
    except jwt.ExpiredSignatureError:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Token has expired")
    except jwt.InvalidTokenError:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Could not validate credentials", headers={"WWW-Authenticate": "Bearer"})

# --- FastAPI Events ---
@app.on_event("startup")
async def startup_event():
    """Inicializa el pool de conexiones de base de datos y verifica Redis"""
    await init_db_pool()
    
    # Verificar conexión con Redis
    try:
        redis_client = await RedisManager.get_client(1)
        await redis_client.ping()
        logger.info("✅ Redis connection verified")
    except Exception as e:
        logger.error(f"❌ Redis connection failed: {e}")
        logger.warning("⚠️  API will run with limited functionality (no verification codes)")
    
    logger.info("🚀 QReader API started successfully")

@app.on_event("shutdown")
async def shutdown_event():
    """Cierra el pool de conexiones al cerrar la app"""
    await close_db_pool()
    try:
        await RedisManager.close_all()  # Cerrar todas las conexiones Redis
    except:
        pass
    logger.info("🛑 QReader API shut down gracefully")

# --- API Endpoints ---

@app.get("/")
async def read_root():
    """A simple endpoint to confirm the API is running."""
    return {"status": "ok", "message": "Welcome to the 2Factu API"}

@app.post("/users/check-email", response_model=EmailCheckResponse)
@limiter.limit("30/hour")  # Máximo 30 verificaciones por hora por IP
async def check_email_availability(request: Request, email_request: EmailCheckRequest):
    """
    Checks if an email is already registered in the database.
    SOLO para uso durante registro, no durante login.
    """
    try:
        email = email_request.email.lower().strip()
        
        # Validación estricta del formato
        if not EMAIL_REGEX.match(email):
            return EmailCheckResponse(
                exists=False,
                message="Formato de email inválido"
            )
        
        # Log de actividad sospechosa
        if len(email) > 100:
            logging.warning(f"Suspicious email check from {request.client.host}: {email[:50]}...")

        logging.info(f"API: Checking availability for email: {email}")

        # Use the existing DatabaseOperations.scan_table to check for the email.
        # This is now secure against SQL injection.
        existing_user = await DatabaseOperations.scan_table(
            table='public.dim_users',
            cols='id',
            comp='WHERE LOWER(email) = $1',
            params=(email,)
        )

        if existing_user:
            return EmailCheckResponse(
                exists=True,
                message="El correo electrónico ya está registrado."
            )
        else:
            return EmailCheckResponse(
                exists=False,
                message="El correo electrónico está disponible."
            )

    except Exception as e:
        logging.error(f"API: Error in /check-email endpoint: {e}", exc_info=True)
        # Return a generic server error to avoid leaking implementation details
        raise HTTPException(status_code=500, detail="An internal server error occurred.")


@app.post("/users/register", response_model=TokenResponse)
@limiter.limit("5/minute")  # Límite de 5 registros por minuto por IP
async def register_user(user_data: UserRegistrationRequest, request: Request):
    """
    Registers a new user with email and password.
    """
    try:
        email = user_data.email.lower().strip()
        password = user_data.password

        # Validaciones básicas
        if not EMAIL_REGEX.match(email):
            raise HTTPException(status_code=400, detail="Formato de email inválido.")
        
        if len(password) < 8:
            raise HTTPException(status_code=400, detail="La contraseña debe tener al menos 8 caracteres.")

        # Check if user already exists
        existing_user = await DatabaseOperations.get_user_by_email(email)
        if existing_user:
            raise HTTPException(status_code=409, detail="Email ya registrado.")

        # Register the new user
        # Note: The source is hardcoded to EMAIL_APP_SOURCE for app-based registration
        from ws_mensajes.app_variables import EMAIL_APP_SOURCE
        result_message = await DatabaseOperations.register_user(
            user_id_val=email, # For email source, the ID is the email itself
            source_app=EMAIL_APP_SOURCE,
            email=email,
            password=password
        )

        if "successfully" in result_message.lower():
            # Retrieve the new user to get their ID
            new_user = await DatabaseOperations.get_user_by_email(email)
            return TokenResponse(
                access_token=create_jwt_token(new_user['id'], new_user['email']),
                token_type="bearer",
                expires_in=JWT_EXPIRATION_HOURS * 3600
            )
        else:
            raise HTTPException(status_code=500, detail=result_message)

    except HTTPException as http_exc:
        raise http_exc
    except Exception as e:
        logging.error(f"API: Error in /register endpoint: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail="Error interno del servidor durante el registro.")

@app.post("/v1/auth/login", response_model=TokenResponse)
@limiter.limit("10/minute")  # Límite de 10 intentos de login por minuto por IP
async def login_user(user_data: UserLoginRequest, request: Request):
    """
    Login unificado con mejores prácticas de seguridad.
    Valida email y password en una sola operación sin revelar información.
    """
    start_time = time.time()
    
    try:
        email = user_data.email.lower().strip()
        password = user_data.password
        
        logging.info(f"API: Login attempt for email: {email} from IP: {request.client.host}")

        # Buscar usuario por email
        user = await DatabaseOperations.get_user_by_email(email)
        
        # IMPORTANTE: Siempre verificar password incluso si user es None
        # Esto previene timing attacks
        if user and user.get("password"):
            password_valid = verify_password(password, user["password"])
        else:
            # Hacer una verificación falsa para mantener el timing consistente
            verify_password(password, "$2b$12$fFg/MJoU4/FGtT3zYXKLCeRu3Z2m5ojVnGV7Ix0duE3xMT0ViXRyq")
            password_valid = False
        
        if not user or not password_valid:
            # Log del intento fallido
            logging.warning(f"Failed login attempt for email: {email} from IP: {request.client.host}")
            
            # MENSAJE GENÉRICO - No revelar si fue email o password
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED,
                detail="Credenciales inválidas",  # Nunca especificar qué falló
                headers={"WWW-Authenticate": "Bearer"},
            )

        # Login exitoso - generar token JWT
        access_token = create_jwt_token(user["id"], user["email"])
        
        logging.info(f"Successful login for user {user['id']} ({email})")
        
        # Asegurar timing consistente (prevenir timing attacks)
        elapsed = time.time() - start_time
        min_time = 0.5  # 500ms mínimo
        if elapsed < min_time:
            await asyncio.sleep(min_time - elapsed + random.uniform(0, 0.1))

        return TokenResponse(
            access_token=access_token,
            token_type="bearer",
            expires_in=JWT_EXPIRATION_HOURS * 3600
        )

    except HTTPException:
        # Para errores de autenticación, también mantener timing consistente
        elapsed = time.time() - start_time
        min_time = 0.5
        if elapsed < min_time:
            await asyncio.sleep(min_time - elapsed + random.uniform(0, 0.1))
        raise
    except Exception as e:
        logging.error(f"API: Error in /login endpoint: {e}", exc_info=True)
        # Incluso en errores del servidor, no revelar detalles
        raise HTTPException(status_code=500, detail="Error del servidor")


@app.post("/invoices/upload-ocr", response_model=OCRResponse)
async def upload_ocr_invoice(
    file: UploadFile = File(...),
    current_user: TokenData = Depends(get_current_user)  # Requiere autenticación JWT
):
    """
    Receives an invoice image, processes it using the OCR flow,
    and returns the result.
    Requiere autenticación JWT.
    """
    try:
        logging.info(f"API: OCR request from user {current_user.user_id} ({current_user.email})")
        
        # Read the image data from the uploaded file
        image_data = await file.read()
        if not image_data:
            raise HTTPException(status_code=400, detail="No image data received.")

        logging.info(f"API: Processing image for user_id: {current_user.user_id}. Size: {len(image_data)} bytes.")

        # Call the existing processing flow, using the user_id from the JWT token
        result = await flow_process_ocr_invoice(
            image_data=image_data,
            user_db_id=current_user.user_id,  # Usar el user_id del token
            source="flutter_app", # New source to identify requests from the app
            message=None,
            whatsapp_id=None,
            chat_id=None,
            message_id=None,
            db_log=None,
            cost_lumis=0 # Continue with the free model
        )

        if not isinstance(result, dict):
            logging.error(f"API: OCR flow returned a non-dict response: {result}")
            raise HTTPException(status_code=500, detail="Internal server error during OCR processing.")

        return result

    except HTTPException as http_exc:
        # Re-raise HTTP exceptions directly
        raise http_exc
    except Exception as e:
        logging.error(f"API: Unhandled error in /upload-ocr endpoint: {e}", exc_info=True)
        return OCRResponse(
            success=False,
            message="An unexpected error occurred on the server.",
            error_details=str(e)
        )


# --- Nuevos endpoints protegidos ---

@app.get("/users/profile")
async def get_user_profile(current_user: dict = Depends(get_current_user)):
    """Obtiene el perfil del usuario autenticado"""
    try:
        # Obtener información adicional del usuario de la BD
        user_summary = await DatabaseOperations.summary_by_user(current_user.get("email"))
        
        return {
            "user_id": current_user.get("user_id"),
            "email": current_user.get("email"),
            "summary": user_summary
        }
    except Exception as e:
        logging.error(f"API: Error getting user profile: {e}")
        raise HTTPException(status_code=500, detail="Error al obtener perfil")

# Endpoint eliminado: /invoices/my-invoices
# Reemplazado por /v1/invoices/header con mejor funcionalidad

@app.post("/v1/auth/check-status", response_model=UserStatusResponse)
@limiter.limit("20/minute")  # Máximo 20 verificaciones por minuto
async def check_user_status(request: Request, status_request: UserStatusRequest):
    """
    Verifica el estado de un usuario: si existe y si tiene contraseña.
    Usado para determinar el flujo correcto en la app.
    """
    try:
        email = status_request.email.lower().strip()
        
        # Validar formato de email
        if not EMAIL_REGEX.match(email):
            return UserStatusResponse(
                exists=False,
                has_password=False,
                message="Formato de email inválido"
            )
        
        logging.info(f"API: Checking status for email: {email}")

        # Buscar usuario con más información (ws_id, telegram_id, password)
        user_data = await DatabaseOperations.scan_table(
            table='public.dim_users',
            cols='id, email, password, ws_id, telegram_id',
            comp='WHERE LOWER(email) = $1',
            params=(email,)
        )
        
        if not user_data:
            return UserStatusResponse(
                exists=False,
                has_password=False,
                message="Usuario no registrado"
            )
        
        # Extraer datos del usuario
        user = user_data[0]  # Primera (y única) fila
        user_id, email_db, password_hash, ws_id, telegram_id = user
        
        # Determinar origen del usuario
        source = "email"  # por defecto
        if ws_id:
            source = "whatsapp"
        elif telegram_id:
            source = "telegram"
        
        has_password = bool(password_hash)
        
        logging.info(f"User {email} - exists: True, has_password: {has_password}, source: {source}")
        
        return UserStatusResponse(
            exists=True,
            has_password=has_password,
            source=source,
            message="Usuario encontrado"
        )
        
    except Exception as e:
        logging.error(f"Error checking user status: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail="Error del servidor")

@app.post("/users/send-verification", response_model=SendVerificationResponse)
@limiter.limit("3/hour")  # Máximo 3 códigos por hora por IP
async def send_verification_code(request: Request, verification_request: SendVerificationRequest):
    """
    Envía código de verificación a usuarios existentes.
    """
    try:
        email = verification_request.email.lower().strip()
        
        logging.info(f"API: Sending verification code to: {email}")
        
        # Buscar usuario
        user_data = await DatabaseOperations.scan_table(
            table='public.dim_users',
            cols='id, email, password, ws_id, telegram_id',
            comp='WHERE LOWER(email) = $1',
            params=(email,)
        )
        
        if not user_data:
            raise HTTPException(status_code=404, detail="Usuario no encontrado")
        
        user = user_data[0]
        user_id, email_db, password_hash, ws_id, telegram_id = user
        
        # Generar código seguro
        code = generate_verification_code()
        
        # Determinar tipo de código basado en si tiene contraseña
        code_type = "reset_password" if password_hash else "set_password"
        
        # Guardar código en Redis con expiración
        await store_verification_code(email, code, code_type)
        
        # Determinar método de envío
        method = "email"
        send_success = False
        
        if ws_id:
            method = "whatsapp"
            # TODO: Integrar con tu sistema de WhatsApp existente
            logger.info(f"Would send WhatsApp verification code to ws_id: {ws_id}")
            # Por ahora, enviar por email como fallback
            send_success = await send_email_verification_code(email, code, code_type)
        else:
            # Enviar por email usando tu sistema existente
            send_success = await send_email_verification_code(email, code, code_type)
        
        if not send_success:
            # Si falla el envío, eliminar el código de Redis
            redis_client = await RedisManager.get_client(1)
            await redis_client.delete(f"verification:{email}:{code_type}")
            raise HTTPException(status_code=500, detail="Error enviando código de verificación")
            logging.info(f"Would send WhatsApp verification code {code} to ws_id: {ws_id}")
        
        # Por ahora, loguear el código para desarrollo
        logging.info(f"Verification code for {email}: {code} (type: {code_type})")
        
        # TODO: En producción, enviar por email usando tu sistema existente
        # await send_email_verification(email, code)
        
        return SendVerificationResponse(
            success=True,
            message=f"Código enviado por {method}",
            method=method
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logging.error(f"Error sending verification: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail="Error del servidor")


@app.post("/users/verify-account", response_model=VerifyAccountResponse)
@limiter.limit("10/minute")
async def verify_account(request: Request, data: VerifyAccountRequest):
    """
    Verifica un código de verificación para un email.
    Intenta validar contra los propósitos 'set_password' y 'reset_password'.
    """
    try:
        email = data.email.lower().strip()
        code = data.verification_code.strip()

        logging.info(f"API: Verification attempt for email: {email}")

        # Intentar verificar para 'set_password'
        is_valid_set = await verify_verification_code(email, code, purpose="set_password")
        
        # Si no es válido, intentar para 'reset_password'
        is_valid_reset = False
        if not is_valid_set:
            is_valid_reset = await verify_verification_code(email, code, purpose="reset_password")

        if is_valid_set or is_valid_reset:
            # Importante: No eliminar el código aquí. Se debe usar en el siguiente paso (set/reset password).
            logging.info(f"Successfully verified code for {email}")
            return VerifyAccountResponse(success=True, message="Código de verificación correcto.")
        else:
            logging.warning(f"Invalid verification code for {email}")
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Código de verificación inválido o expirado."
            )

    except HTTPException:
        raise
    except Exception as e:
        logging.error(f"Error during account verification: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail="Error del servidor")

@app.post("/users/set-password", response_model=MessageResponse)
@limiter.limit("5/minute")  # Límite para establecer contraseña
async def set_user_password(password_data: SetPasswordRequest, request: Request):
    """
    Establece una contraseña para un usuario que no la tiene (registrado via WhatsApp/Telegram).
    Requiere código de verificación.
    """
    try:
        # Verificar código de verificación
        if not verify_verification_code(password_data.email, password_data.verification_code, "set_password"):
            logger.warning(f"Invalid verification code for password set: {password_data.email}")
            raise HTTPException(status_code=400, detail="Invalid or expired verification code")
        
        # Obtener usuario
        user = await get_user_by_email_with_timing_protection(password_data.email)
        if not user:
            logger.warning(f"Password set attempt for non-existent user: {password_data.email}")
            raise HTTPException(status_code=400, detail="User not found")
        
        # Verificar que el usuario no tenga contraseña ya establecida
        if user.get('password'):
            logger.warning(f"Password set attempt for user who already has password: {password_data.email}")
            raise HTTPException(status_code=400, detail="User already has a password set")
        
        # Hash de la nueva contraseña
        password_hash = get_password_hash(password_data.new_password)
        
        # Actualizar contraseña en la base de datos usando PostgreSQL syntax
        query = "UPDATE dim_users SET password = $1 WHERE LOWER(email) = $2"
        success = await fun_execute_update(query, (password_hash, password_data.email.lower()))
        
        if not success:
            logger.error(f"Failed to update password for user: {password_data.email}")
            raise HTTPException(status_code=500, detail="Password update failed")
        
        logger.info(f"Password set successfully for user: {password_data.email}")
        return MessageResponse(message="Password set successfully")
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Set password error: {str(e)}")
        raise HTTPException(status_code=500, detail="Password set failed")


@app.post("/users/register", response_model=RegistrationResponse, status_code=status.HTTP_201_CREATED)
@limiter.limit("5/hour")
async def register_user(user_data: UserRegistrationRequest, request: Request):
    """
    Registra un nuevo usuario en el sistema.
    """
    try:
        email = user_data.email.lower().strip()

        # 1. Verificar si el usuario ya existe
        existing_user = await DatabaseOperations.scan_table(
            table='public.dim_users',
            cols='id',
            comp='WHERE LOWER(email) = $1',
            params=(email,)
        )

        if existing_user:
            raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="El correo electrónico ya está registrado")

        # 2. Hashear la contraseña
        password_hash = get_password_hash(user_data.password)

        # 3. Insertar el nuevo usuario en la base de datos
        query = """
            INSERT INTO public.dim_users (email, password, name, creation_date, last_login_date, source)
            VALUES ($1, $2, $3, NOW(), NOW(), $4)
            RETURNING id;
        """
        params = (email, password_hash, user_data.name, 'email')

        try:
            # Usamos fun_execute_query para obtener el ID devuelto
            result = await fun_execute_query(query, params, fetch_one=True)
            new_user_id = result[0] if result else None

            if not new_user_id:
                logger.error(f"Failed to create user: {email}. No ID returned.")
                raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail="Error al crear el usuario.")

            logger.info(f"New user registered: {email} with ID: {new_user_id}")

            # 4. Devolver respuesta exitosa
            return RegistrationResponse(
                message="Usuario registrado con éxito",
                user_id=new_user_id
            )
        except Exception as db_exc:
            logger.error(f"Database error during user registration for {email}: {db_exc}", exc_info=True)
            # Check for unique constraint violation, in case of a race condition
            if 'unique_violation' in str(db_exc):
                raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="El correo electrónico ya está registrado.")
            raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail="Error de base de datos al registrar.")

    except HTTPException:
        raise
    except Exception as e:
        logging.error(f"Error during user registration for {user_data.email}: {e}", exc_info=True)
        raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail="Error interno del servidor")

@app.post("/users/reset-password", response_model=MessageResponse)
@limiter.limit("5/minute")  # Límite para resetear contraseña
async def reset_user_password(password_data: ResetPasswordRequest, request: Request):
    """
    Resetea la contraseña de un usuario que ya tiene una establecida.
    Requiere código de verificación.
    """
    try:
        # Verificar código de verificación
        if not verify_verification_code(password_data.email, password_data.verification_code, "reset_password"):
            logger.warning(f"Invalid verification code for password reset: {password_data.email}")
            raise HTTPException(status_code=400, detail="Invalid or expired verification code")
        
        # Obtener usuario
        user = await get_user_by_email_with_timing_protection(password_data.email)
        if not user:
            logger.warning(f"Password reset attempt for non-existent user: {password_data.email}")
            raise HTTPException(status_code=400, detail="User not found")
        
        # Verificar que el usuario tenga contraseña establecida
        if not user.get('password'):
            logger.warning(f"Password reset attempt for user without password: {password_data.email}")
            raise HTTPException(status_code=400, detail="User does not have a password set")
        
        # Hash de la nueva contraseña
        password_hash = get_password_hash(password_data.new_password)
        
        # Actualizar contraseña en la base de datos usando PostgreSQL syntax
        query = "UPDATE dim_users SET password = $1 WHERE LOWER(email) = $2"
        success = await fun_execute_update(query, (password_hash, password_data.email.lower()))
        
        if not success:
            logger.error(f"Failed to reset password for user: {password_data.email}")
            raise HTTPException(status_code=500, detail="Password reset failed")
        
        logger.info(f"Password reset successfully for user: {password_data.email}")
        return MessageResponse(message="Password reset successfully")
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Reset password error: {str(e)}")
        raise HTTPException(status_code=500, detail="Password reset failed")

@app.post("/v1/invoices/process-from-url", response_model=MessageResponse, tags=["Invoices"])
async def process_invoice_from_url(
    request_data: ProcessUrlRequest,
    current_user: dict = Depends(get_current_user)
):
    """
    Receives a URL, processes it to extract invoice data, saves it to the database,
    and returns a confirmation message.

    - **Requires authentication**.
    - The processing is done asynchronously.
    """
    user_email = current_user.get('email')
    user_db_id = current_user.get('user_id')
    url = str(request_data.url)

    if not user_email or not user_db_id:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="User information not found in token."
        )

    # Determine process type from URL
    process_type = None
    if 'Consultas/FacturasPorCUFE' in url:
        process_type = 'CUFE'
    elif any(allowed_url in url for allowed_url in allowed_urls):
        process_type = 'QR'
    
    if not process_type:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="The provided URL is not a valid Panama electronic invoice URL."
        )

    try:
        # Prepare parameters for the processing function
        db_log = {}
        source = request_data.source

        # Call the core processing function
        result_message = await _process_invoice_data_from_url(
            url=url,
            chat_id=user_email,  # Use email as a unique identifier
            message_id=None,     # Not applicable for API calls
            source=source,
            db_log=db_log,
            process_type=process_type,
            email=user_email,
            user_db_id=user_db_id,
            header_type='public.invoice_header'  # Especificar el tipo de cabecera
        )

        return MessageResponse(message=result_message)

    except Exception as e:
        logger.error(f"Error processing URL {url} for user {user_email}: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="An unexpected error occurred while processing the invoice."
        )


@app.get("/v1/invoices/details", response_model=InvoiceDetailResponse)
@limiter.limit("50/hour")
async def get_invoice_details(
    request: Request,
    from_date: datetime = Query(..., description="Fecha desde (timezone Panamá)"),
    invoice_type: Optional[str] = Query(None, description="Tipo de factura (ocr_pending, ocr_validated, QR, CUFE) o 'all' para todos"),
    invoice_types: Optional[str] = Query(None, description="Múltiples tipos separados por coma (ej: ocr_validated,QR,CUFE)"),
    current_user: dict = Depends(get_current_user)
):
    """
    Obtiene los detalles de las facturas del usuario desde una fecha específica.
    Requiere autenticación JWT.
    
    Args:
        from_date: Fecha desde la cual buscar facturas (timezone Panamá)
        invoice_type: Tipo de factura opcional para filtrar
        current_user: Usuario autenticado (automático)
    
    Returns:
        Lista de detalles de facturas
    """

    try:
        # Validar timezone de Panamá
        if from_date.tzinfo != panama_timezone:
            from_date = panama_timezone.localize(from_date)
        
        # Construir query
        query_params = [current_user.get("user_id"), from_date]
        where_conditions = ["user_id = $1", "(process_date >= $2 OR reception_date >= $2)"]
        
        # Manejar filtros de tipo de factura
        if invoice_types:
            types_list = [t.strip() for t in invoice_types.split(',') if t.strip()]
            if types_list:
                placeholders = ','.join([f'${i+3}' for i in range(len(types_list))])
                where_conditions.append(f"type IN ({placeholders})")
                query_params.extend(types_list)
        elif invoice_type:
            if invoice_type.lower() != 'all':
                where_conditions.append("type = $3")
                query_params.append(invoice_type)
        
        where_clause = " AND ".join(where_conditions)
        
        # Ejecutar query
        invoices = await DatabaseOperations.scan_table(
            table="public.invoice_with_details",
            cols="cufe, quantity, code, date, total, unit_price, amount, unit_discount, description, user_id",
            comp=f"WHERE {where_clause} ORDER BY date DESC",
            params=tuple(query_params)
        )
        
        # Formatear respuesta
        formatted_invoices = []
        for inv in invoices:
            formatted_invoices.append(InvoiceDetail(
                cufe=inv[0],
                quantity=inv[1],
                code=inv[2],
                date=inv[3],
                total=inv[4],
                unit_price=inv[5],
                amount=inv[6],
                unit_discount=inv[7],
                description=inv[8],
                user_id=inv[9]
            ))
        
        # Determinar filtros aplicados
        filters_applied = ["date"]
        if invoice_types:
            filters_applied.append("types")
        elif invoice_type and invoice_type.lower() != 'all':
            filters_applied.append("type")
        
        return InvoiceDetailResponse(
            invoices=formatted_invoices,
            count=len(formatted_invoices),
            query_info={
                "from_date": from_date.isoformat(),
                "user_id": current_user.get("user_id"),
                "filters_applied": filters_applied
            }
        )
        
    except Exception as e:
        logging.error(f"API: Error getting invoice details: {e}")
        raise HTTPException(status_code=500, detail="Error al obtener detalles de facturas")


@app.get("/v1/invoices/header", response_model=InvoiceHeaderResponse)
@limiter.limit("50/hour")
async def get_invoice_headers(
    request: Request,
    from_date: datetime = Query(..., description="Fecha desde (timezone Panamá)"),
    invoice_type: Optional[str] = Query(None, description="Tipo de factura (ocr_pending, ocr_validated, QR, CUFE) o 'all' para todos"),
    invoice_types: Optional[str] = Query(None, description="Múltiples tipos separados por coma (ej: ocr_validated,QR,CUFE)"),
    current_user: dict = Depends(get_current_user)
):
    """
    Obtiene las facturas del usuario desde una fecha específica.
    Requiere autenticación JWT.
    
    Args:
        from_date: Fecha desde la cual buscar facturas (timezone Panamá)
        invoice_type: Tipo de factura opcional para filtrar
        current_user: Usuario autenticado (automático)
    
    Returns:
        Lista de facturas con detalles básicos
    """
    try:
        # Validar timezone de Panamá
        if from_date.tzinfo != panama_timezone:
            from_date = panama_timezone.localize(from_date)
        
        # Construir query
        query_params = [current_user.get("user_id"), from_date]
        where_conditions = ["user_id = $1", "(process_date >= $2 OR reception_date >= $2)"]
        
        # Manejar filtros de tipo de factura
        if invoice_types:
            # Múltiples tipos separados por coma
            types_list = [t.strip() for t in invoice_types.split(',') if t.strip()]
            if types_list:
                placeholders = ','.join([f'${i+3}' for i in range(len(types_list))])
                where_conditions.append(f"type IN ({placeholders})")
                query_params.extend(types_list)
        elif invoice_type:
            if invoice_type.lower() == 'all':
                # No agregar filtro de tipo - traer todos
                pass
            else:
                # Un solo tipo
                where_conditions.append("type = $3")
                query_params.append(invoice_type)
        
        where_clause = " AND ".join(where_conditions)
        
        # Ejecutar query
        invoices = await DatabaseOperations.scan_table(
            table="public.invoice_header",
            cols="date, tot_itbms, issuer_name, no, tot_amount, url, process_date, reception_date, type, cufe",
            comp=f"WHERE {where_clause} ORDER BY date DESC",
            params=tuple(query_params)
        )
        
        # Formatear respuesta
        formatted_invoices = []
        for inv in invoices:
            formatted_invoices.append(InvoiceHeader(
                date=inv[0],
                tot_itbms=inv[1],
                issuer_name=inv[2],
                no=inv[3],
                tot_amount=inv[4],
                url=inv[5],
                process_date=inv[6],
                reception_date=inv[7],
                type=inv[8],
                cufe=inv[9]
            ))
        
        # Determinar filtros aplicados
        filters_applied = ["date"]
        if invoice_types:
            filters_applied.append("types")
        elif invoice_type and invoice_type.lower() != 'all':
            filters_applied.append("type")
        
        return InvoiceHeaderResponse(
            invoices=formatted_invoices,
            count=len(formatted_invoices),
            query_info={
                "from_date": from_date.isoformat(),
                "user_id": current_user.get("user_id"),
                "filters_applied": filters_applied
            }
        )
        
    except Exception as e:
        logging.error(f"API: Error getting invoice headers: {e}")
        raise HTTPException(status_code=500, detail="Error al obtener facturas")


# --- Visual Dashboard Celery Tasks ---
@celery_app.task(bind=True)
def generate_offers_dashboard_task(self, offers_data: List[dict], category: str, whatsapp_number: str, price_range: str = None):
    """
    Celery task to generate visual dashboard asynchronously
    """
    try:
        # Update task status
        self.update_state(state='PROCESSING', meta={'progress': 10})
        
        # Generate real dashboard with actual data using matplotlib (simplified approach)
        import matplotlib.pyplot as plt
        import matplotlib.patches as patches
        import pandas as pd
        import os
        import uuid
        from datetime import datetime
        from decimal import Decimal
        
        # Create unique filename
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        unique_id = str(uuid.uuid4())[:8]
        filename = f"dashboard_{category.lower()}_{timestamp}_{unique_id}.png"
        
        # Ensure static directory exists
        static_dir = "/home/client_1099_1/scripts/qreader_server/static/dashboards"
        os.makedirs(static_dir, exist_ok=True)
        filepath = os.path.join(static_dir, filename)
        
        # Convert offers_data to DataFrame for processing
        columns = ['comercio', 'producto', 'codigo', 'precio_actual', 
                  'precio_anterior', 'descuento_porc', 'descuento_valor', 'dias']
        
        df_data = []
        for offer in offers_data:
            row = [
                offer.get('comercio', 'Comercio'),
                offer.get('producto', 'Producto'),
                offer.get('codigo', ''),
                float(offer.get('precio_actual', 0)),
                float(offer.get('precio_anterior', 0)),
                float(offer.get('descuento_porc', 0)),
                float(offer.get('descuento_valor', 0)),
                int(offer.get('dias', 1))
            ]
            df_data.append(row)
        
        df = pd.DataFrame(df_data, columns=columns)
        
        # Process data like the real function
        df['producto_limpio'] = df['producto'].str.replace('"', '').str.replace('\xa0', ' ')
        df['producto_corto'] = df['producto_limpio'].apply(
            lambda x: x[:35] + '...' if len(x) > 35 else x
        )
        
        # Calculate real discount percentage
        df['descuento_calc'] = ((df['precio_anterior'] - df['precio_actual']) / df['precio_anterior'] * 100).round(1)
        df = df[df['precio_anterior'] > 0]  # Filter valid prices
        
        # Sort by best discounts (like real function)
        df_top = df.nlargest(8, 'descuento_calc').reset_index(drop=True)
        
        # Create professional dashboard visualization
        fig, ax = plt.subplots(figsize=(14, 10))
        fig.patch.set_facecolor('#ffffff')
        
        # Header with category
        ax.text(0.5, 0.95, f'🎯 RADAR DE OFERTAS - {category.upper()}', 
                horizontalalignment='center', fontsize=22, fontweight='bold',
                transform=ax.transAxes, color='#1a365d')
        
        # Summary stats
        total_offers = len(df)
        avg_discount = df['descuento_calc'].mean() if len(df) > 0 else 0
        max_discount = df['descuento_calc'].max() if len(df) > 0 else 0
        
        ax.text(0.5, 0.88, f'📊 {total_offers} ofertas encontradas | 💰 Descuento promedio: {avg_discount:.1f}% | 🔥 Máximo: {max_discount:.1f}%', 
                horizontalalignment='center', fontsize=14,
                transform=ax.transAxes, color='#2d3748')
        
        # Create offer cards with real data
        y_start = 0.80
        card_height = 0.08
        card_spacing = 0.02
        
        for i, (_, offer) in enumerate(df_top.iterrows()):
            if i >= 8:  # Limit to top 8 offers
                break
                
            y_pos = y_start - (i * (card_height + card_spacing))
            
            # Determine card color based on discount
            discount = offer['descuento_calc']
            if discount >= 30:
                card_color = '#e53e3e'  # Red for high discounts
                text_color = 'white'
            elif discount >= 20:
                card_color = '#dd6b20'  # Orange for medium discounts
                text_color = 'white'
            elif discount >= 10:
                card_color = '#38a169'  # Green for good discounts
                text_color = 'white'
            else:
                card_color = '#4a5568'  # Gray for low discounts
                text_color = 'white'
            
            # Create colored card
            rect = patches.Rectangle((0.05, y_pos - card_height/2), 0.9, card_height, 
                                   linewidth=0, facecolor=card_color, alpha=0.9)
            ax.add_patch(rect)
            
            # Ranking number
            ax.text(0.08, y_pos, f'{i+1}', fontsize=16, fontweight='bold', 
                   color=text_color, va='center')
            
            # Store name
            ax.text(0.12, y_pos + 0.015, f'🏪 {offer["comercio"]}', 
                   fontsize=11, fontweight='bold', color=text_color)
            
            # Product name (truncated)
            ax.text(0.12, y_pos - 0.015, f'📦 {offer["producto_corto"]}', 
                   fontsize=9, color=text_color)
            
            # Prices
            precio_actual = offer['precio_actual']
            precio_anterior = offer['precio_anterior']
            
            ax.text(0.65, y_pos + 0.015, f'💰 ${precio_actual:.2f}', 
                   fontsize=12, fontweight='bold', color=text_color)
            
            # Original price (crossed out effect)
            ax.text(0.65, y_pos - 0.015, f'${precio_anterior:.2f}', 
                   fontsize=10, color=text_color, alpha=0.7)
            
            # Discount percentage
            ax.text(0.85, y_pos, f'-{discount:.1f}%', 
                   fontsize=14, fontweight='bold', color='#ffd700',  # Gold color
                   bbox=dict(boxstyle='round,pad=0.3', facecolor='white', alpha=0.9))
        
        # Footer with generation info
        ax.text(0.5, 0.05, f'📱 Dashboard generado para WhatsApp: {whatsapp_number} | 🕒 {datetime.now().strftime("%d/%m/%Y %H:%M")}', 
                horizontalalignment='center', fontsize=10, style='italic',
                transform=ax.transAxes, color='#718096')
        
        ax.set_xlim(0, 1)
        ax.set_ylim(0, 1)
        ax.axis('off')
        
        # Save the image with high quality
        plt.tight_layout()
        plt.savefig(filepath, dpi=300, bbox_inches='tight', facecolor='white', edgecolor='none')
        plt.close()
        
        # Create public URL accessible by WhatsApp
        # Use the correct public domain that WhatsApp can access
        public_url = f"https://api.lumapp.org/static/dashboards/{filename}"
        
        # Create result
        result = {
            "success": True,
            "image_url": public_url,
            "summary": f"Generated dashboard for {len(offers_data)} offers in {category}",
            "whatsapp_sent": True,
            "category": category,
            "offers_count": len(offers_data),
            "whatsapp_number": whatsapp_number,
            "filepath": filepath
        }
        
        # Update task status
        self.update_state(state='SUCCESS', meta={
            'progress': 100,
            'result': result
        })
        
        return result
        
    except Exception as e:
        logging.error(f"Dashboard generation failed: {e}")
        self.update_state(
            state='FAILURE',
            meta={'error': str(e), 'progress': 0}
        )
        raise


# --- Visual Dashboard API Endpoints ---
@app.post("/api/v2/generate_offers_dashboard", response_model=DashboardResponse)
@limiter.limit("10/minute")
async def generate_offers_dashboard(
    request: Request,
    dashboard_request: DashboardRequest
):
    """
    Generate visual dashboard for offers asynchronously
    """
    try:
        # Generate unique task ID
        task_id = str(uuid.uuid4())
        
        # Convert offers to dict format
        offers_data = [offer.dict() for offer in dashboard_request.offers]
        
        # Submit task to Celery
        task = generate_offers_dashboard_task.apply_async(
            args=[
                offers_data,
                dashboard_request.category,
                dashboard_request.whatsapp_number,
                dashboard_request.price_range
            ],
            task_id=task_id
        )
        
        # Store task info
        task_storage[task_id] = {
            "status": "pending",
            "created_at": datetime.now(),
            "category": dashboard_request.category,
            "offers_count": len(offers_data)
        }
        
        return DashboardResponse(
            task_id=task_id,
            status="pending",
            message="Dashboard generation started"
        )
        
    except Exception as e:
        logging.error(f"Error starting dashboard generation: {e}")
        raise HTTPException(status_code=500, detail="Error starting dashboard generation")


@app.get("/api/v2/task_status/{task_id}", response_model=TaskStatus)
@limiter.limit("30/minute")
async def get_task_status(request: Request, task_id: str):
    """
    Get status of dashboard generation task
    """
    try:
        # Get task from Celery
        logging.info(f"Getting task status for task_id: {task_id}")
        task = celery_app.AsyncResult(task_id)
        logging.info(f"Task object created: {task}")
        logging.info(f"Task state: {task.state}")
        logging.info(f"Task info: {task.info}")
        logging.info(f"Task info type: {type(task.info)}")
        
        if task.state == 'PENDING':
            status = {
                "task_id": task_id,
                "status": "pending",
                "progress": 0
            }
        elif task.state == 'PROCESSING':
            progress = 0
            if task.info and isinstance(task.info, dict):
                progress = task.info.get('progress', 0)
            status = {
                "task_id": task_id,
                "status": "processing",
                "progress": progress
            }
        elif task.state == 'SUCCESS':
            result = None
            if task.info:
                result = task.info
            status = {
                "task_id": task_id,
                "status": "completed",
                "progress": 100,
                "result": result
            }
        elif task.state == 'FAILURE':
            error_msg = "Task failed"
            if task.info:
                error_msg = str(task.info)
            status = {
                "task_id": task_id,
                "status": "failed",
                "progress": 0,
                "error": error_msg
            }
        else:
            status = {
                "task_id": task_id,
                "status": task.state.lower(),
                "progress": 0
            }
        
        return TaskStatus(**status)
        
    except Exception as e:
        logging.error(f"Error getting task status: {e}")
        raise HTTPException(status_code=500, detail="Error getting task status")


@app.get("/api/v2/health")
@limiter.limit("60/minute")
async def health_check(request: Request):
    """
    Health check endpoint for visual dashboard API
    """
    return {
        "status": "healthy",
        "service": "visual_dashboard_api",
        "timestamp": datetime.now().isoformat(),
        "celery_active": True
    }


@app.get("/health")
async def simple_health_check():
    """
    Simple health check endpoint for Rust client
    """
    return {"status": "ok", "service": "qreader_api"}


@app.post("/qr/hybrid-fallback")
async def qr_hybrid_fallback(file: UploadFile = File(...)):
    """
    Endpoint de fallback híbrido para detección QR cuando los decodificadores Rust fallan.
    Utiliza el pipeline Python completo: CV2 → CV2 CURVED → PYZBAR → QREADER S → QREADER L
    """
    try:
        # Validar tipo de archivo
        if not file.content_type.startswith('image/'):
            raise HTTPException(status_code=400, detail="El archivo debe ser una imagen")
        
        # Leer imagen
        image_data = await file.read()
        
        # Procesar imagen con pipeline híbrido
        from ws_qrdetection.app_fun_qrdetection import leer_limpiar_imagen, imagen_a_url
        
        # Limpiar y preparar imagen
        processed_image = leer_limpiar_imagen(image_data)
        
        # Intentar decodificación con pipeline Python completo
        qr_data, detector_model = imagen_a_url(processed_image)
        
        if qr_data:
            return {
                "success": True,
                "qr_data": qr_data,
                "detector_model": detector_model,
                "pipeline": "Python Hybrid Fallback",
                "methods_tried": ["CV2", "CV2_CURVED", "PYZBAR", "QREADER_S", "QREADER_L"]
            }
        else:
            return {
                "success": False,
                "qr_data": None,
                "detector_model": detector_model or "ALL_FAILED",
                "pipeline": "Python Hybrid Fallback",
                "methods_tried": ["CV2", "CV2_CURVED", "PYZBAR", "QREADER_S", "QREADER_L"],
                "message": "No se pudo detectar código QR con ningún método Python"
            }
            
    except Exception as e:
        logger.error(f"Error en hybrid fallback: {e}")
        raise HTTPException(status_code=500, detail=f"Error procesando imagen: {str(e)}")


if __name__ == "__main__":
    # Este bloque es para pruebas locales y no se ejecutará en producción con uvicorn
    # Para ejecutar: python api_main.py
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8008)
