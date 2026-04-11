use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchmakingServerConfig {
    pub bind_host: String,
    pub port: u16,
    pub region_config_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchmakingServerConfigError {
    MissingEnvVar(&'static str),
    InvalidEnvVar { key: &'static str, value: String },
    ReadPortsConfig(PathBuf),
    ParsePortsConfig(PathBuf),
    MissingPortsConfigKey(&'static str),
    InvalidPortsConfigValue { key: &'static str, value: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedRegionCatalog {
    pub matchmaking_keys: HashSet<String>,
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
    matchmaking_server: Option<u16>,
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

pub fn load_matchmaking_server_config(
    env: &impl EnvSource,
) -> Result<MatchmakingServerConfig, MatchmakingServerConfigError> {
    Ok(MatchmakingServerConfig {
        bind_host: required_env_var(env, "MATCHMAKING_SERVER_BIND_HOST")?,
        port: resolve_matchmaking_server_port(env)?,
        region_config_path: env
            .get_var("REGION_CONFIG_PATH")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .ok_or(MatchmakingServerConfigError::MissingEnvVar(
                "REGION_CONFIG_PATH",
            ))?,
    })
}

pub fn load_shared_region_catalog(
    path: impl AsRef<Path>,
) -> Result<SharedRegionCatalog, SharedRegionConfigError> {
    let raw = std::fs::read_to_string(path).map_err(SharedRegionConfigError::ReadFailed)?;
    parse_shared_region_catalog(&raw)
}

fn parse_shared_region_catalog(raw: &str) -> Result<SharedRegionCatalog, SharedRegionConfigError> {
    let parsed: RawSharedRegionConfig =
        toml::from_str(raw).map_err(SharedRegionConfigError::ParseFailed)?;

    if parsed.regions.is_empty() {
        return Err(SharedRegionConfigError::NoRegionsDeclared);
    }

    let mut matchmaking_keys = HashSet::new();

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
        // Matchmaking reads the same shared artifact as head, so startup should fail
        // if the routing schema is malformed rather than accepting a partial config.
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
        if !matches!(parsed_game_server_ws_url.scheme(), "ws" | "wss") {
            return Err(SharedRegionConfigError::InvalidGameServerWsUrlScheme {
                region_entry: region_entry.clone(),
                scheme: parsed_game_server_ws_url.scheme().to_string(),
            });
        }

        if !matchmaking_keys.insert(matchmaking_key.clone()) {
            return Err(SharedRegionConfigError::DuplicateMatchmakingKey { matchmaking_key });
        }
    }

    Ok(SharedRegionCatalog { matchmaking_keys })
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
) -> Result<String, MatchmakingServerConfigError> {
    env.get_var(key)
        .filter(|value| !value.trim().is_empty())
        .ok_or(MatchmakingServerConfigError::MissingEnvVar(key))
}

fn resolve_matchmaking_server_port(
    env: &impl EnvSource,
) -> Result<u16, MatchmakingServerConfigError> {
    if let Some(value) = env.get_var("MATCHMAKING_SERVER_PORT") {
        if value.is_empty() {
            return resolve_port_from_catalog(env);
        }

        let override_port =
            value
                .parse::<u16>()
                .map_err(|_| MatchmakingServerConfigError::InvalidEnvVar {
                    key: "MATCHMAKING_SERVER_PORT",
                    value: value.clone(),
                })?;
        if override_port == 0 {
            return Err(MatchmakingServerConfigError::InvalidEnvVar {
                key: "MATCHMAKING_SERVER_PORT",
                value,
            });
        }
        tracing::warn!(
            service = "matchmaking_server",
            env_var = "MATCHMAKING_SERVER_PORT",
            override_port,
            "using service port override from environment"
        );
        return Ok(override_port);
    }

    resolve_port_from_catalog(env)
}

fn resolve_port_from_catalog(env: &impl EnvSource) -> Result<u16, MatchmakingServerConfigError> {
    let backend_ports_path = resolve_backend_ports_path(env);
    let raw = std::fs::read_to_string(&backend_ports_path)
        .map_err(|_| MatchmakingServerConfigError::ReadPortsConfig(backend_ports_path.clone()))?;
    let parsed: BackendPortsConfig = toml::from_str(&raw)
        .map_err(|_| MatchmakingServerConfigError::ParsePortsConfig(backend_ports_path.clone()))?;
    let port = parsed.ports.matchmaking_server.ok_or(
        MatchmakingServerConfigError::MissingPortsConfigKey("ports.matchmaking_server"),
    )?;
    if port == 0 {
        return Err(MatchmakingServerConfigError::InvalidPortsConfigValue {
            key: "ports.matchmaking_server",
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

    fn temp_config_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("matchmaking-server-{label}-{nanos}.toml"))
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

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempConfigFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn load_shared_region_catalog_reads_concrete_regions() {
        let config_file = TempConfigFile::new(
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

        let result = load_shared_region_catalog(config_file.path()).expect("config should load");

        assert_eq!(
            result.matchmaking_keys,
            HashSet::from(["eu-west".to_string(), "us-east".to_string()])
        );
    }

    #[test]
    fn load_shared_region_catalog_rejects_duplicate_matchmaking_keys() {
        let config_file = TempConfigFile::new(
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

        let result = load_shared_region_catalog(config_file.path());

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::DuplicateMatchmakingKey { matchmaking_key })
                if matchmaking_key == "eu-west"
        ));
    }

    #[test]
    fn load_shared_region_catalog_rejects_mismatched_matchmaking_keys() {
        let config_file = TempConfigFile::new(
            "mismatched-key",
            r#"
[regions.eu_ne]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_catalog(config_file.path());

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
    fn load_shared_region_catalog_rejects_missing_required_fields() {
        let config_file = TempConfigFile::new(
            "missing-fields",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_ws_url = "ws://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_catalog(config_file.path());

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::MissingField { region_entry, field })
                if region_entry == "eu_west" && field == "game_server_base_url"
        ));
    }

    #[test]
    fn load_shared_region_catalog_rejects_invalid_game_server_urls() {
        let config_file = TempConfigFile::new(
            "invalid-urls",
            r#"
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "ftp://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_catalog(config_file.path());

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::InvalidGameServerBaseUrlScheme {
                region_entry,
                scheme,
            }) if region_entry == "eu_west" && scheme == "ftp"
        ));
    }

    #[test]
    fn load_shared_region_catalog_rejects_malformed_toml() {
        let config_file = TempConfigFile::new(
            "malformed-toml",
            r#"
[regions.eu_west
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"
"#,
        );

        let result = load_shared_region_catalog(config_file.path());

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::ParseFailed(_))
        ));
    }

    #[test]
    fn load_shared_region_catalog_rejects_empty_region_set() {
        let config_file = TempConfigFile::new(
            "no-regions",
            r#"
[regions]
"#,
        );

        let result = load_shared_region_catalog(config_file.path());

        assert!(matches!(
            result,
            Err(SharedRegionConfigError::NoRegionsDeclared)
        ));
    }

    #[test]
    fn load_matchmaking_server_config_requires_bind_host() {
        let config = load_matchmaking_server_config(&TestEnv::default());

        assert!(matches!(
            config,
            Err(MatchmakingServerConfigError::MissingEnvVar(
                "MATCHMAKING_SERVER_BIND_HOST"
            ))
        ));
    }

    #[test]
    fn load_matchmaking_server_config_uses_env_override() {
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("MATCHMAKING_SERVER_PORT", "3350"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
        ]))
        .expect("config should load");

        assert_eq!(config.bind_host, "127.0.0.1");
        assert_eq!(config.port, 3350);
        assert_eq!(
            config.region_config_path,
            PathBuf::from("/tmp/regions.custom.toml")
        );
    }

    #[test]
    fn load_matchmaking_server_config_requires_region_config_path() {
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("MATCHMAKING_SERVER_PORT", "3003"),
        ]));

        assert!(matches!(
            config,
            Err(MatchmakingServerConfigError::MissingEnvVar(
                "REGION_CONFIG_PATH"
            ))
        ));
    }

    #[test]
    fn load_matchmaking_server_config_reads_port_from_shared_catalog() {
        let config_file = TempConfigFile::new(
            "matchmaking-port",
            r#"
[ports]
matchmaking_server = 3450
"#,
        );
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]))
        .expect("config should load");

        assert_eq!(config.port, 3450);
    }

    #[test]
    fn load_matchmaking_server_config_treats_empty_override_as_unset() {
        let config_file = TempConfigFile::new(
            "empty-override",
            r#"
[ports]
matchmaking_server = 3550
"#,
        );
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("MATCHMAKING_SERVER_PORT", ""),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]))
        .expect("config should load");

        assert_eq!(config.port, 3550);
    }

    #[test]
    fn load_matchmaking_server_config_rejects_invalid_override() {
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("MATCHMAKING_SERVER_PORT", "nope"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
        ]));

        assert!(matches!(
            config,
            Err(MatchmakingServerConfigError::InvalidEnvVar {
                key: "MATCHMAKING_SERVER_PORT",
                ..
            })
        ));
    }

    #[test]
    fn load_matchmaking_server_config_rejects_zero_override() {
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("MATCHMAKING_SERVER_PORT", "0"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
        ]));

        assert!(matches!(
            config,
            Err(MatchmakingServerConfigError::InvalidEnvVar {
                key: "MATCHMAKING_SERVER_PORT",
                value,
            }) if value == "0"
        ));
    }

    #[test]
    fn load_matchmaking_server_config_uses_default_ports_path_without_override() {
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
        ]))
        .expect("config should load");

        assert_eq!(config.port, 3003);
    }

    #[test]
    fn load_matchmaking_server_config_rejects_missing_matchmaking_port_key() {
        let config_file = TempConfigFile::new(
            "missing-matchmaking-key",
            r#"
[ports]
auth_server = 3002
"#,
        );
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]));

        assert!(matches!(
            config,
            Err(MatchmakingServerConfigError::MissingPortsConfigKey(
                "ports.matchmaking_server"
            ))
        ));
    }

    #[test]
    fn load_matchmaking_server_config_rejects_zero_matchmaking_port_key() {
        let config_file = TempConfigFile::new(
            "zero-matchmaking-key",
            r#"
[ports]
matchmaking_server = 0
"#,
        );
        let config = load_matchmaking_server_config(&TestEnv::from_pairs(&[
            ("MATCHMAKING_SERVER_BIND_HOST", "127.0.0.1"),
            ("REGION_CONFIG_PATH", "/tmp/regions.custom.toml"),
            (
                "BACKEND_PORTS_CONFIG_PATH",
                config_file.path().to_string_lossy().as_ref(),
            ),
        ]));

        assert!(matches!(
            config,
            Err(MatchmakingServerConfigError::InvalidPortsConfigValue {
                key: "ports.matchmaking_server",
                value: 0,
            })
        ));
    }
}
