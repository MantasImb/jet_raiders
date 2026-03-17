extends Node
class_name AuthStateBase

var state_machine: AuthStateMachine
var auth_context: AuthContext
var auth_api_client: AuthApiClient
var _substate_name: StringName = &""

func setup(machine: AuthStateMachine) -> void:
	state_machine = machine
	auth_context = machine.auth_context
	auth_api_client = machine.auth_api_client

func enter(_ctx: Dictionary = {}) -> void:
	_substate_name = &""

func exit() -> void:
	pass

func handle_event(_event: StringName, _payload: Dictionary = {}) -> void:
	pass

func set_substate_name(value: StringName) -> void:
	_substate_name = value

func get_substate_name() -> StringName:
	return _substate_name
