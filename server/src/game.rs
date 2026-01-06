use crate::config;
use crate::protocol::{GameEvent, PlayerInput, WorldUpdate};
use crate::state::{EntityState, ServerState, SimEntity};
use crate::systems::movement;
use crate::tuning::player::PlayerTuning;
use crate::tuning::projectile::ProjectileTuning;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, watch};

pub async fn world_task(
    mut input_rx: mpsc::Receiver<GameEvent>,
    world_tx: broadcast::Sender<WorldUpdate>,
    server_state_tx: watch::Sender<ServerState>,
) {
    let mut tick: u64 = 0;
    let mut entities: Vec<SimEntity> = Vec::new();
    let mut projectiles: Vec<crate::state::SimProjectile> = Vec::new();
    let mut next_projectile_id: u64 = 1;

    let _ = server_state_tx.send(ServerState::MatchStarting { in_seconds: 3 });
    tokio::time::sleep(Duration::from_secs(3)).await;
    let _ = server_state_tx.send(ServerState::MatchRunning);

    let mut interval = tokio::time::interval(config::TICK_INTERVAL);

    // World bounds for wrapping.
    let (min_x, max_x) = (-400.0, 400.0);
    let (min_y, max_y) = (-230.0, 230.0);

    // Projectile tuning (keep in sync with client `projectile.tscn` timer for now).
    let projectile_tuning = ProjectileTuning::default();
    let projectile_speed: f32 = projectile_tuning.speed;
    let projectile_ttl: f32 = projectile_tuning.life_time;
    let projectile_cooldown: f32 = 0.1;

    loop {
        interval.tick().await;

        while let Ok(ev) = input_rx.try_recv() {
            match ev {
                GameEvent::Join { player_id } => {
                    println!("Logic: Player {} joined", player_id);
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_micros();
                    let x = ((now % 800) as f32) - 400.0;
                    let y = ((now % 460) as f32) - 230.0;
                    entities.push(SimEntity {
                        id: player_id,
                        x,
                        y,
                        rot: 0.0,
                        throttle: 0.0,
                        last_input: PlayerInput {
                            thrust: 0.0,
                            turn: 0.0,
                            shoot: false,
                        },
                        shoot_cooldown: 0.0,
                    });
                }
                GameEvent::Leave { player_id } => {
                    println!("Logic: Player {} left", player_id);
                    entities.retain(|e| e.id != player_id);
                    projectiles.retain(|p| p.owner_id != player_id);
                }
                GameEvent::Input { player_id, input } => {
                    if let Some(e) = entities.iter_mut().find(|e| e.id == player_id) {
                        e.last_input = input;
                    }
                }
            }
        }

        let dt = config::TICK_INTERVAL.as_secs_f32();
        let tuning = PlayerTuning::default();
        let cfg = movement::MovementConfig {
            max_speed: tuning.max_speed,
            turn_rate: tuning.turn_rate,
            throttle_rate: tuning.throttle_rate,
            min_x,
            max_x,
            min_y,
            max_y,
        };
        let player_radius = tuning.radius;

        for e in &mut entities {
            // Ship movement.
            movement::tick_entity(e, dt, cfg);

            // Shooting.
            e.shoot_cooldown = (e.shoot_cooldown - dt).max(0.0);
            if e.last_input.shoot && e.shoot_cooldown <= 0.0 {
                // Forward vector (same convention as movement).
                let dir_x = e.rot.sin();
                let dir_y = -e.rot.cos();

                projectiles.push(crate::state::SimProjectile {
                    id: next_projectile_id,
                    owner_id: e.id,
                    // Spawn at the edge of the ship's radius, in the direction it's facing.
                    x: e.x + dir_x * player_radius,
                    y: e.y + dir_y * player_radius,
                    rot: e.rot,
                    vx: dir_x * projectile_speed,
                    vy: dir_y * projectile_speed,
                    ttl: projectile_ttl,
                });
                next_projectile_id = next_projectile_id.wrapping_add(1);
                e.shoot_cooldown = projectile_cooldown;
            }
        }

        for p in &mut projectiles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.ttl -= dt;
        }
        projectiles.retain(|p| p.ttl > 0.0);

        tick += 1;
        let entities_snapshot: Vec<EntityState> = entities.iter().map(EntityState::from).collect();
        let projectiles_snapshot: Vec<crate::state::ProjectileState> =
            projectiles.iter().map(crate::state::ProjectileState::from).collect();

        let _ = world_tx.send(WorldUpdate {
            tick,
            entities: entities_snapshot,
            projectiles: projectiles_snapshot,
        });
    }
}
