#[derive(Debug, Clone, Copy)]
pub struct MovementConfig {
    pub max_speed: f32,     // px/s
    pub turn_rate: f32,     // rad/s
    pub throttle_rate: f32, // throttle units per second

    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

pub fn tick_entity(e: &mut crate::state::SimEntity, dt: f32, cfg: MovementConfig) {
    // rotation
    e.rot += e.last_input.turn * cfg.turn_rate * dt;

    // throttle
    e.throttle += e.last_input.thrust * cfg.throttle_rate * dt;
    e.throttle = e.throttle.clamp(0.0, 1.0);

    // direction (0 rad = up / -Y)
    // Positive rotation should turn the nose right (clockwise in Godot's +Y-down coordinates).
    let dir_x = e.rot.sin();
    let dir_y = -e.rot.cos();

    // velocity = forward * throttle * max_speed
    let vel_x = dir_x * e.throttle * cfg.max_speed;
    let vel_y = dir_y * e.throttle * cfg.max_speed;

    // position integrate
    e.x += vel_x * dt;
    e.y += vel_y * dt;

    // world wrap
    wrap_entity(e, cfg);
}

fn wrap_entity(e: &mut crate::state::SimEntity, cfg: MovementConfig) {
    if e.x < cfg.min_x {
        e.x = cfg.max_x;
    } else if e.x > cfg.max_x {
        e.x = cfg.min_x;
    }

    if e.y < cfg.min_y {
        e.y = cfg.max_y;
    } else if e.y > cfg.max_y {
        e.y = cfg.min_y;
    }
}
