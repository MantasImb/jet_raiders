extends Area2D
class_name Projectile

@export var speed : float = 500.0
var owner_id : int

func _ready() -> void:
	if not multiplayer.is_server():
		set_physics_process(false)

func _physics_process(delta: float) -> void:
	position += -transform.y * speed * delta

func _on_body_entered(body: Player) -> void:
	if not multiplayer.is_server():
		return
	if not body.is_in_group("Player"):
		return
	if body.player_id == owner_id:
		return
	
	body.take_damage(10, owner_id)
	self.queue_free()

func _on_timer_timeout() -> void:
	if multiplayer.is_server():
		queue_free()
