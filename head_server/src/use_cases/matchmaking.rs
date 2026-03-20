use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;

use crate::use_cases::{AuthProvider, AuthProviderError, VerifySession, VerifySessionResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnterMatchmaking {
    pub session_token: String,
    pub player_skill: u32,
    pub region: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchmakingEnqueueResult {
    Waiting {
        ticket_id: String,
        region: String,
    },
    Matched {
        match_id: String,
        opponent_id: String,
        region: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchmakingTicketStatus {
    Waiting {
        ticket_id: String,
        region: String,
    },
    Matched {
        match_id: String,
        opponent_id: String,
        region: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchmakingProviderError {
    BadRequest,
    Conflict,
    NotFound,
    UnexpectedClientError,
    UpstreamUnavailable,
    Unexpected,
}

impl fmt::Display for MatchmakingProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchmakingProviderError::BadRequest => write!(f, "bad request"),
            MatchmakingProviderError::Conflict => write!(f, "conflict"),
            MatchmakingProviderError::NotFound => write!(f, "not found"),
            MatchmakingProviderError::UnexpectedClientError => {
                write!(f, "unexpected upstream client error")
            }
            MatchmakingProviderError::UpstreamUnavailable => {
                write!(f, "upstream unavailable")
            }
            MatchmakingProviderError::Unexpected => {
                write!(f, "unexpected matchmaking provider error")
            }
        }
    }
}

impl std::error::Error for MatchmakingProviderError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnterMatchmakingError {
    Unauthorized,
    BadRequest,
    Conflict,
    UnexpectedClientError,
    UpstreamUnavailable,
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PollMatchmaking {
    pub session_token: String,
    pub ticket_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PollMatchmakingError {
    Unauthorized,
    BadRequest,
    NotFound,
    UnexpectedClientError,
    UpstreamUnavailable,
    Unexpected,
}

impl fmt::Display for EnterMatchmakingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnterMatchmakingError::Unauthorized => write!(f, "unauthorized"),
            EnterMatchmakingError::BadRequest => write!(f, "bad request"),
            EnterMatchmakingError::Conflict => write!(f, "conflict"),
            EnterMatchmakingError::UnexpectedClientError => {
                write!(f, "unexpected upstream client error")
            }
            EnterMatchmakingError::UpstreamUnavailable => {
                write!(f, "upstream unavailable")
            }
            EnterMatchmakingError::Unexpected => write!(f, "unexpected enter matchmaking error"),
        }
    }
}

impl std::error::Error for EnterMatchmakingError {}

impl fmt::Display for PollMatchmakingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PollMatchmakingError::Unauthorized => write!(f, "unauthorized"),
            PollMatchmakingError::BadRequest => write!(f, "bad request"),
            PollMatchmakingError::NotFound => write!(f, "not found"),
            PollMatchmakingError::UnexpectedClientError => {
                write!(f, "unexpected upstream client error")
            }
            PollMatchmakingError::UpstreamUnavailable => write!(f, "upstream unavailable"),
            PollMatchmakingError::Unexpected => write!(f, "unexpected poll matchmaking error"),
        }
    }
}

impl std::error::Error for PollMatchmakingError {}

impl From<AuthProviderError> for EnterMatchmakingError {
    fn from(error: AuthProviderError) -> Self {
        match error {
            AuthProviderError::Unauthorized => EnterMatchmakingError::Unauthorized,
            AuthProviderError::BadRequest => EnterMatchmakingError::BadRequest,
            AuthProviderError::UnexpectedClientError => {
                EnterMatchmakingError::UnexpectedClientError
            }
            AuthProviderError::UpstreamUnavailable => EnterMatchmakingError::UpstreamUnavailable,
            AuthProviderError::Unexpected
            | AuthProviderError::Forbidden
            | AuthProviderError::NotFound
            | AuthProviderError::UnprocessableEntity => EnterMatchmakingError::Unexpected,
        }
    }
}

impl From<MatchmakingProviderError> for EnterMatchmakingError {
    fn from(error: MatchmakingProviderError) -> Self {
        match error {
            MatchmakingProviderError::BadRequest => EnterMatchmakingError::BadRequest,
            MatchmakingProviderError::Conflict => EnterMatchmakingError::Conflict,
            MatchmakingProviderError::NotFound => EnterMatchmakingError::Unexpected,
            MatchmakingProviderError::UnexpectedClientError => {
                EnterMatchmakingError::UnexpectedClientError
            }
            MatchmakingProviderError::UpstreamUnavailable => {
                EnterMatchmakingError::UpstreamUnavailable
            }
            MatchmakingProviderError::Unexpected => EnterMatchmakingError::Unexpected,
        }
    }
}

impl From<AuthProviderError> for PollMatchmakingError {
    fn from(error: AuthProviderError) -> Self {
        match error {
            AuthProviderError::Unauthorized => PollMatchmakingError::Unauthorized,
            AuthProviderError::BadRequest => PollMatchmakingError::BadRequest,
            AuthProviderError::UnexpectedClientError => PollMatchmakingError::UnexpectedClientError,
            AuthProviderError::UpstreamUnavailable => PollMatchmakingError::UpstreamUnavailable,
            AuthProviderError::Unexpected
            | AuthProviderError::NotFound
            | AuthProviderError::Forbidden
            | AuthProviderError::UnprocessableEntity => PollMatchmakingError::Unexpected,
        }
    }
}

impl From<MatchmakingProviderError> for PollMatchmakingError {
    fn from(error: MatchmakingProviderError) -> Self {
        match error {
            MatchmakingProviderError::BadRequest => PollMatchmakingError::BadRequest,
            MatchmakingProviderError::Conflict => PollMatchmakingError::Unexpected,
            MatchmakingProviderError::NotFound => PollMatchmakingError::NotFound,
            MatchmakingProviderError::UnexpectedClientError => {
                PollMatchmakingError::UnexpectedClientError
            }
            MatchmakingProviderError::UpstreamUnavailable => {
                PollMatchmakingError::UpstreamUnavailable
            }
            MatchmakingProviderError::Unexpected => PollMatchmakingError::Unexpected,
        }
    }
}

#[async_trait]
pub trait MatchmakingProvider: Send + Sync {
    async fn enqueue(
        &self,
        request: MatchmakingQueueRequest,
    ) -> Result<MatchmakingEnqueueResult, MatchmakingProviderError>;

    async fn poll_status(
        &self,
        ticket_id: String,
    ) -> Result<MatchmakingTicketStatus, MatchmakingProviderError>;
}

#[derive(Clone)]
pub struct MatchmakingService {
    auth: Arc<dyn AuthProvider>,
    matchmaking: Arc<dyn MatchmakingProvider>,
}

impl MatchmakingService {
    pub fn new(auth: Arc<dyn AuthProvider>, matchmaking: Arc<dyn MatchmakingProvider>) -> Self {
        Self { auth, matchmaking }
    }

    // Keep session verification in one place so queue entry and polling apply
    // the same auth boundary before delegating to matchmaking.
    async fn verify_caller_session(
        &self,
        session_token: String,
    ) -> Result<VerifySessionResult, AuthProviderError> {
        self.auth
            .verify_session(VerifySession { session_token })
            .await
    }

    pub async fn enter_queue(
        &self,
        request: EnterMatchmaking,
    ) -> Result<MatchmakingEnqueueResult, EnterMatchmakingError> {
        let session = self
            .verify_caller_session(request.session_token)
            .await
            .map_err(EnterMatchmakingError::from)?;

        self.matchmaking
            .enqueue(MatchmakingQueueRequest::new(
                session.user_id,
                request.player_skill,
                request.region,
            ))
            .await
            .map_err(EnterMatchmakingError::from)
    }

    pub async fn poll_status(
        &self,
        request: PollMatchmaking,
    ) -> Result<MatchmakingTicketStatus, PollMatchmakingError> {
        self.verify_caller_session(request.session_token)
            .await
            .map_err(PollMatchmakingError::from)?;

        self.matchmaking
            .poll_status(request.ticket_id)
            .await
            .map_err(PollMatchmakingError::from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchmakingQueueRequest {
    pub player_id: String,
    pub player_skill: u32,
    pub region: String,
}

impl MatchmakingQueueRequest {
    fn new(player_id: u64, player_skill: u32, region: String) -> Self {
        Self {
            player_id: player_id.to_string(),
            player_skill,
            region,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::{
        GuestInit, GuestInitResult, GuestLogin, GuestLoginResult, VerifySessionResult,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockAuthProvider {
        verify_requests: Mutex<Vec<VerifySession>>,
        verify_response: Mutex<Option<Result<VerifySessionResult, AuthProviderError>>>,
    }

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn create_guest_identity(
            &self,
            _req: GuestInit,
        ) -> Result<GuestInitResult, AuthProviderError> {
            panic!("guest init should not be called");
        }

        async fn create_guest_session(
            &self,
            _req: GuestLogin,
        ) -> Result<GuestLoginResult, AuthProviderError> {
            panic!("guest login should not be called");
        }

        async fn verify_session(
            &self,
            req: VerifySession,
        ) -> Result<VerifySessionResult, AuthProviderError> {
            self.verify_requests.lock().unwrap().push(req);
            self.verify_response
                .lock()
                .unwrap()
                .take()
                .expect("verify response should be configured")
        }
    }

    #[derive(Default)]
    struct MockMatchmakingProvider {
        enqueue_requests: Mutex<Vec<MatchmakingQueueRequest>>,
        enqueue_response: Mutex<Option<Result<MatchmakingEnqueueResult, MatchmakingProviderError>>>,
        poll_requests: Mutex<Vec<String>>,
        poll_response: Mutex<Option<Result<MatchmakingTicketStatus, MatchmakingProviderError>>>,
    }

    #[async_trait]
    impl MatchmakingProvider for MockMatchmakingProvider {
        async fn enqueue(
            &self,
            request: MatchmakingQueueRequest,
        ) -> Result<MatchmakingEnqueueResult, MatchmakingProviderError> {
            self.enqueue_requests.lock().unwrap().push(request);
            self.enqueue_response
                .lock()
                .unwrap()
                .take()
                .expect("enqueue response should be configured")
        }

        async fn poll_status(
            &self,
            ticket_id: String,
        ) -> Result<MatchmakingTicketStatus, MatchmakingProviderError> {
            self.poll_requests.lock().unwrap().push(ticket_id);
            self.poll_response
                .lock()
                .unwrap()
                .take()
                .expect("poll response should be configured")
        }
    }

    #[tokio::test]
    async fn enter_matchmaking_verifies_session_and_delegates_waiting_outcome_to_provider() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                user_id: 42,
                display_name: "Pilot".into(),
                session_id: "session-1".into(),
                expires_at: 123,
            }))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            enqueue_response: Mutex::new(Some(Ok(MatchmakingEnqueueResult::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });
        let service = MatchmakingService::new(auth.clone(), matchmaking.clone());

        let result = service
            .enter_queue(EnterMatchmaking {
                session_token: "token-123".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .await
            .expect("enqueue should succeed");

        assert_eq!(
            result,
            MatchmakingEnqueueResult::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(
            auth.verify_requests.lock().unwrap().as_slice(),
            &[VerifySession {
                session_token: "token-123".into(),
            }]
        );
        assert_eq!(
            matchmaking.enqueue_requests.lock().unwrap().as_slice(),
            &[MatchmakingQueueRequest {
                player_id: "42".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            }]
        );
    }

    #[tokio::test]
    async fn enter_matchmaking_delegates_matched_outcome_to_provider() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                user_id: 42,
                display_name: "Pilot".into(),
                session_id: "session-1".into(),
                expires_at: 123,
            }))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            enqueue_response: Mutex::new(Some(Ok(MatchmakingEnqueueResult::Matched {
                match_id: "match-123".into(),
                opponent_id: "player-2".into(),
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });
        let service = MatchmakingService::new(auth, matchmaking.clone());

        let result = service
            .enter_queue(EnterMatchmaking {
                session_token: "token-123".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .await
            .expect("enqueue should succeed");

        assert_eq!(
            result,
            MatchmakingEnqueueResult::Matched {
                match_id: "match-123".into(),
                opponent_id: "player-2".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(
            matchmaking.enqueue_requests.lock().unwrap().as_slice(),
            &[MatchmakingQueueRequest {
                player_id: "42".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            }]
        );
    }

    #[tokio::test]
    async fn enter_matchmaking_returns_unauthorized_when_session_token_is_invalid() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Err(AuthProviderError::Unauthorized))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider::default());
        let service = MatchmakingService::new(auth, matchmaking.clone());

        let result = service
            .enter_queue(EnterMatchmaking {
                session_token: "bad-token".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .await;

        assert_eq!(result, Err(EnterMatchmakingError::Unauthorized));
        assert!(matchmaking.enqueue_requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn poll_matchmaking_delegates_waiting_ticket_lookup_to_provider() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                user_id: 42,
                display_name: "Pilot".into(),
                session_id: "session-1".into(),
                expires_at: 123,
            }))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            poll_response: Mutex::new(Some(Ok(MatchmakingTicketStatus::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });
        let service = MatchmakingService::new(auth.clone(), matchmaking.clone());

        let result = service
            .poll_status(PollMatchmaking {
                session_token: "token-123".into(),
                ticket_id: "ticket-123".into(),
            })
            .await
            .expect("poll should succeed");

        assert_eq!(
            result,
            MatchmakingTicketStatus::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(
            matchmaking.poll_requests.lock().unwrap().as_slice(),
            &["ticket-123".to_string()]
        );
        assert_eq!(
            auth.verify_requests.lock().unwrap().as_slice(),
            &[VerifySession {
                session_token: "token-123".into(),
            }]
        );
    }

    #[tokio::test]
    async fn poll_matchmaking_maps_not_found_from_provider() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(VerifySessionResult {
                user_id: 42,
                display_name: "Pilot".into(),
                session_id: "session-1".into(),
                expires_at: 123,
            }))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            poll_response: Mutex::new(Some(Err(MatchmakingProviderError::NotFound))),
            ..Default::default()
        });
        let service = MatchmakingService::new(auth, matchmaking);

        let result = service
            .poll_status(PollMatchmaking {
                session_token: "token-123".into(),
                ticket_id: "missing-ticket".into(),
            })
            .await;

        assert_eq!(result, Err(PollMatchmakingError::NotFound));
    }

    #[tokio::test]
    async fn poll_matchmaking_returns_unauthorized_when_session_token_is_invalid() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Err(AuthProviderError::Unauthorized))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider::default());
        let service = MatchmakingService::new(auth, matchmaking.clone());

        let result = service
            .poll_status(PollMatchmaking {
                session_token: "bad-token".into(),
                ticket_id: "ticket-123".into(),
            })
            .await;

        assert_eq!(result, Err(PollMatchmakingError::Unauthorized));
        assert!(matchmaking.poll_requests.lock().unwrap().is_empty());
    }
}
