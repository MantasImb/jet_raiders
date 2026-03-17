extends Node
class_name WorldSync

var player_scene: PackedScene = preload("res://Scenes/player.tscn")
var projectile_scene: PackedScene = preload("res://Scenes/projectile.tscn")

@onready var spawned_nodes: Node = $"../SpawnedNodes"

func apply_world_update(data: Dictionary) -> void:
	var entities: Array = []
	if data.has("entities") and data.entities is Array:
		entities = data.entities
	var projectiles: Array = []
	if data.has("projectiles"):
		projectiles = data.projectiles

	var current_player_ids: Array = []
	var current_projectile_ids: Array[int] = []

	# Update existing player nodes or spawn newly observed players.
	for entity_item in entities:
		if not (entity_item is Dictionary and entity_item.has("id")):
			push_warning("Skipping malformed entity in world update")
			continue

		var entity_data: Dictionary = entity_item
		# entity_data has: id, x, y, rot
		var id := str(entity_data.id)
		if id.strip_edges().is_empty():
			push_warning("Skipping entity with blank id in world update")
			continue
		current_player_ids.append(id)

		if spawned_nodes.has_node(id):
			var player := spawned_nodes.get_node(id)
			if player.has_method("update_state"):
				player.update_state(entity_data)
		else:
			print("Spawning player ", id)
			var player := player_scene.instantiate()
			player.name = id
			player.player_id = id
			spawned_nodes.add_child(player, true)
			if player.has_method("update_state"):
				player.update_state(entity_data)

	# Update existing projectile nodes or spawn newly observed projectiles.
	for projectile_item in projectiles:
		if not (
			projectile_item is Dictionary
			and projectile_item.has("id")
			and projectile_item.has("owner_id")
		):
			push_warning("Skipping malformed projectile in world update")
			continue

		var proj_data: Dictionary = projectile_item
		# proj_data has: id, owner_id, x, y, rot
		var proj_id_text := str(proj_data.id).strip_edges()
		var owner_id_text := str(proj_data.owner_id).strip_edges()
		if proj_id_text.is_empty() or owner_id_text.is_empty():
			push_warning("Skipping projectile with blank id or owner_id in world update")
			continue

		var proj_id := int(proj_id_text)
		current_projectile_ids.append(proj_id)
		var node_name := "proj_%s" % proj_id

		if spawned_nodes.has_node(node_name):
			var proj := spawned_nodes.get_node(node_name)
			if proj.has_method("update_state"):
				proj.update_state(proj_data)
		else:
			var proj := projectile_scene.instantiate()
			proj.name = node_name
			proj.projectile_id = proj_id
			proj.owner_id = int(owner_id_text)
			spawned_nodes.add_child(proj, true)
			if proj.has_method("update_state"):
				proj.update_state(proj_data)

	# Remove nodes that are absent from the latest authoritative snapshot.
	for node in spawned_nodes.get_children():
		if node is Player:
			if node.player_id not in current_player_ids:
				print("Despawning player ", node.player_id)
				node.queue_free()
		elif node is Projectile:
			if node.projectile_id not in current_projectile_ids:
				node.queue_free()

func clear_world() -> void:
	# Clear all synchronized entities when the active connection is lost.
	for node in spawned_nodes.get_children():
		node.queue_free()
