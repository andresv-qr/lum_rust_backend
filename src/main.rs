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

    // Inicializar ONNX readers para QR detection ML
    use lum_rust_ws::processing::qr_detection;
    qr_detection::initialize_onnx_readers();
    info!("🤖 ONNX ML models initialized for enhanced QR detection");

    // Inicializar scheduler de ofertasws si WS pool está disponible
    if let Some(ref ws_pool) = app_state.ws_pool {
        use lum_rust_ws::tasks::start_ofertasws_refresh_scheduler;
        let ws_pool_clone = ws_pool.clone();
        let redis_pool_clone = app_state.redis_pool.clone();
        
        tokio::spawn(async move {
            if let Err(e) = start_ofertasws_refresh_scheduler(ws_pool_clone, redis_pool_clone).await {
                tracing::error!("❌ Failed to start ofertasws refresh scheduler: {}", e);
            }
        });
        
        info!("⏰ OfertasWs refresh scheduler initialized (10am & 3pm Panamá)");
    }

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
