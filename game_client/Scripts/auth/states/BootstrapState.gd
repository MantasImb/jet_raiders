extends AuthStateBase

func enter(_ctx: Dictionary = {}) -> void:
	super.enter(_ctx)
	set_substate_name(&"STARTUP")
	state_machine.clear_error()
	state_machine.clear_retry(&"guest_init")
	state_machine.clear_retry(&"login")
	state_machine.transition_to(&"GuestIdentityState", "bootstrap complete")
