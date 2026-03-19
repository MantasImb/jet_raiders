use crate::interface_adapters::protocol::{
    HeadEnterMatchmakingRequest, HeadMatchmakingResponse, HeadMatchmakingStatus,
};
use crate::interface_adapters::state::AppState;
use crate::use_cases::{
    AuthProvider, EnterMatchmaking, EnterMatchmakingError, GuestInit, GuestInitResult, GuestLogin,
    GuestLoginResult, GuestSessionService, MatchmakingEnqueueResult, MatchmakingProvider,
    MatchmakingProviderError, MatchmakingQueueRequest, MatchmakingService, VerifySession,
    VerifySessionResult,
};
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use std::sync::{Arc, Mutex};

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

    // Convert the HTTP request into an application command.
    let request = EnterMatchmaking {
        session_token: body.session_token,
        player_skill: body.player_skill,
        region: body.region,
    };

    // Delegate queue orchestration to the use-case layer.
    let result = state
        .matchmaking
        .enter_queue(request)
        .await
        .map_err(|error| {
            tracing::error!(?error, "failed to enter matchmaking.");
            map_matchmaking_error(&error)
        })?;

    let response = match result {
        MatchmakingEnqueueResult::Waiting { ticket_id, region } => HeadMatchmakingResponse {
            status: HeadMatchmakingStatus::Waiting,
            ticket_id: Some(ticket_id),
            match_id: None,
            opponent_id: None,
            region,
        },
        MatchmakingEnqueueResult::Matched {
            match_id,
            opponent_id,
            region,
        } => HeadMatchmakingResponse {
            status: HeadMatchmakingStatus::Matched,
            ticket_id: None,
            match_id: Some(match_id),
            opponent_id: Some(opponent_id),
            region,
        },
    };

    Ok(Json(response))
}

fn map_matchmaking_error(error: &EnterMatchmakingError) -> StatusCode {
    match error {
        EnterMatchmakingError::Unauthorized => StatusCode::UNAUTHORIZED,
        EnterMatchmakingError::BadRequest => StatusCode::BAD_REQUEST,
        EnterMatchmakingError::Conflict => StatusCode::CONFLICT,
        EnterMatchmakingError::UnexpectedClientError
        | EnterMatchmakingError::UpstreamUnavailable
        | EnterMatchmakingError::Unexpected => StatusCode::BAD_GATEWAY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        enqueue_response: Mutex<Option<Result<MatchmakingEnqueueResult, MatchmakingProviderError>>>,
    }

    #[async_trait]
    impl MatchmakingProvider for MockMatchmakingProvider {
        async fn enqueue(
            &self,
            _request: MatchmakingQueueRequest,
        ) -> Result<MatchmakingEnqueueResult, MatchmakingProviderError> {
            self.enqueue_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("enqueue response should be configured")
        }
    }

    fn app_state(
        auth: Arc<dyn AuthProvider>,
        matchmaking: Arc<dyn MatchmakingProvider>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            guest_sessions: Arc::new(GuestSessionService::new(auth.clone())),
            matchmaking: Arc::new(MatchmakingService::new(auth, matchmaking)),
        })
    }

    #[tokio::test]
    async fn enter_matchmaking_rejects_missing_required_fields() {
        let state = app_state(
            Arc::new(MockAuthProvider::default()),
            Arc::new(MockMatchmakingProvider::default()),
        );
        let result = enter_matchmaking(
            State(state),
            Json(HeadEnterMatchmakingRequest {
                session_token: "".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            }),
        )
        .await;

        match result {
            Ok(_) => panic!("missing player id should fail"),
            Err(status) => assert_eq!(status, StatusCode::BAD_REQUEST),
        }
    }

    #[tokio::test]
    async fn enter_matchmaking_returns_waiting_response() {
        let state = app_state(
            Arc::new(MockAuthProvider {
                verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                    user_id: 42,
                    display_name: "Pilot".into(),
                    session_id: "session-1".into(),
                    expires_at: 123,
                }))),
            }),
            Arc::new(MockMatchmakingProvider {
                enqueue_response: Mutex::new(Some(Ok(MatchmakingEnqueueResult::Waiting {
                    ticket_id: "ticket-123".into(),
                    region: "eu-west".into(),
                }))),
            }),
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
        assert_eq!(result.0.opponent_id, None);
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn enter_matchmaking_returns_immediate_match_response() {
        let state = app_state(
            Arc::new(MockAuthProvider {
                verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                    user_id: 42,
                    display_name: "Pilot".into(),
                    session_id: "session-1".into(),
                    expires_at: 123,
                }))),
            }),
            Arc::new(MockMatchmakingProvider {
                enqueue_response: Mutex::new(Some(Ok(MatchmakingEnqueueResult::Matched {
                    match_id: "match-123".into(),
                    opponent_id: "player-2".into(),
                    region: "eu-west".into(),
                }))),
            }),
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
        assert_eq!(result.0.opponent_id.as_deref(), Some("player-2"));
        assert_eq!(result.0.region, "eu-west");
    }

    #[tokio::test]
    async fn enter_matchmaking_maps_errors_to_http_status_codes() {
        let cases = [
            (
                Err(crate::use_cases::AuthProviderError::Unauthorized),
                StatusCode::UNAUTHORIZED,
            ),
            (
                Err(crate::use_cases::AuthProviderError::UpstreamUnavailable),
                StatusCode::BAD_GATEWAY,
            ),
            (
                Ok(MatchmakingProviderError::BadRequest),
                StatusCode::BAD_REQUEST,
            ),
            (Ok(MatchmakingProviderError::Conflict), StatusCode::CONFLICT),
            (
                Ok(MatchmakingProviderError::UnexpectedClientError),
                StatusCode::BAD_GATEWAY,
            ),
            (
                Ok(MatchmakingProviderError::UpstreamUnavailable),
                StatusCode::BAD_GATEWAY,
            ),
            (
                Ok(MatchmakingProviderError::Unexpected),
                StatusCode::BAD_GATEWAY,
            ),
        ];

        for (error_source, expected_status) in cases {
            let (auth, matchmaking) = match error_source {
                Err(auth_error) => (
                    Arc::new(MockAuthProvider {
                        verify_response: Mutex::new(Some(Err(auth_error))),
                    }) as Arc<dyn AuthProvider>,
                    Arc::new(MockMatchmakingProvider::default()) as Arc<dyn MatchmakingProvider>,
                ),
                Ok(matchmaking_error) => (
                    Arc::new(MockAuthProvider {
                        verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                            user_id: 42,
                            display_name: "Pilot".into(),
                            session_id: "session-1".into(),
                            expires_at: 123,
                        }))),
                    }) as Arc<dyn AuthProvider>,
                    Arc::new(MockMatchmakingProvider {
                        enqueue_response: Mutex::new(Some(Err(matchmaking_error))),
                    }) as Arc<dyn MatchmakingProvider>,
                ),
            };
            let state = app_state(auth, matchmaking);

            let result = enter_matchmaking(
                State(state),
                Json(HeadEnterMatchmakingRequest {
                    session_token: "token-123".into(),
                    player_skill: 1200,
                    region: "eu-west".into(),
                }),
            )
            .await;

            match result {
                Ok(_) => panic!("provider errors should fail"),
                Err(status) => assert_eq!(status, expected_status),
            }
        }
    }
}
