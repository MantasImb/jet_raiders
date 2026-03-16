extends Node
class_name AuthStateMachine

signal state_changed(from_state: StringName, to_state: StringName, reason: String)
signal authenticated(session_token: String)

const MAX_GUEST_INIT_RETRIES: int = 5
const MAX_LOGIN_RETRIES: int = 5
const RETRY_BASE_DELAY: float = 0.5
const RETRY_MAX_DELAY: float = 6.0

@onready var auth_context: AuthContext = $"../AuthContext"

var current_state: AuthStateBase
var auth_api_client: AuthApiClient
var retry_timer: Timer
var guest_init_retry_count: int = 0
var login_retry_count: int = 0
var auth_error_code: String = ""
var auth_error_detail: String = ""

var _state_nodes: Dictionary = {}
var _pending_retry_kind: StringName = &""
var _pending_retry_event: StringName = &""
var _transition_map: Dictionary = {
	&"BootstrapState": [&"GuestIdentityState"],
	&"GuestIdentityState": [&"LoginState"],
	&"LoginState": [&"GuestIdentityState", &"AuthenticatedState"],
	&"AuthenticatedState": []
}

func _ready() -> void:
	# One transport client is shared across all auth states.
	auth_api_client = AuthApiClient.new()
	add_child(auth_api_client)

	retry_timer = Timer.new()
	retry_timer.one_shot = true
	retry_timer.timeout.connect(_on_retry_timeout)
	add_child(retry_timer)

	for child in get_children():
		if child is AuthStateBase:
			var state := child as AuthStateBase
			state.setup(self)
			_state_nodes[StringName(state.name)] = state

	transition_to(&"BootstrapState", "startup")

func transition_to(state_name: StringName, reason: String = "", ctx: Dictionary = {}) -> bool:
	if not _state_nodes.has(state_name):
		push_error("Unknown auth state: %s" % state_name)
		return false

	var from_state: StringName = &""
	if current_state != null:
		from_state = StringName(current_state.name)
		if state_name != from_state and not _is_transition_allowed(from_state, state_name):
			push_error("Invalid auth transition %s -> %s" % [from_state, state_name])
			return false
		current_state.exit()

	if current_state == null or StringName(current_state.name) != state_name:
		cancel_retry()

	# States may trigger nested transitions in enter(); only emit if we stayed put.
	current_state = _state_nodes[state_name]
	var entered_state: AuthStateBase = current_state
	current_state.enter(ctx)
	if current_state == entered_state:
		emit_signal("state_changed", from_state, state_name, reason)
	return true

func send_event(event: StringName, payload: Dictionary = {}) -> void:
	if current_state == null:
		return
	current_state.handle_event(event, payload)

func request_login() -> void:
	send_event(&"login_requested")

func is_authenticated() -> bool:
	return get_current_state_name() == &"AuthenticatedState"

func get_current_state_name() -> StringName:
	if current_state == null:
		return &""
	return StringName(current_state.name)

func get_current_substate_name() -> StringName:
	if current_state == null:
		return &""
	return current_state.get_substate_name()

func can_retry(kind: StringName) -> bool:
	return _get_retry_count(kind) < _get_retry_limit(kind)

func schedule_retry(kind: StringName, retry_event: StringName) -> void:
	var next_attempt := _get_retry_count(kind) + 1
	_set_retry_count(kind, next_attempt)
	_pending_retry_kind = kind
	_pending_retry_event = retry_event

	# Share one retry policy across guest init and login failure paths.
	var delay: float = min(
		RETRY_BASE_DELAY * pow(2.0, float(next_attempt - 1)),
		RETRY_MAX_DELAY
	)
	print("Auth retry scheduled in %s seconds for %s" % [delay, kind])
	retry_timer.start(delay)

func clear_retry(kind: StringName) -> void:
	_set_retry_count(kind, 0)
	if _pending_retry_kind == kind:
		cancel_retry()

func cancel_retry() -> void:
	if retry_timer != null:
		retry_timer.stop()
	_pending_retry_kind = &""
	_pending_retry_event = &""

func set_error(code: String, detail: String) -> void:
	auth_error_code = code
	auth_error_detail = detail

func clear_error() -> void:
	auth_error_code = ""
	auth_error_detail = ""

func notify_authenticated(session_token: String) -> void:
	emit_signal("authenticated", session_token)

func _is_transition_allowed(from_state: StringName, to_state: StringName) -> bool:
	if not _transition_map.has(from_state):
		return false
	return to_state in _transition_map[from_state]

func _get_retry_limit(kind: StringName) -> int:
	match kind:
		&"guest_init":
			return MAX_GUEST_INIT_RETRIES
		&"login":
			return MAX_LOGIN_RETRIES
		_:
			return 0

func _get_retry_count(kind: StringName) -> int:
	match kind:
		&"guest_init":
			return guest_init_retry_count
		&"login":
			return login_retry_count
		_:
			return 0

func _set_retry_count(kind: StringName, count: int) -> void:
	match kind:
		&"guest_init":
			guest_init_retry_count = count
		&"login":
			login_retry_count = count

func _on_retry_timeout() -> void:
	var retry_event := _pending_retry_event
	_pending_retry_kind = &""
	_pending_retry_event = &""
	send_event(retry_event)
