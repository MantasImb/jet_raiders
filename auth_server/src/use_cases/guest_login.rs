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

    let len = value.chars().count();

    if !(MIN_LEN..=MAX_LEN).contains(&len) {
        return Err(AuthError::InvalidDisplayName);
    }
    if value.trim() != value {
        return Err(AuthError::InvalidDisplayName);
    }

    // Allow a simple safe charset across the stack.
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
    {
        return Err(AuthError::InvalidDisplayName);
    }

    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::test_support::{FailureFlags, FixedClock, RecordingStore};
    use serde_json::json;

    #[tokio::test]
    async fn when_payload_is_valid_then_session_is_stored_and_response_is_returned() {
        let store = RecordingStore::new();
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: store.clone(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot_42".to_string(),
                metadata: None,
            })
            .await
            .expect("expected guest login to succeed");

        assert_eq!(result.display_name, "Pilot_42");
        assert_eq!(result.expires_at, 1_700_003_600);

        // Verify that the generated token points to the stored canonical session.
        let saved = store
            .get_test_session(&result.token)
            .expect("expected session to be stored");
        assert_eq!(saved.guest_id, 42);
        assert_eq!(saved.display_name, "Pilot_42");
        assert_eq!(saved.expires_at, 1_700_003_600);
    }

    #[tokio::test]
    async fn when_guest_id_is_zero_then_returns_invalid_guest_id() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 0,
                display_name: "Pilot".to_string(),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(AuthError::InvalidGuestId)));
    }

    #[tokio::test]
    async fn when_display_name_contains_invalid_characters_then_returns_invalid_display_name() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot!".to_string(),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(AuthError::InvalidDisplayName)));
    }

    #[tokio::test]
    async fn when_store_insert_fails_then_returns_storage_failure() {
        // This test injects a store failure and checks the use case maps it to
        // the domain-level error contract instead of leaking raw store errors.
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new().with_failures(FailureFlags {
                insert: true,
                ..Default::default()
            }),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot".to_string(),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(AuthError::StorageFailure)));
    }

    #[tokio::test]
    async fn when_display_name_length_is_two_then_returns_invalid_display_name() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "AB".to_string(),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(AuthError::InvalidDisplayName)));
    }

    #[tokio::test]
    async fn when_display_name_length_is_three_then_guest_login_succeeds() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "ABC".to_string(),
                metadata: None,
            })
            .await
            .expect("expected 3-character display name to be valid");

        assert_eq!(result.display_name, "ABC");
    }

    #[tokio::test]
    async fn when_display_name_length_is_thirty_two_then_guest_login_succeeds() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "A".repeat(32),
                metadata: None,
            })
            .await
            .expect("expected 32-character display name to be valid");

        assert_eq!(result.display_name.chars().count(), 32);
    }

    #[tokio::test]
    async fn when_display_name_length_is_thirty_three_then_returns_invalid_display_name() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "A".repeat(33),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(AuthError::InvalidDisplayName)));
    }

    #[tokio::test]
    async fn when_display_name_uses_allowed_symbols_then_guest_login_succeeds() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Ace Pilot-1_2".to_string(),
                metadata: None,
            })
            .await
            .expect("expected allowed symbol set to be valid");

        assert_eq!(result.display_name, "Ace Pilot-1_2");
    }

    #[tokio::test]
    async fn when_display_name_contains_internal_space_then_guest_login_succeeds() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Blue Falcon".to_string(),
                metadata: None,
            })
            .await
            .expect("expected internal spaces to be valid");

        assert_eq!(result.display_name, "Blue Falcon");
    }

    #[tokio::test]
    async fn when_display_name_has_trailing_whitespace_then_returns_invalid_display_name() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Blue Falcon ".to_string(),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(AuthError::InvalidDisplayName)));
    }

    #[tokio::test]
    async fn when_display_name_has_leading_whitespace_then_returns_invalid_display_name() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: " Blue Falcon".to_string(),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(AuthError::InvalidDisplayName)));
    }

    #[tokio::test]
    async fn when_metadata_is_present_then_it_is_saved_in_session() {
        let store = RecordingStore::new();
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: store.clone(),
            ttl_seconds: 3600,
        };
        let metadata = json!({
            "ship": "falcon",
            "rank": 7
        });

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot".to_string(),
                metadata: Some(metadata.clone()),
            })
            .await
            .expect("expected guest login to succeed with metadata");

        let saved = store
            .get_test_session(&result.token)
            .expect("expected session to be stored");
        assert_eq!(saved.metadata, Some(metadata));
    }

    #[tokio::test]
    async fn when_metadata_is_none_then_session_metadata_stays_none() {
        let store = RecordingStore::new();
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: store.clone(),
            ttl_seconds: 3600,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot".to_string(),
                metadata: None,
            })
            .await
            .expect("expected guest login to succeed without metadata");

        let saved = store
            .get_test_session(&result.token)
            .expect("expected session to be stored");
        assert_eq!(saved.metadata, None);
    }

    #[tokio::test]
    async fn when_metadata_is_nested_json_then_it_is_preserved_exactly() {
        let store = RecordingStore::new();
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: store.clone(),
            ttl_seconds: 3600,
        };
        let metadata = json!({
            "device": {
                "platform": "ios",
                "version": "1.2.3"
            },
            "flags": {
                "tutorial_done": true
            }
        });

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot".to_string(),
                metadata: Some(metadata.clone()),
            })
            .await
            .expect("expected guest login to succeed with nested metadata");

        let saved = store
            .get_test_session(&result.token)
            .expect("expected session to be stored");
        assert_eq!(saved.metadata, Some(metadata));
    }

    #[tokio::test]
    async fn when_ttl_is_zero_then_expires_at_matches_current_time() {
        let use_case = GuestLoginUseCase {
            clock: FixedClock(1_700_000_000),
            store: RecordingStore::new(),
            ttl_seconds: 0,
        };

        let result = use_case
            .execute(GuestLoginRequest {
                guest_id: 42,
                display_name: "Pilot".to_string(),
                metadata: None,
            })
            .await
            .expect("expected guest login to succeed with zero ttl");

        assert_eq!(result.expires_at, 1_700_000_000);
    }
}
