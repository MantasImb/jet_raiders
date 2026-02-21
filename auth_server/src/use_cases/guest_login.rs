use uuid::Uuid;

use crate::domain::entities::Session;
use crate::domain::errors::AuthError;
use crate::domain::ports::{Clock, SessionStore};
use crate::interface_adapters::protocol::GuestLoginRequest;

// Response returned by the guest login use case.
pub struct GuestLoginResponse {
    pub token: String,
    pub expires_at: u64,
    pub display_name: String,
}

// Guest login use case with injected dependencies.
pub struct GuestLoginUseCase<C, S> {
    pub clock: C,
    pub store: S,
    pub ttl_seconds: u64,
}

impl<C, S> GuestLoginUseCase<C, S>
where
    C: Clock,
    S: SessionStore,
{
    pub async fn execute(
        &self,
        payload: GuestLoginRequest,
    ) -> Result<GuestLoginResponse, AuthError> {
        if payload.guest_id == 0 {
            return Err(AuthError::InvalidGuestId);
        }
        let display_name = validate_display_name(&payload.display_name)?;

        if display_name.is_empty() {
            return Err(AuthError::InvalidDisplayName);
        }

        let token = Uuid::new_v4().to_string();
        let session_id = Uuid::new_v4().to_string();
        let expires_at = self.clock.now_epoch_seconds() + self.ttl_seconds;

        let session = Session {
            guest_id: payload.guest_id,
            display_name: display_name.clone(),
            metadata: payload.metadata,
            session_id,
            expires_at,
        };

        self.store
            .insert(token.clone(), session)
            .await
            .map_err(|_| AuthError::StorageFailure)?;

        Ok(GuestLoginResponse {
            token,
            expires_at,
            display_name,
        })
    }
}

fn validate_display_name(value: &str) -> Result<String, AuthError> {
    // Keep names compact and readable for game UI and logs.
    const MIN_LEN: usize = 3;
    const MAX_LEN: usize = 32;

    let normalized = value.trim();
    let len = normalized.chars().count();
    if len < MIN_LEN || len > MAX_LEN {
        return Err(AuthError::InvalidDisplayName);
    }

    // Allow a simple safe charset across the stack.
    if !normalized
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
    {
        return Err(AuthError::InvalidDisplayName);
    }

    Ok(normalized.to_string())
}
