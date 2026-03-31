use crate::frameworks::auth_client::AuthClient;
use crate::frameworks::config::{
    load_head_server_config, load_shared_region_config, ProcessEnv,
};
use crate::frameworks::game_server_client::GameServerClient;
use crate::frameworks::game_server_directory::StaticGameServerDirectory;
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

    let config = load_head_server_config(&ProcessEnv);

    tracing::debug!(
        auth_base_url = %config.auth_service_url,
        "auth client configured."
    );
    let auth = match AuthClient::new(&config.auth_service_url) {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(
                auth_base_url = %config.auth_service_url,
                error = %error,
                "failed to parse AUTH_SERVICE_URL"
            );
            return;
        }
    };

    let guest_sessions = Arc::new(GuestSessionService::new(auth.clone()));

    tracing::debug!(
        matchmaking_base_url = %config.matchmaking_service_url,
        "matchmaking client configured."
    );
    let matchmaking = match MatchmakingClient::new(&config.matchmaking_service_url) {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(
                matchmaking_base_url = %config.matchmaking_service_url,
                error = %error,
                "failed to parse MATCHMAKING_SERVICE_URL"
            );
            return;
        }
    };

    tracing::debug!(
        region_config_path = %config.region_config_path.display(),
        "shared region config path configured."
    );
    let shared_region_config = match load_shared_region_config(&config.region_config_path) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(
                region_config_path = %config.region_config_path.display(),
                error = %error,
                "failed to load shared region config"
            );
            return;
        }
    };
    let game_servers = Arc::new(StaticGameServerDirectory::from_shared_region_config(
        shared_region_config,
    ));

    let provisioner = match GameServerClient::new() {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(?error, "failed to build game server client");
            return;
        }
    };

    let matchmaking = Arc::new(MatchmakingService::new(
        auth.clone(),
        matchmaking,
        game_servers,
        provisioner,
    ));

    let state = Arc::new(AppState {
        guest_sessions,
        matchmaking,
    });

    // Start the web server with the HTTP routes wired up.
    let app = routes::app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!(%addr, "listening");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%addr, error = %error, "failed to bind");
            return;
        }
    };

    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(error = %error, "server error");
    }
}
