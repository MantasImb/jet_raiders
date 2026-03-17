extends Node
class_name NetworkManager

enum ConnectionState {
	IDLE,
	CONNECTING,
	CONNECTED,
	RETRY_WAIT
}

@onready var auth_context: AuthContext = $AuthContext
@onready var auth_state_machine: AuthStateMachine = $AuthStateMachine
@onready var world_sync: WorldSync = $WorldSync
@onready var game_manager: GameManager = $"../GameManager"

const HEAD_BASE_URL= "http://127.0.0.1:3000"
const TEST_SERVER_URL= "ws://127.0.0.1:3001/ws"
# will be assigned when the head server provides the game server URL
var game_server_url: String
var lobby_id: String

var game_socket: WebSocketPeer = WebSocketPeer.new()
var connection_state: ConnectionState = ConnectionState.IDLE
# Reconnect state is only used when TEST_MODE is enabled.
var reconnect_attempts: int = 0
var reconnect_timer: Timer

# Reconnect backoff settings (seconds).
const RECONNECT_BASE_DELAY: float = 0.5
const RECONNECT_MAX_DELAY: float = 6.0

@onready var network_ui: Panel = $NetworkUI

func _ready() -> void:
	# Use a timer so reconnect attempts don't hammer a restarting server.
	reconnect_timer = Timer.new()
	reconnect_timer.one_shot = true
	reconnect_timer.timeout.connect(_on_reconnect_timeout)
	add_child(reconnect_timer)

	# Start websocket connection only after auth succeeds.
	auth_state_machine.authenticated.connect(_on_auth_authenticated)
	
	# If auth is already available when this node starts, connect immediately.
	if game_manager.TEST_MODE and auth_state_machine.is_authenticated():
		_connect_after_auth()
	
func _process(_delta: float) -> void:
	_poll_socket()
	_handle_socket_state()

func _poll_socket() -> void:
	# Polling advances the Godot WebSocket state machine.
	game_socket.poll()

func _handle_socket_state() -> void:
	var socket_state := game_socket.get_ready_state()

	match socket_state:
		WebSocketPeer.STATE_OPEN:
			_handle_socket_open()
		WebSocketPeer.STATE_CLOSED:
			_handle_socket_closed()

func _handle_socket_open() -> void:
	# Promote to CONNECTED only once per socket lifetime.
	if connection_state != ConnectionState.CONNECTED:
		connection_state = ConnectionState.CONNECTED
		_on_socket_opened()

	_drain_incoming_packets()

func _handle_socket_closed() -> void:
	if connection_state == ConnectionState.CONNECTED:
		connection_state = ConnectionState.IDLE
		_on_socket_closed()
		return

	if connection_state == ConnectionState.CONNECTING:
		connection_state = ConnectionState.IDLE
		_on_socket_connect_failed()

func has_open_connection() -> bool:
	return connection_state == ConnectionState.CONNECTED

func _drain_incoming_packets() -> void:
	# Drain all buffered frames before the next process tick.
	while game_socket.get_available_packet_count() > 0:
		var packet := game_socket.get_packet()
		var data_str := packet.get_string_from_utf8()
		_handle_server_message(data_str)

func start_client(url: String) -> void:
	# Always reset the socket before connecting to avoid stale states.
	_reset_socket()
	print("Connecting to %s..." % url)
	var err := game_socket.connect_to_url(url)
	if err != OK:
		print("Connection error: %s" % err)
		_on_socket_connect_failed()
		return

	connection_state = ConnectionState.CONNECTING

func join_test_lobby() -> void:
	start_client(TEST_SERVER_URL)

func send_input(input_data: Dictionary) -> void:
	if game_socket.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return

	# Wrap input in the structured message expected by the server.
	var message = {
		"type": "Input",
		"data": input_data
	}
	var json_str = JSON.stringify(message)
	game_socket.send_text(json_str)

func _handle_server_message(json_str: String) -> void:
	var json = JSON.new()
	var error = json.parse(json_str)
	if error != OK:
		print("JSON Parse Error: ", json.get_error_message())
		return

	var msg = json.data
	# Server sends { "type": "...", "data": ... }
	if not (msg is Dictionary and msg.has("type")):
		return

	# Transport only routes messages; scene mutation lives in WorldSync.
	match msg.type:
		"Identity":
			# { "type": "Identity", "data": { "player_id": 123 } }
			if msg.data is Dictionary and msg.data.has("player_id"):
				auth_context.local_player_id = str(msg.data.player_id)
				print("Assigned Player ID: ", auth_context.local_player_id)
		"WorldUpdate":
			# { "type": "WorldUpdate", "data": { "tick": 1, "entities": [...] } }
			if msg.data is Dictionary and msg.data.has("entities"):
				world_sync.apply_world_update(msg.data)
				
		"GameState":
			# { "type": "GameState", "data": { ... } } or "MatchRunning"
			print("Game State Update: ", msg.data)

func _on_socket_opened() -> void:
	print("Connected to server")
	network_ui.visible = false
	# Successful connection resets the reconnect backoff.
	reconnect_attempts = 0
	reconnect_timer.stop()
	if auth_state_machine.is_authenticated():
		_send_join()

func _on_auth_authenticated(_session_token: String) -> void:
	if not game_manager.TEST_MODE:
		return
	_connect_after_auth()

func _connect_after_auth() -> void:
	# Guard against duplicate connect attempts from startup + signal timing.
	if connection_state != ConnectionState.IDLE:
		return
	start_client(TEST_SERVER_URL)

func _on_socket_connect_failed() -> void:
	print("Connection failed")
	# Attempt to reconnect in test mode if the server is still rebooting.
	_schedule_reconnect("connection failed")

func _on_socket_closed() -> void:
	print("Server has been closed")
	network_ui.visible = true
	world_sync.clear_world()
	# Attempt to reconnect in test mode if the server restarts.
	_schedule_reconnect("server closed")


func _send_join() -> void:
	if game_socket.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return

	if auth_context.auth_token.strip_edges().is_empty():
		push_error("Missing auth token for join")
		return
	
	# Send the auth session token for identity verification in game_server.
	var message = {
		"type": "Join",
		"data": {
			"session_token": auth_context.auth_token
		}
	}
	var json_str = JSON.stringify(message)
	game_socket.send_text(json_str)

func _schedule_reconnect(reason: String) -> void:
	# Only auto-reconnect in test mode to avoid surprising players.
	if not game_manager.TEST_MODE:
		return
	# Reconnect only after auth is established; otherwise wait for login flow.
	if not auth_state_machine.is_authenticated():
		return
	# Avoid stacking multiple timers or reconnecting while already active.
	if connection_state == ConnectionState.CONNECTED \
		or connection_state == ConnectionState.CONNECTING \
		or connection_state == ConnectionState.RETRY_WAIT:
		return
	reconnect_attempts += 1
	var delay: float = min(
		RECONNECT_BASE_DELAY * pow(2.0, float(reconnect_attempts - 1)),
		RECONNECT_MAX_DELAY
	)
	connection_state = ConnectionState.RETRY_WAIT
	print("Reconnect scheduled in %s seconds (%s)" % [delay, reason])
	reconnect_timer.start(delay)

func _on_reconnect_timeout() -> void:
	# In test mode, the server is fixed; attempt to reconnect.
	connection_state = ConnectionState.IDLE
	start_client(TEST_SERVER_URL)

func _reset_socket() -> void:
	# Replace the peer to ensure a clean reconnect after server restarts.
	game_socket = WebSocketPeer.new()
	connection_state = ConnectionState.IDLE
