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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface_adapters::protocol::GuestLoginRequest;
    use crate::use_cases::guest_login::GuestLoginUseCase;
    use crate::use_cases::test_support::{FailureFlags, FixedClock, RecordingStore};
    use crate::use_cases::verify_token::VerifyTokenUseCase;

    #[tokio::test]
    async fn when_token_exists_then_logout_returns_revoked_true() {
        let store = RecordingStore::new();
        store.insert_test_token("token-1");
        let use_case = LogoutUseCase { store };

        let result = use_case
            .execute("token-1".to_string())
            .await
            .expect("expected logout to succeed");

        assert!(result.revoked);
    }

    #[tokio::test]
    async fn when_token_does_not_exist_then_logout_returns_revoked_false() {
        let use_case = LogoutUseCase {
            store: RecordingStore::new(),
        };

        let result = use_case
            .execute("missing-token".to_string())
            .await
            .expect("expected logout to succeed");

        assert!(!result.revoked);
    }

    #[tokio::test]
    async fn when_store_remove_fails_then_returns_storage_failure() {
        let use_case = LogoutUseCase {
            store: RecordingStore::new().with_failures(FailureFlags {
                remove: true,
                ..Default::default()
            }),
        };

        let result = use_case.execute("token-1".to_string()).await;

        assert!(matches!(result, Err(AuthError::StorageFailure)));
    }

    #[tokio::test]
    async fn when_token_is_empty_then_logout_returns_revoked_false() {
        let use_case = LogoutUseCase {
            store: RecordingStore::new(),
        };

        let result = use_case
            .execute(String::new())
            .await
            .expect("expected logout to succeed for empty token");

        assert!(!result.revoked);
    }

    #[tokio::test]
    async fn when_token_has_whitespace_then_logout_does_not_trim_and_returns_false() {
        let store = RecordingStore::new();
        store.insert_test_token("token-1");
        let use_case = LogoutUseCase { store };

        let result = use_case
            .execute(" token-1 ".to_string())
            .await
            .expect("expected logout to succeed");

        assert!(!result.revoked);
    }

    #[tokio::test]
    async fn when_token_is_logged_out_then_verify_token_returns_invalid_token() {
        let shared_store = RecordingStore::new();
        let login_use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: shared_store.clone(),
            ttl_seconds: 3600,
        };

        let login_result = login_use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot".to_string(),
                metadata: None,
            })
            .await
            .expect("expected login to succeed");

        let logout_use_case = LogoutUseCase {
            store: shared_store.clone(),
        };
        let logout_result = logout_use_case
            .execute(login_result.token.clone())
            .await
            .expect("expected logout to succeed");
        assert!(logout_result.revoked);

        let verify_use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_001),
            store: shared_store,
        };
        let verify_result = verify_use_case.execute(login_result.token).await;

        assert!(matches!(verify_result, Err(AuthError::InvalidToken)));
    }
}
