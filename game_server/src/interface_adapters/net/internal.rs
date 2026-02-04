use crate::interface_adapters::http::ErrorResponse;
use crate::interface_adapters::net::client::spawn_lobby_serializer;
use crate::interface_adapters::state::AppState;

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug, serde::Deserialize)]
pub struct LobbyInitRequest {
    // Lobby id provided by the head service.
    lobby_id: String,
    // Player ids that are allowed to spawn into the lobby.
    #[serde(default)]
    allowed_player_ids: Vec<u64>,
}

#[derive(Debug, serde::Serialize)]
struct LobbyInitResponse {
    // The lobby id that was created.
    lobby_id: String,
}

pub async fn create_lobby_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LobbyInitRequest>,
) -> impl IntoResponse {
    // Ensure we have a lobby id to create.
    let lobby_id = payload.lobby_id.trim().to_string();
    if lobby_id.is_empty() {
        // Return a JSON error even for head-only routes to keep responses consistent.
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "lobby_id is required".to_string(),
            }),
        )
            .into_response();
    }

    let allowed_players: HashSet<u64> = payload.allowed_player_ids.into_iter().collect();

    // Created lobbies are not pinned and will be removed on last disconnect.
    match state
        .lobby_registry
        .create_lobby(
            lobby_id.clone(),
            allowed_players,
            false,
            state.lobby_registry.default_match_time_limit(),
        )
        .await
    {
        Ok(lobby) => {
            // Create serializers so clients can subscribe immediately.
            spawn_lobby_serializer(&lobby);
            // Watch for match end so empty lobbies can be cleaned up.
            state
                .lobby_registry
                .clone()
                .spawn_match_end_watcher(lobby.lobby_id.clone(), lobby.server_state_tx.subscribe());
            (StatusCode::CREATED, Json(LobbyInitResponse { lobby_id })).into_response()
        }
        Err(crate::use_cases::lobby::LobbyError::AlreadyExists) => {
            // Match the JSON error schema used for other create failures.
            (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "lobby already exists".to_string(),
                }),
            )
                .into_response()
        }
    }
}
