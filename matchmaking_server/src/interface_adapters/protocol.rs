use serde::{Deserialize, Serialize};

// Request payload for enqueueing a player into matchmaking.
#[derive(Debug, Deserialize)]
pub struct QueueRequest {
    pub player_id: String,
    pub player_skill: u32,
    pub region: String,
}

// Response payload returned after attempting to enqueue a player.
#[derive(Debug, Serialize)]
pub struct QueueResponse {
    pub status: QueueStatus,
    pub ticket_id: Option<String>,
    pub match_id: Option<String>,
    pub opponent_id: Option<String>,
    pub region: String,
}

// Outcome status for the queue response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Waiting,
    Matched,
}

// Simple error envelope for JSON responses.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}
