// Domain layer: core simulation types and rules.

pub mod state;
pub mod systems;
pub mod tuning;

pub use state::{EntitySnapshot, PlayerInput, ProjectileSnapshot, SimEntity, SimProjectile};
