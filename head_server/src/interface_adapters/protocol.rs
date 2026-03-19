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

#[derive(Deserialize)]
pub struct HeadEnterMatchmakingRequest {
    // Auth session token used to resolve the canonical player identity.
    pub session_token: String,
    // Player skill is part of the current upstream queue contract.
    pub player_skill: u32,
    // Region preference used for the current queue lookup.
    pub region: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeadMatchmakingStatus {
    Waiting,
    Matched,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HeadMatchmakingResponse {
    // Queue status returned by head after delegating to matchmaking.
    pub status: HeadMatchmakingStatus,
    // Waiting ticket returned when no immediate match is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_id: Option<String>,
    // Match identifier returned when a match is found immediately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_id: Option<String>,
    // Opponent identifier currently surfaced from the upstream match result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent_id: Option<String>,
    // Region that the queue request was evaluated against.
    pub region: String,
}
