extends Area2D
class_name Projectile

# Authoritative state comes from the server; this node is just a visual.
var projectile_id: int = 0
var owner_id: int = 0

# Interpolation
var target_position: Vector2 = Vector2.ZERO
var target_rotation: float = 0.0
var smoothing_speed: float = 25.0
var initialized: bool = false

func _ready() -> void:
	# Disable any legacy local simulation.
	set_physics_process(false)
	
	target_position = position
	target_rotation = rotation

func update_state(state: Dictionary) -> void:
	# state has: id, owner_id, x, y, rot
	target_position = Vector2(state.x, state.y)
	target_rotation = state.rot
	
	# Snap on first update so we don't lerp from the default spawn position (0, 0).
	if not initialized:
		position = target_position
		rotation = target_rotation
		initialized = true

func _process(delta: float) -> void:
	position = position.lerp(target_position, smoothing_speed * delta)
	rotation = lerp_angle(rotation, target_rotation, smoothing_speed * delta)

func _on_body_entered(_body: Node) -> void:
	# Collisions are handled server-side (future).
	pass

func _on_timer_timeout() -> void:
	# Lifetime is controlled by the server snapshot.
	pass
