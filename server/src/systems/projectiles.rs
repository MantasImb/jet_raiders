use crate::state::{SimEntity, SimProjectile};
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub struct ProjectileConfig {
    pub speed: f32,
    pub ttl: f32,
    pub radius: f32,
    pub damage: i32,
    pub cooldown: f32,
    pub player_radius: f32,
    pub respawn_delay: f32,
}

pub fn tick_projectiles(
    entities: &mut [SimEntity],
    projectiles: &mut Vec<SimProjectile>,
    next_projectile_id: &mut u64,
    dt: f32,
    cfg: ProjectileConfig,
) {
    // Spawn new projectiles from player input and cooldowns.
    for e in entities.iter_mut() {
        if !e.alive {
            continue;
        }

        e.shoot_cooldown = (e.shoot_cooldown - dt).max(0.0);
        if e.last_input.shoot && e.shoot_cooldown <= 0.0 {
            // Forward vector (same convention as ship movement).
            let dir_x = e.rot.sin();
            let dir_y = -e.rot.cos();

            projectiles.push(SimProjectile {
                id: *next_projectile_id,
                owner_id: e.id,
                // Spawn at the edge of the ship's radius, in the direction it's facing.
                x: e.x + dir_x * cfg.player_radius,
                y: e.y + dir_y * cfg.player_radius,
                rot: e.rot,
                vx: dir_x * cfg.speed,
                vy: dir_y * cfg.speed,
                ttl: cfg.ttl,
            });
            *next_projectile_id = next_projectile_id.wrapping_add(1);
            e.shoot_cooldown = cfg.cooldown;
        }
    }

    // Integrate projectile movement and lifetimes.
    for p in projectiles.iter_mut() {
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.ttl -= dt;
    }

    // Projectile vs player collision (naive O(P*E) for now).
    // We despawn the projectile on first hit and just log the result.
    let hit_radius = cfg.player_radius + cfg.radius;
    let hit_radius_sq = hit_radius * hit_radius;
    for p in projectiles.iter_mut() {
        // Mark for despawn.
        if p.ttl <= 0.0 {
            continue;
        }

        for e in entities.iter_mut() {
            if !e.alive {
                continue;
            }
            if e.id == p.owner_id {
                continue;
            }

            let dx = e.x - p.x;
            let dy = e.y - p.y;
            if (dx * dx + dy * dy) <= hit_radius_sq {
                e.hp -= cfg.damage;
                if e.hp <= 0 {
                    e.hp = 0;
                    e.alive = false;
                    e.respawn_timer = cfg.respawn_delay;
                    e.throttle = 0.0;
                    e.shoot_cooldown = 0.0;
                }

                info!(
                    victim_id = e.id,
                    shooter_id = p.owner_id,
                    projectile_id = p.id,
                    victim_hp = e.hp,
                    "player hit"
                );
                p.ttl = 0.0;
                break;
            }
        }
    }

    projectiles.retain(|p| p.ttl > 0.0);
}
