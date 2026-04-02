use crate::use_cases::matchmaker::Matchmaker;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

// Shared application state for the HTTP handlers.
pub struct AppState {
    pub allowed_regions: Arc<HashSet<String>>,
    pub matchmaker: Arc<Mutex<Matchmaker>>,
}
