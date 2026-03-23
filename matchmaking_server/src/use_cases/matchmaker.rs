use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tracing::info;

// Application request for queueing a player into matchmaking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueuePlayer {
    pub player_id: u64,
    pub player_skill: u32,
    pub region: String,
}

// Shared lifecycle response returned by enqueue, lookup, and cancel flows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketStatus {
    Waiting {
        ticket_id: String,
        region: String,
    },
    Matched {
        ticket_id: String,
        match_id: String,
        player_ids: Vec<u64>,
        region: String,
    },
    Canceled {
        ticket_id: String,
        region: String,
    },
}

// Errors that can occur while looking up a ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketLookupError {
    NotFound { ticket_id: String },
    Unauthorized { ticket_id: String },
}

// Errors that can occur while canceling a ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelTicketError {
    NotFound { ticket_id: String },
    Unauthorized { ticket_id: String },
    Matched { ticket_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MatchRecord {
    match_id: String,
    player_ids: Vec<u64>,
    region: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TicketRecord {
    Waiting {
        player_id: u64,
        player_skill: u32,
        region: String,
    },
    Matched {
        player_id: u64,
        match_id: String,
    },
    Canceled {
        player_id: u64,
        region: String,
    },
}

// In-memory matchmaker that pairs players based on region.
pub trait MatchIdGenerator: Send + Sync {
    fn next_ticket_id(&self, player_id: u64) -> String;
    fn next_match_id(&self, player_id: u64, opponent_id: u64) -> String;
}

pub struct Matchmaker {
    ids: Arc<dyn MatchIdGenerator>,
    queue: VecDeque<String>,
    tickets_by_id: HashMap<String, TicketRecord>,
    active_ticket_by_player: HashMap<u64, String>,
    matches_by_id: HashMap<String, MatchRecord>,
}

impl Matchmaker {
    // Create a new matchmaker with an empty queue.
    pub fn new(ids: Arc<dyn MatchIdGenerator>) -> Self {
        Self {
            ids,
            queue: VecDeque::new(),
            tickets_by_id: HashMap::new(),
            active_ticket_by_player: HashMap::new(),
            matches_by_id: HashMap::new(),
        }
    }

    // Enqueue a player and attempt to find a match immediately.
    pub fn enqueue(&mut self, request: EnqueuePlayer) -> TicketStatus {
        if let Some(existing_ticket_id) = self
            .active_ticket_by_player
            .get(&request.player_id)
            .cloned()
        {
            return self
                .status_for_ticket(existing_ticket_id.as_str())
                .expect("active ticket should always resolve");
        }

        self.discard_canceled_tickets_for_player(request.player_id);

        if let Some(opponent_ticket_id) = self.find_waiting_opponent_ticket(request.region.as_str())
        {
            return self.create_match(opponent_ticket_id, request);
        }

        let ticket_id = self.ids.next_ticket_id(request.player_id);
        self.tickets_by_id.insert(
            ticket_id.clone(),
            TicketRecord::Waiting {
                player_id: request.player_id,
                player_skill: request.player_skill,
                region: request.region.clone(),
            },
        );
        self.queue.push_back(ticket_id.clone());
        self.active_ticket_by_player
            .insert(request.player_id, ticket_id.clone());

        // Emit lifecycle telemetry only when the queue state changes.
        info!(
            ticket_id = %ticket_id,
            player_id = request.player_id,
            player_skill = request.player_skill,
            region = %request.region,
            "created waiting matchmaking ticket"
        );

        TicketStatus::Waiting {
            ticket_id,
            region: request.region,
        }
    }

    // Look up the current status of a previously issued ticket.
    pub fn lookup_ticket(
        &self,
        player_id: u64,
        ticket_id: &str,
    ) -> Result<TicketStatus, TicketLookupError> {
        let Some(ticket) = self.tickets_by_id.get(ticket_id) else {
            return Err(TicketLookupError::NotFound {
                ticket_id: ticket_id.to_string(),
            });
        };

        if !ticket.belongs_to(player_id) {
            return Err(TicketLookupError::Unauthorized {
                ticket_id: ticket_id.to_string(),
            });
        }

        self.status_for_ticket(ticket_id)
            .ok_or(TicketLookupError::NotFound {
                ticket_id: ticket_id.to_string(),
            })
    }

    // Cancel a waiting ticket so it no longer participates in matching.
    pub fn cancel_ticket(
        &mut self,
        player_id: u64,
        ticket_id: &str,
    ) -> Result<TicketStatus, CancelTicketError> {
        let Some(ticket) = self.tickets_by_id.get(ticket_id).cloned() else {
            return Err(CancelTicketError::NotFound {
                ticket_id: ticket_id.to_string(),
            });
        };

        if !ticket.belongs_to(player_id) {
            return Err(CancelTicketError::Unauthorized {
                ticket_id: ticket_id.to_string(),
            });
        }

        match ticket {
            TicketRecord::Waiting {
                player_id, region, ..
            } => {
                self.queue
                    .retain(|queued_ticket_id| queued_ticket_id != ticket_id);
                self.active_ticket_by_player.remove(&player_id);
                self.tickets_by_id.insert(
                    ticket_id.to_string(),
                    TicketRecord::Canceled {
                        player_id,
                        region: region.clone(),
                    },
                );
                info!(
                    ticket_id = %ticket_id,
                    player_id,
                    region = %region,
                    "canceled matchmaking ticket"
                );

                Ok(TicketStatus::Canceled {
                    ticket_id: ticket_id.to_string(),
                    region,
                })
            }
            TicketRecord::Matched { .. } => Err(CancelTicketError::Matched {
                ticket_id: ticket_id.to_string(),
            }),
            TicketRecord::Canceled { region, .. } => Ok(TicketStatus::Canceled {
                ticket_id: ticket_id.to_string(),
                region,
            }),
        }
    }

    fn create_match(&mut self, opponent_ticket_id: String, request: EnqueuePlayer) -> TicketStatus {
        self.queue
            .retain(|queued_ticket_id| queued_ticket_id != &opponent_ticket_id);

        let opponent_ticket = self
            .tickets_by_id
            .get(&opponent_ticket_id)
            .cloned()
            .expect("queued ticket should exist");

        let TicketRecord::Waiting {
            player_id: opponent_player_id,
            region,
            ..
        } = opponent_ticket
        else {
            panic!("queued ticket should still be waiting");
        };

        self.active_ticket_by_player.remove(&opponent_player_id);

        let match_id = self
            .ids
            .next_match_id(request.player_id, opponent_player_id);
        let mut player_ids = vec![request.player_id, opponent_player_id];
        player_ids.sort_unstable();

        self.matches_by_id.insert(
            match_id.clone(),
            MatchRecord {
                match_id: match_id.clone(),
                player_ids: player_ids.clone(),
                region: region.clone(),
            },
        );
        // Matched tickets and match records currently remain queryable for the
        // process lifetime. Retention and eviction are intentionally out of
        // scope for this plan and should be added in a later slice.

        self.tickets_by_id.insert(
            opponent_ticket_id.clone(),
            TicketRecord::Matched {
                player_id: opponent_player_id,
                match_id: match_id.clone(),
            },
        );

        let ticket_id = self.ids.next_ticket_id(request.player_id);
        self.tickets_by_id.insert(
            ticket_id.clone(),
            TicketRecord::Matched {
                player_id: request.player_id,
                match_id: match_id.clone(),
            },
        );

        // Match formation writes both ticket records and the canonical match
        // record atomically inside this critical section.
        info!(
            match_id = %match_id,
            ticket_id = %ticket_id,
            opponent_ticket_id = %opponent_ticket_id,
            player_ids = ?player_ids,
            region = %region,
            "formed matchmaking match"
        );

        self.active_ticket_by_player
            .insert(opponent_player_id, opponent_ticket_id.clone());
        self.active_ticket_by_player
            .insert(request.player_id, ticket_id.clone());

        TicketStatus::Matched {
            ticket_id,
            match_id,
            player_ids,
            region,
        }
    }

    fn find_waiting_opponent_ticket(&self, region: &str) -> Option<String> {
        self.queue.iter().find_map(|ticket_id| {
            let ticket = self.tickets_by_id.get(ticket_id)?;
            match ticket {
                TicketRecord::Waiting {
                    region: queued_region,
                    ..
                } if queued_region == region => Some(ticket_id.clone()),
                _ => None,
            }
        })
    }

    fn discard_canceled_tickets_for_player(&mut self, player_id: u64) {
        self.tickets_by_id.retain(|_, ticket| {
            !matches!(
                ticket,
                TicketRecord::Canceled {
                    player_id: canceled_player_id,
                    ..
                } if *canceled_player_id == player_id
            )
        });
    }

    fn status_for_ticket(&self, ticket_id: &str) -> Option<TicketStatus> {
        let ticket = self.tickets_by_id.get(ticket_id)?;

        match ticket {
            TicketRecord::Waiting { region, .. } => Some(TicketStatus::Waiting {
                ticket_id: ticket_id.to_string(),
                region: region.clone(),
            }),
            TicketRecord::Matched { match_id, .. } => {
                let matched = self
                    .matches_by_id
                    .get(match_id)
                    .expect("matched ticket should reference a stored match");

                Some(TicketStatus::Matched {
                    ticket_id: ticket_id.to_string(),
                    match_id: matched.match_id.clone(),
                    player_ids: matched.player_ids.clone(),
                    region: matched.region.clone(),
                })
            }
            TicketRecord::Canceled { region, .. } => Some(TicketStatus::Canceled {
                ticket_id: ticket_id.to_string(),
                region: region.clone(),
            }),
        }
    }
}

impl TicketRecord {
    fn belongs_to(&self, player_id: u64) -> bool {
        match self {
            TicketRecord::Waiting {
                player_id: ticket_player_id,
                ..
            }
            | TicketRecord::Matched {
                player_id: ticket_player_id,
                ..
            }
            | TicketRecord::Canceled {
                player_id: ticket_player_id,
                ..
            } => *ticket_player_id == player_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct TestIdGenerator {
        next_id: Mutex<u64>,
    }

    impl MatchIdGenerator for TestIdGenerator {
        fn next_ticket_id(&self, player_id: u64) -> String {
            let mut next_id = self.next_id.lock().expect("lock should not be poisoned");
            *next_id += 1;
            format!("ticket-test-{}-{player_id}", *next_id)
        }

        fn next_match_id(&self, _player_id: u64, _opponent_id: u64) -> String {
            let mut next_id = self.next_id.lock().expect("lock should not be poisoned");
            *next_id += 1;
            format!("match-test-{}", *next_id)
        }
    }

    fn matchmaker() -> Matchmaker {
        Matchmaker::new(Arc::new(TestIdGenerator::default()))
    }

    fn queue_request(player_id: u64, region: &str) -> EnqueuePlayer {
        EnqueuePlayer {
            player_id,
            player_skill: 1200,
            region: region.to_string(),
        }
    }

    #[test]
    fn queued_ticket_can_be_polled_while_still_waiting() {
        let mut matchmaker = matchmaker();
        let outcome = matchmaker.enqueue(queue_request(1, "eu-west"));

        let TicketStatus::Waiting { ticket_id, region } = outcome else {
            panic!("first player should be queued");
        };
        assert_eq!(region, "eu-west");

        assert_eq!(
            matchmaker.lookup_ticket(1, ticket_id.as_str()),
            Ok(TicketStatus::Waiting {
                ticket_id,
                region: "eu-west".into(),
            })
        );
    }

    #[test]
    fn reenqueue_while_waiting_returns_the_existing_waiting_ticket() {
        let mut matchmaker = matchmaker();
        let first_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let first_ticket_id = match first_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        let second_outcome = matchmaker.enqueue(queue_request(1, "us-east"));

        assert_eq!(
            second_outcome,
            TicketStatus::Waiting {
                ticket_id: first_ticket_id,
                region: "eu-west".into(),
            }
        );
    }

    #[test]
    fn queued_ticket_transitions_to_a_shared_matched_record_after_an_opponent_arrives() {
        let mut matchmaker = matchmaker();
        let waiting_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let first_ticket_id = match waiting_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        let matched_outcome = matchmaker.enqueue(queue_request(2, "eu-west"));

        let TicketStatus::Matched {
            ticket_id: second_ticket_id,
            match_id,
            player_ids,
            region,
        } = matched_outcome
        else {
            panic!("second player should be matched immediately");
        };
        assert_ne!(second_ticket_id, first_ticket_id);
        assert_eq!(player_ids, vec![1, 2]);
        assert_eq!(region, "eu-west");

        assert_eq!(
            matchmaker.lookup_ticket(1, first_ticket_id.as_str()),
            Ok(TicketStatus::Matched {
                ticket_id: first_ticket_id.clone(),
                match_id: match_id.clone(),
                player_ids: vec![1, 2],
                region: "eu-west".into(),
            })
        );
        assert_eq!(
            matchmaker.lookup_ticket(2, second_ticket_id.as_str()),
            Ok(TicketStatus::Matched {
                ticket_id: second_ticket_id,
                match_id,
                player_ids: vec![1, 2],
                region: "eu-west".into(),
            })
        );
    }

    #[test]
    fn reenqueue_while_matched_returns_the_existing_matched_result() {
        let mut matchmaker = matchmaker();
        let waiting_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let first_ticket_id = match waiting_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };
        let matched_outcome = matchmaker.enqueue(queue_request(2, "eu-west"));
        let second_ticket_id = match matched_outcome {
            TicketStatus::Matched { ticket_id, .. } => ticket_id,
            _ => panic!("second player should be matched"),
        };

        let reenqueued_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));

        assert_eq!(
            reenqueued_outcome,
            TicketStatus::Matched {
                ticket_id: first_ticket_id,
                match_id: match reenqueued_outcome.clone() {
                    TicketStatus::Matched { match_id, .. } => match_id,
                    _ => unreachable!(),
                },
                player_ids: vec![1, 2],
                region: "eu-west".into(),
            }
        );
        assert!(matches!(
            matchmaker.lookup_ticket(2, second_ticket_id.as_str()),
            Ok(TicketStatus::Matched { .. })
        ));
    }

    #[test]
    fn cancel_waiting_ticket_transitions_to_canceled() {
        let mut matchmaker = matchmaker();
        let waiting_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let ticket_id = match waiting_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        assert_eq!(
            matchmaker.cancel_ticket(1, ticket_id.as_str()),
            Ok(TicketStatus::Canceled {
                ticket_id: ticket_id.clone(),
                region: "eu-west".into(),
            })
        );
        assert_eq!(
            matchmaker.lookup_ticket(1, ticket_id.as_str()),
            Ok(TicketStatus::Canceled {
                ticket_id,
                region: "eu-west".into(),
            })
        );
    }

    #[test]
    fn canceled_waiting_ticket_is_not_matched_by_later_arrivals() {
        let mut matchmaker = matchmaker();
        let waiting_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let first_ticket_id = match waiting_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        assert_eq!(
            matchmaker.cancel_ticket(1, first_ticket_id.as_str()),
            Ok(TicketStatus::Canceled {
                ticket_id: first_ticket_id.clone(),
                region: "eu-west".into(),
            })
        );

        let second_outcome = matchmaker.enqueue(queue_request(2, "eu-west"));
        let second_ticket_id = match second_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("a canceled ticket must not be matched by later arrivals"),
        };

        assert_eq!(
            matchmaker.lookup_ticket(1, first_ticket_id.as_str()),
            Ok(TicketStatus::Canceled {
                ticket_id: first_ticket_id,
                region: "eu-west".into(),
            })
        );
        assert_eq!(
            matchmaker.lookup_ticket(2, second_ticket_id.as_str()),
            Ok(TicketStatus::Waiting {
                ticket_id: second_ticket_id,
                region: "eu-west".into(),
            })
        );
    }

    #[test]
    fn canceling_a_matched_ticket_is_rejected() {
        let mut matchmaker = matchmaker();
        let first_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let first_ticket_id = match first_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };
        let second_outcome = matchmaker.enqueue(queue_request(2, "eu-west"));
        let second_ticket_id = match second_outcome {
            TicketStatus::Matched { ticket_id, .. } => ticket_id,
            _ => panic!("second player should be matched"),
        };

        assert_eq!(
            matchmaker.cancel_ticket(1, first_ticket_id.as_str()),
            Err(CancelTicketError::Matched {
                ticket_id: first_ticket_id,
            })
        );
        assert_eq!(
            matchmaker.cancel_ticket(2, second_ticket_id.as_str()),
            Err(CancelTicketError::Matched {
                ticket_id: second_ticket_id,
            })
        );
    }

    #[test]
    fn reenqueue_after_cancel_creates_a_new_ticket_and_discards_the_old_canceled_ticket() {
        let mut matchmaker = matchmaker();
        let first_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let first_ticket_id = match first_outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };
        matchmaker
            .cancel_ticket(1, first_ticket_id.as_str())
            .expect("cancel should succeed");

        let second_outcome = matchmaker.enqueue(queue_request(1, "eu-west"));

        let TicketStatus::Waiting {
            ticket_id: second_ticket_id,
            region,
        } = second_outcome
        else {
            panic!("re-enqueue after cancel should create a waiting ticket");
        };
        assert_ne!(first_ticket_id, second_ticket_id);
        assert_eq!(region, "eu-west");
        assert_eq!(
            matchmaker.lookup_ticket(1, first_ticket_id.as_str()),
            Err(TicketLookupError::NotFound {
                ticket_id: first_ticket_id,
            })
        );
    }

    #[test]
    fn ticket_lookup_rejects_non_owner() {
        let mut matchmaker = matchmaker();
        let outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let ticket_id = match outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        assert_eq!(
            matchmaker.lookup_ticket(2, ticket_id.as_str()),
            Err(TicketLookupError::Unauthorized { ticket_id })
        );
    }

    #[test]
    fn cancel_rejects_non_owner() {
        let mut matchmaker = matchmaker();
        let outcome = matchmaker.enqueue(queue_request(1, "eu-west"));
        let ticket_id = match outcome {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        assert_eq!(
            matchmaker.cancel_ticket(2, ticket_id.as_str()),
            Err(CancelTicketError::Unauthorized { ticket_id })
        );
    }

    #[test]
    fn unknown_ticket_returns_not_found() {
        let mut matchmaker = matchmaker();

        assert_eq!(
            matchmaker.lookup_ticket(1, "missing-ticket"),
            Err(TicketLookupError::NotFound {
                ticket_id: "missing-ticket".into(),
            })
        );
        assert_eq!(
            matchmaker.cancel_ticket(1, "missing-ticket"),
            Err(CancelTicketError::NotFound {
                ticket_id: "missing-ticket".into(),
            })
        );
    }
}
