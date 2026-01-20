extends Panel

var game_manager: GameManager
@onready var player_scores: Label = $PlayerScores

func _ready() -> void:
	game_manager = get_tree().get_current_scene().get_node("GameManager")

func _process(delta: float) -> void:
	player_scores.text = ""
	
	for player in game_manager.players:
		var text = str(player.player_name, " - ", player.score, "\n")
		player_scores.text += text
