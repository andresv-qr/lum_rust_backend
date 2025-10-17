#!/usr/bin/env python3
"""
Generate a valid JWT token for testing the Lum Rust API
"""

import jwt
import datetime
import os

# JWT Configuration (should match your Rust server configuration)
JWT_SECRET = "lumis_jwt_secret_super_seguro_production_2024_rust_server_key"
JWT_ALGORITHM = "HS256"

def generate_jwt_token(user_id=1, email="test@example.com", hours=24):
    """Generate a JWT token with specified expiration time"""
    
    now = datetime.datetime.utcnow()
    expiration = now + datetime.timedelta(hours=hours)
    
    payload = {
        "sub": str(user_id),
        "user_id": user_id,
        "email": email,
        "iat": now,
        "exp": expiration
    }
    
    token = jwt.encode(payload, JWT_SECRET, algorithm=JWT_ALGORITHM)
    
    return token

if __name__ == "__main__":
    # Generate token valid for 24 hours
    token = generate_jwt_token()
    
    # Save to file
    with open("jwt_token.txt", "w") as f:
        f.write(token)
    
    print(f"✅ JWT token generated successfully!")
    print(f"📁 Saved to: jwt_token.txt")
    print(f"⏰ Valid for: 24 hours")
    print(f"👤 User ID: 1")
    print(f"📧 Email: test@example.com")
    print(f"🔑 Token preview: {token[:50]}...")
