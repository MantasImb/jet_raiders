use crate::domain::PlayerInput;
use crate::interface_adapters::clients::auth::{AuthClient, VerifyTokenError};
use crate::interface_adapters::http::ErrorResponse;
use crate::interface_adapters::protocol::{
    ClientMessage, PlayerInputDto, ServerMessage, WorldUpdateDto,
};
use crate::interface_adapters::state::AppState;
use crate::interface_adapters::utils::rng::rand_id;
use crate::use_cases::{GameEvent, LobbyHandle, LobbyRegistry, ServerState, WorldUpdate};

use axum::{
    Error, Json,
    extract::{
        Query, State,
        ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade, close_code},
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures::SinkExt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::watch::Receiver;
use tokio::sync::{Notify, broadcast, mpsc, watch};
use tokio::time::timeout;
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
    JoinRequired,
    JoinTimeout,
    AuthVerify,
    ClosedBeforeJoin,
}

#[derive(Debug, serde::Deserialize)]
pub struct LobbyQuery {
    // The lobby id the client wants to join.
    #[serde(default)]
    lobby_id: Option<String>,
}

pub async fn world_update_serializer(
    mut world_rx: broadcast::Receiver<WorldUpdate>,
    world_bytes_tx: broadcast::Sender<Utf8Bytes>,
    world_latest_tx: watch::Sender<Utf8Bytes>,
) {
    // Serialize each world update once and broadcast the shared bytes.
    loop {
        match world_rx.recv().await {
            Ok(update) => {
                let msg = ServerMessage::WorldUpdate(WorldUpdateDto::from(update));
                let txt = match serde_json::to_string(&msg) {
                    Ok(txt) => txt,
                    Err(e) => {
                        error!(error = ?e, "failed to serialize world update");
                        continue;
                    }
                };

                // Convert once and broadcast shared UTF-8 bytes to all clients.
                let bytes = Utf8Bytes::from(txt);
                // Store the latest bytes for lag recovery.
                let _ = world_latest_tx.send(bytes.clone());
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

pub fn spawn_lobby_serializer(lobby: &LobbyHandle) {
    // Spawn a task that serializes world updates for this lobby.
    tokio::spawn(world_update_serializer(
        lobby.world_tx.subscribe(),
        lobby.world_bytes_tx.clone(),
        lobby.world_latest_tx.clone(),
    ));
}

impl From<axum::Error> for NetError {
    fn from(e: axum::Error) -> Self {
        NetError::Ws(e)
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<LobbyQuery>,
) -> impl IntoResponse {
    let lobby_id = query
        .lobby_id
        .unwrap_or_else(|| state.default_lobby_id.to_string());

    let lobby = match state.lobby_registry.get_lobby(&lobby_id).await {
        Some(lobby) => lobby,
        None => {
            // Keep not-found responses consistent with the JSON error schema.
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "lobby not found".to_string(),
                }),
            )
                .into_response();
        }
    };

    let lobby_registry = state.lobby_registry.clone();
    let auth_client = state.auth_client.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, lobby, lobby_registry, auth_client))
}

async fn handle_socket(
    mut socket: WebSocket,
    lobby: LobbyHandle,
    lobby_registry: Arc<LobbyRegistry>,
    auth_client: Arc<AuthClient>,
) {
    // Separate connection id for correlating logs before/after a player_id exists.
    let conn_id = rand_id();
    let span = info_span!("conn", conn_id, player_id = tracing::field::Empty);
    let _enter = span.enter();

    let mut ctx = match bootstrap_connection(
        &mut socket,
        &lobby,
        lobby_registry.clone(),
        auth_client,
    )
    .await
    {
        Ok(ctx) => ctx,
        Err(NetError::ClosedBeforeJoin) => {
            info!("client disconnected before join handshake");
            return;
        }
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

    // Register the connection so the lobby stays alive while sockets are active.
    if lobby_registry
        .register_connection(&ctx.lobby_id)
        .await
        .is_none()
    {
        // Release the player slot if the lobby disappeared before registration.
        ctx.lobby
            .unregister_player_connection_if_owner(ctx.player_id, ctx.player_conn_token)
            .await;
        // The lobby can be removed between lookup and registration during shutdown.
        warn!(lobby_id = %ctx.lobby_id, "lobby missing during connection registration");
        // Best-effort cleanup in case the lobby was removed after bootstrap.
        if ctx.can_spawn {
            let _ = ctx
                .input_tx
                .send(GameEvent::Leave {
                    player_id: ctx.player_id,
                })
                .await;
        }
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: close_code::POLICY,
                reason: "lobby unavailable".into(),
            })))
            .await;
        let _ = socket.close().await;
        return;
    }
    ctx.registered = true;

    span.record("player_id", ctx.player_id);
    info!(
        player_id = ctx.player_id,
        session_id = %ctx.session_id,
        display_name = %ctx.display_name,
        "client connected"
    );

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
    pub session_id: String,
    pub display_name: String,
    // Lobby id this connection is attached to.
    pub lobby_id: Arc<str>,
    // Registry access for connection lifecycle updates.
    pub lobby_registry: Arc<LobbyRegistry>,
    // Lobby handle for per-player connection ownership cleanup.
    pub lobby: LobbyHandle,
    // Token used to verify ownership of the player connection slot.
    pub player_conn_token: u64,
    // Shutdown signal used to replace stale connections.
    pub player_conn_shutdown: Arc<Notify>,
    // Whether the connection has been registered in the lobby counter.
    pub registered: bool,
    pub input_tx: mpsc::Sender<GameEvent>,
    pub world_bytes_rx: broadcast::Receiver<Utf8Bytes>,
    pub world_latest_rx: watch::Receiver<Utf8Bytes>,
    pub server_state_rx: watch::Receiver<ServerState>,
    pub can_spawn: bool,
    // Count lag recovery snapshots sent to this client.
    pub lag_recovery_count: u64,

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

#[derive(Debug)]
struct JoinHandshake {
    player_id: u64,
    session_id: String,
    display_name: String,
    bytes_in: u64,
    msgs_in: u64,
}

async fn bootstrap_connection(
    socket: &mut WebSocket,
    lobby: &LobbyHandle,
    lobby_registry: Arc<LobbyRegistry>,
    auth_client: Arc<AuthClient>,
) -> Result<ConnCtx, NetError> {
    // Subscribe to updates *before* doing anything else (awaits) to not miss packets.
    let world_bytes_rx = lobby.world_bytes_tx.subscribe();
    let world_latest_rx = lobby.world_latest_tx.subscribe();
    let server_state_rx = lobby.server_state_tx.subscribe();

    // Authenticate the very first meaningful client message before assigning player ownership.
    let join = match timeout(
        JOIN_HANDSHAKE_TIMEOUT,
        read_join_handshake(socket, auth_client.as_ref()),
    )
    .await
    {
        Ok(result) => result?,
        Err(_) => {
            let _ = send_close_with_reason(socket, close_code::POLICY, "join timeout").await;
            return Err(NetError::JoinTimeout);
        }
    };
    let player_id = join.player_id;

    // Handshake & ID Assignment
    // Player identity is the canonical verified user id from auth.
    // Track this connection with a unique token so newer connections can replace it.
    let player_conn_token = rand_id();
    let player_conn_shutdown = lobby
        .register_or_replace_player_connection(player_id, player_conn_token)
        .await;

    // Send Identity Packet
    // Tell the client "This is who you are".
    let identity_msg = ServerMessage::Identity {
        player_id: player_id.to_string(),
    };
    if let Err(err) = send_message(socket, &identity_msg).await {
        // Ensure the player slot is freed if we fail the handshake early.
        lobby
            .unregister_player_connection_if_owner(player_id, player_conn_token)
            .await;
        return Err(err);
    }

    // Only allow spawning if the lobby explicitly allows this player id.
    let can_spawn = lobby.is_player_allowed(player_id);

    if can_spawn {
        // Notify World Task
        // Tell the game loop to spawn a ship for this ID.
        // Join happens before initial state so the snapshot can include the newly spawned player.
        // If anything after Join fails, compensate with Leave to avoid "spawned but never connected".
        if let Err(err) = lobby
            .input_tx
            .send(GameEvent::Join { player_id })
            .await
            .map_err(|_| NetError::InputClosed)
        {
            lobby
                .unregister_player_connection_if_owner(player_id, player_conn_token)
                .await;
            return Err(err);
        }
    }

    // Send Initial State
    // Keep in mind that we clone as soon as we borrow to avoid holding the lock. (especially
    // during an await)
    let initial_state = server_state_rx.borrow().clone();
    let state_msg = ServerMessage::GameState(initial_state.into());
    if let Err(e) = send_message(socket, &state_msg).await {
        if can_spawn {
            lobby
                .input_tx
                .send(GameEvent::Leave { player_id })
                .await
                .map_err(|_| NetError::InputClosed)?; // InputClosed takes precedence
        }
        lobby
            .unregister_player_connection_if_owner(player_id, player_conn_token)
            .await;
        return Err(e);
    }

    let now = Instant::now() - LOG_THROTTLE;
    Ok(ConnCtx {
        player_id,
        session_id: join.session_id,
        display_name: join.display_name,
        lobby_id: lobby.lobby_id.clone(),
        lobby_registry,
        lobby: lobby.clone(),
        player_conn_token,
        player_conn_shutdown,
        registered: false,
        world_bytes_rx,
        world_latest_rx,
        server_state_rx,
        input_tx: lobby.input_tx.clone(),
        can_spawn,
        lag_recovery_count: 0,

        msgs_in: join.msgs_in,
        msgs_out: 0,
        bytes_in: join.bytes_in,
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
const MAX_SESSION_TOKEN_LEN: usize = 4096;
const JOIN_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

async fn send_close_with_reason(
    socket: &mut WebSocket,
    code: u16,
    reason: &'static str,
) -> Result<(), NetError> {
    socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await
        .map_err(NetError::Ws)?;
    socket.close().await.map_err(NetError::Ws)
}

async fn read_join_handshake(
    socket: &mut WebSocket,
    auth_client: &AuthClient,
) -> Result<JoinHandshake, NetError> {
    loop {
        let Some(incoming) = socket.recv().await else {
            return Err(NetError::ClosedBeforeJoin);
        };

        let message = incoming.map_err(NetError::Ws)?;
        match message {
            Message::Text(text) => {
                let bytes_in = text.len() as u64;
                let parsed = serde_json::from_str::<ClientMessage>(&text);
                let payload = match parsed {
                    Ok(ClientMessage::Join(payload)) => payload,
                    Ok(ClientMessage::Input(_)) => {
                        let _ = send_close_with_reason(socket, close_code::POLICY, "join required")
                            .await;
                        return Err(NetError::JoinRequired);
                    }
                    Err(_) => {
                        let _ = send_close_with_reason(
                            socket,
                            close_code::POLICY,
                            "invalid join payload",
                        )
                        .await;
                        return Err(NetError::JoinRequired);
                    }
                };

                let session_token = payload.session_token.trim();
                if session_token.is_empty() || session_token.len() > MAX_SESSION_TOKEN_LEN {
                    let _ =
                        send_close_with_reason(socket, close_code::POLICY, "invalid session token")
                            .await;
                    return Err(NetError::AuthVerify);
                }

                let identity = match auth_client.verify_token(session_token).await {
                    Ok(identity) => identity,
                    Err(VerifyTokenError::InvalidToken) => {
                        let _ = send_close_with_reason(
                            socket,
                            close_code::POLICY,
                            "invalid session token",
                        )
                        .await;
                        return Err(NetError::AuthVerify);
                    }
                    Err(VerifyTokenError::SessionExpired) => {
                        let _ =
                            send_close_with_reason(socket, close_code::POLICY, "session expired")
                                .await;
                        return Err(NetError::AuthVerify);
                    }
                    Err(VerifyTokenError::UpstreamUnavailable) => {
                        let _ =
                            send_close_with_reason(socket, close_code::ERROR, "auth unavailable")
                                .await;
                        return Err(NetError::AuthVerify);
                    }
                };
                let _token_expires_at = identity.expires_at;

                return Ok(JoinHandshake {
                    player_id: identity.user_id,
                    session_id: identity.session_id,
                    display_name: identity.display_name,
                    // Token expiry is enforced only at join to avoid mid-round disconnects.
                    bytes_in,
                    msgs_in: 1,
                });
            }
            Message::Binary(_) => {
                let _ = send_close_with_reason(
                    socket,
                    close_code::UNSUPPORTED,
                    "binary messages not supported",
                )
                .await;
                return Err(NetError::JoinRequired);
            }
            Message::Ping(_) | Message::Pong(_) => {}
            Message::Close(_) => return Err(NetError::ClosedBeforeJoin),
        }
    }
}

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

// Shared input handling for both legacy and structured messages.
fn process_input_message(
    player_id: u64,
    input_tx: &mpsc::Sender<GameEvent>,
    input: PlayerInput,
    last_input_full_log: &mut Instant,
    last_invalid_input_log: &mut Instant,
) -> Result<LoopControl, NetError> {
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
        Err(tokio::sync::mpsc::error::TrySendError::Closed(_evt)) => Err(NetError::InputClosed),
    }
}

async fn run_client_loop(socket: &mut WebSocket, ctx: &mut ConnCtx) -> Result<(), NetError> {
    let player_id = ctx.player_id;

    // Split borrows so `tokio::select!` can hold them concurrently.
    let ConnCtx {
        lobby_id,
        lobby_registry,
        lobby,
        player_conn_token,
        player_conn_shutdown,
        registered,
        input_tx,
        world_bytes_rx,
        world_latest_rx,
        server_state_rx,
        can_spawn,
        lag_recovery_count,
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
                    *can_spawn,
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

                        // Resync strategy: send the latest world snapshot.
                        let latest = world_latest_rx.borrow().clone();
                        if latest.is_empty() {
                            if should_log(last_world_lag_log) {
                                warn!("world snapshot unavailable during lag recovery");
                            }
                            false
                        } else {
                            let bytes_len = latest.len();
                            // Track how often we need to recover from lag.
                            *lag_recovery_count += 1;
                            let outcome =
                                forward_world_bytes(latest, socket, msgs_out, bytes_out).await;

                            if should_log(last_world_lag_log) {
                                debug!(
                                    player_id,
                                    bytes = bytes_len,
                                    count = *lag_recovery_count,
                                    "sent lag recovery snapshot"
                                );
                            }

                            match outcome {
                                LoopControl::Continue => false,
                                LoopControl::Disconnect => true,
                            }
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

            // Connection replacement signal for duplicate player ids.
            _ = player_conn_shutdown.notified() => {
                // Ask the client to close; a newer connection took ownership.
                *close_frame = Some(CloseFrame {
                    code: close_code::POLICY,
                    reason: "connection replaced".into(),
                });
                info!(player_id, "connection replaced by newer session");
                true
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
        lobby_id,
        lobby_registry,
        lobby,
        *player_conn_token,
        *registered,
        input_tx,
        *can_spawn,
        *msgs_in,
        *msgs_out,
        *bytes_in,
        *bytes_out,
        *invalid_json,
        *lag_recovery_count,
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
    can_spawn: bool,
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

                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Join(_)) => {
                        // Ignore repeated Join packets after bootstrap to keep the session stable.
                        if should_log(last_invalid_input_log) {
                            warn!(player_id, "duplicate join ignored");
                        }
                        Ok(LoopControl::Continue)
                    }
                    Ok(ClientMessage::Input(input)) => {
                        if !can_spawn {
                            // Spectators cannot control ships in the lobby.
                            if should_log(last_invalid_input_log) {
                                warn!(player_id, "spectator input ignored");
                            }
                            return Ok(LoopControl::Continue);
                        }

                        let input: PlayerInput = input.into();
                        process_input_message(
                            player_id,
                            input_tx,
                            input,
                            last_input_full_log,
                            last_invalid_input_log,
                        )
                    }
                    Err(parse_err) => {
                        // Legacy client fallback: accept raw PlayerInput messages.
                        match serde_json::from_str::<PlayerInputDto>(&text) {
                            Ok(input) => {
                                if !can_spawn {
                                    // Legacy input is ignored for spectators.
                                    if should_log(last_invalid_input_log) {
                                        warn!(player_id, "spectator legacy input ignored");
                                    }
                                    Ok(LoopControl::Continue)
                                } else {
                                    process_input_message(
                                        player_id,
                                        input_tx,
                                        input.into(),
                                        last_input_full_log,
                                        last_invalid_input_log,
                                    )
                                }
                            }
                            Err(_) => {
                                *invalid_json += 1;
                                if should_log(last_invalid_input_log) {
                                    warn!(
                                        player_id,
                                        bytes = text.len(),
                                        error = %parse_err,
                                        "failed to parse client message"
                                    );
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
    let msg = ServerMessage::GameState(st.into());
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

#[allow(clippy::too_many_arguments)]
async fn disconnect_cleanup(
    player_id: u64,
    lobby_id: &Arc<str>,
    lobby_registry: &Arc<LobbyRegistry>,
    lobby: &LobbyHandle,
    player_conn_token: u64,
    registered: bool,
    input_tx: &mpsc::Sender<GameEvent>,
    can_spawn: bool,
    msgs_in: u64,
    msgs_out: u64,
    bytes_in: u64,
    bytes_out: u64,
    invalid_json: u32,
    lag_recovery_count: u64,
) -> Result<(), NetError> {
    if can_spawn {
        // Only despawn players that were allowed to join the lobby.
        input_tx
            .send(GameEvent::Leave { player_id })
            .await
            .map_err(|_| NetError::InputClosed)?;
    }

    if registered {
        // Spectators keep lobbies alive by policy, so count every socket.
        lobby_registry.register_disconnect(lobby_id).await;
    }

    // Release the player connection slot if this connection still owns it.
    lobby
        .unregister_player_connection_if_owner(player_id, player_conn_token)
        .await;

    debug!(
        player_id,
        msgs_in,
        msgs_out,
        bytes_in,
        bytes_out,
        invalid_json,
        lag_recovery_count,
        "connection stats"
    );
    info!(player_id, "client disconnected");
    Ok(())
}
