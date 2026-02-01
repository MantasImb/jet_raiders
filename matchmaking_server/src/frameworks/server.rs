use crate::interface_adapters::routes;
use crate::interface_adapters::state::AppState;
use crate::use_cases::matchmaker::Matchmaker;
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

    // Initialize the in-memory matchmaking queue.
    let state = Arc::new(AppState {
        matchmaker: Arc::new(Mutex::new(Matchmaker::new())),
    });

    // Wire the HTTP routes for the matchmaking API.
    let app = routes::app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3003));
    tracing::info!(%addr, "listening");

    // Bind TCP listener with error handling.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%addr, %error, "failed to bind");
            return; // Abort startup on bind failure.
        }
    };

    // Serve app and report errors rather than panicking.
    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(%error, "server error");
    }
}
