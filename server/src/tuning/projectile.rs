/// Gameplay tuning for projectiles.

#[derive(Debug, Clone, Copy)]
pub struct ProjectileTuning {
    /// Initial projectile speed in pixels per second.
    pub speed: f32,

    /// Lifetime in seconds before the projectile is despawned.
    pub life_time: f32,

    /// World-space collision radius in pixels.
    pub radius: f32,
}

impl Default for ProjectileTuning {
    fn default() -> Self {
        Self {
            speed: 400.0,
            life_time: 1.25,
            radius: 5.0,
        }
    }
}
