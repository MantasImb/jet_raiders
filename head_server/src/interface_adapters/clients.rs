use crate::domain::{
    AuthGuestInitRequest, AuthGuestInitResponse, AuthGuestRequest, AuthGuestResponse,
    AuthProvider,
};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::fmt;

// The clients defined here are for reqwest clients to communicate with external services.
// Thin wrapper around reqwest for auth service calls.
#[derive(Clone)]
pub struct AuthClient {
    http: Client,
    pub base_url: String,
}

#[derive(Debug, Deserialize)]
struct AuthErrorResponse {
    message: String,
}

#[derive(Debug)]
pub enum AuthClientError {
    Transport(reqwest::Error),
    Upstream {
        status: StatusCode,
        message: Option<String>,
    },
    Decode(reqwest::Error),
}

impl fmt::Display for AuthClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthClientError::Transport(err) => write!(f, "auth transport error: {err}"),
            AuthClientError::Upstream { status, message } => {
                if let Some(message) = message {
                    write!(f, "auth upstream error {status}: {message}")
                } else {
                    write!(f, "auth upstream error {status}")
                }
            }
            AuthClientError::Decode(err) => write!(f, "auth response decode error: {err}"),
        }
    }
}

impl std::error::Error for AuthClientError {}

impl AuthClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl AuthProvider for AuthClient {
    async fn create_guest_identity(
        &self,
        req: AuthGuestInitRequest,
    ) -> Result<AuthGuestInitResponse, Box<dyn std::error::Error>> {
        // Compose the auth URL and POST the first-time guest payload.
        let url = format!("{}/auth/guest/init", self.base_url);
        let res = self
            .http
            .post(url)
            .json(&req)
            .send()
            .await
            .map_err(AuthClientError::Transport)?;
        let status = res.status();

        // Keep upstream status/message so handlers can preserve 4xx semantics.
        if !status.is_success() {
            let message = res
                .json::<AuthErrorResponse>()
                .await
                .ok()
                .map(|payload| payload.message);
            return Err(Box::new(AuthClientError::Upstream { status, message }));
        }

        // Parse the auth response into our DTO.
        res.json::<AuthGuestInitResponse>()
            .await
            .map_err(|err| Box::new(AuthClientError::Decode(err)) as Box<dyn std::error::Error>)
    }

    async fn create_guest_session(
        &self,
        req: AuthGuestRequest,
    ) -> Result<AuthGuestResponse, Box<dyn std::error::Error>> {
        // Compose the auth URL and POST the guest payload.
        let url = format!("{}/auth/guest", self.base_url);
        let res = self
            .http
            .post(url)
            .json(&req)
            .send()
            .await
            .map_err(AuthClientError::Transport)?;
        let status = res.status();

        // Keep upstream status/message so handlers can preserve 4xx semantics.
        if !status.is_success() {
            let message = res
                .json::<AuthErrorResponse>()
                .await
                .ok()
                .map(|payload| payload.message);
            return Err(Box::new(AuthClientError::Upstream { status, message }));
        }

        // Parse the auth response into our DTO.
        res.json::<AuthGuestResponse>()
            .await
            .map_err(|err| Box::new(AuthClientError::Decode(err)) as Box<dyn std::error::Error>)
    }
}
