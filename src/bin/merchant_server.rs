// ============================================================================
// LUM MERCHANT SERVER - Microservicio independiente para comercios
// ============================================================================
// Este binario sirve únicamente los endpoints de comercios:
// - POST /api/v1/merchant/auth/login
// - POST /api/v1/merchant/validate
// - POST /api/v1/merchant/confirm/:id
// - GET  /api/v1/merchant/stats
// - GET  /api/v1/merchant/analytics
// - GET  /api/v1/merchant/dashboard
// - GET  /api/v1/merchant/dashboard/stats
// - GET  /api/v1/merchant/pending
// ============================================================================

use anyhow::Result;
use axum::{Router, extract::DefaultBodyLimit};
use lum_rust_ws::{
    api::merchant,
    state::AppState,
    security::{security_headers_middleware, get_cors_layer},
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;
use tower_http::compression::{CompressionLayer, predicate::SizeAbove};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use axum::middleware as axum_middleware;

// 🚀 JEMALLOC ALLOCATOR - Optimización de memoria
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

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

    info!("🛑 Signal received, starting graceful shutdown");
}

/// Crear el router minimalista para el servicio de comercios
fn create_merchant_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        // Solo los endpoints de merchant
        .nest("/api/v1/merchant", merchant::router())
        // Health check para monitoreo
        .route("/health", axum::routing::get(|| async { "OK" }))
        // Aplicar estado
        .with_state(app_state)
        // Middlewares ligeros (sin rate limiting pesado, eso va en Nginx)
        .layer(DefaultBodyLimit::max(1024 * 1024))  // 1MB suficiente para merchant
        .layer(
            CompressionLayer::new()
                .gzip(true)
                .br(false)
                .deflate(true)
                .compress_when(SizeAbove::new(512))
        )
        .layer(TraceLayer::new_for_http())
        .layer(get_cors_layer())
        .layer(axum_middleware::from_fn(security_headers_middleware))
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]  // Solo 2 workers, es un servicio ligero
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🏪 Starting Lüm Merchant Server (microservice)");

    // Crear estado (reutiliza la misma conexión a DB/Redis)
    let app_state = AppState::new().await?;
    info!("✅ Database and Redis connections established");

    let app = create_merchant_router(Arc::new(app_state));

    // Puerto configurable, default 8001
    let port = std::env::var("MERCHANT_PORT")
        .unwrap_or_else(|_| "8001".to_string())
        .parse::<u16>()
        .unwrap_or(8001);
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port));  // Solo localhost (Nginx hace el proxy)
    info!("🚀 Merchant server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("✅ Merchant server shutdown completed");
    Ok(())
}
