use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Crea el pool de conexiones para la base de datos WS (ofertas)
/// Separado del pool principal (rewards) para evitar contaminación
pub async fn create_ws_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("WS_DATABASE_URL")
        .expect("WS_DATABASE_URL must be set in environment");

    tracing::info!("🔌 Connecting to WS database...");

    let pool = PgPoolOptions::new()
        .max_connections(5) // Pocas conexiones, solo para ofertas
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600)) // 10 minutos
        .max_lifetime(Duration::from_secs(1800)) // 30 minutos
        .connect(&database_url)
        .await?;

    tracing::info!("✅ WS database pool created successfully");

    Ok(pool)
}

/// Verifica la salud de la conexión WS
pub async fn check_ws_health(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}
