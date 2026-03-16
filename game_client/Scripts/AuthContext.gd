extends Node
class_name AuthContext

const PROFILE_PATH = "user://guest_profile.json"
const DEFAULT_DISPLAY_NAME = "Pilot"
const MAX_U64 = "18446744073709551615"

var local_player_id: String
var auth_token: String
var local_username: String
# Keep guest_id as string at the JSON boundary to avoid large integer precision loss.
var guest_id: String = ""

@onready var username_input: LineEdit = $"../NetworkUI/VBoxContainer/UsernameInput"

func load_profile_data() -> Dictionary:
	var data: Dictionary = {}

	if FileAccess.file_exists(PROFILE_PATH):
		var file = FileAccess.open(PROFILE_PATH, FileAccess.READ)
		if file:
			var text = file.get_as_text()
			file.close()
			var parsed = JSON.parse_string(text)
			if typeof(parsed) == TYPE_DICTIONARY:
				data = parsed

	return data

func apply_profile_data(data: Dictionary) -> void:
	if data.has("display_name"):
		local_username = str(data.display_name)
	else:
		local_username = DEFAULT_DISPLAY_NAME

	if data.has("guest_id"):
		var stored_guest_id := str(data.guest_id).strip_edges()
		if is_guest_id_valid(stored_guest_id):
			guest_id = stored_guest_id
		else:
			push_warning("Invalid stored guest_id. Fetching a new guest identity.")
			guest_id = ""
	else:
		guest_id = ""

func finish_profile_setup() -> void:
	username_input.text = local_username
	_save_profile()

func normalize_display_name_for_auth() -> void:
	if not is_display_name_valid():
		push_warning("Bad username: %s" % local_username)
		local_username = DEFAULT_DISPLAY_NAME
		username_input.text = local_username
		_save_profile()

func has_valid_guest_id() -> bool:
	return is_guest_id_valid(guest_id)

func _save_profile() -> void:
	var file = FileAccess.open(PROFILE_PATH, FileAccess.WRITE)
	if not file:
		return

	# Persist a minimal guest profile for future sessions.
	var data = {
		"guest_id": guest_id,
		"display_name": local_username
	}
	file.store_string(JSON.stringify(data))
	file.close()

func _on_username_input_text_changed(new_text: String) -> void:
	local_username = new_text
	_save_profile()
	
func is_display_name_valid() -> bool:
	# Normalize whitespace and enforce basic length constraints.
	var trimmed := local_username.strip_edges()
	if trimmed.is_empty():
		return false

	if trimmed.length() < 3 or trimmed.length() > 16:
		return false

	# Allow only simple, readable characters for now.
	var regex := RegEx.new()
	regex.compile("^[A-Za-z0-9 _-]+$")
	return regex.search(trimmed) != null

func is_guest_id_valid(value: String) -> bool:
	var trimmed := value.strip_edges()
	if trimmed.is_empty():
		return false

	# guest_id must be a positive numeric identifier issued by auth.
	var regex := RegEx.new()
	regex.compile("^(0|[1-9][0-9]*)$")
	if regex.search(trimmed) == null:
		return false

	if trimmed == "0":
		return false

	if trimmed.length() < MAX_U64.length():
		return true

	if trimmed.length() > MAX_U64.length():
		return false

	return trimmed <= MAX_U64
