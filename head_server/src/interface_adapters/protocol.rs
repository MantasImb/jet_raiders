use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct HeadGuestInitRequest {
    // Initial display name for first-time guests.
    pub display_name: String,
}

#[derive(Serialize)]
pub struct HeadGuestInitResponse {
    // Guest identifier returned as a string for JSON precision safety in clients.
    pub guest_id: String,
    // Session token returned by auth.
    pub session_token: String,
    // Token expiration timestamp from auth.
    pub expires_at: u64,
}

#[derive(Deserialize)]
pub struct HeadGuestLoginRequest {
    // Guest ID supplied by the client as a string for JSON precision safety.
    pub guest_id: String,
    // Display name chosen by the client.
    pub display_name: String,
}

#[derive(Serialize)]
pub struct HeadGuestLoginResponse {
    // Session token returned by auth.
    pub session_token: String,
    // Token expiration timestamp from auth.
    pub expires_at: u64,
}
