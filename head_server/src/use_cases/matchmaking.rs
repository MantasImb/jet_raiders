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
pub struct PollMatchmaking {
    pub session_token: String,
    pub ticket_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancelMatchmaking {
    pub session_token: String,
    pub ticket_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadMatchmakingResult {
    Waiting {
        ticket_id: String,
        region: String,
    },
    Matched {
        ticket_id: String,
        match_id: String,
        lobby_id: String,
        ws_url: String,
        region: String,
    },
    Canceled {
        ticket_id: String,
        region: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchmakingLifecycleState {
    Waiting {
        ticket_id: String,
        region: String,
    },
    Matched {
        ticket_id: String,
        match_id: String,
        player_ids: Vec<u64>,
        region: String,
    },
    Canceled {
        ticket_id: String,
        region: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchmakingProviderError {
    Unauthorized,
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
            MatchmakingProviderError::Unauthorized => write!(f, "unauthorized"),
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
    UnexpectedClientError,
    UpstreamUnavailable,
    Unexpected,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CancelMatchmakingError {
    Unauthorized,
    BadRequest,
    Conflict,
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

impl fmt::Display for CancelMatchmakingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CancelMatchmakingError::Unauthorized => write!(f, "unauthorized"),
            CancelMatchmakingError::BadRequest => write!(f, "bad request"),
            CancelMatchmakingError::Conflict => write!(f, "conflict"),
            CancelMatchmakingError::NotFound => write!(f, "not found"),
            CancelMatchmakingError::UnexpectedClientError => {
                write!(f, "unexpected upstream client error")
            }
            CancelMatchmakingError::UpstreamUnavailable => {
                write!(f, "upstream unavailable")
            }
            CancelMatchmakingError::Unexpected => {
                write!(f, "unexpected cancel matchmaking error")
            }
        }
    }
}

impl std::error::Error for CancelMatchmakingError {}

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

impl From<AuthProviderError> for CancelMatchmakingError {
    fn from(error: AuthProviderError) -> Self {
        match error {
            AuthProviderError::Unauthorized => CancelMatchmakingError::Unauthorized,
            AuthProviderError::BadRequest => CancelMatchmakingError::BadRequest,
            AuthProviderError::UnexpectedClientError => {
                CancelMatchmakingError::UnexpectedClientError
            }
            AuthProviderError::UpstreamUnavailable => CancelMatchmakingError::UpstreamUnavailable,
            AuthProviderError::Unexpected
            | AuthProviderError::NotFound
            | AuthProviderError::Forbidden
            | AuthProviderError::UnprocessableEntity => CancelMatchmakingError::Unexpected,
        }
    }
}

impl From<MatchmakingProviderError> for EnterMatchmakingError {
    fn from(error: MatchmakingProviderError) -> Self {
        match error {
            MatchmakingProviderError::Unauthorized => EnterMatchmakingError::Unauthorized,
            MatchmakingProviderError::BadRequest => EnterMatchmakingError::BadRequest,
            MatchmakingProviderError::Conflict | MatchmakingProviderError::NotFound => {
                EnterMatchmakingError::Unexpected
            }
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

impl From<MatchmakingProviderError> for PollMatchmakingError {
    fn from(error: MatchmakingProviderError) -> Self {
        match error {
            MatchmakingProviderError::Unauthorized => PollMatchmakingError::Unauthorized,
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

impl From<MatchmakingProviderError> for CancelMatchmakingError {
    fn from(error: MatchmakingProviderError) -> Self {
        match error {
            MatchmakingProviderError::Unauthorized => CancelMatchmakingError::Unauthorized,
            MatchmakingProviderError::BadRequest => CancelMatchmakingError::BadRequest,
            MatchmakingProviderError::Conflict => CancelMatchmakingError::Conflict,
            MatchmakingProviderError::NotFound => CancelMatchmakingError::NotFound,
            MatchmakingProviderError::UnexpectedClientError => {
                CancelMatchmakingError::UnexpectedClientError
            }
            MatchmakingProviderError::UpstreamUnavailable => {
                CancelMatchmakingError::UpstreamUnavailable
            }
            MatchmakingProviderError::Unexpected => CancelMatchmakingError::Unexpected,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchmakingQueueRequest {
    pub player_id: u64,
    pub player_skill: u32,
    pub region: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedGameServer {
    pub base_url: String,
    pub ws_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateGameLobby {
    pub base_url: String,
    pub lobby_id: String,
    pub allowed_player_ids: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateGameLobbyResult {
    Created,
    AlreadyExists,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameServerError {
    BadRequest,
    UnexpectedClientError,
    UpstreamUnavailable,
    Unexpected,
}

#[async_trait]
pub trait MatchmakingProvider: Send + Sync {
    async fn enqueue(
        &self,
        request: MatchmakingQueueRequest,
    ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError>;

    async fn poll_status(
        &self,
        player_id: u64,
        ticket_id: String,
    ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError>;

    async fn cancel(
        &self,
        player_id: u64,
        ticket_id: String,
    ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError>;
}

#[async_trait]
pub trait GameServerDirectory: Send + Sync {
    async fn resolve(&self, region: &str) -> Result<ResolvedGameServer, GameServerError>;
}

#[async_trait]
pub trait GameServerProvisioner: Send + Sync {
    async fn create_lobby(
        &self,
        request: CreateGameLobby,
    ) -> Result<CreateGameLobbyResult, GameServerError>;
}

#[derive(Clone)]
pub struct MatchmakingService {
    auth: Arc<dyn AuthProvider>,
    matchmaking: Arc<dyn MatchmakingProvider>,
    game_servers: Arc<dyn GameServerDirectory>,
    provisioner: Arc<dyn GameServerProvisioner>,
}

impl MatchmakingService {
    pub fn new(
        auth: Arc<dyn AuthProvider>,
        matchmaking: Arc<dyn MatchmakingProvider>,
        game_servers: Arc<dyn GameServerDirectory>,
        provisioner: Arc<dyn GameServerProvisioner>,
    ) -> Self {
        Self {
            auth,
            matchmaking,
            game_servers,
            provisioner,
        }
    }

    // Keep session verification in one place so queue entry, polling, and
    // canceling apply the same auth boundary before delegating upstream.
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
    ) -> Result<HeadMatchmakingResult, EnterMatchmakingError> {
        let session = self
            .verify_caller_session(request.session_token)
            .await
            .map_err(EnterMatchmakingError::from)?;

        let state = self
            .matchmaking
            .enqueue(MatchmakingQueueRequest {
                player_id: session.user_id,
                player_skill: request.player_skill,
                region: request.region,
            })
            .await
            .map_err(EnterMatchmakingError::from)?;

        self.map_lifecycle_state(state)
            .await
            .map_err(map_game_server_error_to_enter)
    }

    pub async fn poll_status(
        &self,
        request: PollMatchmaking,
    ) -> Result<HeadMatchmakingResult, PollMatchmakingError> {
        let session = self
            .verify_caller_session(request.session_token)
            .await
            .map_err(PollMatchmakingError::from)?;

        let state = self
            .matchmaking
            .poll_status(session.user_id, request.ticket_id)
            .await
            .map_err(PollMatchmakingError::from)?;

        self.map_lifecycle_state(state)
            .await
            .map_err(map_game_server_error_to_poll)
    }

    pub async fn cancel(
        &self,
        request: CancelMatchmaking,
    ) -> Result<HeadMatchmakingResult, CancelMatchmakingError> {
        let session = self
            .verify_caller_session(request.session_token)
            .await
            .map_err(CancelMatchmakingError::from)?;

        let state = self
            .matchmaking
            .cancel(session.user_id, request.ticket_id)
            .await
            .map_err(CancelMatchmakingError::from)?;

        match state {
            MatchmakingLifecycleState::Canceled { ticket_id, region } => {
                Ok(HeadMatchmakingResult::Canceled { ticket_id, region })
            }
            MatchmakingLifecycleState::Waiting { ticket_id, region } => {
                Ok(HeadMatchmakingResult::Waiting { ticket_id, region })
            }
            MatchmakingLifecycleState::Matched { .. } => Err(CancelMatchmakingError::Conflict),
        }
    }

    async fn map_lifecycle_state(
        &self,
        state: MatchmakingLifecycleState,
    ) -> Result<HeadMatchmakingResult, GameServerError> {
        match state {
            MatchmakingLifecycleState::Waiting { ticket_id, region } => {
                Ok(HeadMatchmakingResult::Waiting { ticket_id, region })
            }
            MatchmakingLifecycleState::Canceled { ticket_id, region } => {
                Ok(HeadMatchmakingResult::Canceled { ticket_id, region })
            }
            MatchmakingLifecycleState::Matched {
                ticket_id,
                match_id,
                player_ids,
                region,
            } => self
                .complete_handoff(ticket_id, match_id, player_ids, region)
                .await,
        }
    }

    async fn complete_handoff(
        &self,
        ticket_id: String,
        match_id: String,
        player_ids: Vec<u64>,
        region: String,
    ) -> Result<HeadMatchmakingResult, GameServerError> {
        let game_server = self.game_servers.resolve(region.as_str()).await?;
        let lobby_id = match_id.clone();

        self.provisioner
            .create_lobby(CreateGameLobby {
                base_url: game_server.base_url,
                lobby_id: lobby_id.clone(),
                allowed_player_ids: player_ids,
            })
            .await?;

        Ok(HeadMatchmakingResult::Matched {
            ticket_id,
            match_id,
            lobby_id,
            ws_url: game_server.ws_url,
            region,
        })
    }
}

fn map_game_server_error_to_enter(error: GameServerError) -> EnterMatchmakingError {
    match error {
        GameServerError::BadRequest => EnterMatchmakingError::BadRequest,
        GameServerError::UnexpectedClientError => EnterMatchmakingError::UnexpectedClientError,
        GameServerError::UpstreamUnavailable => EnterMatchmakingError::UpstreamUnavailable,
        GameServerError::Unexpected => EnterMatchmakingError::Unexpected,
    }
}

fn map_game_server_error_to_poll(error: GameServerError) -> PollMatchmakingError {
    match error {
        GameServerError::BadRequest => PollMatchmakingError::BadRequest,
        GameServerError::UnexpectedClientError => PollMatchmakingError::UnexpectedClientError,
        GameServerError::UpstreamUnavailable => PollMatchmakingError::UpstreamUnavailable,
        GameServerError::Unexpected => PollMatchmakingError::Unexpected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::{GuestInit, GuestInitResult, GuestLogin, GuestLoginResult};
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
        poll_requests: Mutex<Vec<String>>,
        cancel_requests: Mutex<Vec<String>>,
        enqueue_response:
            Mutex<Option<Result<MatchmakingLifecycleState, MatchmakingProviderError>>>,
        poll_response: Mutex<Option<Result<MatchmakingLifecycleState, MatchmakingProviderError>>>,
        cancel_response: Mutex<Option<Result<MatchmakingLifecycleState, MatchmakingProviderError>>>,
    }

    #[async_trait]
    impl MatchmakingProvider for MockMatchmakingProvider {
        async fn enqueue(
            &self,
            request: MatchmakingQueueRequest,
        ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError> {
            self.enqueue_requests.lock().unwrap().push(request);
            self.enqueue_response
                .lock()
                .unwrap()
                .take()
                .expect("enqueue response should be configured")
        }

        async fn poll_status(
            &self,
            _player_id: u64,
            ticket_id: String,
        ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError> {
            self.poll_requests.lock().unwrap().push(ticket_id);
            self.poll_response
                .lock()
                .unwrap()
                .take()
                .expect("poll response should be configured")
        }

        async fn cancel(
            &self,
            _player_id: u64,
            ticket_id: String,
        ) -> Result<MatchmakingLifecycleState, MatchmakingProviderError> {
            self.cancel_requests.lock().unwrap().push(ticket_id);
            self.cancel_response
                .lock()
                .unwrap()
                .take()
                .expect("cancel response should be configured")
        }
    }

    #[derive(Default)]
    struct MockGameServerDirectory {
        resolve_requests: Mutex<Vec<String>>,
        resolve_response: Mutex<Option<Result<ResolvedGameServer, GameServerError>>>,
    }

    #[async_trait]
    impl GameServerDirectory for MockGameServerDirectory {
        async fn resolve(&self, region: &str) -> Result<ResolvedGameServer, GameServerError> {
            self.resolve_requests
                .lock()
                .unwrap()
                .push(region.to_string());
            self.resolve_response
                .lock()
                .unwrap()
                .take()
                .expect("resolve response should be configured")
        }
    }

    #[derive(Default)]
    struct MockGameServerProvisioner {
        create_requests: Mutex<Vec<CreateGameLobby>>,
        create_responses: Mutex<Vec<Result<CreateGameLobbyResult, GameServerError>>>,
    }

    #[async_trait]
    impl GameServerProvisioner for MockGameServerProvisioner {
        async fn create_lobby(
            &self,
            request: CreateGameLobby,
        ) -> Result<CreateGameLobbyResult, GameServerError> {
            self.create_requests.lock().unwrap().push(request);
            self.create_responses.lock().unwrap().remove(0)
        }
    }

    fn matchmaking_service(
        auth: Arc<dyn AuthProvider>,
        matchmaking: Arc<dyn MatchmakingProvider>,
        directory: Arc<dyn GameServerDirectory>,
        provisioner: Arc<dyn GameServerProvisioner>,
    ) -> MatchmakingService {
        MatchmakingService::new(auth, matchmaking, directory, provisioner)
    }

    fn verified_session() -> VerifySessionResult {
        VerifySessionResult {
            user_id: 42,
            display_name: "Pilot".into(),
            session_id: "session-1".into(),
            expires_at: 123,
        }
    }

    fn resolved_server() -> ResolvedGameServer {
        ResolvedGameServer {
            base_url: "http://game.internal".into(),
            ws_url: "ws://game.public/ws".into(),
        }
    }

    #[tokio::test]
    async fn enter_queue_returns_waiting_without_creating_a_lobby() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            enqueue_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });
        let directory = Arc::new(MockGameServerDirectory::default());
        let provisioner = Arc::new(MockGameServerProvisioner::default());

        let result = matchmaking_service(
            auth,
            matchmaking.clone(),
            directory.clone(),
            provisioner.clone(),
        )
        .enter_queue(EnterMatchmaking {
            session_token: "token-123".into(),
            player_skill: 1200,
            region: "eu-west".into(),
        })
        .await
        .expect("enter queue should succeed");

        assert_eq!(
            result,
            HeadMatchmakingResult::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(matchmaking.enqueue_requests.lock().unwrap().len(), 1);
        assert!(directory.resolve_requests.lock().unwrap().is_empty());
        assert!(provisioner.create_requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn enter_queue_completes_handoff_for_immediate_matches() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            enqueue_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Matched {
                ticket_id: "ticket-123".into(),
                match_id: "match-123".into(),
                player_ids: vec![7, 42],
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });
        let directory = Arc::new(MockGameServerDirectory {
            resolve_response: Mutex::new(Some(Ok(resolved_server()))),
            ..Default::default()
        });
        let provisioner = Arc::new(MockGameServerProvisioner {
            create_responses: Mutex::new(vec![Ok(CreateGameLobbyResult::Created)]),
            ..Default::default()
        });

        let result = matchmaking_service(auth, matchmaking, directory.clone(), provisioner.clone())
            .enter_queue(EnterMatchmaking {
                session_token: "token-123".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .await
            .expect("enter queue should succeed");

        assert_eq!(
            result,
            HeadMatchmakingResult::Matched {
                ticket_id: "ticket-123".into(),
                match_id: "match-123".into(),
                lobby_id: "match-123".into(),
                ws_url: "ws://game.public/ws".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(
            provisioner.create_requests.lock().unwrap().as_slice(),
            &[CreateGameLobby {
                base_url: "http://game.internal".into(),
                lobby_id: "match-123".into(),
                allowed_player_ids: vec![7, 42],
            }]
        );
        assert_eq!(
            directory.resolve_requests.lock().unwrap().as_slice(),
            &["eu-west".to_string()]
        );
    }

    #[tokio::test]
    async fn poll_status_treats_duplicate_lobby_create_as_success() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            poll_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Matched {
                ticket_id: "ticket-123".into(),
                match_id: "match-123".into(),
                player_ids: vec![7, 42],
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });
        let directory = Arc::new(MockGameServerDirectory {
            resolve_response: Mutex::new(Some(Ok(resolved_server()))),
            ..Default::default()
        });
        let provisioner = Arc::new(MockGameServerProvisioner {
            create_responses: Mutex::new(vec![Ok(CreateGameLobbyResult::AlreadyExists)]),
            ..Default::default()
        });

        let result = matchmaking_service(auth, matchmaking, directory, provisioner)
            .poll_status(PollMatchmaking {
                session_token: "token-123".into(),
                ticket_id: "ticket-123".into(),
            })
            .await
            .expect("poll should succeed");

        assert_eq!(
            result,
            HeadMatchmakingResult::Matched {
                ticket_id: "ticket-123".into(),
                match_id: "match-123".into(),
                lobby_id: "match-123".into(),
                ws_url: "ws://game.public/ws".into(),
                region: "eu-west".into(),
            }
        );
    }

    #[tokio::test]
    async fn poll_status_maps_matchmaking_not_found() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            poll_response: Mutex::new(Some(Err(MatchmakingProviderError::NotFound))),
            ..Default::default()
        });

        let result = matchmaking_service(
            auth,
            matchmaking,
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        )
        .poll_status(PollMatchmaking {
            session_token: "token-123".into(),
            ticket_id: "ticket-123".into(),
        })
        .await;

        assert_eq!(result, Err(PollMatchmakingError::NotFound));
    }

    #[tokio::test]
    async fn cancel_delegates_to_matchmaking_and_returns_canceled_state() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            cancel_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Canceled {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });

        let result = matchmaking_service(
            auth,
            matchmaking.clone(),
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        )
        .cancel(CancelMatchmaking {
            session_token: "token-123".into(),
            ticket_id: "ticket-123".into(),
        })
        .await
        .expect("cancel should succeed");

        assert_eq!(
            result,
            HeadMatchmakingResult::Canceled {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(
            matchmaking.cancel_requests.lock().unwrap().as_slice(),
            &["ticket-123".to_string()]
        );
    }

    #[tokio::test]
    async fn cancel_treats_duplicate_cancel_as_canceled_state() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            cancel_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Canceled {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });

        let result = matchmaking_service(
            auth,
            matchmaking,
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        )
        .cancel(CancelMatchmaking {
            session_token: "token-123".into(),
            ticket_id: "ticket-123".into(),
        })
        .await;

        assert_eq!(
            result,
            Ok(HeadMatchmakingResult::Canceled {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            })
        );
    }

    #[tokio::test]
    async fn cancel_maps_matched_conflict() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            cancel_response: Mutex::new(Some(Err(MatchmakingProviderError::Conflict))),
            ..Default::default()
        });

        let result = matchmaking_service(
            auth,
            matchmaking,
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        )
        .cancel(CancelMatchmaking {
            session_token: "token-123".into(),
            ticket_id: "ticket-123".into(),
        })
        .await;

        assert_eq!(result, Err(CancelMatchmakingError::Conflict));
    }

    #[tokio::test]
    async fn cancel_maps_matchmaking_not_found() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            cancel_response: Mutex::new(Some(Err(MatchmakingProviderError::NotFound))),
            ..Default::default()
        });

        let result = matchmaking_service(
            auth,
            matchmaking,
            Arc::new(MockGameServerDirectory::default()),
            Arc::new(MockGameServerProvisioner::default()),
        )
        .cancel(CancelMatchmaking {
            session_token: "token-123".into(),
            ticket_id: "missing-ticket".into(),
        })
        .await;

        assert_eq!(result, Err(CancelMatchmakingError::NotFound));
    }

    #[tokio::test]
    async fn poll_status_surfaces_handoff_failure() {
        let auth = Arc::new(MockAuthProvider {
            verify_response: Mutex::new(Some(Ok(verified_session()))),
            ..Default::default()
        });
        let matchmaking = Arc::new(MockMatchmakingProvider {
            poll_response: Mutex::new(Some(Ok(MatchmakingLifecycleState::Matched {
                ticket_id: "ticket-123".into(),
                match_id: "match-123".into(),
                player_ids: vec![7, 42],
                region: "eu-west".into(),
            }))),
            ..Default::default()
        });
        let directory = Arc::new(MockGameServerDirectory {
            resolve_response: Mutex::new(Some(Ok(resolved_server()))),
            ..Default::default()
        });
        let provisioner = Arc::new(MockGameServerProvisioner {
            create_responses: Mutex::new(vec![Err(GameServerError::UpstreamUnavailable)]),
            ..Default::default()
        });

        let result = matchmaking_service(auth, matchmaking, directory, provisioner)
            .poll_status(PollMatchmaking {
                session_token: "token-123".into(),
                ticket_id: "ticket-123".into(),
            })
            .await;

        assert_eq!(result, Err(PollMatchmakingError::UpstreamUnavailable));
    }
}
