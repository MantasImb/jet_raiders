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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::test_support::{FailureFlags, FixedClock, RecordingStore};
    use serde_json::json;

    #[tokio::test]
    async fn when_token_exists_and_not_expired_then_returns_session_identity() {
        let token = "session-token".to_string();
        let session = Session {
            guest_id: 9,
            display_name: "Pilot".to_string(),
            metadata: None,
            session_id: "session-1".to_string(),
            expires_at: 1_700_000_100,
        };
        let store = RecordingStore::new();
        store.insert_test_session(token.clone(), session);

        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store,
        };

        let result = use_case
            .execute(token)
            .await
            .expect("expected token verification to succeed");

        assert_eq!(result.user_id, 9);
        assert_eq!(result.display_name, "Pilot");
        assert_eq!(result.session_id, "session-1");
        assert_eq!(result.expires_at, 1_700_000_100);
    }

    #[tokio::test]
    async fn when_token_does_not_exist_then_returns_invalid_token() {
        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
        };

        let result = use_case.execute("missing".to_string()).await;

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn when_session_is_expired_then_returns_session_expired_and_cleans_up_token() {
        let token = "expired-token".to_string();
        let session = Session {
            guest_id: 9,
            display_name: "Pilot".to_string(),
            metadata: None,
            session_id: "session-1".to_string(),
            expires_at: 1_700_000_000,
        };
        let store = RecordingStore::new();
        store.insert_test_session(token.clone(), session);
        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store,
        };

        let result = use_case.execute(token.clone()).await;

        assert!(matches!(result, Err(AuthError::SessionExpired)));
    }

    #[tokio::test]
    async fn when_store_get_fails_then_returns_storage_failure() {
        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new().with_failures(FailureFlags {
                get: true,
                ..Default::default()
            }),
        };

        let result = use_case.execute("any-token".to_string()).await;

        assert!(matches!(result, Err(AuthError::StorageFailure)));
    }

    #[tokio::test]
    async fn when_token_has_surrounding_whitespace_then_returns_invalid_token() {
        let stored_token = "session-token".to_string();
        let session = Session {
            guest_id: 9,
            display_name: "Pilot".to_string(),
            metadata: None,
            session_id: "session-1".to_string(),
            expires_at: 1_700_000_100,
        };

        let store = RecordingStore::new();
        store.insert_test_session(stored_token, session);

        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store,
        };

        let result = use_case.execute("  session-token  ".to_string()).await;

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn when_token_is_empty_then_returns_invalid_token() {
        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
        };

        let result = use_case.execute(String::new()).await;

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn when_session_expiry_equals_now_then_returns_session_expired() {
        let token = "edge-expired-token".to_string();
        let session = Session {
            guest_id: 9,
            display_name: "Pilot".to_string(),
            metadata: None,
            session_id: "session-1".to_string(),
            expires_at: 1_700_000_000,
        };
        let store = RecordingStore::new();
        store.insert_test_session(token.clone(), session);

        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store,
        };

        let result = use_case.execute(token).await;

        assert!(matches!(result, Err(AuthError::SessionExpired)));
    }

    #[tokio::test]
    async fn when_session_contains_metadata_then_verify_response_keeps_it() {
        let token = "session-token-with-metadata".to_string();
        let metadata = json!({
            "device": "ios",
            "build": "1.2.3"
        });
        let session = Session {
            guest_id: 9,
            display_name: "Pilot".to_string(),
            metadata: Some(metadata.clone()),
            session_id: "session-1".to_string(),
            expires_at: 1_700_000_100,
        };
        let store = RecordingStore::new();
        store.insert_test_session(token.clone(), session);

        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store,
        };

        let result = use_case
            .execute(token)
            .await
            .expect("expected token verification to succeed");

        assert_eq!(result.metadata, Some(metadata));
    }

    #[tokio::test]
    async fn when_token_is_random_garbage_then_returns_invalid_token() {
        let use_case = VerifyTokenUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
        };

        let result = use_case.execute("%%%not-a-token%%%".to_string()).await;

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }
}
