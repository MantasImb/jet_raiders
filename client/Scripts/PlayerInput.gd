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
	acc += delta
	if acc < 1.0:
		return
	acc -= 1.0  # keep remainder so it stays stable over time
	
	# 1. Gather Input
	throttle_input = Input.get_axis("throttle_down", "throttle_up")
	turn_input = Input.get_axis("turn_left", "turn_right")
	shoot_input = Input.is_action_pressed("shoot")
	
	# 2. Validate Network State
	if not network_manager or not network_manager.connected:
		return
		
	# 3. Check Ownership
	# Only the local player instance should send inputs to the server.
	var parent = get_parent()
	if parent is Player:
		if parent.player_id != network_manager.local_player_id:
			return
		
		# Debug: Verify we are sending
		# print("Sending input for ID: ", parent.player_id)
			
		# Calculate movement vector based on current rotation
		var rotation = parent.rotation
			
		# Godot's UP is (0, -1).
		var direction = Vector2.UP.rotated(rotation)
		
		# "Speed" and "Turn Rate" should ideally be server constants or synced.
		# For now, we estimate a movement delta to send.
		var speed_factor = 5.0 
		var turn_factor = 0.1
		
		var movement = direction * throttle_input * speed_factor
		
		var packet = {
			"dx": movement.x,
			"dy": movement.y,
			"rot": turn_input * turn_factor,
			"shoot": shoot_input
		}
		
		network_manager.send_input(packet)
