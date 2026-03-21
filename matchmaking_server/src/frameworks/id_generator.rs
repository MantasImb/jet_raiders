use crate::use_cases::matchmaker::MatchIdGenerator;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct SystemMatchIdGenerator;

impl MatchIdGenerator for SystemMatchIdGenerator {
    fn next_ticket_id(&self, player_id: u64) -> String {
        format!(
            "ticket-{}-{}-{}",
            current_epoch_seconds(),
            Uuid::new_v4(),
            player_id
        )
    }

    fn next_match_id(&self, player_id: u64, opponent_id: u64) -> String {
        let mut ids = [player_id, opponent_id];
        ids.sort_unstable();

        format!("match-{}-{}-{}", current_epoch_seconds(), ids[0], ids[1])
    }
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
        let generator = SystemMatchIdGenerator;
        let first = generator.next_ticket_id(1);
        let second = generator.next_ticket_id(1);

        assert_ne!(first, second);
    }
}
