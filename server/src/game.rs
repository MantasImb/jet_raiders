use crate::config;
use crate::protocol::{GameEvent, PlayerInput, WorldUpdate};
use crate::state::{EntityState, ServerState, SimEntity};
use crate::systems::movement;
use crate::tuning::player::PlayerTuning;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, watch};

pub async fn world_task(
    mut input_rx: mpsc::Receiver<GameEvent>,
    world_tx: broadcast::Sender<WorldUpdate>,
    server_state_tx: watch::Sender<ServerState>,
) {
    let mut tick: u64 = 0;
    let mut entities: Vec<SimEntity> = Vec::new();

    let _ = server_state_tx.send(ServerState::MatchStarting { in_seconds: 3 });
    tokio::time::sleep(Duration::from_secs(3)).await;
    let _ = server_state_tx.send(ServerState::MatchRunning);

    // Tick rate (leave at 1 tick/sec for now).
    let mut interval = tokio::time::interval(config::TICK_INTERVAL);

    // World bounds for wrapping.
    let (min_x, max_x) = (-400.0, 400.0);
    let (min_y, max_y) = (-230.0, 230.0);

    loop {
        // Wait for next tick
        interval.tick().await;

        // Process all pending inputs/events
        while let Ok(ev) = input_rx.try_recv() {
            match ev {
                GameEvent::Join { player_id } => {
                    println!("Logic: Player {} joined", player_id);
                    // Spawn player at random position
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
                    });
                }
                GameEvent::Leave { player_id } => {
                    println!("Logic: Player {} left", player_id);
                    entities.retain(|e| e.id != player_id);
                }
                GameEvent::Input { player_id, input } => {
                    if let Some(e) = entities.iter_mut().find(|e| e.id == player_id) {
                        // Store intent; movement is applied by the tick system.
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

        for e in &mut entities {
            movement::tick_entity(e, dt, cfg);
        }

        tick += 1;
        let snapshot: Vec<EntityState> = entities.iter().map(EntityState::from).collect();
        let _ = world_tx.send(WorldUpdate {
            tick,
            entities: snapshot,
        });
    }
}
