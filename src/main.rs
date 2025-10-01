use anyhow::Result;
use lum_rust_ws::{
    create_app_router, 
    state::AppState,
    monitoring,
};
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("signal received, starting graceful shutdown");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Carga las variables de entorno desde el archivo .env. Falla silenciosamente si no existe.
    dotenvy::dotenv().ok();

    // Configura el subscriber de tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Inicializar sistema de monitoreo
    monitoring::init_monitoring();
    info!("🔍 Monitoring system initialized");

    // Crea el estado de la aplicación con configuración optimizada
    let app_state = AppState::new().await?;
    info!("🚀 Application state initialized with optimized configuration");

    // Crea el router de la aplicación
    let app = create_app_router(Arc::new(app_state));

    // Inicia el servidor
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
