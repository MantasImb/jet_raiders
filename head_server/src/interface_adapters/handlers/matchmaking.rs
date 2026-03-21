use crate::interface_adapters::protocol::{
    HeadEnterMatchmakingRequest, HeadMatchmakingResponse, HeadMatchmakingStatus,
    HeadPollMatchmakingQuery,
};
use crate::interface_adapters::state::AppState;
use crate::use_cases::{
    CancelMatchmaking, CancelMatchmakingError, EnterMatchmaking, EnterMatchmakingError,
    HeadMatchmakingResult, PollMatchmaking, PollMatchmakingError,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use std::sync::Arc;

#[tracing::instrument(
    name = "enter_matchmaking",
    skip_all,
    fields(region = %body.region)
)]
pub async fn enter_matchmaking(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HeadEnterMatchmakingRequest>,
) -> Result<Json<HeadMatchmakingResponse>, StatusCode> {
    if body.session_token.trim().is_empty() || body.region.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let request = EnterMatchmaking {
        session_token: body.session_token,
        player_skill: body.player_skill,
        region: body.region,
    };

    let result = state
        .matchmaking
        .enter_queue(request)
        .await
        .map_err(|error| {
            tracing::error!(?error, "failed to enter matchmaking.");
            map_matchmaking_error(&error)
        })?;

    Ok(Json(map_head_result(result)))
}

#[tracing::instrument(name = "poll_matchmaking", skip_all, fields(ticket_id = %ticket_id))]
pub async fn poll_matchmaking(
    State(state): State<Arc<AppState>>,
    Path(ticket_id): Path<String>,
    Query(query): Query<HeadPollMatchmakingQuery>,
) -> Result<Json<HeadMatchmakingResponse>, StatusCode> {
    if ticket_id.trim().is_empty() || query.session_token.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = state
        .matchmaking
        .poll_status(PollMatchmaking {
            session_token: query.session_token,
            ticket_id,
        })
        .await
        .map_err(|error| {
            tracing::error!(?error, "failed to poll matchmaking.");
            map_poll_matchmaking_error(&error)
        })?;

    Ok(Json(map_head_result(result)))
}

#[tracing::instrument(name = "cancel_matchmaking", skip_all, fields(ticket_id = %ticket_id))]
pub async fn cancel_matchmaking(
    State(state): State<Arc<AppState>>,
    Path(ticket_id): Path<String>,
    Query(query): Query<HeadPollMatchmakingQuery>,
) -> Result<Json<HeadMatchmakingResponse>, StatusCode> {
    if ticket_id.trim().is_empty() || query.session_token.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = state
        .matchmaking
        .cancel(CancelMatchmaking {
            session_token: query.session_token,
            ticket_id,
        })
        .await
        .map_err(|error| {
            tracing::error!(?error, "failed to cancel matchmaking.");
            map_cancel_matchmaking_error(&error)
        })?;

    Ok(Json(map_head_result(result)))
}

fn map_head_result(result: HeadMatchmakingResult) -> HeadMatchmakingResponse {
    match result {
        HeadMatchmakingResult::Waiting { ticket_id, region } => HeadMatchmakingResponse {
            status: HeadMatchmakingStatus::Waiting,
            ticket_id: Some(ticket_id),
            match_id: None,
            lobby_id: None,
            ws_url: None,
            region,
        },
        HeadMatchmakingResult::Matched {
            match_id,
            lobby_id,
            ws_url,
            region,
        } => HeadMatchmakingResponse {
            status: HeadMatchmakingStatus::Matched,
            ticket_id: None,
            match_id: Some(match_id),
            lobby_id: Some(lobby_id),
            ws_url: Some(ws_url),
            region,
        },
        HeadMatchmakingResult::Canceled { ticket_id, region } => HeadMatchmakingResponse {
            status: HeadMatchmakingStatus::Canceled,
            ticket_id: Some(ticket_id),
            match_id: None,
            lobby_id: None,
            ws_url: None,
            region,
        },
    }
}

fn map_matchmaking_error(error: &EnterMatchmakingError) -> StatusCode {
    match error {
        EnterMatchmakingError::Unauthorized => StatusCode::UNAUTHORIZED,
        EnterMatchmakingError::BadRequest => StatusCode::BAD_REQUEST,
        EnterMatchmakingError::UnexpectedClientError
        | EnterMatchmakingError::UpstreamUnavailable
        | EnterMatchmakingError::Unexpected => StatusCode::BAD_GATEWAY,
    }
}

fn map_poll_matchmaking_error(error: &PollMatchmakingError) -> StatusCode {
    match error {
        PollMatchmakingError::Unauthorized => StatusCode::UNAUTHORIZED,
        PollMatchmakingError::BadRequest => StatusCode::BAD_REQUEST,
        PollMatchmakingError::NotFound => StatusCode::NOT_FOUND,
        PollMatchmakingError::UnexpectedClientError
        | PollMatchmakingError::UpstreamUnavailable
        | PollMatchmakingError::Unexpected => StatusCode::BAD_GATEWAY,
    }
}

fn map_cancel_matchmaking_error(error: &CancelMatchmakingError) -> StatusCode {
    match error {
        CancelMatchmakingError::Unauthorized => StatusCode::UNAUTHORIZED,
        CancelMatchmakingError::BadRequest => StatusCode::BAD_REQUEST,
        CancelMatchmakingError::Conflict => StatusCode::CONFLICT,
        CancelMatchmakingError::NotFound => StatusCode::NOT_FOUND,
        CancelMatchmakingError::UnexpectedClientError
        | CancelMatchmakingError::UpstreamUnavailable
        | CancelMatchmakingError::Unexpected => StatusCode::BAD_GATEWAY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::{
        AuthProvider, CreateGameLobby, CreateGameLobbyResult, GameServerDirectory, GameServerError,
        GameServerProvisioner, GuestInit, GuestInitResult, GuestLogin, GuestLoginResult,
        MatchmakingLifecycleState, MatchmakingProvider, MatchmakingProviderError,
        MatchmakingQueueRequest, MatchmakingService, ResolvedGameServer, VerifySession,
        VerifySessionResult,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockAuthProvider {
        verify_response:
            Mutex<Option<Result<VerifySessionResult, crate::use_cases::AuthProviderError>>>,
    }

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn create_guest_identity(
            &self,
            _req: GuestInit,
        ) -> Result<GuestInitResult, crate::use_cases::AuthProviderError> {
            panic!("guest init should not be called");
        }

        async fn create_guest_session(
            &self,
            _req: GuestLogin,
        ) -> Result<GuestLoginResult, crate::use_cases::AuthProviderError> {
            panic!("guest login should not be called");
        }

        async fn verify_session(
            &self,
            _req: VerifySession,
        ) -> Result<VerifySessionResult, crate::use_cases::AuthProviderError> {
            self.verify_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("verify response should be configured")
        }
    }

    #[derive(Default)]
    struct MockMatchmakingProvider {
        enqueue_response:
            Mutex<Option<Result<MatchmakingLifecycleState, MatchmakingProviderError>>>,
        poll_response: Mutex<Option<Result<MatchmakingLifecycleState, MatchmakingProviderError>>>,
        cancel_response: Mutex<Option<Result<MatchmakingLifecycleState, MatchmakingProviderError>>>,
    }

    #[async_trait]
    impl MatchmakingProvider for MockMatchmakingProvider {
        async fn enqueue(
            &self,
            _request: MatchmakingQueueRequest,
        ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError> {
            self.enqueue_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("enqueue response should be configured")
        }

        async fn poll_status(
            &self,
            _ticket_id: String,
        ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError> {
            self.poll_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("poll response should be configured")
        }

        async fn cancel(
            &self,
            _ticket_id: String,
        ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError> {
            self.cancel_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("cancel response should be configured")
        }
    }

    #[derive(Default)]
    struct MockGameServerDirectory {
        resolve_response: Mutex<Option<Result<ResolvedGameServer, GameServerError>>>,
    }

    #[async_trait]
    impl GameServerDirectory for MockGameServerDirectory {
        async fn resolve(&self, _region: &str) -> Result<ResolvedGameServer, GameServerError> {
            self.resolve_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("resolve response should be configured")
        }
    }

    #[derive(Default)]
    struct MockGameServerProvisioner {
        create_response: Mutex<Option<Result<CreateGameLobbyResult, GameServerError>>>,
    }

    #[async_trait]
    impl GameServerProvisioner for MockGameServerProvisioner {
        async fn create_lobby(
            &self,
            _request: CreateGameLobby,
        ) -> Result<CreateGameLobbyResult, GameServerError> {
            self.create_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("create response should be configured")
        }
    }

    fn app_state(
        auth: Arc<dyn AuthProvider>,
        matchmaking: Arc<dyn MatchmakingProvider>,
        directory: Arc<dyn GameServerDirectory>,
        provisioner: Arc<dyn GameServerProvisioner>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            guest_sessions: Arc::new(crate::use_cases::GuestSessionService::new(auth.clone())),
            matchmaking: Arc::new(MatchmakingService::new(
                auth,
                matchmaking,
                directory,
                provisioner,
            )),
        })
    }

    fn verified_auth() -> Arc<dyn AuthProvider> {
        Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                user_id: 42,
                display_name: "Pilot".into(),
                session_id: "session-1".into(),
                expires_at: 123,
            }))),
        })
    }

    fn resolved_server() -> Arc<dyn GameServerDirectory> {
        Arc::new(MockGameServerDirectory {
            resolve_response: Mutex::new(Some(Ok(ResolvedGameServer {
                base_url: "http://game.internal".into(),
                ws_url: "ws://game.public/ws".into(),
            }))),
        })
    }

    fn provisioner_created() -> Arc<dyn GameServerProvisioner> {
        Arc::new(MockGameServerProvisioner {
            create_response: Mutex::new(Some(Ok(CreateGameLobbyResult::Created))),
        })
    }

    #[tokio::test]
    async fn enter_matchmaking_returns_waiting_response() {
        let state = app_state(
            verified_auth(),
            Arc::new(MockMatchmakingProvider {
                enqueue_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Waiting {
                    ticket_id: "ticket-123".into(),
                    region: "eu-west".into(),
                }))),
                ..Default::default()
            }),
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        );

        let result = enter_matchmaking(
            State(state),
            Json(HeadEnterMatchmakingRequest {
                session_token: "token-123".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            }),
        )
        .await
        .expect("enqueue should succeed");

        assert_eq!(result.0.status, HeadMatchmakingStatus::Waiting);
        assert_eq!(result.0.ticket_id.as_deref(), Some("ticket-123"));
        assert_eq!(result.0.match_id, None);
        assert_eq!(result.0.lobby_id, None);
        assert_eq!(result.0.ws_url, None);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn enter_matchmaking_returns_game_ready_match_response() {
        let state = app_state(
            verified_auth(),
            Arc::new(MockMatchmakingProvider {
                enqueue_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Matched {
                    ticket_id: "ticket-123".into(),
                    match_id: "match-123".into(),
                    player_ids: vec![7, 42],
                    region: "eu-west".into(),
                }))),
                ..Default::default()
            }),
            resolved_server(),
            provisioner_created(),
        );

        let result = enter_matchmaking(
            State(state),
            Json(HeadEnterMatchmakingRequest {
                session_token: "token-123".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            }),
        )
        .await
        .expect("enqueue should succeed");

        assert_eq!(result.0.status, HeadMatchmakingStatus::Matched);
        assert_eq!(result.0.ticket_id, None);
        assert_eq!(result.0.match_id.as_deref(), Some("match-123"));
        assert_eq!(result.0.lobby_id.as_deref(), Some("match-123"));
        assert_eq!(result.0.ws_url.as_deref(), Some("ws://game.public/ws"));
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn poll_matchmaking_returns_canceled_response() {
        let state = app_state(
            verified_auth(),
            Arc::new(MockMatchmakingProvider {
                poll_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Canceled {
                    ticket_id: "ticket-123".into(),
                    region: "eu-west".into(),
                }))),
                ..Default::default()
            }),
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        );

        let result = poll_matchmaking(
            State(state),
            Path("ticket-123".to_string()),
            Query(HeadPollMatchmakingQuery {
                session_token: "token-123".into(),
            }),
        )
        .await
        .expect("poll should succeed");

        assert_eq!(result.0.status, HeadMatchmakingStatus::Canceled);
        assert_eq!(result.0.ticket_id.as_deref(), Some("ticket-123"));
        assert_eq!(result.0.match_id, None);
        assert_eq!(result.0.lobby_id, None);
        assert_eq!(result.0.ws_url, None);
    }

    #[tokio::test]
    async fn cancel_matchmaking_returns_canceled_response() {
        let state = app_state(
            verified_auth(),
            Arc::new(MockMatchmakingProvider {
                cancel_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Canceled {
                    ticket_id: "ticket-123".into(),
                    region: "eu-west".into(),
                }))),
                ..Default::default()
            }),
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        );

        let result = cancel_matchmaking(
            State(state),
            Path("ticket-123".to_string()),
            Query(HeadPollMatchmakingQuery {
                session_token: "token-123".into(),
            }),
        )
        .await
        .expect("cancel should succeed");

        assert_eq!(result.0.status, HeadMatchmakingStatus::Canceled);
        assert_eq!(result.0.ticket_id.as_deref(), Some("ticket-123"));
    }

    #[tokio::test]
    async fn cancel_matchmaking_maps_conflict_to_http_409() {
        let state = app_state(
            verified_auth(),
            Arc::new(MockMatchmakingProvider {
                cancel_response: Mutex::new(Some(Err(MatchmakingProviderError::Conflict))),
                ..Default::default()
            }),
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        );

        let result = cancel_matchmaking(
            State(state),
            Path("ticket-123".to_string()),
            Query(HeadPollMatchmakingQuery {
                session_token: "token-123".into(),
            }),
        )
        .await;

        match result {
            Ok(_) => panic!("cancel conflict should fail"),
            Err(status) => assert_eq!(status, StatusCode::CONFLICT),
        }
    }
}
