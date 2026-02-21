// Framework bootstrap for the game server runtime.

use crate::frameworks::config;
use crate::interface_adapters::clients::auth::AuthClient;
use crate::interface_adapters::net::{create_lobby_handler, spawn_lobby_serializer, ws_handler};
use crate::interface_adapters::state::AppState;
use crate::use_cases::{LobbyRegistry, LobbySettings};

use axum::{
    Router,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::{collections::HashSet, io::Result, sync::Arc, time::Duration};

fn init_runtime() {
    let _ = dotenvy::dotenv();

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

pub async fn run(listener: tokio::net::TcpListener) -> Result<()> {
    let address = listener.local_addr()?;
    // build state
    let state = build_state().await?;
    // Start the Web Server
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/lobbies", post(create_lobby_handler))
        .with_state(state);

    tracing::info!(%address, "listening");

    // Serve app and report errors rather than panicking
    axum::serve(listener, app).await.inspect_err(|e| {
        tracing::error!(error = %e, "server error");
    })
}

pub async fn run_with_config() -> Result<()> {
    init_runtime();

    let address = SocketAddr::from(([127, 0, 0, 1], config::http_port()));

    // Bind TCP listener with error handling
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .inspect_err(|e| {
            tracing::error!(%address, error = %e, "failed to bind");
        })?;

    run(listener).await
}

async fn build_state() -> Result<Arc<AppState>> {
    let auth_base_url = config::auth_service_url();
    let auth_verify_timeout = config::auth_verify_timeout();
    let auth_client = AuthClient::new(auth_base_url.clone(), auth_verify_timeout)
        .map_err(|e| std::io::Error::other(format!("failed to initialize auth client: {e}")))?;
    tracing::debug!(
        auth_base_url = %auth_base_url,
        auth_verify_timeout_ms = auth_verify_timeout.as_millis(),
        "auth client configured"
    );

    // Setup Lobby Registry
    // This owns the set of active lobby world tasks.
    let lobby_registry = Arc::new(LobbyRegistry::new(LobbySettings {
        input_channel_capacity: config::INPUT_CHANNEL_CAPACITY,
        world_broadcast_capacity: config::WORLD_BROADCAST_CAPACITY,
        tick_interval: config::TICK_INTERVAL,
        default_match_time_limit: config::DEFAULT_MATCH_TIME_LIMIT,
    }));

    // Create the default test lobby and spawn its world task.
    let test_lobby_id = "test".to_string();
    // Keep the default test lobby pinned so it never gets deleted.
    let test_lobby = lobby_registry
        .create_lobby(
            test_lobby_id.clone(),
            HashSet::new(),
            true,
            Duration::from_secs(0),
        )
        .await
        .expect("test lobby should initialize");
    spawn_lobby_serializer(&test_lobby);
    lobby_registry.clone().spawn_match_end_watcher(
        test_lobby.lobby_id.clone(),
        test_lobby.server_state_tx.subscribe(),
    );

    Ok(Arc::new(AppState {
        lobby_registry,
        default_lobby_id: Arc::from(test_lobby_id.as_str()),
        auth_client: Arc::new(auth_client),
    }))
}
