use crate::frameworks::config::SharedRegionConfig;
use crate::use_cases::{GameServerDirectory, GameServerError, ResolvedGameServer};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticGameServerDirectory {
    regional_targets: HashMap<String, ResolvedGameServer>,
}

impl StaticGameServerDirectory {
    pub fn new(regional_targets: HashMap<String, ResolvedGameServer>) -> Self {
        Self { regional_targets }
    }

    pub fn from_shared_region_config(config: SharedRegionConfig) -> Self {
        let regional_targets = config
            .regions
            .into_iter()
            .map(|entry| (entry.matchmaking_key, entry.game_server))
            .collect();

        Self::new(regional_targets)
    }
}

#[async_trait]
impl GameServerDirectory for StaticGameServerDirectory {
    async fn resolve(&self, region: &str) -> Result<ResolvedGameServer, GameServerError> {
        self.regional_targets
            .get(region)
            .cloned()
            .ok_or_else(|| GameServerError::UnknownRegion {
                region: region.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frameworks::config::{RegionRoutingEntry, SharedRegionConfig};

    fn target(base_url: &str, ws_url: &str) -> ResolvedGameServer {
        ResolvedGameServer {
            base_url: base_url.into(),
            ws_url: ws_url.into(),
        }
    }

    #[tokio::test]
    async fn resolve_returns_exact_region_mapping_when_present() {
        let directory = StaticGameServerDirectory::new(HashMap::from([(
            "eu-west".to_string(),
            target("http://eu.internal", "ws://eu/ws"),
        )]));

        let result = directory
            .resolve("eu-west")
            .await
            .expect("resolution should succeed");

        assert_eq!(result, target("http://eu.internal", "ws://eu/ws"));
    }

    #[tokio::test]
    async fn from_shared_region_config_resolves_each_configured_region_exactly() {
        let directory = StaticGameServerDirectory::from_shared_region_config(SharedRegionConfig {
            regions: vec![
                RegionRoutingEntry {
                    matchmaking_key: "eu-west".into(),
                    game_server: target("http://eu.internal", "ws://eu/ws"),
                },
                RegionRoutingEntry {
                    matchmaking_key: "us-east".into(),
                    game_server: target("http://us.internal", "ws://us/ws"),
                },
            ],
        });

        let eu_result = directory
            .resolve("eu-west")
            .await
            .expect("eu-west resolution should succeed");
        let us_result = directory
            .resolve("us-east")
            .await
            .expect("us-east resolution should succeed");

        assert_eq!(eu_result, target("http://eu.internal", "ws://eu/ws"));
        assert_eq!(us_result, target("http://us.internal", "ws://us/ws"));
    }

    #[tokio::test]
    async fn resolve_rejects_unknown_regions() {
        let directory = StaticGameServerDirectory::new(HashMap::new());

        let result = directory.resolve("us-east").await;

        assert_eq!(
            result,
            Err(GameServerError::UnknownRegion {
                region: "us-east".into(),
            })
        );
    }
}
