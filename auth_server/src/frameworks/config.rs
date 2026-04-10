use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthServerConfig {
    pub database_url: String,
    pub bind_host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthServerConfigError {
    MissingEnvVar(&'static str),
    InvalidEnvVar { key: &'static str, value: String },
    ReadPortsConfig(PathBuf),
    ParsePortsConfig(PathBuf),
    MissingPortsConfigKey(&'static str),
    InvalidPortsConfigValue { key: &'static str, value: u16 },
}

pub trait EnvSource {
    fn get_var(&self, key: &str) -> Option<String>;
}

pub struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

#[derive(Debug, Deserialize)]
struct BackendPortsConfig {
    ports: RawPorts,
}

#[derive(Debug, Deserialize)]
struct RawPorts {
    auth_server: Option<u16>,
}

pub fn load_auth_server_config(
    env: &impl EnvSource,
) -> Result<AuthServerConfig, AuthServerConfigError> {
    Ok(AuthServerConfig {
        database_url: required_env_var(env, "DATABASE_URL")?,
        bind_host: required_env_var(env, "AUTH_SERVER_BIND_HOST")?,
        port: resolve_auth_server_port(env)?,
    })
}

fn resolve_auth_server_port(env: &impl EnvSource) -> Result<u16, AuthServerConfigError> {
    if let Some(value) = env.get_var("AUTH_SERVER_PORT") {
        if value.is_empty() {
            return resolve_port_from_catalog(env);
        }

        let override_port =
            value
                .parse::<u16>()
                .map_err(|_| AuthServerConfigError::InvalidEnvVar {
                    key: "AUTH_SERVER_PORT",
                    value: value.clone(),
                })?;
        if override_port == 0 {
            return Err(AuthServerConfigError::InvalidEnvVar {
                key: "AUTH_SERVER_PORT",
                value,
            });
        }
        tracing::warn!(
            service = "auth_server",
            env_var = "AUTH_SERVER_PORT",
            override_port,
            "using service port override from environment"
        );
        return Ok(override_port);
    }

    resolve_port_from_catalog(env)
}

fn resolve_port_from_catalog(env: &impl EnvSource) -> Result<u16, AuthServerConfigError> {
    let backend_ports_path = resolve_backend_ports_path(env);
    let raw = std::fs::read_to_string(&backend_ports_path)
        .map_err(|_| AuthServerConfigError::ReadPortsConfig(backend_ports_path.clone()))?;

    let parsed: BackendPortsConfig = toml::from_str(&raw)
        .map_err(|_| AuthServerConfigError::ParsePortsConfig(backend_ports_path.clone()))?;
    let port = parsed
        .ports
        .auth_server
        .ok_or(AuthServerConfigError::MissingPortsConfigKey(
            "ports.auth_server",
        ))?;
    if port == 0 {
        return Err(AuthServerConfigError::InvalidPortsConfigValue {
            key: "ports.auth_server",
            value: port,
        });
    }
    Ok(port)
}

fn resolve_backend_ports_path(env: &impl EnvSource) -> PathBuf {
    if let Some(path) = env
        .get_var("BACKEND_PORTS_CONFIG_PATH")
        .filter(|value| !value.trim().is_empty())
    {
        return PathBuf::from(path);
    }

    for candidate in default_backend_ports_paths() {
        if candidate.exists() {
            return candidate;
        }
    }

    PathBuf::from("../config/backend_ports.toml")
}

fn default_backend_ports_paths() -> [PathBuf; 2] {
    [
        PathBuf::from("../config/backend_ports.toml"),
        PathBuf::from("/app/config/backend_ports.toml"),
    ]
}

fn required_env_var(
    env: &impl EnvSource,
    key: &'static str,
) -> Result<String, AuthServerConfigError> {
    env.get_var(key)
        .filter(|value| !value.trim().is_empty())
        .ok_or(AuthServerConfigError::MissingEnvVar(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn temp_config_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("auth-server-{label}-{nanos}.toml"))
    }

    struct TempConfigFile {
        path: PathBuf,
    }

    impl TempConfigFile {
        fn new(label: &str, contents: &str) -> Self {
            let path = temp_config_path(label);
            std::fs::write(&path, contents).expect("temporary config should be written");
            Self { path }
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }
    }

    impl Drop for TempConfigFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn load_auth_server_config_requires_database_url() {
        let config = load_auth_server_config(&TestEnv::default());

        assert!(matches!(
            config,
            Err(AuthServerConfigError::MissingEnvVar("DATABASE_URL"))
        ));
    }

    #[test]
    fn load_auth_server_config_requires_bind_host() {
        let config =
            load_auth_server_config(&TestEnv::from_pairs(&[("DATABASE_URL", "postgres://db")]));

        assert!(matches!(
            config,
            Err(AuthServerConfigError::MissingEnvVar(
                "AUTH_SERVER_BIND_HOST"
            ))
        ));
    }

    #[test]
    fn load_auth_server_config_reads_port_from_shared_catalog() {
        let config_file = TempConfigFile::new(
            "valid-shared-ports",
            r#"
[ports]
auth_server = 4102
"#,
        );

        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]))
        .expect("config should load");

        assert_eq!(config.port, 4102);
    }

    #[test]
    fn load_auth_server_config_uses_override_port_when_present() {
        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
            ("AUTH_SERVER_PORT", "4310"),
        ]))
        .expect("config should load");

        assert_eq!(config.port, 4310);
    }

    #[test]
    fn load_auth_server_config_treats_empty_override_as_unset() {
        let config_file = TempConfigFile::new(
            "empty-override-fallback",
            r#"
[ports]
auth_server = 4202
"#,
        );

        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
            ("AUTH_SERVER_PORT", ""),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]))
        .expect("config should load");

        assert_eq!(config.port, 4202);
    }

    #[test]
    fn load_auth_server_config_rejects_invalid_override_port() {
        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
            ("AUTH_SERVER_PORT", "not-a-number"),
        ]));

        assert!(matches!(
            config,
            Err(AuthServerConfigError::InvalidEnvVar {
                key: "AUTH_SERVER_PORT",
                ..
            })
        ));
    }

    #[test]
    fn load_auth_server_config_rejects_zero_override_port() {
        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
            ("AUTH_SERVER_PORT", "0"),
        ]));

        assert!(matches!(
            config,
            Err(AuthServerConfigError::InvalidEnvVar {
                key: "AUTH_SERVER_PORT",
                value,
            }) if value == "0"
        ));
    }

    #[test]
    fn load_auth_server_config_uses_default_ports_path_when_override_absent() {
        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
        ]))
        .expect("config should load from default backend ports path");

        assert_eq!(config.port, 3002);
    }

    #[test]
    fn load_auth_server_config_rejects_missing_auth_port_key() {
        let config_file = TempConfigFile::new(
            "missing-auth-key",
            r#"
[ports]
head_server = 3000
"#,
        );

        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]));

        assert!(matches!(
            config,
            Err(AuthServerConfigError::MissingPortsConfigKey(
                "ports.auth_server"
            ))
        ));
    }

    #[test]
    fn load_auth_server_config_rejects_zero_auth_port_key() {
        let config_file = TempConfigFile::new(
            "zero-auth-key",
            r#"
[ports]
auth_server = 0
"#,
        );

        let config = load_auth_server_config(&TestEnv::from_pairs(&[
            ("DATABASE_URL", "postgres://db"),
            ("AUTH_SERVER_BIND_HOST", "0.0.0.0"),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]));

        assert!(matches!(
            config,
            Err(AuthServerConfigError::InvalidPortsConfigValue {
                key: "ports.auth_server",
                value: 0,
            })
        ));
    }
}
