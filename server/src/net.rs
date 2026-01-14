use crate::app_state::AppState;
use crate::protocol::{GameEvent, PlayerInput, ServerMessage, WorldUpdate};
use crate::state::ServerState;
use crate::utils::rng::rand_id;

use axum::{
    Error,
    extract::{
        State,
        ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade, close_code},
    },
    response::IntoResponse,
};
use futures::SinkExt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::watch::Receiver;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{debug, error, info, info_span, warn};

#[derive(Debug)]
enum NetError {
    // Categorizes connection lifecycle failures so callers can decide policy.
    #[allow(dead_code)]
    Ws(axum::Error),
    #[allow(dead_code)]
    Serialization(serde_json::Error),
    InputClosed,
    WorldUpdatesClosed,
    ServerStateClosed,
}

pub async fn world_update_serializer(
    mut world_rx: broadcast::Receiver<WorldUpdate>,
    world_bytes_tx: broadcast::Sender<Utf8Bytes>,
) {
    // Serialize each world update once and broadcast the shared bytes.
    loop {
        match world_rx.recv().await {
            Ok(update) => {
                let msg = ServerMessage::WorldUpdate(update);
                let txt = match serde_json::to_string(&msg) {
                    Ok(txt) => txt,
                    Err(e) => {
                        error!(error = ?e, "failed to serialize world update");
                        continue;
                    }
                };

                // Convert once and broadcast shared UTF-8 bytes to all clients.
                let bytes = Utf8Bytes::from(txt);
                let _ = world_bytes_tx.send(bytes);
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!(
                    missed = n,
                    "world serializer lagged; skipping to latest update"
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                warn!("world updates channel closed; serializer exiting");
                break;
            }
        }
    }
}

impl From<axum::Error> for NetError {
    fn from(e: axum::Error) -> Self {
        NetError::Ws(e)
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // Separate connection id for correlating logs before/after a player_id exists.
    let conn_id = rand_id();
    let span = info_span!("conn", conn_id, player_id = tracing::field::Empty);
    let _enter = span.enter();

    let mut ctx = match bootstrap_connection(&mut socket, &state).await {
        Ok(ctx) => ctx,
        Err(e) => {
            error!(error = ?e, "failed to bootstrap connection");
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: close_code::POLICY,
                    reason: "bootstrap failed".into(),
                })))
                .await;
            let _ = socket.close().await;
            return;
        }
    };

    span.record("player_id", ctx.player_id);
    info!("client connected");

    // Main Client Loop
    if let Err(e) = run_client_loop(&mut socket, &mut ctx).await {
        warn!(error = ?e, "client loop exited with error");
    }
}

async fn send_message(socket: &mut WebSocket, msg: &ServerMessage) -> Result<usize, NetError> {
    // Serialize message safely; log JSON errors instead of panicking
    // TODO: Consider reducing per-message allocations (e.g. reuse buffers) if this becomes hot.
    let txt = serde_json::to_string(msg).map_err(NetError::Serialization)?;
    let bytes = txt.len();
    socket
        .send(Message::Text(txt.into()))
        .await
        .map_err(NetError::Ws)?;
    Ok(bytes)
}

struct ConnCtx {
    pub player_id: u64,
    pub input_tx: mpsc::Sender<GameEvent>,
    pub world_bytes_rx: broadcast::Receiver<Utf8Bytes>,
    pub server_state_rx: watch::Receiver<ServerState>,

    pub msgs_in: u64,
    pub msgs_out: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,

    pub invalid_json: u32,

    pub last_input_full_log: Instant,
    pub last_world_lag_log: Instant,
    pub last_invalid_input_log: Instant,

    pub close_frame: Option<CloseFrame>,
}

async fn bootstrap_connection(
    socket: &mut WebSocket,
    state: &AppState,
) -> Result<ConnCtx, NetError> {
    // Subscribe to updates *before* doing anything else (awaits) to not miss packets.
    let world_bytes_rx = state.world_bytes_tx.subscribe();
    let server_state_rx = state.server_state_tx.subscribe();

    // Handshake & ID Assignment
    // `rand_id()` is process-unique and monotonic, so IDs won't collide within a running server.
    // TODO: If/when auth exists, bind player identity to auth/session instead of random IDs.
    let player_id = rand_id();

    // Send Identity Packet
    // Tell the client "This is who you are".
    let identity_msg = ServerMessage::Identity { player_id };
    let _ = send_message(socket, &identity_msg).await?;

    // Notify World Task
    // Tell the game loop to spawn a ship for this ID.
    // Join happens before initial state so the snapshot can include the newly spawned player.
    // If anything after Join fails, compensate with Leave to avoid "spawned but never connected".
    state
        .input_tx
        .send(GameEvent::Join { player_id })
        .await
        .map_err(|_| NetError::InputClosed)?;

    // Send Initial State
    // Keep in mind that we clone as soon as we borrow to avoid holding the lock. (especially
    // during an await)
    let initial_state = server_state_rx.borrow().clone();
    let state_msg = ServerMessage::GameState(initial_state);
    if let Err(e) = send_message(socket, &state_msg).await {
        state
            .input_tx
            .send(GameEvent::Leave { player_id })
            .await
            .map_err(|_| NetError::InputClosed)?; // InputClosed takes precedence
        return Err(e);
    }

    let now = Instant::now() - LOG_THROTTLE;
    Ok(ConnCtx {
        player_id,
        world_bytes_rx,
        server_state_rx,
        input_tx: state.input_tx.clone(),

        msgs_in: 0,
        msgs_out: 0,
        bytes_in: 0,
        bytes_out: 0,

        invalid_json: 0,

        last_input_full_log: now,
        last_world_lag_log: now,
        last_invalid_input_log: now,

        close_frame: None,
    })
}

enum LoopControl {
    Continue,
    Disconnect,
}

const LOG_THROTTLE: Duration = Duration::from_secs(2);
const MAX_INVALID_JSON: u32 = 10;

fn should_log(last: &mut Instant) -> bool {
    if last.elapsed() >= LOG_THROTTLE {
        *last = Instant::now();
        true
    } else {
        false
    }
}

fn sanitize_input(mut input: PlayerInput) -> Option<PlayerInput> {
    if !input.thrust.is_finite() || !input.turn.is_finite() {
        return None;
    }

    input.thrust = input.thrust.clamp(-1.0, 1.0);
    input.turn = input.turn.clamp(-1.0, 1.0);

    Some(input)
}

async fn run_client_loop(socket: &mut WebSocket, ctx: &mut ConnCtx) -> Result<(), NetError> {
    let player_id = ctx.player_id;

    // Split borrows so `tokio::select!` can hold them concurrently.
    let ConnCtx {
        input_tx,
        world_bytes_rx,
        server_state_rx,
        msgs_in,
        msgs_out,
        bytes_in,
        bytes_out,
        invalid_json,
        last_input_full_log,
        last_world_lag_log,
        last_invalid_input_log,
        close_frame,
        ..
    } = ctx;

    let mut fatal: Option<NetError> = None;

    loop {
        // disconnect becomes true on error
        let disconnect: bool = tokio::select! {
            // Incoming Message from Client
            incoming = socket.recv() => {
                match handle_incoming_ws(
                    socket,
                    incoming,
                    player_id,
                    input_tx,
                    msgs_in,
                    bytes_in,
                    invalid_json,
                    last_input_full_log,
                    last_invalid_input_log,
                    close_frame,
                ).await {
                    Ok(LoopControl::Continue) => false,
                    Ok(LoopControl::Disconnect) => true,
                    Err(e) => {
                        fatal = Some(e);
                        true
                    }
                }
            }

            // Outgoing World Update
            world_msg = world_bytes_rx.recv() => {
                match world_msg {
                    Ok(bytes) => match forward_world_bytes(bytes, socket, msgs_out, bytes_out).await {
                        LoopControl::Continue => false,
                        LoopControl::Disconnect => true,
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        if should_log(last_world_lag_log) {
                            warn!(missed = n, "world updates lagged; sending snapshot");
                        }

                        // Resync strategy: send the latest GameState snapshot.
                        let st = server_state_rx.borrow().clone();
                        match send_message(socket, &ServerMessage::GameState(st)).await {
                            Ok(bytes) => {
                                *msgs_out += 1;
                                *bytes_out += bytes as u64;
                                false
                            }
                            Err(_) => true,
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        fatal = Some(NetError::WorldUpdatesClosed);
                        true
                    }
                }
            }

            // Outgoing Server State
            changed_state = server_state_rx.changed() => {
                match changed_state {
                    Ok(()) => match forward_server_state(server_state_rx, socket, msgs_out, bytes_out).await {
                        LoopControl::Continue => false,
                        LoopControl::Disconnect => true,
                    },
                    Err(_) => {
                        warn!(player_id, "server state channel closed; disconnecting");
                        fatal = Some(NetError::ServerStateClosed);
                        true
                    }
                }
            }
        };

        if disconnect {
            if let Some(frame) = close_frame.take() {
                let _ = socket.send(Message::Close(Some(frame))).await;
            }
            if let Err(err) = socket.close().await.map_err(NetError::Ws) {
                debug!(error = ?err, "socket close error");
            }
            break;
        }
    }

    if let Err(e) = disconnect_cleanup(
        player_id,
        input_tx,
        *msgs_in,
        *msgs_out,
        *bytes_in,
        *bytes_out,
        *invalid_json,
    )
    .await
    {
        warn!(error = ?e, "error during disconnect cleanup");
        if fatal.is_none() {
            fatal = Some(e);
        }
    }

    if let Some(err) = fatal {
        Err(err)
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_incoming_ws(
    _socket: &mut WebSocket,
    incoming: Option<Result<Message, Error>>,
    player_id: u64,
    input_tx: &mpsc::Sender<GameEvent>,
    msgs_in: &mut u64,
    bytes_in: &mut u64,
    invalid_json: &mut u32,
    last_input_full_log: &mut Instant,
    last_invalid_input_log: &mut Instant,
    close_frame: &mut Option<CloseFrame>,
) -> Result<LoopControl, NetError> {
    match incoming {
        Some(Ok(msg)) => match msg {
            Message::Text(text) => {
                *msgs_in += 1;
                *bytes_in += text.len() as u64;

                match serde_json::from_str::<PlayerInput>(&text) {
                    Ok(input) => {
                        let Some(input) = sanitize_input(input) else {
                            if should_log(last_invalid_input_log) {
                                warn!(player_id, "invalid input values (NaN/inf); dropping");
                            }
                            return Ok(LoopControl::Continue);
                        };

                        match input_tx.try_send(GameEvent::Input { player_id, input }) {
                            Ok(()) => Ok(LoopControl::Continue),
                            Err(tokio::sync::mpsc::error::TrySendError::Full(_evt)) => {
                                if should_log(last_input_full_log) {
                                    warn!(player_id, "input channel full; dropping input");
                                }
                                Ok(LoopControl::Continue)
                            }
                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_evt)) => {
                                Err(NetError::InputClosed)
                            }
                        }
                    }
                    Err(e) => {
                        *invalid_json += 1;
                        if should_log(last_invalid_input_log) {
                            warn!(player_id, bytes = text.len(), error = %e, "failed to parse player input");
                        }

                        if *invalid_json > MAX_INVALID_JSON {
                            *close_frame = Some(CloseFrame {
                                code: close_code::POLICY,
                                reason: "too many invalid messages".into(),
                            });
                            return Ok(LoopControl::Disconnect);
                        }

                        Ok(LoopControl::Continue)
                    }
                }
            }
            Message::Binary(_) => {
                *close_frame = Some(CloseFrame {
                    code: close_code::UNSUPPORTED,
                    reason: "binary messages not supported".into(),
                });
                Ok(LoopControl::Disconnect)
            }
            Message::Ping(_) | Message::Pong(_) => Ok(LoopControl::Continue),
            Message::Close(_) => Ok(LoopControl::Disconnect),
        },
        Some(Err(e)) => {
            warn!(player_id, error = %e, "websocket recv error");
            Ok(LoopControl::Disconnect)
        }
        None => {
            info!(player_id, "websocket closed");
            Ok(LoopControl::Disconnect)
        }
    }
}

async fn forward_world_bytes(
    world_msg: Utf8Bytes,
    socket: &mut WebSocket,
    msgs_out: &mut u64,
    bytes_out: &mut u64,
) -> LoopControl {
    let bytes_len = world_msg.len();
    match socket
        .send(Message::Text(world_msg))
        .await
        .map_err(NetError::Ws)
    {
        Ok(()) => {
            *msgs_out += 1;
            *bytes_out += bytes_len as u64;
            LoopControl::Continue
        }
        Err(err) => {
            // Log unexpected send failures; disconnect will follow immediately.
            warn!(error = ?err, "failed to send world update");
            LoopControl::Disconnect
        }
    }
}

async fn forward_server_state(
    server_state_rx: &Receiver<ServerState>,
    socket: &mut WebSocket,
    msgs_out: &mut u64,
    bytes_out: &mut u64,
) -> LoopControl {
    let st = server_state_rx.borrow().clone();
    let msg = ServerMessage::GameState(st);
    match send_message(socket, &msg).await {
        Ok(bytes) => {
            *msgs_out += 1;
            *bytes_out += bytes as u64;
            LoopControl::Continue
        }
        Err(err) => {
            // Log unexpected send failures; disconnect will follow immediately.
            warn!(error = ?err, "failed to send server state");
            LoopControl::Disconnect
        }
    }
}

async fn disconnect_cleanup(
    player_id: u64,
    input_tx: &mpsc::Sender<GameEvent>,
    msgs_in: u64,
    msgs_out: u64,
    bytes_in: u64,
    bytes_out: u64,
    invalid_json: u32,
) -> Result<(), NetError> {
    input_tx
        .send(GameEvent::Leave { player_id })
        .await
        .map_err(|_| NetError::InputClosed)?;

    debug!(
        player_id,
        msgs_in, msgs_out, bytes_in, bytes_out, invalid_json, "connection stats"
    );
    info!(player_id, "client disconnected");
    Ok(())
}
