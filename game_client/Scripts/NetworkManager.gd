extends Node
class_name NetworkManager

var user: UserManager

const HEAD_BASE_URL= "http://127.0.0.1:3000"
# will be assigned when the head server provides the game server URL
var game_server_url: String
var lobby_id: String

var game_socket: WebSocketPeer = WebSocketPeer.new()
var connected: bool = false

var player_scene: PackedScene = preload("res://Scenes/player.tscn")
var projectile_scene: PackedScene = preload("res://Scenes/projectile.tscn")
@onready var spawned_nodes: Node = $SpawnedNodes
@onready var network_ui: Panel = $NetworkUI


func _ready() -> void:
	pass
	# Load or create a local guest profile before connecting.
	#start_client()
	
func _process(_delta: float) -> void:
	return
	
	game_socket.poll()
	var state = game_socket.get_ready_state()
	
	if state == WebSocketPeer.STATE_OPEN:
		if not connected:
			connected = true
			_connected_to_server()
		
		# Process incoming packets
		while game_socket.get_available_packet_count() > 0:
			var packet = game_socket.get_packet()
			var data_str = packet.get_string_from_utf8()
			_handle_server_message(data_str)
			
	elif state == WebSocketPeer.STATE_CLOSED:
		if connected:
			connected = false
			_server_closed()

func start_client() -> void:
	print("Connecting to %s..." % game_server_url)
	var err = game_socket.connect_to_url(game_server_url)
	if err != OK:
		print("Connection error: %s" % err)
		_connection_failed()

func send_input(input_data: Dictionary) -> void:
	if game_socket.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return

	# Wrap input in the structured message expected by the server.
	var message = {
		"type": "Input",
		"data": input_data
	}
	var json_str = JSON.stringify(message)
	game_socket.send_text(json_str)

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

func _connected_to_server() -> void:
	print("Connected to server")
	network_ui.visible = false
	_send_join()

func _connection_failed() -> void:
	print("Connection failed")

func _server_closed() -> void:
	print("Server has been closed")
	network_ui.visible = true
	spawned_nodes.get_children().map(func(n): n.queue_free())


func _send_join() -> void:
	if game_socket.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return

	# Send a minimal guest join payload for persistence.
	var message = {
		"type": "Join",
		"data": {
			"guest_id": user.guest_id,
			"display_name": user.local_username
		}
	}
	var json_str = JSON.stringify(message)
	game_socket.send_text(json_str)
