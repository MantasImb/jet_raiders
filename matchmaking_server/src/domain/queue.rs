use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// A player waiting to be matched.
#[derive(Debug, Clone)]
pub struct WaitingPlayer {
    pub ticket_id: String,
    pub player_id: String,
    pub player_skill: u32,
    pub region: String,
    pub enqueued_at: u64,
}

impl WaitingPlayer {
    // Create a new waiting player record with a timestamp.
    pub fn new(ticket_id: String, player_id: String, player_skill: u32, region: String) -> Self {
        Self {
            ticket_id,
            player_id,
            player_skill,
            region,
            enqueued_at: current_epoch_seconds(),
        }
    }
}

// Build a simple queue ticket identifier.
pub fn build_ticket_id(player_id: &str) -> String {
    // Include a random nonce so rapid re-queues by the same player produce
    // distinct ticket IDs even within the same second.
    format!(
        "ticket-{}-{}-{}",
        current_epoch_seconds(),
        Uuid::new_v4(),
        player_id
    )
}

// Build a simple match identifier with deterministically ordered player IDs.
pub fn build_match_id(player_id: &str, opponent_id: &str) -> String {
    let mut ids = [player_id, opponent_id];
    // Sort the player IDs alphabetically to keep the match ID stable.
    ids.sort_unstable();

    format!("match-{}-{}-{}", current_epoch_seconds(), ids[0], ids[1])
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_ticket_generation_for_the_same_player_returns_distinct_ids() {
        let first = build_ticket_id("player-1");
        let second = build_ticket_id("player-1");

        assert_ne!(first, second);
    }
}
