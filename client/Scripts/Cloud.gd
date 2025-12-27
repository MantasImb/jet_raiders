extends Sprite2D

@export var min_x: float
@export var max_x: float
@export var speed: float = 100.0

func _process(delta: float) -> void:
	position.x += speed * delta
	if position.x > max_x:
		position.x = min_x
