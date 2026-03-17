use crate::use_cases::{
    AuthProvider, AuthProviderError, GuestInit, GuestInitResult, GuestLogin, GuestLoginResult,
};
use async_trait::async_trait;
use reqwest::{Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

#[derive(Clone)]
pub struct AuthClient {
    http: Client,
    pub base_url: Url,
}

#[derive(Debug)]
pub struct AuthClientConfigError {
    message: String,
}

impl fmt::Display for AuthClientConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AuthClientConfigError {}

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
    pub fn new(base_url: &str) -> Result<Self, AuthClientConfigError> {
        let mut base_url = Url::parse(base_url).map_err(|error| AuthClientConfigError {
            message: format!("invalid auth base URL: {error}"),
        })?;
        if !base_url.path().ends_with('/') {
            let normalized_path = format!("{}/", base_url.path());
            base_url.set_path(&normalized_path);
        }

        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| AuthClientConfigError {
                message: format!("failed to build auth HTTP client: {error}"),
            })?;

        Ok(Self { http, base_url })
    }

    fn endpoint(&self, path: &str) -> Result<Url, AuthProviderError> {
        self.base_url
            .join(path)
            .map_err(|_| AuthProviderError::Unexpected)
    }
}

#[async_trait]
impl AuthProvider for AuthClient {
    async fn create_guest_identity(
        &self,
        req: GuestInit,
    ) -> Result<GuestInitResult, AuthProviderError> {
        let url = self.endpoint("auth/guest/init")?;
        let response = self
            .http
            .post(url)
            .json(&AuthGuestInitRequest {
                display_name: req.display_name,
            })
            .send()
            .await
            .map_err(|_| AuthProviderError::UpstreamUnavailable)?;
        let response = ensure_success_response(response).await?;

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
        let url = self.endpoint("auth/guest")?;
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
        let response = ensure_success_response(response).await?;

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

async fn ensure_success_response(response: Response) -> Result<Response, AuthProviderError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    // Drain the response body so the underlying connection can be reused.
    let _ = response.bytes().await;
    Err(map_status_to_error(status))
}

fn map_status_to_error(status: StatusCode) -> AuthProviderError {
    match status {
        StatusCode::BAD_REQUEST => AuthProviderError::BadRequest,
        StatusCode::UNAUTHORIZED => AuthProviderError::Unauthorized,
        StatusCode::FORBIDDEN => AuthProviderError::Forbidden,
        StatusCode::NOT_FOUND => AuthProviderError::NotFound,
        StatusCode::UNPROCESSABLE_ENTITY => AuthProviderError::UnprocessableEntity,
        _ if status.is_client_error() => AuthProviderError::UnexpectedClientError,
        _ => AuthProviderError::UpstreamUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, extract::State, http::StatusCode as AxumStatusCode, routing::post};
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;

    #[derive(Clone, Default)]
    struct RequestLog {
        paths: Arc<Mutex<Vec<String>>>,
    }

    async fn capture_request_path(
        State(log): State<RequestLog>,
        request: axum::extract::Request,
    ) -> (AxumStatusCode, Json<serde_json::Value>) {
        log.paths
            .lock()
            .expect("lock should not be poisoned")
            .push(request.uri().path().to_string());

        (
            AxumStatusCode::OK,
            Json(json!({
                "token": "token",
                "expires_at": 123
            })),
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

    #[test]
    fn new_rejects_invalid_base_url() {
        match AuthClient::new("not a url") {
            Ok(_) => panic!("invalid URLs should fail"),
            Err(error) => assert!(error.to_string().contains("invalid auth base URL")),
        }
    }

    #[test]
    fn endpoint_normalizes_base_urls_with_and_without_trailing_slashes() {
        let without_slash =
            AuthClient::new("http://localhost:3002/prefix").expect("client should build");
        let with_slash =
            AuthClient::new("http://localhost:3002/prefix/").expect("client should build");

        assert_eq!(
            without_slash
                .endpoint("auth/guest")
                .expect("endpoint should build")
                .as_str(),
            "http://localhost:3002/prefix/auth/guest"
        );
        assert_eq!(
            with_slash
                .endpoint("auth/guest")
                .expect("endpoint should build")
                .as_str(),
            "http://localhost:3002/prefix/auth/guest"
        );
    }

    #[tokio::test]
    async fn create_guest_session_joins_against_base_path_prefix() {
        let log = RequestLog::default();
        let router = Router::new()
            .route("/prefix/auth/guest", post(capture_request_path))
            .with_state(log.clone());
        let base_url = spawn_test_server(router).await;
        let client = AuthClient::new(&format!("{base_url}/prefix")).expect("client should build");

        let result = client
            .create_guest_session(GuestLogin {
                guest_id: 42,
                display_name: "Pilot".into(),
            })
            .await
            .expect("request should succeed");

        assert_eq!(
            result,
            GuestLoginResult {
                session_token: "token".into(),
                expires_at: 123,
            }
        );
        assert_eq!(
            log.paths
                .lock()
                .expect("lock should not be poisoned")
                .as_slice(),
            &["/prefix/auth/guest".to_string()]
        );
    }

    #[test]
    fn known_and_unknown_statuses_map_as_expected() {
        assert_eq!(
            map_status_to_error(StatusCode::BAD_REQUEST),
            AuthProviderError::BadRequest
        );
        assert_eq!(
            map_status_to_error(StatusCode::UNAUTHORIZED),
            AuthProviderError::Unauthorized
        );
        assert_eq!(
            map_status_to_error(StatusCode::FORBIDDEN),
            AuthProviderError::Forbidden
        );
        assert_eq!(
            map_status_to_error(StatusCode::NOT_FOUND),
            AuthProviderError::NotFound
        );
        assert_eq!(
            map_status_to_error(StatusCode::UNPROCESSABLE_ENTITY),
            AuthProviderError::UnprocessableEntity
        );
        assert_eq!(
            map_status_to_error(StatusCode::CONFLICT),
            AuthProviderError::UnexpectedClientError
        );
        assert_eq!(
            map_status_to_error(StatusCode::TOO_MANY_REQUESTS),
            AuthProviderError::UnexpectedClientError
        );
        assert_eq!(
            map_status_to_error(StatusCode::INTERNAL_SERVER_ERROR),
            AuthProviderError::UpstreamUnavailable
        );
    }
}
