use crate::domain::errors::AuthError;
use crate::interface_adapters::protocol::{
    ErrorResponse, GuestLoginRequest, GuestLoginResponse, LogoutRequest, LogoutResponse,
    VerifyTokenRequest, VerifyTokenResponse,
};
use crate::interface_adapters::state::{
    AppState,
    InMemorySessionStore,
    PostgresGuestProfileStore,
    SystemClock,
};
use crate::use_cases::guest_login::GuestLoginUseCase;
use crate::use_cases::logout::LogoutUseCase;
use crate::use_cases::verify_token::VerifyTokenUseCase;
use axum::{extract::State, http::StatusCode, Json};
use tracing::warn;

// Basic session lifetime for guest tokens (in seconds).
const GUEST_SESSION_TTL_SECONDS: u64 = 60 * 60;

// Handler for issuing a guest session token.
pub async fn guest_login(
    State(state): State<AppState>,
    Json(payload): Json<GuestLoginRequest>,
) -> Result<Json<GuestLoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Capture guest identity fields before moving the payload into the use case.
    let guest_id = payload.guest_id.clone();
    let display_name = payload.display_name.clone();
    let metadata_json = payload
        .metadata
        .as_ref()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let store = InMemorySessionStore {
        sessions: state.sessions.clone(),
    };
    let use_case = GuestLoginUseCase {
        clock: SystemClock,
        store,
        ttl_seconds: GUEST_SESSION_TTL_SECONDS,
    };

    let result = use_case
        .execute(payload)
        .await
        .map_err(|err| map_auth_error(err, AuthErrorContext::GuestLogin))?;

    // Best-effort persistence of the guest profile for downstream services.
    let profile_store = PostgresGuestProfileStore {
        db: state.db.clone(),
    };
    if let Err(err) = profile_store
        .upsert_guest_profile(&guest_id, &display_name, &metadata_json)
        .await
    {
        warn!(error = %err, "failed to upsert guest profile");
    }

    Ok(Json(GuestLoginResponse {
        token: result.token,
        expires_at: result.expires_at,
    }))
}

// Handler for verifying a session token.
pub async fn verify_token(
    State(state): State<AppState>,
    Json(payload): Json<VerifyTokenRequest>,
) -> Result<Json<VerifyTokenResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = InMemorySessionStore {
        sessions: state.sessions.clone(),
    };
    let use_case = VerifyTokenUseCase {
        clock: SystemClock,
        store,
    };

    let result = use_case
        .execute(payload.token)
        .await
        .map_err(|err| map_auth_error(err, AuthErrorContext::VerifyToken))?;

    Ok(Json(VerifyTokenResponse {
        guest_id: result.guest_id,
        display_name: result.display_name,
        metadata: result.metadata,
        session_id: result.session_id,
        expires_at: result.expires_at,
    }))
}

// Handler for revoking a session token.
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = InMemorySessionStore {
        sessions: state.sessions.clone(),
    };
    let use_case = LogoutUseCase { store };

    let result = use_case
        .execute(payload.token)
        .await
        .map_err(|err| map_auth_error(err, AuthErrorContext::Logout))?;

    Ok(Json(LogoutResponse {
        revoked: result.revoked,
    }))
}

// Helper to build a JSON error response.
fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            message: message.to_string(),
        }),
    )
}

// Maps domain errors to HTTP responses by endpoint context.
enum AuthErrorContext {
    GuestLogin,
    VerifyToken,
    Logout,
}

fn map_auth_error(err: AuthError, context: AuthErrorContext) -> (StatusCode, Json<ErrorResponse>) {
    match context {
        AuthErrorContext::GuestLogin => match err {
            AuthError::InvalidGuestId => {
                error_response(StatusCode::BAD_REQUEST, "guest_id is required")
            }
            AuthError::InvalidDisplayName => {
                error_response(StatusCode::BAD_REQUEST, "display_name is required")
            }
            AuthError::StorageFailure
            | AuthError::InvalidToken
            | AuthError::SessionExpired => {
                error_response(StatusCode::BAD_GATEWAY, "storage error")
            }
        },
        AuthErrorContext::VerifyToken => match err {
            AuthError::InvalidToken => {
                error_response(StatusCode::UNAUTHORIZED, "invalid session token")
            }
            AuthError::SessionExpired => error_response(StatusCode::UNAUTHORIZED, "session expired"),
            AuthError::StorageFailure => error_response(StatusCode::BAD_GATEWAY, "storage error"),
            AuthError::InvalidGuestId | AuthError::InvalidDisplayName => {
                error_response(StatusCode::BAD_REQUEST, "invalid session data")
            }
        },
        AuthErrorContext::Logout => match err {
            AuthError::StorageFailure => error_response(StatusCode::BAD_GATEWAY, "storage error"),
            AuthError::InvalidGuestId
            | AuthError::InvalidDisplayName
            | AuthError::InvalidToken
            | AuthError::SessionExpired => error_response(StatusCode::BAD_REQUEST, "invalid token"),
        },
    }
}
