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

    fn next_match_id(&self, _player_id: u64, _opponent_id: u64) -> String {
        // Match IDs are opaque external handles; roster membership lives in the
        // stored match record rather than being encoded into the identifier.
        format!("match-{}-{}", current_epoch_seconds(), Uuid::new_v4())
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

    #[test]
    fn generated_match_ids_are_opaque_and_distinct() {
        let generator = SystemMatchIdGenerator;
        let first = generator.next_match_id(12_345_678_901, 98_765_432_109);
        let second = generator.next_match_id(12_345_678_901, 98_765_432_109);

        assert_ne!(first, second);
        assert!(first.starts_with("match-"));
        assert!(!first.contains("12345678901"));
        assert!(!first.contains("98765432109"));
    }
}
