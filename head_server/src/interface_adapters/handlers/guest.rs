use crate::interface_adapters::protocol::{
    HeadGuestInitRequest, HeadGuestInitResponse, HeadGuestLoginRequest, HeadGuestLoginResponse,
};
use crate::interface_adapters::state::AppState;
use crate::use_cases::{AuthProviderError, GuestInit, GuestLogin};
use axum::{Json, extract::State, http::StatusCode};
use std::sync::Arc;

#[tracing::instrument(name = "guest_init", skip_all)]
pub async fn guest_init(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HeadGuestInitRequest>,
) -> Result<Json<HeadGuestInitResponse>, StatusCode> {
    // Convert the HTTP request into an application command.
    let request = GuestInit {
        display_name: body.display_name,
    };

    // Delegate workflow orchestration to the use-case layer.
    let result = state
        .guest_sessions
        .guest_init(request)
        .await
        .map_err(|error| {
            tracing::error!(?error, "failed to create guest identity.");
            map_guest_session_error(&error)
        })?;

    tracing::info!(
        guest_id = result.guest_id,
        "guest identity created successfully."
    );

    Ok(Json(HeadGuestInitResponse {
        // Keep guest_id stringly-typed on the client boundary to avoid JSON number precision loss.
        guest_id: result.guest_id.to_string(),
        session_token: result.session_token,
        expires_at: result.expires_at,
    }))
}

#[tracing::instrument(
    name = "guest_login",
    skip_all,
    fields(guest_id = ?body.guest_id)
)]
pub async fn guest_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HeadGuestLoginRequest>,
) -> Result<Json<HeadGuestLoginResponse>, StatusCode> {
    // Parse guest_id at the adapter boundary; application paths keep numeric IDs.
    let guest_id = body
        .guest_id
        .trim()
        .parse::<u64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Convert the HTTP request into an application command.
    let request = GuestLogin {
        guest_id,
        display_name: body.display_name,
    };

    // Delegate workflow orchestration to the use-case layer.
    let result = state
        .guest_sessions
        .guest_login(request)
        .await
        .map_err(|error| {
            tracing::error!(?error, "failed to create guest session.");
            map_guest_session_error(&error)
        })?;

    tracing::info!("guest session created successfully.");

    // Return the session token to the client.
    Ok(Json(HeadGuestLoginResponse {
        session_token: result.session_token,
        expires_at: result.expires_at,
    }))
}

fn map_guest_session_error(error: &AuthProviderError) -> StatusCode {
    match error {
        AuthProviderError::BadRequest => StatusCode::BAD_REQUEST,
        AuthProviderError::Unauthorized => StatusCode::UNAUTHORIZED,
        AuthProviderError::Forbidden => StatusCode::FORBIDDEN,
        AuthProviderError::NotFound => StatusCode::NOT_FOUND,
        AuthProviderError::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
        AuthProviderError::UnexpectedClientError
        | AuthProviderError::UpstreamUnavailable
        | AuthProviderError::Unexpected => StatusCode::BAD_GATEWAY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::{
        AuthProvider, GuestInitResult, GuestLoginResult, GuestSessionService,
        MatchmakingEnqueueResult, MatchmakingProvider, MatchmakingProviderError,
        MatchmakingService,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockAuthProvider {
        init_response: Mutex<Option<Result<GuestInitResult, AuthProviderError>>>,
        login_response: Mutex<Option<Result<GuestLoginResult, AuthProviderError>>>,
    }

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn create_guest_identity(
            &self,
            _req: GuestInit,
        ) -> Result<GuestInitResult, AuthProviderError> {
            self.init_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("init response should be configured")
        }

        async fn create_guest_session(
            &self,
            _req: GuestLogin,
        ) -> Result<GuestLoginResult, AuthProviderError> {
            self.login_response
                .lock()
                .expect("lock should not be poisoned")
                .take()
                .expect("login response should be configured")
        }

        async fn verify_session(
            &self,
            _req: crate::use_cases::VerifySession,
        ) -> Result<crate::use_cases::VerifySessionResult, AuthProviderError> {
            panic!("verify session should not be called");
        }
    }

    #[derive(Default)]
    struct UnusedMatchmakingProvider;

    #[async_trait]
    impl MatchmakingProvider for UnusedMatchmakingProvider {
        async fn enqueue(
            &self,
            _request: crate::use_cases::MatchmakingQueueRequest,
        ) -> Result<MatchmakingEnqueueResult, MatchmakingProviderError> {
            panic!("matchmaking should not be called");
        }

        async fn poll_status(
            &self,
            _ticket_id: String,
        ) -> Result<crate::use_cases::MatchmakingTicketStatus, MatchmakingProviderError> {
            panic!("matchmaking should not be called");
        }
    }

    fn app_state(auth: Arc<dyn AuthProvider>) -> Arc<AppState> {
        Arc::new(AppState {
            guest_sessions: Arc::new(GuestSessionService::new(auth)),
            matchmaking: Arc::new(MatchmakingService::new(
                Arc::new(MockAuthProvider::default()),
                Arc::new(UnusedMatchmakingProvider),
            )),
        })
    }

    #[tokio::test]
    async fn guest_login_rejects_invalid_guest_id() {
        let state = app_state(Arc::new(MockAuthProvider::default()));
        let result = guest_login(
            State(state),
            Json(HeadGuestLoginRequest {
                guest_id: "abc".into(),
                display_name: "Pilot".into(),
            }),
        )
        .await;

        match result {
            Ok(_) => panic!("invalid guest ids should fail"),
            Err(error) => assert_eq!(error, StatusCode::BAD_REQUEST),
        }
    }

    #[tokio::test]
    async fn guest_login_maps_provider_errors_to_http_status_codes() {
        let cases = [
            (AuthProviderError::BadRequest, StatusCode::BAD_REQUEST),
            (AuthProviderError::Unauthorized, StatusCode::UNAUTHORIZED),
            (AuthProviderError::Forbidden, StatusCode::FORBIDDEN),
            (AuthProviderError::NotFound, StatusCode::NOT_FOUND),
            (
                AuthProviderError::UnprocessableEntity,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                AuthProviderError::UnexpectedClientError,
                StatusCode::BAD_GATEWAY,
            ),
            (
                AuthProviderError::UpstreamUnavailable,
                StatusCode::BAD_GATEWAY,
            ),
            (AuthProviderError::Unexpected, StatusCode::BAD_GATEWAY),
        ];

        for (provider_error, expected_status) in cases {
            let state = app_state(Arc::new(MockAuthProvider {
                login_response: Mutex::new(Some(Err(provider_error))),
                ..Default::default()
            }));

            let result = guest_login(
                State(state),
                Json(HeadGuestLoginRequest {
                    guest_id: "42".into(),
                    display_name: "Pilot".into(),
                }),
            )
            .await;

            match result {
                Ok(_) => panic!("provider errors should fail"),
                Err(status) => assert_eq!(status, expected_status),
            }
        }
    }

    #[tokio::test]
    async fn guest_init_maps_provider_errors_to_http_status_codes() {
        let cases = [
            (AuthProviderError::BadRequest, StatusCode::BAD_REQUEST),
            (AuthProviderError::Unauthorized, StatusCode::UNAUTHORIZED),
            (AuthProviderError::Forbidden, StatusCode::FORBIDDEN),
            (AuthProviderError::NotFound, StatusCode::NOT_FOUND),
            (
                AuthProviderError::UnprocessableEntity,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                AuthProviderError::UnexpectedClientError,
                StatusCode::BAD_GATEWAY,
            ),
            (
                AuthProviderError::UpstreamUnavailable,
                StatusCode::BAD_GATEWAY,
            ),
            (AuthProviderError::Unexpected, StatusCode::BAD_GATEWAY),
        ];

        for (provider_error, expected_status) in cases {
            let state = app_state(Arc::new(MockAuthProvider {
                init_response: Mutex::new(Some(Err(provider_error))),
                ..Default::default()
            }));

            let result = guest_init(
                State(state),
                Json(HeadGuestInitRequest {
                    display_name: "Pilot".into(),
                }),
            )
            .await;

            match result {
                Ok(_) => panic!("provider errors should fail"),
                Err(status) => assert_eq!(status, expected_status),
            }
        }
    }

    #[test]
    fn auth_provider_errors_map_to_expected_http_status_codes() {
        assert_eq!(
            map_guest_session_error(&AuthProviderError::BadRequest),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            map_guest_session_error(&AuthProviderError::Unauthorized),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            map_guest_session_error(&AuthProviderError::Forbidden),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            map_guest_session_error(&AuthProviderError::NotFound),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            map_guest_session_error(&AuthProviderError::UnprocessableEntity),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            map_guest_session_error(&AuthProviderError::UnexpectedClientError),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            map_guest_session_error(&AuthProviderError::UpstreamUnavailable),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            map_guest_session_error(&AuthProviderError::Unexpected),
            StatusCode::BAD_GATEWAY
        );
    }
}
