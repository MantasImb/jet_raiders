use crate::use_cases::matchmaker::Matchmaker;
use std::sync::Arc;
use tokio::sync::Mutex;

// Shared application state for the HTTP handlers.
pub struct AppState {
    pub matchmaker: Arc<Mutex<Matchmaker>>,
}
