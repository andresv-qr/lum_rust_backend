#!/usr/bin/env python3
import jwt
import json
from datetime import datetime, timedelta
import uuid

# JWT Secret (debe coincidir con el que usa la aplicación)
SECRET = "lumis_jwt_secret_super_seguro_production_2024_rust_server_key"

# Payload del JWT
payload = {
    "sub": "1",  # user_id 1 que probablemente tenga facturas
    "email": "user1@example.com",
    "source": "api",
    "roles": ["user"],
    "iat": int(datetime.utcnow().timestamp()),
    "exp": int((datetime.utcnow() + timedelta(hours=1)).timestamp()),
    "jti": str(uuid.uuid4()),
    "token_type": "access"
}

# Generar JWT
token = jwt.encode(payload, SECRET, algorithm='HS256')

print("JWT generado:")
print(token)
print()
print("Payload:")
print(json.dumps(payload, indent=2))