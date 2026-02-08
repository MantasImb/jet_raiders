use std::{env, time::Duration};

// Runtime/server constants (not gameplay tuning).

pub fn http_port() -> u16 {
    env::var("GAME_SERVER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3001)
}
pub const INPUT_CHANNEL_CAPACITY: usize = 1024;
pub const WORLD_BROADCAST_CAPACITY: usize = 128;

pub const TICK_INTERVAL: Duration = Duration::from_millis(1000 / 60);
// Default time limit for non-test lobbies (0 disables match end).
pub const DEFAULT_MATCH_TIME_LIMIT: Duration = Duration::from_secs(600);
