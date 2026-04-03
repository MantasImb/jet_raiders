use crate::interface_adapters::protocol::{
    ErrorResponse, QueueRequest, QueueResponse, QueueStatus, TicketOwnerQuery,
};
use crate::interface_adapters::state::AppState;
use crate::use_cases::matchmaker::{
    CancelTicketError, EnqueuePlayer, TicketLookupError, TicketStatus,
};
use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
};
use std::sync::Arc;

// Enqueue a player for matchmaking and attempt to match immediately.
pub async fn enqueue(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueueRequest>,
) -> Result<Json<QueueResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.region.trim().is_empty() {
        return Err(bad_request("region is required"));
    }

    if !state.allowed_regions.contains(request.region.as_str()) {
        return Err(bad_request(format!(
            "region '{}' is not configured",
            request.region
        )));
    }

    // Convert the HTTP DTO into an application request at the adapter boundary.
    let request = EnqueuePlayer {
        player_id: request.player_id,
        player_skill: request.player_skill,
        region: request.region,
    };

    let outcome = {
        let mut matchmaker = state.matchmaker.lock().await;
        matchmaker.enqueue(request)
    };

    Ok(Json(map_ticket_status(outcome)))
}

// Look up the current state of a previously issued matchmaking ticket.
pub async fn lookup_ticket(
    State(state): State<Arc<AppState>>,
    Path(ticket_id): Path<String>,
    query: Result<Query<TicketOwnerQuery>, QueryRejection>,
) -> Result<Json<QueueResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_ticket_id(ticket_id.as_str())?;
    let query = extract_owner_query(query)?;

    let outcome = {
        let matchmaker = state.matchmaker.lock().await;
        matchmaker.lookup_ticket(query.player_id, ticket_id.as_str())
    };

    match outcome {
        Ok(status) => Ok(Json(map_ticket_status(status))),
        Err(TicketLookupError::Unauthorized { ticket_id }) => Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: format!("ticket_id {ticket_id} is not owned by the caller"),
            }),
        )),
        Err(TicketLookupError::NotFound { ticket_id }) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                message: format!("ticket_id {ticket_id} was not found"),
            }),
        )),
    }
}

// Cancel a waiting ticket so it is removed from the active queue.
pub async fn cancel_ticket(
    State(state): State<Arc<AppState>>,
    Path(ticket_id): Path<String>,
    query: Result<Query<TicketOwnerQuery>, QueryRejection>,
) -> Result<Json<QueueResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_ticket_id(ticket_id.as_str())?;
    let query = extract_owner_query(query)?;

    let outcome = {
        let mut matchmaker = state.matchmaker.lock().await;
        matchmaker.cancel_ticket(query.player_id, ticket_id.as_str())
    };

    match outcome {
        Ok(status) => Ok(Json(map_ticket_status(status))),
        Err(CancelTicketError::Unauthorized { ticket_id }) => Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: format!("ticket_id {ticket_id} is not owned by the caller"),
            }),
        )),
        Err(CancelTicketError::NotFound { ticket_id }) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                message: format!("ticket_id {ticket_id} was not found"),
            }),
        )),
        Err(CancelTicketError::Matched { ticket_id }) => Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                message: format!("ticket_id {ticket_id} is already matched"),
            }),
        )),
    }
}

fn map_ticket_status(status: TicketStatus) -> QueueResponse {
    match status {
        TicketStatus::Waiting { ticket_id, region } => QueueResponse {
            status: QueueStatus::Waiting,
            ticket_id,
            match_id: None,
            player_ids: None,
            region,
        },
        TicketStatus::Matched {
            ticket_id,
            match_id,
            player_ids,
            region,
        } => QueueResponse {
            status: QueueStatus::Matched,
            ticket_id,
            match_id: Some(match_id),
            player_ids: Some(player_ids),
            region,
        },
        TicketStatus::Canceled { ticket_id, region } => QueueResponse {
            status: QueueStatus::Canceled,
            ticket_id,
            match_id: None,
            player_ids: None,
            region,
        },
    }
}

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            message: message.into(),
        }),
    )
}

fn validate_ticket_id(ticket_id: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ticket_id.trim().is_empty() {
        return Err(bad_request("ticket_id is required"));
    }

    Ok(())
}

fn extract_owner_query(
    query: Result<Query<TicketOwnerQuery>, QueryRejection>,
) -> Result<TicketOwnerQuery, (StatusCode, Json<ErrorResponse>)> {
    match query {
        Ok(Query(query)) => Ok(query),
        Err(_) => Err(bad_request("player_id query parameter is required")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::matchmaker::{MatchIdGenerator, Matchmaker};
    use axum::{extract::rejection::QueryRejection, http::Uri};
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::sync::Mutex;

    #[derive(Debug, Default)]
    struct TestIdGenerator {
        next_id: StdMutex<u64>,
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

    fn app_state(matchmaker: Matchmaker) -> Arc<AppState> {
        app_state_with_regions(matchmaker, &["eu-west", "us-east"])
    }

    fn app_state_with_regions(matchmaker: Matchmaker, allowed_regions: &[&str]) -> Arc<AppState> {
        Arc::new(AppState {
            allowed_regions: Arc::new(
                allowed_regions
                    .iter()
                    .map(|region| (*region).to_string())
                    .collect(),
            ),
            matchmaker: Arc::new(Mutex::new(matchmaker)),
        })
    }

    fn invalid_owner_query() -> Result<Query<TicketOwnerQuery>, QueryRejection> {
        let uri: Uri = "/matchmaking/queue/ticket-123?player_id=abc"
            .parse()
            .expect("uri should parse");
        Query::<TicketOwnerQuery>::try_from_uri(&uri)
    }

    #[tokio::test]
    async fn enqueue_returns_waiting_response() {
        let result = enqueue(
            State(app_state(matchmaker())),
            Json(QueueRequest {
                player_id: 1,
                player_skill: 1200,
                region: "eu-west".into(),
            }),
        )
        .await
        .expect("enqueue should succeed");

        assert!(matches!(result.0.status, QueueStatus::Waiting));
        assert!(result.0.ticket_id.starts_with("ticket-"));
        assert_eq!(result.0.match_id, None);
        assert_eq!(result.0.player_ids, None);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn enqueue_rejects_region_that_is_not_in_shared_catalog() {
        let result = enqueue(
            State(app_state_with_regions(matchmaker(), &["eu-west"])),
            Json(QueueRequest {
                player_id: 1,
                player_skill: 1200,
                region: "us-east".into(),
            }),
        )
        .await;

        let Err((status, Json(error))) = result else {
            panic!("unknown region should be rejected");
        };

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!error.message.is_empty());
    }

    #[tokio::test]
    async fn enqueue_returns_matched_response_with_shared_roster() {
        let mut matchmaker = matchmaker();
        let first_result = matchmaker.enqueue(EnqueuePlayer {
            player_id: 1,
            player_skill: 1200,
            region: "eu-west".into(),
        });
        let first_ticket_id = match first_result {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        let result = enqueue(
            State(app_state(matchmaker)),
            Json(QueueRequest {
                player_id: 2,
                player_skill: 1200,
                region: "eu-west".into(),
            }),
        )
        .await
        .expect("enqueue should succeed");

        assert!(matches!(result.0.status, QueueStatus::Matched));
        assert_ne!(result.0.ticket_id, first_ticket_id);
        assert!(result.0.match_id.as_deref().is_some());
        assert_eq!(result.0.player_ids, Some(vec![1, 2]));
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn lookup_ticket_returns_canceled_response_for_canceled_ticket() {
        let mut matchmaker = matchmaker();
        let queued = matchmaker.enqueue(EnqueuePlayer {
            player_id: 1,
            player_skill: 1200,
            region: "eu-west".into(),
        });
        let ticket_id = match queued {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };
        matchmaker
            .cancel_ticket(1, ticket_id.as_str())
            .expect("cancel should succeed");

        let result = lookup_ticket(
            State(app_state(matchmaker)),
            Path(ticket_id),
            Ok(Query(TicketOwnerQuery { player_id: 1 })),
        )
        .await
        .expect("lookup should succeed");

        assert!(matches!(result.0.status, QueueStatus::Canceled));
        assert_eq!(result.0.match_id, None);
        assert_eq!(result.0.player_ids, None);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn lookup_ticket_returns_not_found_for_unknown_ticket() {
        let result = lookup_ticket(
            State(app_state(matchmaker())),
            Path("missing-ticket".to_string()),
            Ok(Query(TicketOwnerQuery { player_id: 1 })),
        )
        .await;

        match result {
            Ok(_) => panic!("missing ticket should not succeed"),
            Err((status, error)) => {
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(error.0.message, "ticket_id missing-ticket was not found");
            }
        }
    }

    #[tokio::test]
    async fn lookup_ticket_rejects_non_owner() {
        let mut matchmaker = matchmaker();
        let queued = matchmaker.enqueue(EnqueuePlayer {
            player_id: 1,
            player_skill: 1200,
            region: "eu-west".into(),
        });
        let ticket_id = match queued {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        let result = lookup_ticket(
            State(app_state(matchmaker)),
            Path(ticket_id.clone()),
            Ok(Query(TicketOwnerQuery { player_id: 2 })),
        )
        .await;

        match result {
            Ok(_) => panic!("non-owner lookup should fail"),
            Err((status, error)) => {
                assert_eq!(status, StatusCode::UNAUTHORIZED);
                assert_eq!(
                    error.0.message,
                    format!("ticket_id {ticket_id} is not owned by the caller")
                );
            }
        }
    }

    #[tokio::test]
    async fn lookup_ticket_returns_json_bad_request_for_invalid_query() {
        let state = app_state(matchmaker());
        let result = lookup_ticket(
            State(state),
            Path("ticket-123".to_string()),
            invalid_owner_query(),
        )
        .await;

        match result {
            Ok(_) => panic!("invalid query should fail"),
            Err((status, Json(error))) => {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(error.message, "player_id query parameter is required");
            }
        }
    }

    #[tokio::test]
    async fn cancel_ticket_returns_canceled_response() {
        let mut matchmaker = matchmaker();
        let queued = matchmaker.enqueue(EnqueuePlayer {
            player_id: 1,
            player_skill: 1200,
            region: "eu-west".into(),
        });
        let ticket_id = match queued {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        let result = cancel_ticket(
            State(app_state(matchmaker)),
            Path(ticket_id.clone()),
            Ok(Query(TicketOwnerQuery { player_id: 1 })),
        )
        .await
        .expect("cancel should succeed");

        assert!(matches!(result.0.status, QueueStatus::Canceled));
        assert_eq!(result.0.ticket_id, ticket_id);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn cancel_ticket_returns_canceled_response_for_duplicate_cancel() {
        let mut matchmaker = matchmaker();
        let queued = matchmaker.enqueue(EnqueuePlayer {
            player_id: 1,
            player_skill: 1200,
            region: "eu-west".into(),
        });
        let ticket_id = match queued {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };
        matchmaker
            .cancel_ticket(1, ticket_id.as_str())
            .expect("first cancel should succeed");

        let result = cancel_ticket(
            State(app_state(matchmaker)),
            Path(ticket_id.clone()),
            Ok(Query(TicketOwnerQuery { player_id: 1 })),
        )
        .await
        .expect("duplicate cancel should succeed");

        assert!(matches!(result.0.status, QueueStatus::Canceled));
        assert_eq!(result.0.ticket_id, ticket_id);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn cancel_ticket_rejects_matched_tickets() {
        let mut matchmaker = matchmaker();
        let queued = matchmaker.enqueue(EnqueuePlayer {
            player_id: 1,
            player_skill: 1200,
            region: "eu-west".into(),
        });
        let first_ticket_id = match queued {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };
        matchmaker.enqueue(EnqueuePlayer {
            player_id: 2,
            player_skill: 1200,
            region: "eu-west".into(),
        });

        let result = cancel_ticket(
            State(app_state(matchmaker)),
            Path(first_ticket_id.clone()),
            Ok(Query(TicketOwnerQuery { player_id: 1 })),
        )
        .await;

        match result {
            Ok(_) => panic!("matched ticket cancel should fail"),
            Err((status, error)) => {
                assert_eq!(status, StatusCode::CONFLICT);
                assert_eq!(
                    error.0.message,
                    format!("ticket_id {first_ticket_id} is already matched")
                );
            }
        }
    }

    #[tokio::test]
    async fn cancel_ticket_returns_not_found_for_unknown_ticket() {
        let result = cancel_ticket(
            State(app_state(matchmaker())),
            Path("missing-ticket".to_string()),
            Ok(Query(TicketOwnerQuery { player_id: 1 })),
        )
        .await;

        match result {
            Ok(_) => panic!("missing ticket should not succeed"),
            Err((status, error)) => {
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(error.0.message, "ticket_id missing-ticket was not found");
            }
        }
    }

    #[tokio::test]
    async fn cancel_ticket_returns_json_bad_request_for_invalid_query() {
        let state = app_state(matchmaker());
        let result = cancel_ticket(
            State(state),
            Path("ticket-123".to_string()),
            invalid_owner_query(),
        )
        .await;

        match result {
            Ok(_) => panic!("invalid query should fail"),
            Err((status, Json(error))) => {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(error.message, "player_id query parameter is required");
            }
        }
    }
}
