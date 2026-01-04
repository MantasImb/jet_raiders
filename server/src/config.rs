use std::time::Duration;

// Runtime/server constants (not gameplay tuning).

pub const INPUT_CHANNEL_CAPACITY: usize = 1024;
pub const WORLD_BROADCAST_CAPACITY: usize = 128;

// Keep 1 tick/sec for now; adjust later.
pub const TICK_INTERVAL: Duration = Duration::from_millis(1000);
