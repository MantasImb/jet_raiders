// Framework bootstrap for the game server runtime.

use crate::frameworks::config;
use crate::frameworks::config::{GameServerConfigError, ProcessEnv};
use crate::interface_adapters::clients::auth::AuthClient;
use crate::interface_adapters::http::health;
use crate::interface_adapters::net::{create_lobby_handler, spawn_lobby_serializer, ws_handler};
use crate::interface_adapters::state::AppState;
use crate::use_cases::{LobbyRegistry, LobbySettings};

use axum::{
    Router,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::{collections::HashSet, io::Result as IoResult, sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupFailure {
    MissingRequiredConfig,
    InvalidConfiguration,
    Initialization,
    Bind,
    Serve,
}

impl StartupFailure {
    pub const fn exit_code(self) -> i32 {
        match self {
            StartupFailure::MissingRequiredConfig => 1,
            StartupFailure::InvalidConfiguration => 2,
            StartupFailure::Initialization => 3,
            StartupFailure::Bind => 4,
            StartupFailure::Serve => 5,
        }
    }
}

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

pub async fn run(listener: tokio::net::TcpListener) -> IoResult<()> {
    let runtime_config = config::load_runtime_config(&ProcessEnv).map_err(|error| match error {
        GameServerConfigError::MissingEnvVar(key) => std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("missing required environment variable: {key}"),
        ),
        GameServerConfigError::InvalidEnvVar { key, value } => std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid environment variable {key}={value}"),
        ),
    })?;
    let state = build_state_with_auth_config(
        runtime_config.auth_service_url,
        runtime_config.auth_verify_timeout,
    )
    .await?;
    run_with_state(listener, state).await
}

/// Test-only compatibility entrypoint that bypasses runtime-config validation.
///
/// This preserves existing integration tests that spawn an ephemeral listener
/// and only need the server loop plus default auth client settings.
pub async fn run_for_tests(listener: tokio::net::TcpListener) -> IoResult<()> {
    let state =
        build_state_with_auth_config(config::auth_service_url(), config::auth_verify_timeout())
            .await?;
    run_with_state(listener, state).await
}

async fn run_with_state(listener: tokio::net::TcpListener, state: Arc<AppState>) -> IoResult<()> {
    let address = listener.local_addr()?;
    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .route("/lobbies", post(create_lobby_handler))
        .with_state(state);

    tracing::info!(%address, "listening");

    // Serve app and report errors rather than panicking
    axum::serve(listener, app).await.inspect_err(|e| {
        tracing::error!(error = %e, "server error");
    })
}

pub async fn run_with_config() -> std::result::Result<(), StartupFailure> {
    init_runtime();

    let runtime_config = config::load_runtime_config(&ProcessEnv).map_err(|error| match error {
        GameServerConfigError::MissingEnvVar(key) => {
            tracing::error!(env_var = key, "required environment variable is missing");
            StartupFailure::MissingRequiredConfig
        }
        GameServerConfigError::InvalidEnvVar { key, value } => {
            tracing::error!(
                env_var = key,
                value = %value,
                "environment variable has invalid numeric value"
            );
            StartupFailure::InvalidConfiguration
        }
    })?;

    let address = format!("{}:{}", runtime_config.bind_host, runtime_config.http_port)
        .parse::<SocketAddr>()
        .map_err(|error| {
            tracing::error!(
                bind_host = %runtime_config.bind_host,
                port = runtime_config.http_port,
                error = %error,
                "invalid bind host or port"
            );
            StartupFailure::InvalidConfiguration
        })?;

    // Bind TCP listener with error handling
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .inspect_err(|e| {
            tracing::error!(%address, error = %e, "failed to bind");
        })
        .map_err(|_| StartupFailure::Bind)?;

    let state = build_state_with_auth_config(
        runtime_config.auth_service_url,
        runtime_config.auth_verify_timeout,
    )
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "failed to initialize game server state");
        StartupFailure::Initialization
    })?;

    run_with_state(listener, state).await.map_err(|error| {
        tracing::error!(error = %error, "server error");
        StartupFailure::Serve
    })
}

async fn build_state_with_auth_config(
    auth_base_url: String,
    auth_verify_timeout: Duration,
) -> IoResult<Arc<AppState>> {
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

#[cfg(test)]
mod tests {
    use super::StartupFailure;

    #[test]
    fn startup_failures_map_to_expected_exit_codes() {
        assert_eq!(StartupFailure::MissingRequiredConfig.exit_code(), 1);
        assert_eq!(StartupFailure::InvalidConfiguration.exit_code(), 2);
        assert_eq!(StartupFailure::Initialization.exit_code(), 3);
        assert_eq!(StartupFailure::Bind.exit_code(), 4);
        assert_eq!(StartupFailure::Serve.exit_code(), 5);
    }
}
