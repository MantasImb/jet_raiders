use crate::use_cases::ResolvedGameServer;
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadServerConfig {
    pub bind_host: String,
    pub port: u16,
    pub auth_service_url: String,
    pub matchmaking_service_url: String,
    pub region_config_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadServerConfigError {
    MissingEnvVar(&'static str),
    InvalidEnvVar { key: &'static str, value: String },
    ReadPortsConfig(PathBuf),
    ParsePortsConfig(PathBuf),
    MissingPortsConfigKey(&'static str),
    InvalidPortsConfigValue { key: &'static str, value: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionRoutingEntry {
    pub matchmaking_key: String,
    pub game_server: ResolvedGameServer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedRegionConfig {
    pub regions: Vec<RegionRoutingEntry>,
}

#[derive(Debug)]
pub enum SharedRegionConfigError {
    ReadFailed(std::io::Error),
    ParseFailed(toml::de::Error),
    NoRegionsDeclared,
    MissingField {
        region_entry: String,
        field: &'static str,
    },
    DuplicateMatchmakingKey {
        matchmaking_key: String,
    },
    MismatchedMatchmakingKey {
        region_entry: String,
        matchmaking_key: String,
        expected_matchmaking_key: String,
    },
    InvalidGameServerBaseUrl {
        region_entry: String,
        source: url::ParseError,
    },
    InvalidGameServerBaseUrlScheme {
        region_entry: String,
        scheme: String,
    },
    InvalidGameServerWsUrl {
        region_entry: String,
        source: url::ParseError,
    },
    InvalidGameServerWsUrlScheme {
        region_entry: String,
        scheme: String,
    },
}

impl fmt::Display for SharedRegionConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedRegionConfigError::ReadFailed(error) => {
                write!(f, "failed to read shared region config: {error}")
            }
            SharedRegionConfigError::ParseFailed(error) => {
                write!(f, "failed to parse shared region config TOML: {error}")
            }
            SharedRegionConfigError::NoRegionsDeclared => {
                write!(f, "shared region config must declare at least one region")
            }
            SharedRegionConfigError::MissingField {
                region_entry,
                field,
            } => {
                write!(
                    f,
                    "shared region config entry '{region_entry}' is missing required field '{field}'"
                )
            }
            SharedRegionConfigError::DuplicateMatchmakingKey { matchmaking_key } => {
                write!(
                    f,
                    "shared region config declares duplicate matchmaking_key '{matchmaking_key}'"
                )
            }
            SharedRegionConfigError::MismatchedMatchmakingKey {
                region_entry,
                matchmaking_key,
                expected_matchmaking_key,
            } => {
                write!(
                    f,
                    "shared region config entry '{region_entry}' declares matchmaking_key '{matchmaking_key}', expected '{expected_matchmaking_key}'"
                )
            }
            SharedRegionConfigError::InvalidGameServerBaseUrl {
                region_entry,
                source,
            } => {
                write!(
                    f,
                    "shared region config entry '{region_entry}' has invalid game_server_base_url: {source}"
                )
            }
            SharedRegionConfigError::InvalidGameServerBaseUrlScheme {
                region_entry,
                scheme,
            } => {
                write!(
                    f,
                    "shared region config entry '{region_entry}' has invalid game_server_base_url scheme '{scheme}'; expected http or https"
                )
            }
            SharedRegionConfigError::InvalidGameServerWsUrl {
                region_entry,
                source,
            } => {
                write!(
                    f,
                    "shared region config entry '{region_entry}' has invalid game_server_ws_url: {source}"
                )
            }
            SharedRegionConfigError::InvalidGameServerWsUrlScheme {
                region_entry,
                scheme,
            } => {
                write!(
                    f,
                    "shared region config entry '{region_entry}' has invalid game_server_ws_url scheme '{scheme}'; expected ws or wss"
                )
            }
        }
    }
}

impl std::error::Error for SharedRegionConfigError {}

#[derive(Debug, Deserialize)]
struct RawSharedRegionConfig {
    regions: std::collections::BTreeMap<String, RawRegionRoutingEntry>,
}

#[derive(Debug, Deserialize)]
struct RawRegionRoutingEntry {
    matchmaking_key: Option<String>,
    game_server_base_url: Option<String>,
    game_server_ws_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BackendPortsConfig {
    ports: RawBackendPorts,
}

#[derive(Debug, Deserialize)]
struct RawBackendPorts {
    head_server: Option<u16>,
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

pub fn load_shared_region_config(
    path: impl AsRef<Path>,
) -> Result<SharedRegionConfig, SharedRegionConfigError> {
    let raw = std::fs::read_to_string(path).map_err(SharedRegionConfigError::ReadFailed)?;
    parse_shared_region_config(&raw)
}

pub fn load_head_server_config(
    env: &impl EnvSource,
) -> Result<HeadServerConfig, HeadServerConfigError> {
    Ok(HeadServerConfig {
        bind_host: required_env_var(env, "HEAD_SERVER_BIND_HOST")?,
        port: resolve_head_server_port(env)?,
        auth_service_url: env
            .get_var("AUTH_SERVICE_URL")
            .unwrap_or_else(|| "http://localhost:3002".into()),
        matchmaking_service_url: env
            .get_var("MATCHMAKING_SERVICE_URL")
            .unwrap_or_else(|| "http://localhost:3003".into()),
        region_config_path: env
            .get_var("REGION_CONFIG_PATH")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .ok_or(HeadServerConfigError::MissingEnvVar("REGION_CONFIG_PATH"))?,
    })
}

fn parse_shared_region_config(raw: &str) -> Result<SharedRegionConfig, SharedRegionConfigError> {
    let parsed: RawSharedRegionConfig =
        toml::from_str(raw).map_err(SharedRegionConfigError::ParseFailed)?;

    if parsed.regions.is_empty() {
        return Err(SharedRegionConfigError::NoRegionsDeclared);
    }

    let mut seen_matchmaking_keys = HashSet::new();
    let mut regions = Vec::with_capacity(parsed.regions.len());

    for (region_entry, raw_entry) in parsed.regions {
        let matchmaking_key =
            required_field(raw_entry.matchmaking_key, &region_entry, "matchmaking_key")?;
        validate_matchmaking_key(&matchmaking_key, &region_entry)?;
        let game_server_base_url = required_field(
            raw_entry.game_server_base_url,
            &region_entry,
            "game_server_base_url",
        )?;
        let game_server_ws_url = required_field(
            raw_entry.game_server_ws_url,
            &region_entry,
            "game_server_ws_url",
        )?;

        let parsed_game_server_base_url =
            url::Url::parse(&game_server_base_url).map_err(|source| {
                SharedRegionConfigError::InvalidGameServerBaseUrl {
                    region_entry: region_entry.clone(),
                    source,
                }
            })?;
        // Lobby creation uses this URL as an HTTP endpoint, so startup must reject
        // syntactically valid but unsupported schemes before runtime handoff fails.
        if !matches!(parsed_game_server_base_url.scheme(), "http" | "https") {
            return Err(SharedRegionConfigError::InvalidGameServerBaseUrlScheme {
                region_entry: region_entry.clone(),
                scheme: parsed_game_server_base_url.scheme().to_string(),
            });
        }
        let parsed_game_server_ws_url = url::Url::parse(&game_server_ws_url).map_err(|source| {
            SharedRegionConfigError::InvalidGameServerWsUrl {
                region_entry: region_entry.clone(),
                source,
            }
        })?;
        // Match handoff returns this URL directly to clients, so startup must reject
        // non-WebSocket schemes instead of letting them fail later at connect time.
        if !matches!(parsed_game_server_ws_url.scheme(), "ws" | "wss") {
            return Err(SharedRegionConfigError::InvalidGameServerWsUrlScheme {
                region_entry: region_entry.clone(),
                scheme: parsed_game_server_ws_url.scheme().to_string(),
            });
        }

        if !seen_matchmaking_keys.insert(matchmaking_key.clone()) {
            return Err(SharedRegionConfigError::DuplicateMatchmakingKey { matchmaking_key });
        }

        regions.push(RegionRoutingEntry {
            matchmaking_key,
            game_server: ResolvedGameServer {
                base_url: game_server_base_url,
                ws_url: game_server_ws_url,
            },
        });
    }

    Ok(SharedRegionConfig { regions })
}

fn required_field(
    value: Option<String>,
    region_entry: &str,
    field: &'static str,
) -> Result<String, SharedRegionConfigError> {
    match value {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(SharedRegionConfigError::MissingField {
            region_entry: region_entry.to_string(),
            field,
        }),
    }
}

fn validate_matchmaking_key(
    matchmaking_key: &str,
    region_entry: &str,
) -> Result<(), SharedRegionConfigError> {
    let expected_matchmaking_key = region_entry.replace('_', "-");

    if matchmaking_key != expected_matchmaking_key {
        return Err(SharedRegionConfigError::MismatchedMatchmakingKey {
            region_entry: region_entry.to_string(),
            matchmaking_key: matchmaking_key.to_string(),
            expected_matchmaking_key,
        });
    }

    Ok(())
}

fn required_env_var(
    env: &impl EnvSource,
    key: &'static str,
) -> Result<String, HeadServerConfigError> {
    env.get_var(key)
        .filter(|value| !value.trim().is_empty())
        .ok_or(HeadServerConfigError::MissingEnvVar(key))
}

fn resolve_head_server_port(env: &impl EnvSource) -> Result<u16, HeadServerConfigError> {
    if let Some(value) = env.get_var("HEAD_SERVER_PORT") {
        if value.is_empty() {
            return resolve_port_from_catalog(env);
        }

        let override_port =
            value
                .parse::<u16>()
                .map_err(|_| HeadServerConfigError::InvalidEnvVar {
                    key: "HEAD_SERVER_PORT",
                    value: value.clone(),
                })?;
        if override_port == 0 {
            return Err(HeadServerConfigError::InvalidEnvVar {
                key: "HEAD_SERVER_PORT",
                value,
            });
        }
        tracing::warn!(
            service = "head_server",
            env_var = "HEAD_SERVER_PORT",
            override_port,
            "using service port override from environment"
        );
        return Ok(override_port);
    }

    resolve_port_from_catalog(env)
}

fn resolve_port_from_catalog(env: &impl EnvSource) -> Result<u16, HeadServerConfigError> {
    let backend_ports_path = resolve_backend_ports_path(env);
    let raw = std::fs::read_to_string(&backend_ports_path)
        .map_err(|_| HeadServerConfigError::ReadPortsConfig(backend_ports_path.clone()))?;
    let parsed: BackendPortsConfig = toml::from_str(&raw)
        .map_err(|_| HeadServerConfigError::ParsePortsConfig(backend_ports_path.clone()))?;
    let port = parsed
        .ports
        .head_server
        .ok_or(HeadServerConfigError::MissingPortsConfigKey(
            "ports.head_server",
        ))?;
    if port == 0 {
        return Err(HeadServerConfigError::InvalidPortsConfigValue {
            key: "ports.head_server",
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

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../config/backend_ports.toml")
}

fn default_backend_ports_paths() -> [PathBuf; 2] {
    [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../config/backend_ports.toml"),
        PathBuf::from("/app/config/backend_ports.toml"),
    ]
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

    fn temp_config_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("head-server-{label}-{nanos}.toml"))
    }

    fn write_temp_config(label: &str, contents: &str) -> std::path::PathBuf {
        let path = temp_config_path(label);
        std::fs::write(&path, contents).expect("temporary config should be written");
        path
    }

    #[test]
    fn load_shared_region_config_reads_concrete_regions() {
        let path = write_temp_config(
            "valid-regions",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"

[regions.us_east]
matchmaking_key = "us-east"
game_server_base_url = "http://localhost:3002"
game_server_ws_url = "ws://localhost:3002/ws"
"#,
        );

        let result = load_shared_region_config(&path).expect("config should load");

        assert_eq!(
            result,
            SharedRegionConfig {
                regions: vec![
                    RegionRoutingEntry {
                        matchmaking_key: "eu-west".into(),
                        game_server: ResolvedGameServer {
                            base_url: "http://localhost:3001".into(),
                            ws_url: "ws://localhost:3001/ws".into(),
                        },
                    },
                    RegionRoutingEntry {
                        matchmaking_key: "us-east".into(),
                        game_server: ResolvedGameServer {
                            base_url: "http://localhost:3002".into(),
                            ws_url: "ws://localhost:3002/ws".into(),
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn load_shared_region_config_rejects_duplicate_matchmaking_keys() {
        let path = write_temp_config(
            "duplicate-keys",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"

[regions."eu-west"]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3002"
game_server_ws_url = "ws://localhost:3002/ws"
"#,
        );

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::DuplicateMatchmakingKey { matchmaking_key })
                if matchmaking_key == "eu-west"
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_mismatched_matchmaking_keys() {
        let path = write_temp_config(
            "mismatched-key",
            r#"
[regions.eu_ne]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::MismatchedMatchmakingKey {
                region_entry,
                matchmaking_key,
                expected_matchmaking_key,
            }) if region_entry == "eu_ne"
                && matchmaking_key == "eu-west"
                && expected_matchmaking_key == "eu-ne"
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_missing_required_fields() {
        let path = write_temp_config(
            "missing-field",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
"#,
        );

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::MissingField {
                region_entry,
                field: "game_server_ws_url",
            }) if region_entry == "eu_west"
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_unreadable_paths() {
        let path = temp_config_path("missing-file");

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::ReadFailed(_))
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_malformed_toml() {
        let path = write_temp_config("malformed", "[regions.eu_west");

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::ParseFailed(_))
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_empty_region_catalog() {
        let path = write_temp_config("empty-regions", "[regions]");

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::NoRegionsDeclared)
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_invalid_game_server_urls() {
        let path = write_temp_config(
            "invalid-url",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "not-a-url"
game_server_ws_url = "ws://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::InvalidGameServerBaseUrl { .. })
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_non_http_game_server_base_urls() {
        let path = write_temp_config(
            "invalid-base-scheme",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "ftp://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::InvalidGameServerBaseUrlScheme {
                region_entry,
                scheme,
            }) if region_entry == "eu_west" && scheme == "ftp"
        ));
    }

    #[test]
    fn load_shared_region_config_rejects_non_websocket_game_server_ws_urls() {
        let path = write_temp_config(
            "invalid-ws-scheme",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "http://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_config(&path);

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::InvalidGameServerWsUrlScheme {
                region_entry,
                scheme,
            }) if region_entry == "eu_west" && scheme == "http"
        ));
    }

    #[test]
    fn load_head_server_config_requires_bind_host_and_region_config_path() {
        let env = TestEnv::default();
        let config = load_head_server_config(&env);

        assert!(matches!(
            config,
            Err(HeadServerConfigError::MissingEnvVar(
                "HEAD_SERVER_BIND_HOST"
            ))
        ));
    }

    #[test]
    fn load_head_server_config_reads_env_overrides() {
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("HEAD_SERVER_PORT", "3400"),
            ("AUTH_SERVICE_URL", "http://auth.internal:9000"),
            (
                "MATCHMAKING_SERVICE_URL",
                "http://matchmaking.internal:9001",
            ),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
        ]);

        let config = load_head_server_config(&env).expect("config should load");

        assert_eq!(config.bind_host, "127.0.0.1");
        assert_eq!(config.port, 3400);
        assert_eq!(config.auth_service_url, "http://auth.internal:9000");
        assert_eq!(
            config.matchmaking_service_url,
            "http://matchmaking.internal:9001"
        );
        assert_eq!(
            config.region_config_path,
            PathBuf::from("/tmp/regions.custom.toml")
        );
    }

    #[test]
    fn load_head_server_config_requires_region_config_path() {
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("HEAD_SERVER_PORT", "3000"),
        ]);
        let config = load_head_server_config(&env);

        assert!(matches!(
            config,
            Err(HeadServerConfigError::MissingEnvVar("REGION_CONFIG_PATH"))
        ));
    }

    #[test]
    fn load_head_server_config_reads_port_from_shared_catalog() {
        let path = write_temp_config(
            "head-port",
            r#"
[ports]
head_server = 3200
"#,
        );
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            ("BACKEND_PORTS_CONFIG_PATH", path.to_string_lossy().as_ref()),
        ]);

        let config = load_head_server_config(&env).expect("config should load");

        assert_eq!(config.port, 3200);
    }

    #[test]
    fn load_head_server_config_treats_empty_port_override_as_unset() {
        let path = write_temp_config(
            "head-empty-override",
            r#"
[ports]
head_server = 3300
"#,
        );
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("HEAD_SERVER_PORT", ""),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            ("BACKEND_PORTS_CONFIG_PATH", path.to_string_lossy().as_ref()),
        ]);

        let config = load_head_server_config(&env).expect("config should load");

        assert_eq!(config.port, 3300);
    }

    #[test]
    fn load_head_server_config_rejects_invalid_port_override() {
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("HEAD_SERVER_PORT", "not-a-port"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
        ]);

        let config = load_head_server_config(&env);

        assert!(matches!(
            config,
            Err(HeadServerConfigError::InvalidEnvVar {
                key: "HEAD_SERVER_PORT",
                ..
            })
        ));
    }

    #[test]
    fn load_head_server_config_rejects_zero_port_override() {
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("HEAD_SERVER_PORT", "0"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
        ]);

        let config = load_head_server_config(&env);

        assert!(matches!(
            config,
            Err(HeadServerConfigError::InvalidEnvVar {
                key: "HEAD_SERVER_PORT",
                value,
            }) if value == "0"
        ));
    }

    #[test]
    fn load_head_server_config_uses_ports_config_path_when_override_absent() {
        let path = write_temp_config(
            "head-default-via-path",
            r#"
[ports]
head_server = 3000
"#,
        );
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            ("BACKEND_PORTS_CONFIG_PATH", path.to_string_lossy().as_ref()),
        ]);

        let config = load_head_server_config(&env).expect("config should load");

        assert_eq!(config.port, 3000);
    }

    #[test]
    fn load_head_server_config_rejects_missing_head_port_key() {
        let path = write_temp_config(
            "missing-head-key",
            r#"
[ports]
auth_server = 3002
"#,
        );
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            ("BACKEND_PORTS_CONFIG_PATH", path.to_string_lossy().as_ref()),
        ]);

        let config = load_head_server_config(&env);

        assert!(matches!(
            config,
            Err(HeadServerConfigError::MissingPortsConfigKey(
                "ports.head_server"
            ))
        ));
    }

    #[test]
    fn load_head_server_config_rejects_zero_head_port_key() {
        let path = write_temp_config(
            "zero-head-key",
            r#"
[ports]
head_server = 0
"#,
        );
        let env = TestEnv::from_pairs(&[
            ("HEAD_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            ("BACKEND_PORTS_CONFIG_PATH", path.to_string_lossy().as_ref()),
        ]);

        let config = load_head_server_config(&env);

        assert!(matches!(
            config,
            Err(HeadServerConfigError::InvalidPortsConfigValue {
                key: "ports.head_server",
                value: 0,
            })
        ));
    }
}
