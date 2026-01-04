use std::time::{SystemTime, UNIX_EPOCH};

// TODO: Make this proper
pub fn rand_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
