use crate::domain::queue::{WaitingPlayer, build_match_id, build_ticket_id};
use std::collections::{HashMap, VecDeque};

// Application request for queueing a player into matchmaking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueuePlayer {
    pub player_id: String,
    pub player_skill: u32,
    pub region: String,
}

// Outcome returned after enqueueing a player into matchmaking.
#[derive(Debug)]
pub enum MatchOutcome {
    Waiting {
        ticket_id: String,
        region: String,
    },
    Matched {
        match_id: String,
        opponent_id: String,
        region: String,
    },
}

// Status returned when polling a previously issued matchmaking ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketStatus {
    Waiting {
        ticket_id: String,
        region: String,
    },
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

// Errors that can occur while looking up a ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketLookupError {
    NotFound { ticket_id: String },
}

#[derive(Debug, Clone)]
struct CompletedMatch {
    match_id: String,
    opponent_id: String,
    region: String,
}

// In-memory matchmaker that pairs players based on region.
#[derive(Debug, Default)]
pub struct Matchmaker {
    queue: VecDeque<WaitingPlayer>,
    active_tickets_by_player: HashMap<String, String>,
    waiting_tickets_by_id: HashMap<String, WaitingPlayer>,
    completed_matches_by_ticket: HashMap<String, CompletedMatch>,
}

impl Matchmaker {
    // Create a new matchmaker with an empty queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            active_tickets_by_player: HashMap::new(),
            waiting_tickets_by_id: HashMap::new(),
            completed_matches_by_ticket: HashMap::new(),
        }
    }

    // Enqueue a player and attempt to find a match immediately.
    pub fn enqueue(&mut self, request: EnqueuePlayer) -> Result<MatchOutcome, MatchError> {
        // NOTE: player_skill is not used for matching yet (MVP implementation).
        if self
            .active_tickets_by_player
            .contains_key(request.player_id.as_str())
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
            self.active_tickets_by_player
                .remove(opponent.player_id.as_str());
            self.waiting_tickets_by_id
                .remove(opponent.ticket_id.as_str());

            let match_id = build_match_id(&request.player_id, &opponent.player_id);
            self.completed_matches_by_ticket.insert(
                opponent.ticket_id,
                CompletedMatch {
                    match_id: match_id.clone(),
                    opponent_id: request.player_id.clone(),
                    region: request.region.clone(),
                },
            );
            // TODO: completed_matches_by_ticket currently grows monotonically.
            // Add a retention policy such as TTL eviction, size-bounded
            // pruning, or removal after the completed ticket has been polled
            // and acknowledged.

            return Ok(MatchOutcome::Matched {
                match_id,
                opponent_id: opponent.player_id,
                region: request.region,
            });
        }

        let ticket_id = build_ticket_id(&request.player_id);
        let waiting_player = WaitingPlayer::new(
            ticket_id.clone(),
            request.player_id.clone(),
            request.player_skill,
            request.region.clone(),
        );

        self.queue.push_back(waiting_player.clone());
        self.active_tickets_by_player
            .insert(request.player_id, ticket_id.clone());
        self.waiting_tickets_by_id
            .insert(ticket_id.clone(), waiting_player);

        Ok(MatchOutcome::Waiting {
            ticket_id,
            region: request.region,
        })
    }

    // Look up the current status of a previously issued ticket.
    pub fn lookup_ticket(&self, ticket_id: &str) -> Result<TicketStatus, TicketLookupError> {
        if let Some(waiting_player) = self.waiting_tickets_by_id.get(ticket_id) {
            return Ok(TicketStatus::Waiting {
                ticket_id: waiting_player.ticket_id.clone(),
                region: waiting_player.region.clone(),
            });
        }

        if let Some(completed_match) = self.completed_matches_by_ticket.get(ticket_id) {
            return Ok(TicketStatus::Matched {
                match_id: completed_match.match_id.clone(),
                opponent_id: completed_match.opponent_id.clone(),
                region: completed_match.region.clone(),
            });
        }

        Err(TicketLookupError::NotFound {
            ticket_id: ticket_id.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_request(player_id: &str, region: &str) -> EnqueuePlayer {
        EnqueuePlayer {
            player_id: player_id.to_string(),
            player_skill: 1200,
            region: region.to_string(),
        }
    }

    #[test]
    fn queued_ticket_can_be_polled_while_still_waiting() {
        let mut matchmaker = Matchmaker::new();
        let outcome = matchmaker
            .enqueue(queue_request("player-1", "eu-west"))
            .expect("enqueue should succeed");

        let MatchOutcome::Waiting { ticket_id, region } = outcome else {
            panic!("first player should be queued");
        };
        assert_eq!(region, "eu-west");

        assert_eq!(
            matchmaker.lookup_ticket(ticket_id.as_str()),
            Ok(TicketStatus::Waiting {
                ticket_id,
                region: "eu-west".into(),
            })
        );
    }

    #[test]
    fn duplicate_enqueue_returns_already_queued_error() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .enqueue(queue_request("player-1", "eu-west"))
            .expect("first enqueue should succeed");

        let result = matchmaker.enqueue(queue_request("player-1", "us-east"));

        assert!(matches!(
            result,
            Err(MatchError::AlreadyQueued { player_id }) if player_id == "player-1"
        ));
    }

    #[test]
    fn queued_ticket_transitions_to_matched_after_an_opponent_arrives() {
        let mut matchmaker = Matchmaker::new();
        let waiting_outcome = matchmaker
            .enqueue(queue_request("player-1", "eu-west"))
            .expect("first enqueue should succeed");
        let MatchOutcome::Waiting { ticket_id, .. } = waiting_outcome else {
            panic!("first player should be queued");
        };

        let matched_outcome = matchmaker
            .enqueue(queue_request("player-2", "eu-west"))
            .expect("second enqueue should succeed");

        let MatchOutcome::Matched {
            match_id,
            opponent_id,
            region,
        } = matched_outcome
        else {
            panic!("second player should be matched immediately");
        };
        assert_eq!(opponent_id, "player-1");
        assert_eq!(region, "eu-west");

        assert_eq!(
            matchmaker.lookup_ticket(ticket_id.as_str()),
            Ok(TicketStatus::Matched {
                match_id,
                opponent_id: "player-2".into(),
                region: "eu-west".into(),
            })
        );
    }

    #[test]
    fn unknown_ticket_returns_not_found() {
        let matchmaker = Matchmaker::new();

        assert_eq!(
            matchmaker.lookup_ticket("missing-ticket"),
            Err(TicketLookupError::NotFound {
                ticket_id: "missing-ticket".into(),
            })
        );
    }
}
