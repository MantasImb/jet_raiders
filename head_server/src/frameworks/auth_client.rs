use crate::use_cases::{
    AuthProvider, AuthProviderError, GuestInit, GuestInitResult, GuestLogin, GuestLoginResult,
};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AuthClient {
    http: Client,
    pub base_url: String,
}

#[derive(Debug, Serialize)]
struct AuthGuestInitRequest {
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct AuthGuestInitResponse {
    guest_id: u64,
    token: String,
    expires_at: u64,
}

#[derive(Debug, Serialize)]
struct AuthGuestLoginRequest {
    guest_id: u64,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct AuthGuestLoginResponse {
    token: String,
    expires_at: u64,
}

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
        req: GuestInit,
    ) -> Result<GuestInitResult, AuthProviderError> {
        let url = format!("{}/auth/guest/init", self.base_url);
        let response = self
            .http
            .post(url)
            .json(&AuthGuestInitRequest {
                display_name: req.display_name,
            })
            .send()
            .await
            .map_err(|_| AuthProviderError::UpstreamUnavailable)?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_status_to_error(status));
        }

        let payload = response
            .json::<AuthGuestInitResponse>()
            .await
            .map_err(|_| AuthProviderError::Unexpected)?;

        Ok(GuestInitResult {
            guest_id: payload.guest_id,
            session_token: payload.token,
            expires_at: payload.expires_at,
        })
    }

    async fn create_guest_session(
        &self,
        req: GuestLogin,
    ) -> Result<GuestLoginResult, AuthProviderError> {
        let url = format!("{}/auth/guest", self.base_url);
        let response = self
            .http
            .post(url)
            .json(&AuthGuestLoginRequest {
                guest_id: req.guest_id,
                display_name: req.display_name,
            })
            .send()
            .await
            .map_err(|_| AuthProviderError::UpstreamUnavailable)?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_status_to_error(status));
        }

        let payload = response
            .json::<AuthGuestLoginResponse>()
            .await
            .map_err(|_| AuthProviderError::Unexpected)?;

        Ok(GuestLoginResult {
            session_token: payload.token,
            expires_at: payload.expires_at,
        })
    }
}

fn map_status_to_error(status: StatusCode) -> AuthProviderError {
    match status {
        StatusCode::BAD_REQUEST => AuthProviderError::BadRequest,
        StatusCode::UNAUTHORIZED => AuthProviderError::Unauthorized,
        StatusCode::FORBIDDEN => AuthProviderError::Forbidden,
        StatusCode::NOT_FOUND => AuthProviderError::NotFound,
        StatusCode::UNPROCESSABLE_ENTITY => AuthProviderError::UnprocessableEntity,
        _ if status.is_client_error() => AuthProviderError::BadRequest,
        _ => AuthProviderError::UpstreamUnavailable,
    }
}
