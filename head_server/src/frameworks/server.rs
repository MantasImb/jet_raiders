use crate::frameworks::auth_client::AuthClient;
use crate::frameworks::config::{
    HeadServerConfigError, ProcessEnv, load_head_server_config, load_shared_region_config,
};
use crate::frameworks::game_server_client::GameServerClient;
use crate::frameworks::game_server_directory::StaticGameServerDirectory;
use crate::frameworks::matchmaking_client::MatchmakingClient;
use crate::interface_adapters::routes;
use crate::interface_adapters::state::AppState;
use crate::use_cases::{GuestSessionService, MatchmakingService};
use reqwest::{Client, Url};
use std::net::SocketAddr;
use std::sync::Arc;

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

pub async fn run() -> Result<(), StartupFailure> {
    // Load .env locally; safe to ignore when not present.
    let _ = dotenvy::dotenv();
    init_tracing();

    let config = load_head_server_config(&ProcessEnv).map_err(|error| match error {
        HeadServerConfigError::MissingEnvVar(key) => {
            tracing::error!(env_var = key, "required environment variable is missing");
            StartupFailure::MissingRequiredConfig
        }
        HeadServerConfigError::InvalidEnvVar { key, value } => {
            tracing::error!(
                env_var = key,
                value = %value,
                "environment variable has invalid numeric value"
            );
            StartupFailure::InvalidConfiguration
        }
        HeadServerConfigError::ReadPortsConfig(path) => {
            tracing::error!(
                backend_ports_config_path = %path.display(),
                "failed to read backend ports config"
            );
            StartupFailure::InvalidConfiguration
        }
        HeadServerConfigError::ParsePortsConfig(path) => {
            tracing::error!(
                backend_ports_config_path = %path.display(),
                "failed to parse backend ports config"
            );
            StartupFailure::InvalidConfiguration
        }
        HeadServerConfigError::MissingPortsConfigKey(key) => {
            tracing::error!(
                config_key = key,
                "backend ports config is missing required key"
            );
            StartupFailure::InvalidConfiguration
        }
        HeadServerConfigError::InvalidPortsConfigValue { key, value } => {
            tracing::error!(
                config_key = key,
                value,
                "backend ports config has invalid port value"
            );
            StartupFailure::InvalidConfiguration
        }
    })?;
    let startup_http = Client::new();
    check_upstream_health(&startup_http, &config.auth_service_url, "auth").await?;
    check_upstream_health(
        &startup_http,
        &config.matchmaking_service_url,
        "matchmaking",
    )
    .await?;

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
            return Err(StartupFailure::InvalidConfiguration);
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
            return Err(StartupFailure::InvalidConfiguration);
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
            return Err(StartupFailure::InvalidConfiguration);
        }
    };
    let game_servers = Arc::new(StaticGameServerDirectory::from_shared_region_config(
        shared_region_config,
    ));

    let provisioner = match GameServerClient::new() {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(?error, "failed to build game server client");
            return Err(StartupFailure::Initialization);
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

    let addr = format!("{}:{}", config.bind_host, config.port)
        .parse::<SocketAddr>()
        .map_err(|error| {
            tracing::error!(
                bind_host = %config.bind_host,
                port = config.port,
                error = %error,
                "invalid bind host or port"
            );
            StartupFailure::InvalidConfiguration
        })?;
    tracing::info!(%addr, "listening");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%addr, error = %error, "failed to bind");
            return Err(StartupFailure::Bind);
        }
    };

    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(error = %error, "server error");
        return Err(StartupFailure::Serve);
    }

    Ok(())
}

async fn check_upstream_health(
    http: &Client,
    base_url: &str,
    service_name: &'static str,
) -> Result<(), StartupFailure> {
    let mut health_url = Url::parse(base_url).map_err(|error| {
        tracing::error!(
            upstream = service_name,
            base_url = %base_url,
            error = %error,
            "failed to parse upstream URL for startup health check"
        );
        StartupFailure::InvalidConfiguration
    })?;
    health_url.set_path("/health");
    health_url.set_query(None);
    health_url.set_fragment(None);

    let response = http.get(health_url.clone()).send().await.map_err(|error| {
        tracing::error!(
            upstream = service_name,
            health_url = %health_url,
            error = %error,
            "upstream health check request failed"
        );
        StartupFailure::Initialization
    })?;
    let status = response.status();
    if !status.is_success() {
        tracing::error!(
            upstream = service_name,
            health_url = %health_url,
            status = %status,
            "upstream health check returned non-success status"
        );
        return Err(StartupFailure::Initialization);
    }

    tracing::info!(
        upstream = service_name,
        health_url = %health_url,
        status = %status,
        "upstream health check passed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{StartupFailure, check_upstream_health};
    use axum::{Json, Router, http::StatusCode, routing::get};
    use reqwest::Client;
    use serde_json::json;
    use tokio::net::TcpListener;

    #[test]
    fn startup_failures_map_to_expected_exit_codes() {
        assert_eq!(StartupFailure::MissingRequiredConfig.exit_code(), 1);
        assert_eq!(StartupFailure::InvalidConfiguration.exit_code(), 2);
        assert_eq!(StartupFailure::Initialization.exit_code(), 3);
        assert_eq!(StartupFailure::Bind.exit_code(), 4);
        assert_eq!(StartupFailure::Serve.exit_code(), 5);
    }

    async fn spawn_test_server(router: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("address should be available");

        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("test server should run");
        });

        format!("http://{address}")
    }

    #[tokio::test]
    async fn upstream_health_check_passes_on_successful_response() {
        let router = Router::new().route(
            "/health",
            get(|| async { (StatusCode::OK, Json(json!({ "status": "ok" }))) }),
        );
        let base_url = spawn_test_server(router).await;

        let result = check_upstream_health(&Client::new(), &base_url, "auth").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn upstream_health_check_fails_on_non_success_response() {
        let router = Router::new().route("/health", get(|| async { StatusCode::BAD_GATEWAY }));
        let base_url = spawn_test_server(router).await;

        let result = check_upstream_health(&Client::new(), &base_url, "matchmaking").await;

        assert_eq!(result, Err(StartupFailure::Initialization));
    }
}
