use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// The serialization within this layer is a dependency leak, but its a pragmatic approach
// Payload sent to the auth service when creating a guest session.
#[derive(Serialize)]
pub struct AuthGuestRequest {
    // Guest identity data sent to auth for session creation.
    pub guest_id: String,
    pub display_name: String,
}

// Payload returned by the auth service after guest session creation.
#[derive(Deserialize)]
pub struct AuthGuestResponse {
    // Token issued by auth for client use.
    pub token: String,
}

// The handler depends on this trait, not the concrete client implementation.
// Dependencies point inwards to the domain layer.
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn create_guest_session(
        &self,
        req: AuthGuestRequest,
    ) -> Result<AuthGuestResponse, Box<dyn std::error::Error>>;
}
