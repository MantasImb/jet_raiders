use async_trait::async_trait;
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
}

// In-memory session store adapter for the auth service.
#[derive(Clone)]
pub struct InMemorySessionStore {
    pub sessions: Arc<Mutex<HashMap<String, Session>>>,
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
