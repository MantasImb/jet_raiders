extends CharacterBody2D
class_name Player

# General vars
@onready var input: PlayerInput = $InputSynchronizer
@onready var shadow: Sprite2D = $Shadow
@onready var respawn_timer: Timer = $RespawnTimer
@onready var audio_player: AudioStreamPlayer2D = $AudioPlayer
@onready var ship_sprite: Sprite2D = $Ship
@onready var hit_particle: CPUParticles2D = $HitParticle

@export var player_name: String
@export var player_id: int = 1:
	set(id):
		player_id = id
		$InputSynchronizer.set_multiplayer_authority(id)

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

func _ready() -> void:
	game_manager = get_tree().get_current_scene().get_node("GameManager")
	game_manager.players.append(self)
	
	if input.is_multiplayer_authority():
		game_manager.local_player = self
		network_manager = get_tree().get_current_scene().get_node("Network")
		set_player_name.rpc(network_manager.local_username)
	
	if multiplayer.is_server():
		self.position = game_manager.get_random_position()

func _process(delta: float) -> void:
	shadow.global_position = position + Vector2(0, 20)
	if multiplayer.is_server() and is_alive:
		_check_border()
		_try_shoot()
		_manage_weapon_heat(delta)

func _physics_process(delta: float) -> void:
	if multiplayer.is_server() and is_alive:
		_move(delta)

@rpc("any_peer", "call_local", "reliable")
func set_player_name(new_name: String) -> void:
	player_name = new_name

@rpc("authority", "call_local", "reliable")
func play_shoot_sfx() -> void:
	audio_player.stream = PLANE_SHOOT
	audio_player.play()

func _try_shoot() -> void:
	if not input.shoot_input:
		return
	if Time.get_unix_time_from_system() - last_shoot_time < shoot_rate:
		return
	if cur_weapon_heat >= max_weapon_heat:
		return
	
	last_shoot_time = Time.get_unix_time_from_system()
	
	var proj : Projectile = projectile_scene.instantiate()
	proj.position = muzzle.global_position
	proj.rotation = rotation + deg_to_rad(randf_range(-2, 2))
	proj.owner_id = player_id
	
	get_tree().get_current_scene().get_node("Network/SpawnedNodes").add_child(proj, true)
	play_shoot_sfx.rpc()
	
	cur_weapon_heat += weapon_heat_increase_rate
	cur_weapon_heat = clamp(cur_weapon_heat, 0.0, max_weapon_heat)

func _manage_weapon_heat(delta) -> void:
	if cur_weapon_heat < max_weapon_heat and not cur_weapon_heat == 0:
		cur_weapon_heat -= weapon_heat_cool_rate * delta
		if cur_weapon_heat < 0:
			cur_weapon_heat = 0.0
	elif weapon_heat_waiting:
		return
	else:
		weapon_heat_waiting = true
		await get_tree().create_timer(weapon_heat_cap_wait_time).timeout
		weapon_heat_waiting = false
		cur_weapon_heat -= weapon_heat_cool_rate * delta

@rpc("authority", "call_local", "reliable")
func take_damage_fx() -> void:
	audio_player.stream = PLANE_HIT 
	audio_player.play()
	
	# To me it would make more sense if the hit particle would be emitted by the bullet,
	# at the location of impact.
	hit_particle.emitting = true
	
	if input.is_multiplayer_authority():
		game_manager.camera.shake(0.1, 3.0)
	
	ship_sprite.modulate = Color(1, 0, 0)
	await get_tree().create_timer(0.05).timeout
	ship_sprite.modulate = Color(1, 1, 1)

func take_damage(damage_amount: int, attacker_player_id: int) -> void:
	current_hp -= damage_amount
	last_attacker_id = attacker_player_id
	
	if current_hp <= 0:
		die()
	else:
		take_damage_fx.rpc()

@rpc("authority", "call_local", "reliable")
func die_fx() -> void:
	audio_player.stream = PLANE_EXPLODE
	audio_player.play()
	
	if input.is_multiplayer_authority():
		game_manager.camera.shake(0.1, 3.0)
	

# Definitely needs to be remade so that the plane becomes invisible rather than moved
func die() -> void:
	is_alive = false
	self.position = Vector2(0, 1900)
	respawn_timer.start(2)
	print("Player %s died" % self.player_id)
	game_manager.on_player_die(player_id, last_attacker_id)
	die_fx.rpc()

func respawn() -> void:
	print("Player %s respawning" % self.player_id)
	is_alive = true
	current_hp = max_hp
	throttle = 0.0
	last_attacker_id = 0
	rotation = 0
	self.position = game_manager.get_random_position()

func _move(delta) -> void:
	rotate(input.turn_input * turn_rate * delta)
	
	throttle += input.throttle_input * delta
	throttle = clamp(throttle, 0.0, 1.0)
	
	self.velocity = -transform.y * throttle * max_speed
	
	move_and_slide()

func _check_border() -> void:
	if position.x < game_manager.border_min_x:
		position.x = game_manager.border_max_x
	if position.x > game_manager.border_max_x:
		position.x = game_manager.border_min_x
	if position.y < game_manager.border_min_y:
		position.y = game_manager.border_max_y
	if position.y > game_manager.border_max_y:
		position.y = game_manager.border_min_y

func increase_score(amount: int) -> void:
	score += amount

func _exit_tree():
	game_manager.players.erase(self)
