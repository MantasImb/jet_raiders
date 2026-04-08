use std::{env, time::Duration};

// Runtime/server constants (not gameplay tuning).

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameServerRuntimeConfig {
    pub bind_host: String,
    pub http_port: u16,
    pub auth_service_url: String,
    pub auth_verify_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameServerConfigError {
    MissingEnvVar(&'static str),
    InvalidEnvVar { key: &'static str, value: String },
}

pub trait EnvSource {
    fn get_var(&self, key: &str) -> Option<String>;
}

pub struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn get_var(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

pub fn load_runtime_config(
    env: &impl EnvSource,
) -> Result<GameServerRuntimeConfig, GameServerConfigError> {
    let auth_verify_timeout_millis =
        parse_optional_u64(env, "AUTH_VERIFY_TIMEOUT_MS")?.unwrap_or(1500);

    Ok(GameServerRuntimeConfig {
        bind_host: required_env_var(env, "GAME_SERVER_BIND_HOST")?,
        http_port: parse_optional_u16(env, "GAME_SERVER_PORT")?.unwrap_or(3001),
        auth_service_url: env
            .get_var("AUTH_SERVICE_URL")
            .unwrap_or_else(|| "http://127.0.0.1:3002".to_string()),
        auth_verify_timeout: Duration::from_millis(auth_verify_timeout_millis),
    })
}

pub fn http_port() -> u16 {
    env::var("GAME_SERVER_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3001)
}

pub fn auth_service_url() -> String {
    env::var("AUTH_SERVICE_URL").unwrap_or_else(|_| "http://127.0.0.1:3002".to_string())
}

pub fn auth_verify_timeout() -> Duration {
    let millis = env::var("AUTH_VERIFY_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1500);
    Duration::from_millis(millis)
}

fn required_env_var(
    env: &impl EnvSource,
    key: &'static str,
) -> Result<String, GameServerConfigError> {
    env.get_var(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or(GameServerConfigError::MissingEnvVar(key))
}

fn parse_optional_u16(
    env: &impl EnvSource,
    key: &'static str,
) -> Result<Option<u16>, GameServerConfigError> {
    match env.get_var(key) {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            trimmed
                .parse::<u16>()
                .map(Some)
                .map_err(|_| GameServerConfigError::InvalidEnvVar {
                    key,
                    value: value.to_string(),
                })
        }
        None => Ok(None),
    }
}

fn parse_optional_u64(
    env: &impl EnvSource,
    key: &'static str,
) -> Result<Option<u64>, GameServerConfigError> {
    match env.get_var(key) {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            trimmed
                .parse::<u64>()
                .map(Some)
                .map_err(|_| GameServerConfigError::InvalidEnvVar {
                    key,
                    value: value.to_string(),
                })
        }
        None => Ok(None),
    }
}
pub const INPUT_CHANNEL_CAPACITY: usize = 1024;
pub const WORLD_BROADCAST_CAPACITY: usize = 128;

pub const TICK_INTERVAL: Duration = Duration::from_millis(1000 / 60);
// Default time limit for non-test lobbies (0 disables match end).
pub const DEFAULT_MATCH_TIME_LIMIT: Duration = Duration::from_secs(600);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct TestEnv {
        vars: HashMap<String, String>,
    }

    impl TestEnv {
        fn from_pairs(pairs: &[(&str, &str)]) -> Self {
            Self {
                vars: pairs
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            }
        }
    }

    impl EnvSource for TestEnv {
        fn get_var(&self, key: &str) -> Option<String> {
            self.vars.get(key).cloned()
        }
    }

    #[test]
    fn load_runtime_config_requires_bind_host() {
        let config = load_runtime_config(&TestEnv::default());

        assert!(matches!(
            config,
            Err(GameServerConfigError::MissingEnvVar(
                "GAME_SERVER_BIND_HOST"
            ))
        ));
    }

    #[test]
    fn load_runtime_config_reads_env_and_defaults() {
        let config = load_runtime_config(&TestEnv::from_pairs(&[
            ("GAME_SERVER_BIND_HOST", " 127.0.0.1 "),
            ("GAME_SERVER_PORT", "5001"),
            ("AUTH_SERVICE_URL", "http://auth.internal:9000"),
            ("AUTH_VERIFY_TIMEOUT_MS", "3200"),
        ]))
        .expect("runtime config should load");

        assert_eq!(config.bind_host, "127.0.0.1");
        assert_eq!(config.http_port, 5001);
        assert_eq!(config.auth_service_url, "http://auth.internal:9000");
        assert_eq!(config.auth_verify_timeout, Duration::from_millis(3200));
    }

    #[test]
    fn load_runtime_config_rejects_invalid_numeric_env_values() {
        let invalid_port = load_runtime_config(&TestEnv::from_pairs(&[
            ("GAME_SERVER_BIND_HOST", "127.0.0.1"),
            ("GAME_SERVER_PORT", "not-a-port"),
        ]));
        assert!(matches!(
            invalid_port,
            Err(GameServerConfigError::InvalidEnvVar {
                key: "GAME_SERVER_PORT",
                ..
            })
        ));

        let invalid_timeout = load_runtime_config(&TestEnv::from_pairs(&[
            ("GAME_SERVER_BIND_HOST", "127.0.0.1"),
            ("AUTH_VERIFY_TIMEOUT_MS", "oops"),
        ]));
        assert!(matches!(
            invalid_timeout,
            Err(GameServerConfigError::InvalidEnvVar {
                key: "AUTH_VERIFY_TIMEOUT_MS",
                ..
            })
        ));
    }
}
