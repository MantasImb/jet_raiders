use std::time::Duration;

// Runtime/server constants (not gameplay tuning).

pub const INPUT_CHANNEL_CAPACITY: usize = 1024;
pub const WORLD_BROADCAST_CAPACITY: usize = 128;

pub const TICK_INTERVAL: Duration = Duration::from_millis(1000 / 60);
