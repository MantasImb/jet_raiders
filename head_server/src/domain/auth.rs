use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Payload sent to the auth service for first-time guest identity creation.
#[derive(Serialize)]
pub struct AuthGuestInitRequest {
    pub display_name: String,
}

// Payload returned by the auth service for first-time guest identity creation.
#[derive(Deserialize)]
pub struct AuthGuestInitResponse {
    pub guest_id: u64,
    pub token: String,
    pub expires_at: u64,
}

// The serialization within this layer is a dependency leak, but its a pragmatic approach
// Payload sent to the auth service when creating a guest session.
#[derive(Serialize)]
pub struct AuthGuestRequest {
    // Guest identity data sent to auth for session creation.
    pub guest_id: u64,
    pub display_name: String,
}

// Payload returned by the auth service after guest session creation.
#[derive(Deserialize)]
pub struct AuthGuestResponse {
    // Token issued by auth for client use.
    pub token: String,
    pub expires_at: u64,
}

// The handler depends on this trait, not the concrete client implementation.
// Dependencies point inwards to the domain layer.
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn create_guest_identity(
        &self,
        req: AuthGuestInitRequest,
    ) -> Result<AuthGuestInitResponse, Box<dyn std::error::Error>>;

    async fn create_guest_session(
        &self,
        req: AuthGuestRequest,
    ) -> Result<AuthGuestResponse, Box<dyn std::error::Error>>;
}
