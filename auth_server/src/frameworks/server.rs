use crate::frameworks::config::{load_auth_server_config, AuthServerConfigError, ProcessEnv};
use crate::frameworks::db;
use crate::interface_adapters::routes::app;
use crate::interface_adapters::state::AppState;
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupFailure {
    MissingRequiredConfig,
    InvalidConfiguration,
    DatabaseConnection,
    Migration,
    Bind,
    Serve,
}

impl StartupFailure {
    pub const fn exit_code(self) -> i32 {
        match self {
            StartupFailure::MissingRequiredConfig => 1,
            StartupFailure::InvalidConfiguration => 2,
            StartupFailure::DatabaseConnection => 3,
            StartupFailure::Migration => 4,
            StartupFailure::Bind => 5,
            StartupFailure::Serve => 6,
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
    let config = load_auth_server_config(&ProcessEnv).map_err(|error| match error {
        AuthServerConfigError::MissingEnvVar(key) => {
            tracing::error!(env_var = key, "required environment variable is missing");
            StartupFailure::MissingRequiredConfig
        }
        AuthServerConfigError::InvalidEnvVar { key, value } => {
            tracing::error!(
                env_var = key,
                value = %value,
                "environment variable has invalid numeric value"
            );
            StartupFailure::InvalidConfiguration
        }
        AuthServerConfigError::ReadPortsConfig(path) => {
            tracing::error!(
                backend_ports_config_path = %path.display(),
                "failed to read backend ports config"
            );
            StartupFailure::InvalidConfiguration
        }
        AuthServerConfigError::ParsePortsConfig(path) => {
            tracing::error!(
                backend_ports_config_path = %path.display(),
                "failed to parse backend ports config"
            );
            StartupFailure::InvalidConfiguration
        }
        AuthServerConfigError::MissingPortsConfigKey(key) => {
            tracing::error!(
                config_key = key,
                "backend ports config is missing required key"
            );
            StartupFailure::InvalidConfiguration
        }
        AuthServerConfigError::InvalidPortsConfigValue { key, value } => {
            tracing::error!(
                config_key = key,
                value,
                "backend ports config has invalid port value"
            );
            StartupFailure::InvalidConfiguration
        }
    })?;
    let db = initialize_database(&config.database_url).await?;

    // Shared, in-memory store for guest sessions.
    let state = AppState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
        db,
    };

    // Wire routes for the guest-only auth flow.
    let app = app(state);

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
    let listener = bind_listener(addr).await?;
    tracing::info!(%addr, "listening");

    axum::serve(listener, app).await.map_err(|error| {
        tracing::error!(error = %error, "server error");
        StartupFailure::Serve
    })
}

async fn initialize_database(database_url: &str) -> Result<PgPool, StartupFailure> {
    let pool = db::connect_pool(database_url).await.map_err(|error| {
        tracing::error!(error = %error, "failed to connect to database");
        StartupFailure::DatabaseConnection
    })?;

    db::run_migrations(&pool).await.map_err(|error| {
        tracing::error!(error = %error, "failed to run migrations");
        StartupFailure::Migration
    })?;

    Ok(pool)
}

async fn bind_listener(addr: SocketAddr) -> Result<TcpListener, StartupFailure> {
    TcpListener::bind(addr).await.map_err(|error| {
        tracing::error!("Failed to bind to address {}: {}", addr, error);
        StartupFailure::Bind
    })
}

#[cfg(test)]
mod tests {
    use super::StartupFailure;

    #[test]
    fn when_startup_failure_is_mapped_then_expected_exit_code_is_used() {
        assert_eq!(StartupFailure::MissingRequiredConfig.exit_code(), 1);
        assert_eq!(StartupFailure::InvalidConfiguration.exit_code(), 2);
        assert_eq!(StartupFailure::DatabaseConnection.exit_code(), 3);
        assert_eq!(StartupFailure::Migration.exit_code(), 4);
        assert_eq!(StartupFailure::Bind.exit_code(), 5);
        assert_eq!(StartupFailure::Serve.exit_code(), 6);
    }
}
