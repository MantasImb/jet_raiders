use crate::domain::AuthGuestRequest;
use crate::interface_adapters::protocol::{HeadGuestLoginRequest, HeadGuestLoginResponse};
use crate::interface_adapters::state::AppState;
use axum::{Json, extract::State, http::StatusCode};
use std::sync::Arc;

#[tracing::instrument(
    name = "guest_login",
    skip_all,
    fields(guest_id = ?body.guest_id)
)]
pub async fn guest_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HeadGuestLoginRequest>,
) -> Result<Json<HeadGuestLoginResponse>, StatusCode> {
    // Map the head request into the auth request payload.
    let auth_req = AuthGuestRequest {
        guest_id: body.guest_id,
        display_name: body.display_name,
    };

    // Call auth to create or validate the guest session.
    let auth_res = state
        .auth
        .create_guest_session(auth_req)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to create guest session.");
            StatusCode::BAD_GATEWAY
        })?;

    tracing::info!("guest session created successfully.");

    // Return the session token to the client.
    Ok(Json(HeadGuestLoginResponse {
        session_token: auth_res.token,
    }))
}
