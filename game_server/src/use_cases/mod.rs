// Use cases layer: application workflows for the game server.

pub mod game;
pub mod lobby;
pub mod types;

pub use types::{GameEvent, ServerState, WorldUpdate};
