use crate::domain::errors::AuthError;
use crate::domain::ports::SessionStore;

// Response returned by the logout use case.
pub struct LogoutResponse {
    pub revoked: bool,
}

// Logout use case with injected dependencies.
pub struct LogoutUseCase<S> {
    pub store: S,
}

impl<S> LogoutUseCase<S>
where
    S: SessionStore,
{
    pub async fn execute(&self, token: String) -> Result<LogoutResponse, AuthError> {
        let revoked = self
            .store
            .remove(&token)
            .await
            .map_err(|_| AuthError::StorageFailure)?;

        Ok(LogoutResponse { revoked })
    }
}
