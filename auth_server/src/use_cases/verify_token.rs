use crate::domain::entities::Session;
use crate::domain::errors::AuthError;
use crate::domain::ports::{Clock, SessionStore};

// Response returned by the token verification use case.
pub struct VerifyTokenResponse {
    pub user_id: u64,
    pub display_name: String,
    pub metadata: Option<serde_json::Value>,
    pub session_id: String,
    pub expires_at: u64,
}

// Token verification use case with injected dependencies.
pub struct VerifyTokenUseCase<C, S> {
    pub clock: C,
    pub store: S,
}

impl<C, S> VerifyTokenUseCase<C, S>
where
    C: Clock,
    S: SessionStore,
{
    pub async fn execute(&self, token: String) -> Result<VerifyTokenResponse, AuthError> {
        let session = self
            .store
            .get(&token)
            .await
            .map_err(|_| AuthError::StorageFailure)?
            .ok_or(AuthError::InvalidToken)?;

        if session.expires_at <= self.clock.now_epoch_seconds() {
            // Best-effort cleanup of expired session.
            let _ = self.store.remove(&token).await;
            return Err(AuthError::SessionExpired);
        }

        Ok(map_session(session))
    }
}

fn map_session(session: Session) -> VerifyTokenResponse {
    VerifyTokenResponse {
        // Canonical identity used by downstream services.
        user_id: session.guest_id,
        display_name: session.display_name,
        metadata: session.metadata,
        session_id: session.session_id,
        expires_at: session.expires_at,
    }
}
