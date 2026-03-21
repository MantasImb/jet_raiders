use crate::interface_adapters::protocol::{
    ErrorResponse, QueueRequest, QueueResponse, QueueStatus,
};
use crate::interface_adapters::state::AppState;
use crate::use_cases::matchmaker::{
    CancelTicketError, EnqueuePlayer, TicketLookupError, TicketStatus,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;

// Enqueue a player for matchmaking and attempt to match immediately.
pub async fn enqueue(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueueRequest>,
) -> Result<Json<QueueResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.region.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "region is required".to_string(),
            }),
        ));
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
) -> Result<Json<QueueResponse>, (StatusCode, Json<ErrorResponse>)> {
    if ticket_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "ticket_id is required".to_string(),
            }),
        ));
    }

    let outcome = {
        let matchmaker = state.matchmaker.lock().await;
        matchmaker.lookup_ticket(ticket_id.as_str())
    };

    match outcome {
        Ok(status) => Ok(Json(map_ticket_status(status))),
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
) -> Result<Json<QueueResponse>, (StatusCode, Json<ErrorResponse>)> {
    if ticket_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "ticket_id is required".to_string(),
            }),
        ));
    }

    let outcome = {
        let mut matchmaker = state.matchmaker.lock().await;
        matchmaker.cancel_ticket(ticket_id.as_str())
    };

    match outcome {
        Ok(status) => Ok(Json(map_ticket_status(status))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::matchmaker::Matchmaker;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn app_state(matchmaker: Matchmaker) -> Arc<AppState> {
        Arc::new(AppState {
            matchmaker: Arc::new(Mutex::new(matchmaker)),
        })
    }

    #[tokio::test]
    async fn enqueue_returns_waiting_response() {
        let result = enqueue(
            State(app_state(Matchmaker::new())),
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
    async fn enqueue_returns_matched_response_with_shared_roster() {
        let mut matchmaker = Matchmaker::new();
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
        let mut matchmaker = Matchmaker::new();
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
            .cancel_ticket(ticket_id.as_str())
            .expect("cancel should succeed");

        let result = lookup_ticket(State(app_state(matchmaker)), Path(ticket_id))
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
            State(app_state(Matchmaker::new())),
            Path("missing-ticket".to_string()),
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
    async fn cancel_ticket_returns_canceled_response() {
        let mut matchmaker = Matchmaker::new();
        let queued = matchmaker.enqueue(EnqueuePlayer {
            player_id: 1,
            player_skill: 1200,
            region: "eu-west".into(),
        });
        let ticket_id = match queued {
            TicketStatus::Waiting { ticket_id, .. } => ticket_id,
            _ => panic!("first player should be queued"),
        };

        let result = cancel_ticket(State(app_state(matchmaker)), Path(ticket_id.clone()))
            .await
            .expect("cancel should succeed");

        assert!(matches!(result.0.status, QueueStatus::Canceled));
        assert_eq!(result.0.ticket_id, ticket_id);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn cancel_ticket_rejects_matched_tickets() {
        let mut matchmaker = Matchmaker::new();
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

        let result =
            cancel_ticket(State(app_state(matchmaker)), Path(first_ticket_id.clone())).await;

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
}
