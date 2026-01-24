use axum::{Router, routing::get};
use std::net::SocketAddr;

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

#[tokio::main]
async fn main() {
    // Load .env locally; safe to ignore when not present.
    let _ = dotenvy::dotenv();
    init_tracing();

    // let (server_state_tx, _server_state_rx) = watch::channel::<ServerState>(ServerState::Lobby);

    // let database_url = match std::env::var("DATABASE_URL") {
    //     Ok(value) => value,
    //     Err(_) => {
    //         tracing::error!("DATABASE_URL must be set");
    //         return;
    //     }
    // };

    // let db = match db::connect_pool(&database_url).await {
    //     Ok(pool) => pool,
    //     Err(e) => {
    //         tracing::error!(error = %e, "failed to connect to database");
    //         return;
    //     }
    // };

    // if let Err(e) = MIGRATOR.run(&db).await {
    //     tracing::error!(error = %e, "failed to run migrations");
    //     return;
    // }

    // let state = Arc::new(AppState {
    //     db,
    // });

    // Start the Web Server
    let app = Router::new();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!(%addr, "listening");

    // Bind TCP listener with error handling
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(%addr, error = %e, "failed to bind");
            return; // Abort startup on bind failure
        }
    };

    // Serve app and report errors rather than panicking
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "server error");
    }
}
