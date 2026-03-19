use crate::frameworks::auth_client::AuthClient;
use crate::frameworks::matchmaking_client::MatchmakingClient;
use crate::interface_adapters::routes;
use crate::interface_adapters::state::AppState;
use crate::use_cases::{GuestSessionService, MatchmakingService};
use std::net::SocketAddr;
use std::sync::Arc;

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

    let auth_base_url =
        std::env::var("AUTH_SERVICE_URL").unwrap_or_else(|_| "http://localhost:3002".into());
    tracing::debug!(auth_base_url = %auth_base_url, "auth client configured.");
    let auth = match AuthClient::new(&auth_base_url) {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(
                auth_base_url = %auth_base_url,
                error = %error,
                "failed to parse AUTH_SERVICE_URL"
            );
            return;
        }
    };
    let guest_sessions = Arc::new(GuestSessionService::new(auth.clone()));
    let matchmaking_base_url =
        std::env::var("MATCHMAKING_SERVICE_URL").unwrap_or_else(|_| "http://localhost:3003".into());
    tracing::debug!(
        matchmaking_base_url = %matchmaking_base_url,
        "matchmaking client configured."
    );
    let matchmaking = match MatchmakingClient::new(&matchmaking_base_url) {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(
                matchmaking_base_url = %matchmaking_base_url,
                error = %error,
                "failed to parse MATCHMAKING_SERVICE_URL"
            );
            return;
        }
    };
    let matchmaking = Arc::new(MatchmakingService::new(auth.clone(), matchmaking));

    let state = Arc::new(AppState {
        guest_sessions,
        matchmaking,
    });

    // Start the web server with the HTTP routes wired up.
    let app = routes::app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!(%addr, "listening");

    // Bind TCP listener with error handling.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(%addr, error = %e, "failed to bind");
            return; // Abort startup on bind failure.
        }
    };

    // Serve app and report errors rather than panicking.
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "server error");
    }
}
