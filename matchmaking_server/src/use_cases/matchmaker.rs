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

// Errors that can occur while enqueueing a player.
#[derive(Debug)]
pub enum MatchError {
    AlreadyQueued { player_id: String },
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
    pub fn enqueue(&mut self, request: QueueRequest) -> Result<MatchOutcome, MatchError> {
        // NOTE: player_skill is not used for matching yet (MVP implementation).
        if self
            .queue
            .iter()
            .any(|player| player.player_id == request.player_id)
        {
            return Err(MatchError::AlreadyQueued {
                player_id: request.player_id,
            });
        }

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

            return Ok(MatchOutcome::Matched {
                match_id: build_match_id(&request.player_id, &opponent.player_id),
                opponent_id: opponent.player_id,
                region: request.region,
            });
        }

        let waiting_player = WaitingPlayer::new(
            request.player_id.clone(),
            request.player_skill,
            request.region.clone(),
        );

        self.queue.push_back(waiting_player);

        Ok(MatchOutcome::Waiting {
            ticket_id: build_ticket_id(&request.player_id),
            region: request.region,
        })
    }
}
