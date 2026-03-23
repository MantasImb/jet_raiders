use serde::{Deserialize, Serialize};

// Request payload for enqueueing a player into matchmaking.
#[derive(Debug, Deserialize)]
pub struct QueueRequest {
    pub player_id: u64,
    pub player_skill: u32,
    pub region: String,
}

// Query parameters for owner-scoped ticket operations.
#[derive(Debug, Deserialize)]
pub struct TicketOwnerQuery {
    pub player_id: u64,
}

// Response payload returned after queue lifecycle operations.
#[derive(Debug, Serialize)]
pub struct QueueResponse {
    pub status: QueueStatus,
    pub ticket_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_ids: Option<Vec<u64>>,
    pub region: String,
}

// Outcome status for queue lifecycle responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Waiting,
    Matched,
    Canceled,
}

// Simple error envelope for JSON responses.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}
