use crate::use_cases::{GuestSessionService, MatchmakingService};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    // The HTTP layer depends on the use-case service instead of orchestrating flows itself.
    pub guest_sessions: Arc<GuestSessionService>,
    // Matchmaking stays behind a use-case service so handlers do not call upstream clients directly.
    pub matchmaking: Arc<MatchmakingService>,
}
