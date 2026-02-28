use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Auth verification response consumed by the game server join path.
#[derive(Debug, Clone, Deserialize)]
pub struct VerifiedIdentity {
    pub user_id: u64,
    pub display_name: String,
    pub session_id: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize)]
struct VerifyTokenRequest<'a> {
    token: &'a str,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    message: String,
}

#[derive(Debug)]
pub enum VerifyTokenError {
    InvalidToken,
    SessionExpired,
    UpstreamUnavailable,
}

// Thin reqwest client for auth token verification.
#[derive(Clone)]
pub struct AuthClient {
    http: reqwest::Client,
    base_url: String,
}

impl AuthClient {
    pub fn new(base_url: impl Into<String>, timeout: Duration) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder().timeout(timeout).build()?;
        Ok(Self {
            http,
            base_url: base_url.into(),
        })
    }

    pub async fn verify_token(&self, token: &str) -> Result<VerifiedIdentity, VerifyTokenError> {
        let url = format!("{}/auth/verify-token", self.base_url);
        let response = self
            .http
            .post(url)
            .json(&VerifyTokenRequest { token })
            .send()
            .await
            .map_err(|_| VerifyTokenError::UpstreamUnavailable)?;

        if response.status().is_success() {
            return response
                .json::<VerifiedIdentity>()
                .await
                .map_err(|_| VerifyTokenError::UpstreamUnavailable);
        }

        if response.status() == StatusCode::UNAUTHORIZED {
            let error = response
                .json::<ErrorResponse>()
                .await
                .map_err(|_| VerifyTokenError::UpstreamUnavailable)?;

            // TODO: switch to stable machine-readable error codes from auth.
            if error.message == "session expired" {
                return Err(VerifyTokenError::SessionExpired);
            }
            return Err(VerifyTokenError::InvalidToken);
        }

        Err(VerifyTokenError::UpstreamUnavailable)
    }
}
