use crate::interface_adapters::clients::auth::AuthClient;
use crate::use_cases::LobbyRegistry;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    // Shared registry of active lobbies.
    pub lobby_registry: Arc<LobbyRegistry>,
    // Default lobby id when a client does not specify one.
    pub default_lobby_id: Arc<str>,
    // Outbound auth service client used to verify join session tokens.
    pub auth_client: Arc<AuthClient>,
}
