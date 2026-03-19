use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestInit {
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestInitResult {
    pub guest_id: u64,
    pub session_token: String,
    pub expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestLogin {
    pub guest_id: u64,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestLoginResult {
    pub session_token: String,
    pub expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifySession {
    pub session_token: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifySessionResult {
    pub user_id: u64,
    pub display_name: String,
    pub session_id: String,
    pub expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthProviderError {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    UnprocessableEntity,
    UnexpectedClientError,
    UpstreamUnavailable,
    Unexpected,
}

impl fmt::Display for AuthProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthProviderError::BadRequest => write!(f, "bad request"),
            AuthProviderError::Unauthorized => write!(f, "unauthorized"),
            AuthProviderError::Forbidden => write!(f, "forbidden"),
            AuthProviderError::NotFound => write!(f, "not found"),
            AuthProviderError::UnprocessableEntity => write!(f, "unprocessable entity"),
            AuthProviderError::UnexpectedClientError => {
                write!(f, "unexpected upstream client error")
            }
            AuthProviderError::UpstreamUnavailable => write!(f, "upstream unavailable"),
            AuthProviderError::Unexpected => write!(f, "unexpected auth provider error"),
        }
    }
}

impl std::error::Error for AuthProviderError {}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn create_guest_identity(
        &self,
        req: GuestInit,
    ) -> Result<GuestInitResult, AuthProviderError>;

    async fn create_guest_session(
        &self,
        req: GuestLogin,
    ) -> Result<GuestLoginResult, AuthProviderError>;

    async fn verify_session(
        &self,
        req: VerifySession,
    ) -> Result<VerifySessionResult, AuthProviderError>;
}

#[derive(Clone)]
pub struct GuestSessionService {
    auth: Arc<dyn AuthProvider>,
}

impl GuestSessionService {
    pub fn new(auth: Arc<dyn AuthProvider>) -> Self {
        Self { auth }
    }

    pub async fn guest_init(
        &self,
        request: GuestInit,
    ) -> Result<GuestInitResult, AuthProviderError> {
        self.auth.create_guest_identity(request).await
    }

    pub async fn guest_login(
        &self,
        request: GuestLogin,
    ) -> Result<GuestLoginResult, AuthProviderError> {
        self.auth.create_guest_session(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockAuthProvider {
        init_requests: Mutex<Vec<GuestInit>>,
        login_requests: Mutex<Vec<GuestLogin>>,
        verify_requests: Mutex<Vec<VerifySession>>,
        init_response: Mutex<Option<Result<GuestInitResult, AuthProviderError>>>,
        login_response: Mutex<Option<Result<GuestLoginResult, AuthProviderError>>>,
        verify_response: Mutex<Option<Result<VerifySessionResult, AuthProviderError>>>,
    }

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn create_guest_identity(
            &self,
            req: GuestInit,
        ) -> Result<GuestInitResult, AuthProviderError> {
            self.init_requests.lock().unwrap().push(req);
            self.init_response
                .lock()
                .unwrap()
                .take()
                .expect("init response should be configured")
        }

        async fn create_guest_session(
            &self,
            req: GuestLogin,
        ) -> Result<GuestLoginResult, AuthProviderError> {
            self.login_requests.lock().unwrap().push(req);
            self.login_response
                .lock()
                .unwrap()
                .take()
                .expect("login response should be configured")
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

    #[tokio::test]
    async fn guest_init_delegates_to_auth_provider() {
        let auth = Arc::new(MockAuthProvider {
            init_response: Mutex::new(Some(Ok(GuestInitResult {
                guest_id: 42,
                session_token: "token".into(),
                expires_at: 99,
            }))),
            ..Default::default()
        });
        let service = GuestSessionService::new(auth.clone());

        let result = service
            .guest_init(GuestInit {
                display_name: "Pilot".into(),
            })
            .await
            .expect("guest init should succeed");

        assert_eq!(
            result,
            GuestInitResult {
                guest_id: 42,
                session_token: "token".into(),
                expires_at: 99,
            }
        );
        assert_eq!(
            auth.init_requests.lock().unwrap().as_slice(),
            &[GuestInit {
                display_name: "Pilot".into(),
            }]
        );
    }

    #[tokio::test]
    async fn guest_login_delegates_to_auth_provider() {
        let auth = Arc::new(MockAuthProvider {
            login_response: Mutex::new(Some(Ok(GuestLoginResult {
                session_token: "token".into(),
                expires_at: 123,
            }))),
            ..Default::default()
        });
        let service = GuestSessionService::new(auth.clone());

        let result = service
            .guest_login(GuestLogin {
                guest_id: 7,
                display_name: "Pilot".into(),
            })
            .await
            .expect("guest login should succeed");

        assert_eq!(
            result,
            GuestLoginResult {
                session_token: "token".into(),
                expires_at: 123,
            }
        );
        assert_eq!(
            auth.login_requests.lock().unwrap().as_slice(),
            &[GuestLogin {
                guest_id: 7,
                display_name: "Pilot".into(),
            }]
        );
    }

}
