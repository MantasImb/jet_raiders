extends Node
class_name PlayerInput

# These variables are kept so Player.gd can read them for client-side prediction/smoothing if needed
var throttle_input: float
var turn_input: float
var shoot_input: bool

var network_manager: NetworkManager

func _ready() -> void:
	# Find NetworkManager in the scene tree
	var root = get_tree().get_current_scene()
	if root.has_node("Network"):
		network_manager = root.get_node("Network")
		

var acc := 0.0

func _physics_process(delta: float) -> void:
	# Keep this for debugging purposes
	# acc += delta
	# if acc < 1.0:
	# 	return
	# acc -= 1.0  # keep remainder so it stays stable over time
	
	# 1. Gather Input
	throttle_input = Input.get_axis("throttle_down", "throttle_up")
	turn_input = Input.get_axis("turn_left", "turn_right")
	shoot_input = Input.is_action_just_pressed("shoot")
	
	# 2. Validate Network State
	if not network_manager or not network_manager.connected:
		return
		
	# 3. Check Ownership
	# Only the local player instance should send inputs to the server.
	var parent = get_parent()
	if parent is Player:
		if parent.player_id != network_manager.local_player_id:
			return
		
		var packet = {
			"thrust": throttle_input,
			"turn": turn_input,
			"shoot": shoot_input
		}
		
		network_manager.send_input(packet)
