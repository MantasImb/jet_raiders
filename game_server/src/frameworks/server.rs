// Framework bootstrap for the game server runtime.

use crate::frameworks::config;
use crate::interface_adapters::net::{create_lobby_handler, spawn_lobby_serializers, ws_handler};
use crate::interface_adapters::state::AppState;
use crate::use_cases::{LobbyRegistry, LobbySettings};

use axum::{
    Router,
    routing::{get, post},
};
use std::{collections::HashSet, net::SocketAddr, sync::Arc, time::Duration};

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
    spawn_lobby_serializers(&test_lobby);
    lobby_registry.clone().spawn_match_end_watcher(
        test_lobby.lobby_id.clone(),
        test_lobby.server_state_tx.subscribe(),
    );

    let state = Arc::new(AppState {
        lobby_registry,
        default_lobby_id: Arc::from(test_lobby_id.as_str()),
    });

    // Start the Web Server
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/lobbies", post(create_lobby_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
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
