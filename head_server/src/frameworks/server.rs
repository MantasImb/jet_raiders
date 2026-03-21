use crate::frameworks::auth_client::AuthClient;
use crate::frameworks::game_server_client::GameServerClient;
use crate::frameworks::game_server_directory::StaticGameServerDirectory;
use crate::frameworks::matchmaking_client::MatchmakingClient;
use crate::interface_adapters::routes;
use crate::interface_adapters::state::AppState;
use crate::use_cases::{GuestSessionService, MatchmakingService, ResolvedGameServer};
use std::collections::HashMap;
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

    let default_game_server = ResolvedGameServer {
        base_url: std::env::var("GAME_SERVER_DEFAULT_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3001".into()),
        ws_url: std::env::var("GAME_SERVER_DEFAULT_WS_URL")
            .unwrap_or_else(|_| "ws://localhost:3001/ws".into()),
    };
    let regional_game_servers = load_regional_game_servers();
    let game_servers = Arc::new(StaticGameServerDirectory::new(
        default_game_server,
        regional_game_servers,
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

fn load_regional_game_servers() -> HashMap<String, ResolvedGameServer> {
    let Ok(raw_mappings) = std::env::var("GAME_SERVER_REGION_MAP") else {
        return HashMap::new();
    };

    raw_mappings
        .split(';')
        .filter_map(|mapping| {
            let trimmed = mapping.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut parts = trimmed.splitn(3, '=');
            let region = parts.next()?.trim();
            let base_url = parts.next()?.trim();
            let ws_url = parts.next()?.trim();

            if region.is_empty() || base_url.is_empty() || ws_url.is_empty() {
                tracing::warn!(mapping = %trimmed, "ignoring invalid GAME_SERVER_REGION_MAP entry");
                return None;
            }

            Some((
                region.to_string(),
                ResolvedGameServer {
                    base_url: base_url.to_string(),
                    ws_url: ws_url.to_string(),
                },
            ))
        })
        .collect()
}
