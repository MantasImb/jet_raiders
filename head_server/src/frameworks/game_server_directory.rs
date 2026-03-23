use crate::use_cases::{GameServerDirectory, GameServerError, ResolvedGameServer};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticGameServerDirectory {
    default_target: ResolvedGameServer,
    regional_targets: HashMap<String, ResolvedGameServer>,
}

impl StaticGameServerDirectory {
    pub fn new(
        default_target: ResolvedGameServer,
        regional_targets: HashMap<String, ResolvedGameServer>,
    ) -> Self {
        Self {
            default_target,
            regional_targets,
        }
    }
}

#[async_trait]
impl GameServerDirectory for StaticGameServerDirectory {
    async fn resolve(&self, region: &str) -> Result<ResolvedGameServer, GameServerError> {
        Ok(self
            .regional_targets
            .get(region)
            .cloned()
            .unwrap_or_else(|| self.default_target.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(base_url: &str, ws_url: &str) -> ResolvedGameServer {
        ResolvedGameServer {
            base_url: base_url.into(),
            ws_url: ws_url.into(),
        }
    }

    #[tokio::test]
    async fn resolve_returns_exact_region_mapping_when_present() {
        let directory = StaticGameServerDirectory::new(
            target("http://default.internal", "ws://default/ws"),
            HashMap::from([(
                "eu-west".to_string(),
                target("http://eu.internal", "ws://eu/ws"),
            )]),
        );

        let result = directory
            .resolve("eu-west")
            .await
            .expect("resolution should succeed");

        assert_eq!(result, target("http://eu.internal", "ws://eu/ws"));
    }

    #[tokio::test]
    async fn resolve_falls_back_to_default_mapping() {
        let directory = StaticGameServerDirectory::new(
            target("http://default.internal", "ws://default/ws"),
            HashMap::new(),
        );

        let result = directory
            .resolve("us-east")
            .await
            .expect("resolution should succeed");

        assert_eq!(result, target("http://default.internal", "ws://default/ws"));
    }
}
