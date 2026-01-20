extends ProgressBar

@export var fill_color: Gradient
var game_manager: GameManager

func _ready() -> void:
	game_manager = get_tree().get_current_scene().get_node("GameManager")

func _process(delta: float) -> void:
	if game_manager.local_player == null:
		return
	
	var player: Player = game_manager.local_player
	
	max_value = player.max_weapon_heat
	value = player.cur_weapon_heat
	
	get("theme_override_styles/fill").bg_color = fill_color.sample(value / max_value)
