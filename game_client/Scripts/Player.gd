extends CharacterBody2D
class_name Player

# General vars
@onready var input: PlayerInput = $PlayerInput
@onready var shadow: Sprite2D = $Shadow
@onready var respawn_timer: Timer = $RespawnTimer
@onready var audio_player: AudioStreamPlayer2D = $AudioPlayer
@onready var ship_sprite: Sprite2D = $Ship
@onready var hit_particle: CPUParticles2D = $HitParticle

@export var player_name: String
@export var player_id: String = "0"

# Interpolation
var target_position: Vector2 = Vector2.ZERO
var target_rotation: float = 0.0
var smoothing_speed: float = 15.0

@export var max_speed: float = 150.0
@export var turn_rate: float = 2.5
var throttle: float = 0.0

@export var current_hp: int = 100
@export var max_hp: int = 100
@export var score: int = 0
var last_attacker_id: int
var is_alive: bool = true

# Fire vars
@export var shoot_rate: float = 0.1
var last_shoot_time: float
var projectile_scene: PackedScene = preload("res://Scenes/projectile.tscn")
@onready var muzzle: Node2D = $Muzzle

@export var cur_weapon_heat: float = 0.0
@export var max_weapon_heat: float = 100.0
var weapon_heat_increase_rate: float = 7.0
var weapon_heat_cool_rate: float = 25.0
var weapon_heat_cap_wait_time: float = 1.5
var weapon_heat_waiting: bool = false

# Sound FX
const PLANE_EXPLODE = preload("res://Audio/PlaneExplode.wav")
const PLANE_HIT = preload("res://Audio/PlaneHit.wav")
const PLANE_SHOOT = preload("res://Audio/PlaneShoot.wav")

var game_manager: GameManager
var network_manager: NetworkManager
var user: UserManager

func _ready() -> void:
	game_manager = get_tree().get_current_scene().get_node("GameManager")
	game_manager.players.append(self)
	
	if get_tree().get_current_scene().has_node("Network"):
		network_manager = get_tree().get_current_scene().get_node("Network")
		user = network_manager.get_node("UserManager")

	target_position = position
	target_rotation = rotation

func update_state(state: Dictionary) -> void:
	target_position = Vector2(state.x, state.y)
	target_rotation = state.rot
	if state.has("hp"):
		current_hp = int(state.hp)
	
	# If this is our local player and game manager doesn't know it yet, register it
	if network_manager and player_id == user.local_player_id:
		if game_manager and game_manager.local_player != self:
			game_manager.local_player = self
			print("Local player registered with GameManager: ", player_id)

func _process(delta: float) -> void:
	shadow.global_position = position + Vector2(0, 20)
	
	# Client-side Interpolation
	if position.distance_to(target_position) < 50:
		position = position.lerp(target_position, smoothing_speed * delta)
	else:
		position = target_position
	rotation = lerp_angle(rotation, target_rotation, smoothing_speed * delta)

func _physics_process(_delta: float) -> void:
	pass

func increase_score(amount: int) -> void:
	score += amount

func _exit_tree():
	if game_manager:
		game_manager.players.erase(self)
