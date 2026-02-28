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
    use crate::domain::entities::Session;
    use crate::domain::ports::Clock;
    use crate::interface_adapters::protocol::GuestLoginRequest;
    use crate::use_cases::guest_login::GuestLoginUseCase;
    use crate::use_cases::verify_token::VerifyTokenUseCase;
    use async_trait::async_trait;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct RecordingStore {
        // Active token set used as a minimal fake store for remove().
        tokens: Arc<Mutex<HashSet<String>>>,
        // Toggle used to simulate persistence failure in remove().
        should_fail_remove: bool,
    }

    struct FixedClock {
        now: u64,
    }

    impl Clock for FixedClock {
        fn now_epoch_seconds(&self) -> u64 {
            self.now
        }
    }

    #[async_trait]
    impl SessionStore for RecordingStore {
        async fn insert(&self, token: String, _session: Session) -> Result<(), String> {
            let mut guard = self.tokens.lock().expect("tokens mutex poisoned");
            guard.insert(token);
            Ok(())
        }

        async fn get(&self, token: &str) -> Result<Option<Session>, String> {
            // This use case never calls get(); this stub exists only because
            // SessionStore requires all trait methods to be implemented.
            // We return a tiny hardcoded session for completeness.
            let guard = self.tokens.lock().expect("tokens mutex poisoned");
            if guard.contains(token) {
                return Ok(Some(Session {
                    guest_id: 1,
                    display_name: "Pilot".to_string(),
                    metadata: None,
                    session_id: "session".to_string(),
                    expires_at: 0,
                }));
            }
            Ok(None)
        }

        async fn remove(&self, token: &str) -> Result<bool, String> {
            if self.should_fail_remove {
                return Err("remove failed".to_string());
            }
            let mut guard = self.tokens.lock().expect("tokens mutex poisoned");
            Ok(guard.remove(token))
        }
    }

    #[tokio::test]
    async fn when_token_exists_then_logout_returns_revoked_true() {
        let tokens = Arc::new(Mutex::new(HashSet::from([String::from("token-1")])));
        let use_case = LogoutUseCase {
            store: RecordingStore {
                tokens,
                should_fail_remove: false,
            },
        };

        let result = use_case
            .execute("token-1".to_string())
            .await
            .expect("expected logout to succeed");

        assert!(result.revoked);
    }

    #[tokio::test]
    async fn when_token_does_not_exist_then_logout_returns_revoked_false() {
        let use_case = LogoutUseCase {
            store: RecordingStore {
                tokens: Arc::new(Mutex::new(HashSet::new())),
                should_fail_remove: false,
            },
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
            store: RecordingStore {
                tokens: Arc::new(Mutex::new(HashSet::new())),
                should_fail_remove: true,
            },
        };

        let result = use_case.execute("token-1".to_string()).await;

        assert!(matches!(result, Err(AuthError::StorageFailure)));
    }

    #[tokio::test]
    async fn when_token_is_empty_then_logout_returns_revoked_false() {
        let use_case = LogoutUseCase {
            store: RecordingStore {
                tokens: Arc::new(Mutex::new(HashSet::new())),
                should_fail_remove: false,
            },
        };

        let result = use_case
            .execute(String::new())
            .await
            .expect("expected logout to succeed for empty token");

        assert!(!result.revoked);
    }

    #[tokio::test]
    async fn when_token_has_whitespace_then_logout_does_not_trim_and_returns_false() {
        let use_case = LogoutUseCase {
            store: RecordingStore {
                tokens: Arc::new(Mutex::new(HashSet::from([String::from("token-1")]))),
                should_fail_remove: false,
            },
        };

        let result = use_case
            .execute(" token-1 ".to_string())
            .await
            .expect("expected logout to succeed");

        assert!(!result.revoked);
    }

    #[tokio::test]
    async fn when_token_is_logged_out_then_verify_token_returns_invalid_token() {
        let shared_store = RecordingStore {
            tokens: Arc::new(Mutex::new(HashSet::new())),
            should_fail_remove: false,
        };
        let login_use_case = GuestLoginUseCase {
            clock: FixedClock { now: 1_700_000_000 },
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
            clock: FixedClock { now: 1_700_000_001 },
            store: shared_store,
        };
        let verify_result = verify_use_case.execute(login_result.token).await;

        assert!(matches!(verify_result, Err(AuthError::InvalidToken)));
    }
}
