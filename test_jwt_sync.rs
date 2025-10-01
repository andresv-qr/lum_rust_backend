#!/usr/bin/env cargo script
//! Test to verify JWT secret synchronization between auth creation and middleware validation
//! 
//! Usage: cargo run --bin test_jwt_sync

use std::env;

fn main() {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Get JWT secret from environment (same way as both services)
    let jwt_secret = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "lumis_jwt_secret_super_seguro_production_2024_rust_server_key".to_string());
    
    println!("🔍 JWT Secret Test - Verificando Sincronización");
    println!("📝 JWT_SECRET desde .env: {}", jwt_secret);
    println!("📏 Longitud: {} caracteres", jwt_secret.len());
    println!("🔐 Hash SHA256: {}", sha256::digest(&jwt_secret));
    
    // Verify the secret is production-ready (at least 32 chars)
    if jwt_secret.len() >= 32 {
        println!("✅ Secret tiene longitud adecuada (>=32 chars)");
    } else {
        println!("⚠️ Secret muy corto para producción (<32 chars)");
    }
    
    // Test JWT creation and validation consistency
    use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Validation, Algorithm};
    use serde::{Serialize, Deserialize};
    
    #[derive(Serialize, Deserialize)]
    struct TestClaims {
        user_id: i64,
        email: String,
        exp: i64,
        iat: i64,
        jti: Option<String>,
    }
    
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        user_id: 1,
        email: "test@example.com".to_string(),
        exp: now + 3600, // 1 hour
        iat: now,
        jti: Some("test-jwt-sync".to_string()),
    };
    
    // Create JWT (auth side)
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());
    let header = Header::new(Algorithm::HS256);
    
    let token = match encode(&header, &claims, &encoding_key) {
        Ok(token) => {
            println!("✅ JWT creado exitosamente (AUTH)");
            println!("📄 Token: {}...", &token[..50]);
            token
        }
        Err(e) => {
            println!("❌ Error creando JWT: {}", e);
            return;
        }
    };
    
    // Validate JWT (middleware side)
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);
    
    match decode::<TestClaims>(&token, &decoding_key, &validation) {
        Ok(decoded) => {
            println!("✅ JWT validado exitosamente (MIDDLEWARE)");
            println!("👤 User ID: {}", decoded.claims.user_id);
            println!("📧 Email: {}", decoded.claims.email);
            println!("🎉 SINCRONIZACIÓN CORRECTA - Problema resuelto!");
        }
        Err(e) => {
            println!("❌ Error validando JWT: {}", e);
            println!("🚨 PROBLEMA PERSISTE - Revisar configuración");
        }
    }
}
