use crate::domain::queue::{WaitingPlayer, build_match_id, build_ticket_id};
use crate::interface_adapters::protocol::QueueRequest;
use std::collections::VecDeque;

// Outcome returned after enqueueing a player into matchmaking.
#[derive(Debug)]
pub enum MatchOutcome {
    Waiting { ticket_id: String, region: String },
    Matched {
        match_id: String,
        opponent_id: String,
        region: String,
    },
}

// In-memory matchmaker that pairs players based on region.
#[derive(Debug, Default)]
pub struct Matchmaker {
    queue: VecDeque<WaitingPlayer>,
}

impl Matchmaker {
    // Create a new matchmaker with an empty queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    // Enqueue a player and attempt to find a match immediately.
    pub fn enqueue(&mut self, request: QueueRequest) -> MatchOutcome {
        if let Some((index, opponent)) = self
            .queue
            .iter()
            .enumerate()
            .find(|(_, player)| player.region == request.region)
        {
            let opponent = opponent.clone();
            // NOTE: VecDeque::remove(index) shifts elements after the index.
            // For better performance at scale, consider per-region queues or a
            // data structure that supports efficient arbitrary removals.
            self.queue.remove(index);

            return MatchOutcome::Matched {
                match_id: build_match_id(&request.player_id, &opponent.player_id),
                opponent_id: opponent.player_id,
                region: request.region,
            };
        }

        let waiting_player = WaitingPlayer::new(
            request.player_id.clone(),
            request.player_skill,
            request.region.clone(),
        );

        self.queue.push_back(waiting_player);

        MatchOutcome::Waiting {
            ticket_id: build_ticket_id(&request.player_id),
            region: request.region,
        }
    }
}
