use crate::interface_adapters::protocol::{ErrorResponse, QueueRequest, QueueResponse, QueueStatus};
use crate::interface_adapters::state::AppState;
use crate::use_cases::matchmaker::MatchOutcome;
use axum::{Json, extract::State, http::StatusCode};
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

    let outcome = {
        let mut matchmaker = state.matchmaker.lock().await;
        matchmaker.enqueue(request)
    };

    let response = match outcome {
        MatchOutcome::Waiting { ticket_id, region } => QueueResponse {
            status: QueueStatus::Waiting,
            ticket_id: Some(ticket_id),
            match_id: None,
            opponent_id: None,
            region,
        },
        MatchOutcome::Matched {
            match_id,
            opponent_id,
            region,
        } => QueueResponse {
            status: QueueStatus::Matched,
            ticket_id: None,
            match_id: Some(match_id),
            opponent_id: Some(opponent_id),
            region,
        },
    };

    Ok(Json(response))
}
