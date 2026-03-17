use crate::use_cases::GuestSessionService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    // The HTTP layer depends on the use-case service instead of orchestrating flows itself.
    pub guest_sessions: Arc<GuestSessionService>,
}
