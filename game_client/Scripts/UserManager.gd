extends Node
class_name UserManager

signal authenticated(session_token: String)

var network : NetworkManager

const PROFILE_PATH = "user://guest_profile.json"
const DEFAULT_DISPLAY_NAME = "Pilot"

var local_player_id: String
var auth_token: String
var is_authenticated: bool = false
var authenticating: bool = false

# Local player info
var local_username: String
# Keep guest_id as string at the JSON boundary to avoid large integer precision loss.
var guest_id: String = ""

@onready var username_input: LineEdit = $"../NetworkUI/VBoxContainer/UsernameInput"

func _ready() -> void:
	load_or_create_profile()

func login() -> void:
	if guest_id.is_empty():
		push_error("guest_id undefined")
		return

	authenticating = true
	var http := HTTPRequest.new()
	add_child(http)
	
	# Signal once the request completes
	http.request_completed.connect(
		func(result, response_code, _headers, body):
			http.queue_free()

			if result != HTTPRequest.RESULT_SUCCESS:
				push_error("Request failed")
				print(result)
				authenticating = false
				return

			if response_code != 200:
				push_error("HTTP error %d" % response_code)
				print(response_code)
				authenticating = false
				return

			var text : String = body.get_string_from_utf8()

			var parsed : Variant = JSON.parse_string(text)
			if typeof(parsed) != TYPE_DICTIONARY:
				push_error("Invalid JSON response")
				authenticating = false
				return

			var json: Dictionary = parsed
			if !json.has("session_token"):
				push_error("Missing session_token")
				authenticating = false
				return

			auth_token = json.session_token
			is_authenticated = true
			authenticating = false
			# Notify listeners that auth is ready for downstream connections.
			emit_signal("authenticated", auth_token)
	)
	
	if !is_display_name_valid():
		push_error("Bad username: " + local_username)
		local_username = DEFAULT_DISPLAY_NAME
	
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

	var request_error = http.request(
		url,
		headers,
		HTTPClient.METHOD_POST,
		JSON.stringify(payload)
	)
	if request_error != OK:
		push_error("Failed to start login request: %s" % request_error)
		http.queue_free()
		authenticating = false

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
		guest_id = str(data.guest_id).strip_edges()
		print("Guest_id: ", guest_id)
	else:
		_init_guest_id()

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

func _init_guest_id() -> void:
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
			if !json.has("guest_id"):
				push_error("Missing guest_id")
				return

			guest_id = str(json.guest_id).strip_edges()
			if guest_id.is_empty():
				push_error("Empty guest_id")
				return

			print("Acquired guest id: ", guest_id)
			_save_profile()
	)
	
	
	if !is_display_name_valid():
		push_error("Bad username: " + local_username)
		local_username = DEFAULT_DISPLAY_NAME
			
	var payload = {
		"display_name": local_username
	}

	var headers = [
        "Content-Type: application/json"
	]
	
	var url = NetworkManager.HEAD_BASE_URL + "/guest/init"
	#var url = "http://127.0.0.1:3000/guest/login"
	
	print("Trying to init user on url: ", url)
	print(payload)

	var request_error = http.request(
		url,
		headers,
		HTTPClient.METHOD_POST,
		JSON.stringify(payload)
	)
	if request_error != OK:
		push_error("Failed to start guest init request: %s" % request_error)
		http.queue_free()

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
