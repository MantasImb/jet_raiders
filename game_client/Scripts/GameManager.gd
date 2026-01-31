extends Node
class_name GameManager

const TEST_MODE: bool = true

var players: Array[Player]
var local_player: Player
var score_to_win: int = 3

@onready var end_screen: Panel = $"../EndScreen"
@onready var win_text: Label = $"../EndScreen/WinText"
@onready var camera: Camera = $"../Camera2D"

# SoundFX
const PLANE_EXPLODE = preload("uid://cs54j3q23irur")
const PLANE_HIT = preload("uid://b5mmqnrs07h81")
const PLANE_SHOOT = preload("uid://doo3isbmwgwtt")
