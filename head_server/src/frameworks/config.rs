use crate::use_cases::ResolvedGameServer;
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadServerConfig {
    pub auth_service_url: String,
    pub matchmaking_service_url: String,
    pub region_config_path: PathBuf,
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
    InvalidGameServerWsUrl {
        region_entry: String,
        source: url::ParseError,
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
            SharedRegionConfigError::InvalidGameServerWsUrl {
                region_entry,
                source,
            } => {
                write!(
                    f,
                    "shared region config entry '{region_entry}' has invalid game_server_ws_url: {source}"
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

pub fn load_shared_region_config(
    path: impl AsRef<Path>,
) -> Result<SharedRegionConfig, SharedRegionConfigError> {
    let raw = std::fs::read_to_string(path).map_err(SharedRegionConfigError::ReadFailed)?;
    parse_shared_region_config(&raw)
}

pub fn load_head_server_config() -> HeadServerConfig {
    HeadServerConfig {
        auth_service_url: std::env::var("AUTH_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3002".into()),
        matchmaking_service_url: std::env::var("MATCHMAKING_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3003".into()),
        region_config_path: std::env::var("REGION_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("../config/regions.toml")),
    }
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

        let _ = url::Url::parse(&game_server_base_url).map_err(|source| {
            SharedRegionConfigError::InvalidGameServerBaseUrl {
                region_entry: region_entry.clone(),
                source,
            }
        })?;
        let _ = url::Url::parse(&game_server_ws_url).map_err(|source| {
            SharedRegionConfigError::InvalidGameServerWsUrl {
                region_entry: region_entry.clone(),
                source,
            }
        })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }

        fn unset(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
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
    fn load_head_server_config_uses_defaults_when_env_is_unset() {
        let _lock = env_lock();
        let _auth_guard = EnvVarGuard::unset("AUTH_SERVICE_URL");
        let _matchmaking_guard = EnvVarGuard::unset("MATCHMAKING_SERVICE_URL");
        let _region_guard = EnvVarGuard::unset("REGION_CONFIG_PATH");

        let config = load_head_server_config();

        assert_eq!(config.auth_service_url, "http://localhost:3002");
        assert_eq!(config.matchmaking_service_url, "http://localhost:3003");
        assert_eq!(
            config.region_config_path,
            PathBuf::from("../config/regions.toml")
        );
    }

    #[test]
    fn load_head_server_config_reads_env_overrides() {
        let _lock = env_lock();
        let _auth_guard = EnvVarGuard::set("AUTH_SERVICE_URL", "http://auth.internal:9000");
        let _matchmaking_guard = EnvVarGuard::set(
            "MATCHMAKING_SERVICE_URL",
            "http://matchmaking.internal:9001",
        );
        let _region_guard = EnvVarGuard::set("REGION_CONFIG_PATH", "/tmp/regions.custom.toml");

        let config = load_head_server_config();

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
}
