use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::entities::Session;
use crate::domain::ports::{Clock, SessionStore};

pub(crate) type SessionTable = Arc<Mutex<HashMap<String, Session>>>;

// Shared fixed time source for deterministic use-case tests.
pub(crate) struct FixedClock(pub(crate) u64);

impl Clock for FixedClock {
    fn now_epoch_seconds(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FailureFlags {
    pub insert: bool,
    pub get: bool,
    pub remove: bool,
}

#[derive(Clone)]
pub(crate) struct RecordingStore {
    sessions: SessionTable,
    failures: FailureFlags,
}

impl RecordingStore {
    pub(crate) fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            failures: FailureFlags::default(),
        }
    }

    pub(crate) fn with_failures(mut self, failures: FailureFlags) -> Self {
        self.failures = failures;
        self
    }

    pub(crate) fn insert_test_session(&self, token: impl Into<String>, session: Session) {
        let mut guard = self.sessions.lock().expect("sessions mutex poisoned");
        guard.insert(token.into(), session);
    }

    pub(crate) fn insert_test_token(&self, token: impl Into<String>) {
        let session = Session {
            guest_id: 0,
            display_name: "Test".to_string(),
            metadata: None,
            session_id: "test-session".to_string(),
            expires_at: 0,
        };
        self.insert_test_session(token, session);
    }

    pub(crate) fn get_test_session(&self, token: &str) -> Option<Session> {
        let guard = self.sessions.lock().expect("sessions mutex poisoned");
        guard.get(token).cloned()
    }
}

#[async_trait]
impl SessionStore for RecordingStore {
    async fn insert(&self, token: String, session: Session) -> Result<(), String> {
        if self.failures.insert {
            return Err("insert failed".to_string());
        }

        let mut guard = self.sessions.lock().expect("sessions mutex poisoned");
        guard.insert(token, session);
        Ok(())
    }

    async fn get(&self, token: &str) -> Result<Option<Session>, String> {
        if self.failures.get {
            return Err("get failed".to_string());
        }

        let guard = self.sessions.lock().expect("sessions mutex poisoned");
        Ok(guard.get(token).cloned())
    }

    async fn remove(&self, token: &str) -> Result<bool, String> {
        if self.failures.remove {
            return Err("remove failed".to_string());
        }

        let mut guard = self.sessions.lock().expect("sessions mutex poisoned");
        Ok(guard.remove(token).is_some())
    }
}
