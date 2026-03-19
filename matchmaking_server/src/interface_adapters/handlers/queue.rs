use crate::interface_adapters::protocol::{
    ErrorResponse, QueueRequest, QueueResponse, QueueStatus,
};
use crate::interface_adapters::state::AppState;
use crate::use_cases::matchmaker::{
    EnqueuePlayer, MatchError, MatchOutcome, TicketLookupError, TicketStatus,
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
    if request.player_id.trim().is_empty() || request.region.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "player_id and region are required".to_string(),
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

    let response = match outcome {
        Ok(MatchOutcome::Waiting { ticket_id, region }) => QueueResponse {
            status: QueueStatus::Waiting,
            ticket_id: Some(ticket_id),
            match_id: None,
            opponent_id: None,
            region,
        },
        Ok(MatchOutcome::Matched {
            match_id,
            opponent_id,
            region,
        }) => QueueResponse {
            status: QueueStatus::Matched,
            ticket_id: None,
            match_id: Some(match_id),
            opponent_id: Some(opponent_id),
            region,
        },
        Err(MatchError::AlreadyQueued { player_id }) => {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    message: format!("player_id {} is already queued", player_id),
                }),
            ));
        }
    };

    Ok(Json(response))
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

    let response = match outcome {
        Ok(TicketStatus::Waiting { ticket_id, region }) => QueueResponse {
            status: QueueStatus::Waiting,
            ticket_id: Some(ticket_id),
            match_id: None,
            opponent_id: None,
            region,
        },
        Ok(TicketStatus::Matched {
            match_id,
            opponent_id,
            region,
        }) => QueueResponse {
            status: QueueStatus::Matched,
            ticket_id: None,
            match_id: Some(match_id),
            opponent_id: Some(opponent_id),
            region,
        },
        Err(TicketLookupError::NotFound { ticket_id }) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: format!("ticket_id {ticket_id} was not found"),
                }),
            ));
        }
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::matchmaker::{EnqueuePlayer, Matchmaker};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn app_state(matchmaker: Matchmaker) -> Arc<AppState> {
        Arc::new(AppState {
            matchmaker: Arc::new(Mutex::new(matchmaker)),
        })
    }

    #[tokio::test]
    async fn lookup_ticket_returns_waiting_response_for_queued_ticket() {
        let mut matchmaker = Matchmaker::new();
        let queued = matchmaker
            .enqueue(EnqueuePlayer {
                player_id: "player-1".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .expect("enqueue should succeed");
        let MatchOutcome::Waiting { ticket_id, .. } = queued else {
            panic!("first player should be queued");
        };

        let result = lookup_ticket(State(app_state(matchmaker)), Path(ticket_id))
            .await
            .expect("lookup should succeed");

        assert!(matches!(result.0.status, QueueStatus::Waiting));
        assert!(
            result
                .0
                .ticket_id
                .as_deref()
                .is_some_and(|ticket_id| ticket_id.starts_with("ticket-"))
        );
        assert_eq!(result.0.match_id, None);
        assert_eq!(result.0.opponent_id, None);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn lookup_ticket_returns_matched_response_after_transition() {
        let mut matchmaker = Matchmaker::new();
        let queued = matchmaker
            .enqueue(EnqueuePlayer {
                player_id: "player-1".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .expect("first enqueue should succeed");
        let MatchOutcome::Waiting { ticket_id, .. } = queued else {
            panic!("first player should be queued");
        };
        matchmaker
            .enqueue(EnqueuePlayer {
                player_id: "player-2".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .expect("second enqueue should succeed");

        let result = lookup_ticket(State(app_state(matchmaker)), Path(ticket_id))
            .await
            .expect("lookup should succeed");

        assert!(matches!(result.0.status, QueueStatus::Matched));
        assert_eq!(result.0.ticket_id, None);
        assert!(result.0.match_id.as_deref().is_some());
        assert_eq!(result.0.opponent_id.as_deref(), Some("player-2"));
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
}
