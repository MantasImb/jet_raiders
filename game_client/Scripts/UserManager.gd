extends Node
class_name UserManager

var network : NetworkManager

const PROFILE_PATH = "user://guest_profile.json"
const DEFAULT_DISPLAY_NAME = "Pilot"

var local_player_id: int
var auth_token: String

# Local player info
var local_username: String
var guest_id: String = "0"

@onready var username_input: LineEdit = $"../NetworkUI/VBoxContainer/UsernameInput"

func _ready() -> void:
	load_or_create_profile()

func login() -> void:
	var http := HTTPRequest.new()
	add_child(http)
	
	# Signal once the request completes
	http.request_completed.connect(
		func(result, response_code, _headers, body):
			http.queue_free()

			if result != HTTPRequest.RESULT_SUCCESS:
				push_error("Request failed")
				print(result)
				return

			if response_code != 200:
				push_error("HTTP error %d" % response_code)
				print(response_code)
				return

			var text : String = body.get_string_from_utf8()

			var parsed : Variant = JSON.parse_string(text)
			if typeof(parsed) != TYPE_DICTIONARY:
				push_error("Invalid JSON response")
				return

			var json: Dictionary = parsed
			if !json.has("session_token"):
				push_error("Missing session_token")
				return

			auth_token = json.session_token
	)
	
	if !guest_id:
		push_error("guest_id undefined")
		return
	
	if !is_display_name_valid():
		push_error("Bad username: " + local_username)
		return
	
	var payload = {
		"guest_id": guest_id,
		"display_name": local_username
	}

	var headers = [
        "Content-Type: application/json"
	]
	
	var url = NetworkManager.HEAD_BASE_URL + "/guest/login"
	#var url = "http://127.0.0.1:3000/guest/login"
	
	print("Trying to log in on url: ", url)
	print(payload)

	http.request(
		url,
		headers,
		HTTPClient.METHOD_POST,
		JSON.stringify(payload)
	)

func load_or_create_profile() -> void:
	var data: Dictionary = {}

	if FileAccess.file_exists(PROFILE_PATH):
		var file = FileAccess.open(PROFILE_PATH, FileAccess.READ)
		if file:
			var text = file.get_as_text()
			file.close()
			var parsed = JSON.parse_string(text)
			if typeof(parsed) == TYPE_DICTIONARY:
				data = parsed

	if data.has("guest_id"):
		guest_id = str(data.guest_id)
	else:
		guest_id = _generate_guest_id()

	if data.has("display_name"):
		local_username = str(data.display_name)
	else:
		local_username = DEFAULT_DISPLAY_NAME

	username_input.text = local_username
	_save_profile()

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

func _generate_guest_id() -> String:
	var rng = RandomNumberGenerator.new()
	rng.randomize()
	var parts: Array = []

	# Generate a 128-bit hex string with four 32-bit chunks.
	for i in range(4):
		parts.append("%08x" % rng.randi())

	return "".join(parts)

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
