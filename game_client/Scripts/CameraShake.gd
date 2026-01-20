extends Camera2D
class_name Camera

var intensity: float = 3.0
var max_duration: float
var cur_duration: float

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta: float) -> void:
	if cur_duration <= 0:
		return
	
	cur_duration = move_toward(cur_duration, 0.0, delta)
	var dur_prc = cur_duration / max_duration
	
	var x: float = randf_range(-dur_prc, dur_prc)
	var y: float = randf_range(-dur_prc, dur_prc)
	var pos: Vector2 = Vector2(x, y) * intensity
	
	offset = pos

func shake(set_duration: float, set_intensity: float) -> void:
	intensity = set_intensity
	cur_duration = set_duration
	max_duration = set_duration
