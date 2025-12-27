extends Node
class_name NetworkManager

const MAX_CLIENTS: int = 4
@onready var network_ui: Panel = $NetworkUI
@onready var ip_input: LineEdit = $NetworkUI/VBoxContainer/IPInput
@onready var port_input: LineEdit = $NetworkUI/VBoxContainer/PortInput

var player_scene: PackedScene = preload("res://Scenes/player.tscn")
@onready var spawned_nodes: Node = $SpawnedNodes

var local_username: String

func _ready() -> void:
	pass
	
func start_host() -> void:
	var peer = ENetMultiplayerPeer.new()
	peer.create_server(int(port_input.text), MAX_CLIENTS)
	multiplayer.multiplayer_peer = peer
	
	multiplayer.peer_connected.connect(_on_player_connected)
	multiplayer.peer_disconnected.connect(_on_player_disconnected)
	
	_on_player_connected(multiplayer.get_unique_id())
	
	network_ui.visible = false
	
func start_client() -> void:
	var peer = ENetMultiplayerPeer.new()
	peer.create_client(ip_input.text, int(port_input.text))
	multiplayer.multiplayer_peer = peer
	
	multiplayer.connected_to_server.connect(_connected_to_server)
	multiplayer.connection_failed.connect(_connection_failed)
	multiplayer.server_disconnected.connect(_server_closed)
	
# Server fn
func _on_player_connected(id: int) -> void:
	print("Player %s connected" % id)
	
	var player: Player = player_scene.instantiate()
	player.name = str(id)
	player.player_id = id
	spawned_nodes.add_child(player, true)
	
# Server fn
func _on_player_disconnected(id: int) -> void:
	print("Player %s disconnected" % id)	
	
	if not spawned_nodes.has_node(str(id)):
		return
	
	spawned_nodes.get_node(str(id)).queue_free()
	
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

func _on_username_input_text_changed(new_text: String) -> void:
	local_username = new_text
