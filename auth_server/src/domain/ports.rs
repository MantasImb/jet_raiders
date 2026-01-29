use async_trait::async_trait;

use crate::domain::entities::Session;

// Port for session storage used by auth use cases.
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn insert(&self, token: String, session: Session) -> Result<(), String>;
    async fn get(&self, token: &str) -> Result<Option<Session>, String>;
    async fn remove(&self, token: &str) -> Result<bool, String>;
}

// Port for retrieving the current time.
pub trait Clock: Send + Sync {
    fn now_epoch_seconds(&self) -> u64;
}
