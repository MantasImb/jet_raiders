use crate::use_cases::{
    MatchmakingEnqueueResult, MatchmakingProvider, MatchmakingProviderError,
    MatchmakingQueueRequest, MatchmakingTicketStatus,
};
use async_trait::async_trait;
use reqwest::{Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

#[derive(Clone)]
pub struct MatchmakingClient {
    http: Client,
    pub base_url: Url,
}

#[derive(Debug)]
pub struct MatchmakingClientConfigError {
    message: String,
}

impl fmt::Display for MatchmakingClientConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MatchmakingClientConfigError {}

#[derive(Debug, Serialize)]
struct MatchmakingQueueHttpRequest {
    player_id: String,
    player_skill: u32,
    region: String,
}

#[derive(Debug, Deserialize)]
struct MatchmakingHttpResponse {
    status: MatchmakingHttpStatus,
    ticket_id: Option<String>,
    match_id: Option<String>,
    opponent_id: Option<String>,
    region: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MatchmakingHttpStatus {
    Waiting,
    Matched,
}

impl MatchmakingClient {
    pub fn new(base_url: &str) -> Result<Self, MatchmakingClientConfigError> {
        let mut base_url = Url::parse(base_url).map_err(|error| MatchmakingClientConfigError {
            message: format!("invalid matchmaking base URL: {error}"),
        })?;
        if !base_url.path().ends_with('/') {
            let normalized_path = format!("{}/", base_url.path());
            base_url.set_path(&normalized_path);
        }

        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| MatchmakingClientConfigError {
                message: format!("failed to build matchmaking HTTP client: {error}"),
            })?;

        Ok(Self { http, base_url })
    }

    fn endpoint(&self, path: &str) -> Result<Url, MatchmakingProviderError> {
        self.base_url
            .join(path)
            .map_err(|_| MatchmakingProviderError::Unexpected)
    }
}

#[async_trait]
impl MatchmakingProvider for MatchmakingClient {
    async fn enqueue(
        &self,
        request: MatchmakingQueueRequest,
    ) -> Result<MatchmakingEnqueueResult, MatchmakingProviderError> {
        let url = self.endpoint("matchmaking/queue")?;
        let response = self
            .http
            .post(url)
            .json(&MatchmakingQueueHttpRequest {
                player_id: request.player_id,
                player_skill: request.player_skill,
                region: request.region,
            })
            .send()
            .await
            .map_err(|_| MatchmakingProviderError::UpstreamUnavailable)?;
        let response = ensure_success_response(response).await?;

        let payload = response
            .json::<MatchmakingHttpResponse>()
            .await
            .map_err(|_| MatchmakingProviderError::Unexpected)?;

        match payload.status {
            MatchmakingHttpStatus::Waiting => Ok(MatchmakingEnqueueResult::Waiting {
                ticket_id: payload
                    .ticket_id
                    .ok_or(MatchmakingProviderError::Unexpected)?,
                region: payload.region,
            }),
            MatchmakingHttpStatus::Matched => Ok(MatchmakingEnqueueResult::Matched {
                match_id: payload
                    .match_id
                    .ok_or(MatchmakingProviderError::Unexpected)?,
                opponent_id: payload
                    .opponent_id
                    .ok_or(MatchmakingProviderError::Unexpected)?,
                region: payload.region,
            }),
        }
    }

    async fn poll_status(
        &self,
        ticket_id: String,
    ) -> Result<MatchmakingTicketStatus, MatchmakingProviderError> {
        let url = self.endpoint(&format!("matchmaking/queue/{ticket_id}"))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|_| MatchmakingProviderError::UpstreamUnavailable)?;
        let response = ensure_success_response(response).await?;

        let payload = response
            .json::<MatchmakingHttpResponse>()
            .await
            .map_err(|_| MatchmakingProviderError::Unexpected)?;

        match payload.status {
            MatchmakingHttpStatus::Waiting => Ok(MatchmakingTicketStatus::Waiting {
                ticket_id: payload
                    .ticket_id
                    .ok_or(MatchmakingProviderError::Unexpected)?,
                region: payload.region,
            }),
            MatchmakingHttpStatus::Matched => Ok(MatchmakingTicketStatus::Matched {
                match_id: payload
                    .match_id
                    .ok_or(MatchmakingProviderError::Unexpected)?,
                opponent_id: payload
                    .opponent_id
                    .ok_or(MatchmakingProviderError::Unexpected)?,
                region: payload.region,
            }),
        }
    }
}

async fn ensure_success_response(response: Response) -> Result<Response, MatchmakingProviderError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    // Drain the response body so the underlying connection can be reused.
    let _ = response.bytes().await;
    Err(map_status_to_error(status))
}

fn map_status_to_error(status: StatusCode) -> MatchmakingProviderError {
    match status {
        StatusCode::BAD_REQUEST => MatchmakingProviderError::BadRequest,
        StatusCode::CONFLICT => MatchmakingProviderError::Conflict,
        StatusCode::NOT_FOUND => MatchmakingProviderError::NotFound,
        _ if status.is_client_error() => MatchmakingProviderError::UnexpectedClientError,
        _ => MatchmakingProviderError::UpstreamUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::State,
        http::StatusCode as AxumStatusCode,
        routing::{get, post},
    };
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
                "status": "waiting",
                "ticket_id": "ticket-123",
                "match_id": null,
                "opponent_id": null,
                "region": "eu-west"
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
        match MatchmakingClient::new("not a url") {
            Ok(_) => panic!("invalid URLs should fail"),
            Err(error) => assert!(error.to_string().contains("invalid matchmaking base URL")),
        }
    }

    #[test]
    fn endpoint_normalizes_base_urls_with_and_without_trailing_slashes() {
        let without_slash =
            MatchmakingClient::new("http://localhost:3003/prefix").expect("client should build");
        let with_slash =
            MatchmakingClient::new("http://localhost:3003/prefix/").expect("client should build");

        assert_eq!(
            without_slash
                .endpoint("matchmaking/queue")
                .expect("endpoint should build")
                .as_str(),
            "http://localhost:3003/prefix/matchmaking/queue"
        );
        assert_eq!(
            with_slash
                .endpoint("matchmaking/queue")
                .expect("endpoint should build")
                .as_str(),
            "http://localhost:3003/prefix/matchmaking/queue"
        );
    }

    #[tokio::test]
    async fn enqueue_joins_against_base_path_prefix() {
        let log = RequestLog::default();
        let router = Router::new()
            .route("/prefix/matchmaking/queue", post(capture_request_path))
            .with_state(log.clone());
        let base_url = spawn_test_server(router).await;
        let client =
            MatchmakingClient::new(&format!("{base_url}/prefix")).expect("client should build");

        let result = client
            .enqueue(MatchmakingQueueRequest {
                player_id: "player-1".into(),
                player_skill: 1200,
                region: "eu-west".into(),
            })
            .await
            .expect("request should succeed");

        assert_eq!(
            result,
            MatchmakingEnqueueResult::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(
            log.paths
                .lock()
                .expect("lock should not be poisoned")
                .as_slice(),
            &["/prefix/matchmaking/queue".to_string()]
        );
    }

    #[tokio::test]
    async fn poll_status_joins_against_base_path_prefix() {
        let log = RequestLog::default();
        let router = Router::new()
            .route(
                "/prefix/matchmaking/queue/ticket-123",
                get(capture_request_path),
            )
            .with_state(log.clone());
        let base_url = spawn_test_server(router).await;
        let client =
            MatchmakingClient::new(&format!("{base_url}/prefix")).expect("client should build");

        let result = client
            .poll_status("ticket-123".into())
            .await
            .expect("request should succeed");

        assert_eq!(
            result,
            MatchmakingTicketStatus::Waiting {
                ticket_id: "ticket-123".into(),
                region: "eu-west".into(),
            }
        );
        assert_eq!(
            log.paths
                .lock()
                .expect("lock should not be poisoned")
                .as_slice(),
            &["/prefix/matchmaking/queue/ticket-123".to_string()]
        );
    }

    #[test]
    fn conflict_maps_to_matchmaking_conflict_error() {
        assert_eq!(
            map_status_to_error(StatusCode::CONFLICT),
            MatchmakingProviderError::Conflict
        );
    }

    #[test]
    fn not_found_maps_to_matchmaking_not_found_error() {
        assert_eq!(
            map_status_to_error(StatusCode::NOT_FOUND),
            MatchmakingProviderError::NotFound
        );
    }
}
