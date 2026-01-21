use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::SystemTime};
use tokio::sync::Mutex;
use uuid::Uuid;

// Basic session lifetime for guest tokens (in seconds).
const GUEST_SESSION_TTL_SECONDS: u64 = 60 * 60;

#[tokio::main]
async fn main() {
    // Shared, in-memory store for guest sessions.
    let state = AppState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    // Wire routes for the guest-only auth flow.
    let app = Router::new()
        .route("/auth/guest", post(guest_login))
        .route("/auth/verify-token", post(verify_token))
        .route("/auth/logout", post(logout))
        .with_state(state);

    // Bind and serve the Axum application.
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("auth server failed");
}

// Application state holding session storage.
#[derive(Clone)]
struct AppState {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

// Guest session record stored in memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Session {
    guest_id: String,
    display_name: String,
    metadata: Option<Value>,
    session_id: String,
    expires_at: u64,
}

// Request payload for guest login.
#[derive(Debug, Deserialize)]
struct GuestLoginRequest {
    guest_id: String,
    display_name: String,
    metadata: Option<Value>,
}

// Response payload for guest login.
#[derive(Debug, Serialize)]
struct GuestLoginResponse {
    token: String,
    expires_at: u64,
}

// Request payload for token verification.
#[derive(Debug, Deserialize)]
struct VerifyTokenRequest {
    token: String,
}

// Response payload for token verification.
#[derive(Debug, Serialize)]
struct VerifyTokenResponse {
    guest_id: String,
    display_name: String,
    metadata: Option<Value>,
    session_id: String,
    expires_at: u64,
}

// Request payload for logout.
#[derive(Debug, Deserialize)]
struct LogoutRequest {
    token: String,
}

// Response payload for logout.
#[derive(Debug, Serialize)]
struct LogoutResponse {
    revoked: bool,
}

// Handler for issuing a guest session token.
async fn guest_login(
    State(state): State<AppState>,
    Json(payload): Json<GuestLoginRequest>,
) -> Result<Json<GuestLoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate the incoming guest payload.
    validate_guest_payload(&payload)?;

    // Create a new session token and expiry.
    let token = Uuid::new_v4().to_string();
    let session_id = Uuid::new_v4().to_string();
    let expires_at = current_epoch_seconds() + GUEST_SESSION_TTL_SECONDS;

    let session = Session {
        guest_id: payload.guest_id,
        display_name: payload.display_name,
        metadata: payload.metadata,
        session_id,
        expires_at,
    };

    // Store the session keyed by token.
    let mut sessions = state.sessions.lock().await;
    sessions.insert(token.clone(), session);

    Ok(Json(GuestLoginResponse { token, expires_at }))
}

// Handler for verifying a session token.
async fn verify_token(
    State(state): State<AppState>,
    Json(payload): Json<VerifyTokenRequest>,
) -> Result<Json<VerifyTokenResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Load the session if it exists and is not expired.
    let mut sessions = state.sessions.lock().await;
    let session = sessions.get(&payload.token).cloned();

    match session {
        Some(session) if session.expires_at > current_epoch_seconds() => Ok(Json(
            VerifyTokenResponse {
                guest_id: session.guest_id,
                display_name: session.display_name,
                metadata: session.metadata,
                session_id: session.session_id,
                expires_at: session.expires_at,
            },
        )),
        Some(_) => {
            // Remove expired sessions to keep the store tidy.
            sessions.remove(&payload.token);
            Err(error_response(
                StatusCode::UNAUTHORIZED,
                "session expired",
            ))
        }
        None => Err(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid session token",
        )),
    }
}

// Handler for revoking a session token.
async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Remove the session if it exists.
    let mut sessions = state.sessions.lock().await;
    let revoked = sessions.remove(&payload.token).is_some();

    Ok(Json(LogoutResponse { revoked }))
}

// Simple error envelope for JSON responses.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

// Helper to build a JSON error response.
fn error_response(
    status: StatusCode,
    message: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            message: message.to_string(),
        }),
    )
}

// Validate guest payload fields with minimal checks.
fn validate_guest_payload(
    payload: &GuestLoginRequest,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if payload.guest_id.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "guest_id is required",
        ));
    }

    if payload.display_name.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "display_name is required",
        ));
    }

    Ok(())
}

// Get the current time as epoch seconds.
fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
