use crate::use_cases::{
    CreateGameLobby, CreateGameLobbyResult, GameServerError, GameServerProvisioner,
};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct GameServerClient {
    http: Client,
}

#[derive(Debug, Serialize)]
struct CreateLobbyHttpRequest {
    lobby_id: String,
    allowed_player_ids: Vec<u64>,
}

impl GameServerClient {
    pub fn new() -> Result<Self, GameServerError> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| GameServerError::Unexpected)?;

        Ok(Self { http })
    }
}

#[async_trait]
impl GameServerProvisioner for GameServerClient {
    async fn create_lobby(
        &self,
        request: CreateGameLobby,
    ) -> Result<CreateGameLobbyResult, GameServerError> {
        let base_url = request.base_url.trim_end_matches('/');
        let url = format!("{base_url}/lobbies");

        let response = self
            .http
            .post(url)
            .json(&CreateLobbyHttpRequest {
                lobby_id: request.lobby_id,
                allowed_player_ids: request.allowed_player_ids,
            })
            .send()
            .await
            .map_err(|_| GameServerError::UpstreamUnavailable)?;

        match response.status() {
            StatusCode::CREATED => Ok(CreateGameLobbyResult::Created),
            StatusCode::CONFLICT => Ok(CreateGameLobbyResult::AlreadyExists),
            StatusCode::BAD_REQUEST => Err(GameServerError::BadRequest),
            status if status.is_client_error() => Err(GameServerError::UnexpectedClientError),
            _ => Err(GameServerError::UpstreamUnavailable),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, http::StatusCode as AxumStatusCode, routing::post};
    use serde_json::json;
    use tokio::net::TcpListener;

    async fn created_handler(
        Json(payload): Json<serde_json::Value>,
    ) -> (AxumStatusCode, Json<serde_json::Value>) {
        (
            AxumStatusCode::CREATED,
            Json(json!({ "lobby_id": payload["lobby_id"] })),
        )
    }

    async fn conflict_handler() -> (AxumStatusCode, Json<serde_json::Value>) {
        (
            AxumStatusCode::CONFLICT,
            Json(json!({ "error": "lobby already exists" })),
        )
    }

    async fn spawn_test_server(router: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("address should be available");

        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("test server should run");
        });

        format!("http://{address}")
    }

    #[tokio::test]
    async fn create_lobby_treats_created_as_success() {
        let base_url =
            spawn_test_server(Router::new().route("/lobbies", post(created_handler))).await;
        let client = GameServerClient::new().expect("client should build");

        let result = client
            .create_lobby(CreateGameLobby {
                base_url,
                lobby_id: "match-123".into(),
                allowed_player_ids: vec![1, 2],
            })
            .await
            .expect("create lobby should succeed");

        assert_eq!(result, CreateGameLobbyResult::Created);
    }

    #[tokio::test]
    async fn create_lobby_treats_conflict_as_retry_safe_success() {
        let base_url =
            spawn_test_server(Router::new().route("/lobbies", post(conflict_handler))).await;
        let client = GameServerClient::new().expect("client should build");

        let result = client
            .create_lobby(CreateGameLobby {
                base_url,
                lobby_id: "match-123".into(),
                allowed_player_ids: vec![1, 2],
            })
            .await
            .expect("duplicate create should still succeed");

        assert_eq!(result, CreateGameLobbyResult::AlreadyExists);
    }
}
