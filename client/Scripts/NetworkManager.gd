extends Node
class_name NetworkManager

const SOCKET_URL = "ws://127.0.0.1:3000/ws"

var socket: WebSocketPeer = WebSocketPeer.new()
var connected: bool = false
var local_player_id: int = 0

var player_scene: PackedScene = preload("res://Scenes/player.tscn")
var projectile_scene: PackedScene = preload("res://Scenes/projectile.tscn")
@onready var spawned_nodes: Node = $SpawnedNodes
@onready var network_ui: Panel = $NetworkUI

# Local player info
var local_username: String

func _ready() -> void:
	start_client()

func _process(_delta: float) -> void:
	socket.poll()
	var state = socket.get_ready_state()
	
	if state == WebSocketPeer.STATE_OPEN:
		if not connected:
			connected = true
			_connected_to_server()
		
		# Process incoming packets
		while socket.get_available_packet_count() > 0:
			var packet = socket.get_packet()
			var data_str = packet.get_string_from_utf8()
			_handle_server_message(data_str)
			
	elif state == WebSocketPeer.STATE_CLOSED:
		if connected:
			connected = false
			_server_closed()

func start_client() -> void:
	print("Connecting to %s..." % SOCKET_URL)
	var err = socket.connect_to_url(SOCKET_URL)
	if err != OK:
		print("Connection error: %s" % err)
		_connection_failed()

func send_input(input_data: Dictionary) -> void:
	if socket.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return
		
	var json_str = JSON.stringify(input_data)
	socket.send_text(json_str)

func _handle_server_message(json_str: String) -> void:
	var json = JSON.new()
	var error = json.parse(json_str)
	if error != OK:
		print("JSON Parse Error: ", json.get_error_message())
		return

	var msg = json.data
	# Server sends { "type": "...", "data": ... }
	if not (msg is Dictionary and msg.has("type")):
		return

	match msg.type:
		"Identity":
			# { "type": "Identity", "data": { "player_id": 123 } }
			if msg.data.has("player_id"):
				local_player_id = int(msg.data.player_id)
				print("Assigned Player ID: ", local_player_id)
				
		"WorldUpdate":
			# { "type": "WorldUpdate", "data": { "tick": 1, "entities": [...] } }
			if msg.data.has("entities"):
				_handle_world_update(msg.data)
				
		"GameState":
			# { "type": "GameState", "data": { ... } } or "MatchRunning"
			print("Game State Update: ", msg.data)

func _handle_world_update(data: Dictionary) -> void:
	var entities = data.entities
	var projectiles = []
	if data.has("projectiles"):
		projectiles = data.projectiles
	
	var current_player_ids = []
	var current_projectile_ids = []

	for entity_data in entities:
		# entity_data has: id, x, y, rot
		var id = int(entity_data.id)
		current_player_ids.append(id)
		
		if spawned_nodes.has_node(str(id)):
			var player = spawned_nodes.get_node(str(id))
			if player.has_method("update_state"):
				player.update_state(entity_data)
		else:
			print("Spawning player ", id)
			var player = player_scene.instantiate()
			player.name = str(id)
			player.player_id = id
			spawned_nodes.add_child(player, true)
			if player.has_method("update_state"):
				player.update_state(entity_data)

	for proj_data in projectiles:
		# proj_data has: id, owner_id, x, y, rot
		var proj_id = int(proj_data.id)
		current_projectile_ids.append(proj_id)
		var node_name = "proj_%s" % proj_id
		
		if spawned_nodes.has_node(node_name):
			var proj = spawned_nodes.get_node(node_name)
			if proj.has_method("update_state"):
				proj.update_state(proj_data)
		else:
			var proj = projectile_scene.instantiate()
			proj.name = node_name
			proj.projectile_id = proj_id
			proj.owner_id = int(proj_data.owner_id)
			spawned_nodes.add_child(proj, true)
			if proj.has_method("update_state"):
				proj.update_state(proj_data)

	# Despawn missing players/projectiles
	for node in spawned_nodes.get_children():
		if node is Player:
			if node.player_id not in current_player_ids:
				print("Despawning player ", node.player_id)
				node.queue_free()
		elif node is Projectile:
			if node.projectile_id not in current_projectile_ids:
				node.queue_free()

# Client fn
func _connected_to_server() -> void:
	print("Connected to server")
	network_ui.visible = false

# Client fn
func _connection_failed() -> void:
	print("Connection failed")

# Client fn
func _server_closed() -> void:
	print("Server has been closed")
	network_ui.visible = true
	spawned_nodes.get_children().map(func(n): n.queue_free())

func _on_username_input_text_changed(new_text: String) -> void:
	local_username = new_text
