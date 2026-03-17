extends AuthStateBase

func enter(_ctx: Dictionary = {}) -> void:
	super.enter(_ctx)
	set_substate_name(&"ACTIVE")

	if auth_context.auth_token.strip_edges().is_empty():
		var detail := "AuthenticatedState entered without auth_token"
		push_error(detail)
		state_machine.set_error("missing_auth_token", detail)
		# Clear stale session state before re-entering the login flow.
		auth_context.auth_token = ""
		auth_context.local_player_id = ""
		state_machine.transition_to(&"LoginState", "missing auth token in authenticated state")
		return

	state_machine.notify_authenticated(auth_context.auth_token)
