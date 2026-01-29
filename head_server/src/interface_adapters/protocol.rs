use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct HeadGuestLoginRequest {
    // Guest ID supplied by the client if already known.
    pub guest_id: String,
    // Display name chosen by the client.
    pub display_name: String,
}

#[derive(Serialize)]
pub struct HeadGuestLoginResponse {
    // Session token returned by auth.
    pub session_token: String,
}
