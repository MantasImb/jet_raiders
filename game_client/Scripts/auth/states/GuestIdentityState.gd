extends AuthStateBase

func enter(_ctx: Dictionary = {}) -> void:
	super.enter(_ctx)
	_load_profile()

func handle_event(event: StringName, _payload: Dictionary = {}) -> void:
	if event == &"retry_timeout" and get_substate_name() == &"RETRY_WAIT":
		_request_guest_id()
		return

	if event == &"login_requested" and get_substate_name() == &"FAILED":
		state_machine.clear_error()
		_request_guest_id()

func _load_profile() -> void:
	set_substate_name(&"LOAD_PROFILE")
	state_machine.clear_error()

	var data := auth_context.load_profile_data()
	auth_context.apply_profile_data(data)

	if auth_context.has_valid_guest_id():
		auth_context.finish_profile_setup()
		state_machine.clear_retry(&"guest_init")
		set_substate_name(&"READY")
		state_machine.transition_to(&"LoginState", "existing guest id")
		return

	_request_guest_id()

func _request_guest_id() -> void:
	set_substate_name(&"REQUEST_GUEST_ID")
	auth_context.normalize_display_name_for_auth()
	print("Requesting guest identity")

	auth_api_client.request_guest_init(
		auth_context.local_username,
		Callable(self, "_on_guest_id_response")
	)

func _on_guest_id_response(response: Dictionary) -> void:
	if get_substate_name() != &"REQUEST_GUEST_ID":
		return

	if not response.get("ok", false):
		_handle_failure(str(response.get("code", "guest_init_failed")), str(response.get("detail", "")))
		return

	var json: Dictionary = response.get("json", {})
	if not json.has("guest_id"):
		_handle_failure("missing_guest_id", "Guest init response did not include guest_id")
		return

	var resolved_guest_id := str(json.guest_id).strip_edges()
	if not auth_context.is_guest_id_valid(resolved_guest_id):
		_handle_failure("invalid_guest_id", "Guest init returned an invalid guest_id")
		return

	auth_context.guest_id = resolved_guest_id
	auth_context.finish_profile_setup()
	state_machine.clear_retry(&"guest_init")
	state_machine.clear_error()
	set_substate_name(&"READY")
	print("Acquired guest id: ", auth_context.guest_id)
	state_machine.transition_to(&"LoginState", "guest identity resolved")

func _handle_failure(code: String, detail: String) -> void:
	state_machine.set_error(code, detail)
	push_warning("Guest identity failed: %s (%s)" % [code, detail])

	if state_machine.can_retry(&"guest_init"):
		set_substate_name(&"RETRY_WAIT")
		state_machine.schedule_retry(&"guest_init", &"retry_timeout")
		return

	set_substate_name(&"FAILED")
