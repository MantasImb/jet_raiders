pub mod guest;
pub mod matchmaking;

pub use guest::{
    AuthProvider, AuthProviderError, GuestInit, GuestInitResult, GuestLogin, GuestLoginResult,
    GuestSessionService, VerifySession, VerifySessionResult,
};
pub use matchmaking::{
    EnterMatchmaking, EnterMatchmakingError, MatchmakingEnqueueResult, MatchmakingProvider,
    MatchmakingProviderError, MatchmakingQueueRequest, MatchmakingService, MatchmakingTicketStatus,
    PollMatchmaking, PollMatchmakingError,
};
