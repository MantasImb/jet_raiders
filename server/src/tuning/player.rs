/// Gameplay tuning for player-controlled ships.
///
/// Keep this separate from runtime/server configuration (tick rates, buffer sizes, etc.).

#[derive(Debug, Clone, Copy)]
pub struct PlayerTuning {
    /// Maximum forward speed in pixels per second.
    pub max_speed: f32,

    /// Rotation speed in radians per second.
    pub turn_rate: f32,

    /// How fast throttle changes per second.
    pub throttle_rate: f32,

    /// World-space collision radius in pixels (server-side hit checks).
    pub radius: f32,
}

impl Default for PlayerTuning {
    fn default() -> Self {
        Self {
            max_speed: 150.0,
            turn_rate: 3.0,
            throttle_rate: 2.0,
            radius: 24.0,
        }
    }
}
