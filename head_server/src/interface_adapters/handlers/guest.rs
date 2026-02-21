use crate::domain::{AuthGuestInitRequest, AuthGuestRequest};
use crate::interface_adapters::clients::AuthClientError;
use crate::interface_adapters::protocol::{
    HeadGuestInitRequest, HeadGuestInitResponse, HeadGuestLoginRequest, HeadGuestLoginResponse,
};
use crate::interface_adapters::state::AppState;
use axum::{Json, extract::State, http::StatusCode};
use std::sync::Arc;

#[tracing::instrument(name = "guest_init", skip_all)]
pub async fn guest_init(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HeadGuestInitRequest>,
) -> Result<Json<HeadGuestInitResponse>, StatusCode> {
    // Map the head request into the auth init payload.
    let auth_req = AuthGuestInitRequest {
        display_name: body.display_name,
    };

    // Call auth to create a first-time guest identity and session.
    let auth_res = state.auth.create_guest_identity(auth_req).await.map_err(|e| {
        tracing::error!(error = ?e, "failed to create guest identity.");
        map_auth_provider_error(e.as_ref())
    })?;

    tracing::info!(guest_id = auth_res.guest_id, "guest identity created successfully.");

    Ok(Json(HeadGuestInitResponse {
        // Keep guest_id stringly-typed on the client boundary to avoid JSON number precision loss.
        guest_id: auth_res.guest_id.to_string(),
        session_token: auth_res.token,
        expires_at: auth_res.expires_at,
    }))
}

#[tracing::instrument(
    name = "guest_login",
    skip_all,
    fields(guest_id = ?body.guest_id)
)]
pub async fn guest_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HeadGuestLoginRequest>,
) -> Result<Json<HeadGuestLoginResponse>, StatusCode> {
    // Parse guest_id at the adapter boundary; domain/auth paths keep numeric IDs.
    let guest_id = body
        .guest_id
        .trim()
        .parse::<u64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Map the head request into the auth request payload.
    let auth_req = AuthGuestRequest {
        guest_id,
        display_name: body.display_name,
    };

    // Call auth to create or validate the guest session.
    let auth_res = state
        .auth
        .create_guest_session(auth_req)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to create guest session.");
            map_auth_provider_error(e.as_ref())
        })?;

    tracing::info!("guest session created successfully.");

    // Return the session token to the client.
    Ok(Json(HeadGuestLoginResponse {
        session_token: auth_res.token,
        expires_at: auth_res.expires_at,
    }))
}

fn map_auth_provider_error(err: &(dyn std::error::Error + 'static)) -> StatusCode {
    // Preserve upstream client errors for better API semantics and UX.
    if let Some(auth_err) = err.downcast_ref::<AuthClientError>() {
        if let AuthClientError::Upstream { status, .. } = auth_err {
            return match *status {
                StatusCode::BAD_REQUEST => StatusCode::BAD_REQUEST,
                StatusCode::UNAUTHORIZED => StatusCode::UNAUTHORIZED,
                StatusCode::FORBIDDEN => StatusCode::FORBIDDEN,
                StatusCode::NOT_FOUND => StatusCode::NOT_FOUND,
                StatusCode::UNPROCESSABLE_ENTITY => StatusCode::UNPROCESSABLE_ENTITY,
                _ if status.is_client_error() => StatusCode::BAD_REQUEST,
                _ => StatusCode::BAD_GATEWAY,
            };
        }
    }

    StatusCode::BAD_GATEWAY
}
