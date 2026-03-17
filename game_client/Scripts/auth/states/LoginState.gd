extends AuthStateBase

func enter(_ctx: Dictionary = {}) -> void:
	super.enter(_ctx)
	set_substate_name(&"IDLE_READY")
	state_machine.clear_error()
	_begin_login()

func handle_event(event: StringName, _payload: Dictionary = {}) -> void:
	if event == &"login_requested":
		if get_substate_name() == &"IDLE_READY" or get_substate_name() == &"FAILED_TERMINAL":
			_begin_login()
		return

	if event == &"retry_timeout" and get_substate_name() == &"RETRY_WAIT":
		_begin_login()

func _begin_login() -> void:
	if not auth_context.has_valid_guest_id():
		state_machine.transition_to(&"GuestIdentityState", "guest id missing before login")
		return

	set_substate_name(&"REQUEST_LOGIN")
	auth_context.normalize_display_name_for_auth()
	print("Trying to log in")

	auth_api_client.request_guest_login(
		auth_context.guest_id,
		auth_context.local_username,
		Callable(self, "_on_login_response")
	)

func _on_login_response(response: Dictionary) -> void:
	if get_substate_name() != &"REQUEST_LOGIN":
		return

	if not response.get("ok", false):
		_handle_failure(str(response.get("code", "login_failed")), str(response.get("detail", "")))
		return

	var json: Dictionary = response.get("json", {})
	if not json.has("session_token"):
		_handle_failure("missing_session_token", "Login response did not include session_token")
		return

	var session_token := str(json.session_token).strip_edges()
	if session_token.is_empty():
		_handle_failure("missing_session_token", "Login response did not include a valid session_token")
		return

	auth_context.auth_token = session_token
	state_machine.clear_retry(&"login")
	state_machine.clear_error()
	set_substate_name(&"SUCCESS")
	state_machine.transition_to(&"AuthenticatedState", "login succeeded")

func _handle_failure(code: String, detail: String) -> void:
	state_machine.set_error(code, detail)
	push_warning("Login failed: %s (%s)" % [code, detail])

	if state_machine.can_retry(&"login"):
		set_substate_name(&"RETRY_WAIT")
		state_machine.schedule_retry(&"login", &"retry_timeout")
		return

	set_substate_name(&"FAILED_TERMINAL")
