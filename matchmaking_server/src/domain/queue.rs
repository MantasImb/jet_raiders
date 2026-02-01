use std::time::{SystemTime, UNIX_EPOCH};

// A player waiting to be matched.
#[derive(Debug, Clone)]
pub struct WaitingPlayer {
    pub player_id: String,
    pub player_skill: u32,
    pub region: String,
    pub enqueued_at: u64,
}

impl WaitingPlayer {
    // Create a new waiting player record with a timestamp.
    pub fn new(player_id: String, player_skill: u32, region: String) -> Self {
        Self {
            player_id,
            player_skill,
            region,
            enqueued_at: current_epoch_seconds(),
        }
    }
}

// Build a simple queue ticket identifier.
pub fn build_ticket_id(player_id: &str) -> String {
    format!("ticket-{}-{}", current_epoch_seconds(), player_id)
}

// Build a simple match identifier.
pub fn build_match_id(player_id: &str, opponent_id: &str) -> String {
    format!(
        "match-{}-{}-{}",
        current_epoch_seconds(),
        player_id,
        opponent_id
    )
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
