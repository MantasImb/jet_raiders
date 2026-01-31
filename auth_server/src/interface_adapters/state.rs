use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use crate::domain::entities::Session;
use crate::domain::ports::{Clock, SessionStore};

// Application state holding session storage.
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<String, Session>>>,
    // Shared database pool for guest profile persistence.
    pub db: PgPool,
}

// In-memory session store adapter for the auth service.
#[derive(Clone)]
pub struct InMemorySessionStore {
    pub sessions: Arc<Mutex<HashMap<String, Session>>>,
}

// PostgreSQL-backed guest profile store for persistence.
#[derive(Clone)]
pub struct PostgresGuestProfileStore {
    pub db: PgPool,
}

impl PostgresGuestProfileStore {
    // Upsert a guest profile record using the latest identity data.
    pub async fn upsert_guest_profile(
        &self,
        guest_id: &str,
        display_name: &str,
        metadata_json: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO guest_profiles (guest_id, display_name, metadata)
            VALUES ($1, $2, $3)
            ON CONFLICT (guest_id) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(guest_id)
        .bind(display_name)
        .bind(metadata_json)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn insert(&self, token: String, session: Session) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(token, session);
        Ok(())
    }

    async fn get(&self, token: &str) -> Result<Option<Session>, String> {
        let sessions = self.sessions.lock().await;
        Ok(sessions.get(token).cloned())
    }

    async fn remove(&self, token: &str) -> Result<bool, String> {
        let mut sessions = self.sessions.lock().await;
        Ok(sessions.remove(token).is_some())
    }
}

// System clock adapter used by auth use cases.
#[derive(Clone)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_epoch_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
