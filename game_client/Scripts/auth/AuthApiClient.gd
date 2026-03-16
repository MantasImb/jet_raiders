extends Node
class_name AuthApiClient

func request_guest_init(display_name: String, callback: Callable) -> void:
	_request_json(
		NetworkManager.HEAD_BASE_URL + "/guest/init",
		{
			"display_name": display_name
		},
		callback
	)

func request_guest_login(guest_id: String, display_name: String, callback: Callable) -> void:
	_request_json(
		NetworkManager.HEAD_BASE_URL + "/guest/login",
		{
			"guest_id": guest_id,
			"display_name": display_name
		},
		callback
	)

func _request_json(url: String, payload: Dictionary, callback: Callable) -> void:
	var http := HTTPRequest.new()
	add_child(http)
	http.request_completed.connect(
		func(result: int, response_code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
			http.queue_free()
			callback.call(_build_response(result, response_code, body))
	)

	var request_error := http.request(
		url,
		["Content-Type: application/json"],
		HTTPClient.METHOD_POST,
		JSON.stringify(payload)
	)
	if request_error != OK:
		http.queue_free()
		callback.call({
			"ok": false,
			"code": "request_start_failed",
			"detail": "Failed to start request: %s" % request_error
		})

func _build_response(result: int, response_code: int, body: PackedByteArray) -> Dictionary:
	if result != HTTPRequest.RESULT_SUCCESS:
		return {
			"ok": false,
			"code": "request_failed",
			"detail": "HTTPRequest failed with result %s" % result
		}

	if response_code != 200:
		return {
			"ok": false,
			"code": "http_error",
			"detail": "HTTP error %s" % response_code
		}

	var parsed: Variant = JSON.parse_string(body.get_string_from_utf8())
	if typeof(parsed) != TYPE_DICTIONARY:
		return {
			"ok": false,
			"code": "invalid_json",
			"detail": "Response body was not a JSON object"
		}

	return {
		"ok": true,
		"json": parsed
	}
