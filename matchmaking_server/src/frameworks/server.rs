use crate::frameworks::config::{
    MatchmakingServerConfigError, ProcessEnv, load_matchmaking_server_config,
    load_shared_region_catalog,
};
use crate::frameworks::id_generator::SystemMatchIdGenerator;
use crate::interface_adapters::routes;
use crate::interface_adapters::state::AppState;
use crate::use_cases::matchmaker::Matchmaker;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupFailure {
    MissingRequiredConfig,
    InvalidConfiguration,
    // Reserved for future startup dependency initialization failures.
    #[allow(dead_code)]
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
    let config = load_matchmaking_server_config(&ProcessEnv).map_err(|error| match error {
        MatchmakingServerConfigError::MissingEnvVar(key) => {
            tracing::error!(env_var = key, "required environment variable is missing");
            StartupFailure::MissingRequiredConfig
        }
        MatchmakingServerConfigError::InvalidEnvVar { key, value } => {
            tracing::error!(
                env_var = key,
                value = %value,
                "environment variable has invalid numeric value"
            );
            StartupFailure::InvalidConfiguration
        }
        MatchmakingServerConfigError::ReadPortsConfig(path) => {
            tracing::error!(
                backend_ports_config_path = %path.display(),
                "failed to read backend ports config"
            );
            StartupFailure::InvalidConfiguration
        }
        MatchmakingServerConfigError::ParsePortsConfig(path) => {
            tracing::error!(
                backend_ports_config_path = %path.display(),
                "failed to parse backend ports config"
            );
            StartupFailure::InvalidConfiguration
        }
        MatchmakingServerConfigError::MissingPortsConfigKey(key) => {
            tracing::error!(
                config_key = key,
                "backend ports config is missing required key"
            );
            StartupFailure::InvalidConfiguration
        }
        MatchmakingServerConfigError::InvalidPortsConfigValue { key, value } => {
            tracing::error!(
                config_key = key,
                value,
                "backend ports config has invalid port value"
            );
            StartupFailure::InvalidConfiguration
        }
    })?;

    tracing::debug!(
        region_config_path = %config.region_config_path.display(),
        "shared region config path configured."
    );
    let shared_region_catalog = match load_shared_region_catalog(&config.region_config_path) {
        Ok(loaded_region_catalog) => loaded_region_catalog,
        Err(error) => {
            tracing::error!(
                region_config_path = %config.region_config_path.display(),
                error = %error,
                "failed to load shared region config"
            );
            return Err(StartupFailure::InvalidConfiguration);
        }
    };

    // Initialize the in-memory matchmaking queue.
    let state = Arc::new(AppState {
        allowed_regions: Arc::new(shared_region_catalog.matchmaking_keys),
        matchmaker: Arc::new(Mutex::new(Matchmaker::new(Arc::new(
            SystemMatchIdGenerator,
        )))),
    });

    // Wire the HTTP routes for the matchmaking API.
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

    // Bind TCP listener with error handling.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%addr, %error, "failed to bind");
            return Err(StartupFailure::Bind);
        }
    };

    // Serve app and report errors rather than panicking.
    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(%error, "server error");
        return Err(StartupFailure::Serve);
    }

    Ok(())
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
