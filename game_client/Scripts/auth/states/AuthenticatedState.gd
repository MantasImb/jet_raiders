extends AuthStateBase

func enter(_ctx: Dictionary = {}) -> void:
	super.enter(_ctx)
	set_substate_name(&"ACTIVE")

	if auth_context.auth_token.strip_edges().is_empty():
		push_error("AuthenticatedState entered without auth_token")
		return

	state_machine.notify_authenticated(auth_context.auth_token)
