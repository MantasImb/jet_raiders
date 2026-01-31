use crate::frameworks::db;
use crate::interface_adapters::routes::app;
use crate::interface_adapters::state::AppState;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let json = matches!(std::env::var("LOG_FORMAT").as_deref(), Ok("json"));
    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .json()
            .with_current_span(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    }

    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        tracing::error!(%info, ?backtrace, "panic");
    }));
}

pub async fn run() {
    // Load .env locally; safe to ignore when not present.
    let _ = dotenvy::dotenv();
    init_tracing();
    // Load database configuration from the environment.
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(value) => value,
        Err(_) => {
            tracing::error!("DATABASE_URL must be set");
            return;
        }
    };

    // Connect to Postgres and run migrations on startup.
    let db = match db::connect_pool(&database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!(error = %e, "failed to connect to database");
            return;
        }
    };
    if let Err(e) = db::run_migrations(&db).await {
        tracing::error!(error = %e, "failed to run migrations");
        return;
    }

    // Shared, in-memory store for guest sessions.
    let state = AppState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
        db,
    };

    // Wire routes for the guest-only auth flow.
    let app = app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3002));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind to address {}: {}", addr, e);
            return;
        }
    };
    tracing::info!(%addr, "listening");

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "server error");
    }
}
