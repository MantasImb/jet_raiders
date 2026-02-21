use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Returns a process-unique, monotonically increasing identifier.
///
/// This avoids collisions that can happen with "timestamp only" IDs when multiple IDs are
/// generated in the same instant.
pub fn rand_id() -> u64 {
    static COUNTER: OnceLock<AtomicU64> = OnceLock::new();
    let counter = COUNTER.get_or_init(|| AtomicU64::new(now_nanos()));
    counter.fetch_add(1, Ordering::Relaxed)
}
