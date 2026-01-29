use serde::{Deserialize, Serialize};
use serde_json::Value;

// Request payload for guest login.
#[derive(Debug, Deserialize)]
pub struct GuestLoginRequest {
    pub guest_id: String,
    pub display_name: String,
    pub metadata: Option<Value>,
}

// Response payload for guest login.
#[derive(Debug, Serialize)]
pub struct GuestLoginResponse {
    pub token: String,
    pub expires_at: u64,
}

// Request payload for token verification.
#[derive(Debug, Deserialize)]
pub struct VerifyTokenRequest {
    pub token: String,
}

// Response payload for token verification.
#[derive(Debug, Serialize)]
pub struct VerifyTokenResponse {
    pub guest_id: String,
    pub display_name: String,
    pub metadata: Option<Value>,
    pub session_id: String,
    pub expires_at: u64,
}

// Request payload for logout.
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub token: String,
}

// Response payload for logout.
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub revoked: bool,
}

// Simple error envelope for JSON responses.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}
