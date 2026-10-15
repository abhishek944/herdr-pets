extends Node
signal snapshot_received(data: Dictionary)
signal response_received(request: Dictionary, result: Dictionary)
signal disconnected
var peer := StreamPeerTCP.new()
var token := ""
var port := 0
var connected := false
var buffer := PackedByteArray()
var pending: Array[Dictionary] = []
# A single coalesced close barrier has reserved capacity ahead of ordinary work.
var terminal_close: Dictionary = {}
var in_flight: Dictionary = {}
var sequence := 0
var elapsed := 0.0
var retry := 0.0
var focus := false
var window_serial := 0
var desktop_pid := 0
var parent_check := 0.0
var last_contact := 0
var last_request_at := 0

func _ready() -> void:
	port = int(OS.get_environment("PET_TOWN_BRIDGE_PORT"))
	token = OS.get_environment("PET_TOWN_BRIDGE_TOKEN")
	desktop_pid = int(OS.get_environment("PET_TOWN_DESKTOP_PID"))
	# Child processes and external commands must not inherit the capability.
	OS.unset_environment("PET_TOWN_BRIDGE_TOKEN")
	OS.unset_environment("PET_TOWN_BRIDGE_PORT")
	OS.unset_environment("PET_TOWN_DESKTOP_PID")
	var dock_menu := preload("dock_menu.gd").new()
	dock_menu.transport=self
	add_child(dock_menu)
	focus = DisplayServer.window_is_focused()
	last_contact=Time.get_ticks_msec()

func _process(delta: float) -> void:
	parent_check -= delta
	if desktop_pid > 0 and not connected and parent_check <= 0:
		parent_check = 1.0
		# OS.is_process_running accepts only Godot-spawned child PIDs on Unix.
		var alive: bool=OS.execute("/bin/kill",["-0",str(desktop_pid)],[],false)==0 if FileAccess.file_exists("/bin/kill") else Time.get_ticks_msec()-last_contact<30000
		if not alive:
			get_tree().quit()
			return
	if port <= 0 or token.is_empty(): return
	retry -= delta
	peer.poll()
	var status := peer.get_status()
	if status == StreamPeerTCP.STATUS_NONE or status == StreamPeerTCP.STATUS_ERROR:
		if connected: _lost()
		if retry <= 0:
			retry = 2.0
			peer.connect_to_host("127.0.0.1", port)
		return
	if status != StreamPeerTCP.STATUS_CONNECTED: return
	if not connected: _send({"type":"poll", "focused":focus})
	connected = true
	last_contact=Time.get_ticks_msec()
	elapsed = (Time.get_ticks_msec()-last_request_at)/1000.0
	var count := peer.get_available_bytes()
	if count > 0:
		var received := peer.get_data(count)
		if received[0] != OK:
			_lost()
			return
		buffer.append_array(received[1])
		if buffer.size() > 48_000_000:
			_lost()
			return
		_read_lines()
	if request_timed_out(in_flight, elapsed):
		_lost()
		return
	if in_flight.is_empty():
		if not terminal_close.is_empty():
			var close := terminal_close
			terminal_close = {}
			_send(close)
		else:
			var next := -1
			for i in range(pending.size()):
				if str(pending[i].get("type", "")).begins_with("terminal."):
					next = i
					break
			if next >= 0: _send(pending.pop_at(next))
			elif not pending.is_empty(): _send(pending.pop_front())
			elif elapsed >= 0.35: _send({"type":"poll", "focused":focus})

func request(data: Dictionary) -> bool:
	if not connected:
		response_received.emit(data, {"ok":false,"error":"Open the town from Pet Town desktop to connect live companions."})
		return false
	if data.get("type", "") == "terminal.close":
		terminal_close = data.duplicate(true)
		return true
	if pending.size() >= 32:
		response_received.emit(data, {"ok":false,"error":"The desktop connection is busy. Try again."})
		return false
	if str(data.get("type", "")) == "terminal.send" and not pending.is_empty():
		if _merge_terminal_input(data): return true
	pending.append(data.duplicate(true))
	return true

## Fold rapid keystrokes into one queued write so bursts cross the bridge once.
func _merge_terminal_input(data: Dictionary) -> bool:
	var command: Dictionary = data.get("command", {})
	if str(command.get("type", "")) != "terminal.input": return false
	var last: Dictionary = pending[pending.size() - 1]
	if str(last.get("type", "")) != "terminal.send": return false
	if str(last.get("session", "")) != str(data.get("session", "")): return false
	var prior: Dictionary = last.get("command", {})
	if str(prior.get("type", "")) != "terminal.input": return false
	var combined := str(prior.get("text", "")) + str(command.get("text", ""))
	if combined.is_empty() or combined.to_utf8_buffer().size() > 32768: return false
	prior["text"] = combined
	return true

func _send(data: Dictionary) -> void:
	if data.get("type","")=="poll": data.focused=focus
	sequence += 1
	in_flight = data.duplicate(true)
	data.token = token
	data.request = sequence
	elapsed = 0.0
	last_request_at=Time.get_ticks_msec()
	if peer.put_data((JSON.stringify(data)+"\n").to_utf8_buffer()) != OK: _lost()

func _read_lines() -> void:
	var newline := buffer.find(10)
	while newline >= 0:
		var value = JSON.parse_string(buffer.slice(0,newline).get_string_from_utf8())
		buffer = buffer.slice(newline+1)
		if value is Dictionary and int(value.get("request",-1)) == sequence:
			var request_data := in_flight
			in_flight = {}
			if request_data.get("type","") == "poll" and value.get("ok",false):
				var data: Dictionary=value.get("result",{})
				var next_serial:=int(data.get("windowSerial",window_serial))
				if next_serial!=window_serial:
					window_serial=next_serial
					DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)
					DisplayServer.window_move_to_foreground()
				snapshot_received.emit(data)
			else: response_received.emit(request_data,value)
			if str(request_data.get("type", "")).begins_with("terminal."): elapsed = 99.0
		newline = buffer.find(10)

func _lost() -> void:
	peer.disconnect_from_host()
	connected = false
	buffer.clear()
	pending.clear()
	terminal_close.clear()
	if not in_flight.is_empty():
		response_received.emit(in_flight,{"ok":false,"error":"Connection ended. Check Herdr before retrying terminal input."})
	in_flight = {}
	disconnected.emit()

func _notification(what: int) -> void:
	if what not in [NOTIFICATION_APPLICATION_FOCUS_IN,NOTIFICATION_APPLICATION_FOCUS_OUT]: return
	focus = what==NOTIFICATION_APPLICATION_FOCUS_IN
	if connected:
		pending=pending.filter(func(data: Dictionary) -> bool: return data.get("type","")!="poll")
		pending.push_front({"type":"poll","focused":focus})

func _exit_tree() -> void:
	peer.disconnect_from_host()

func cancel_terminal_pending() -> void:
	pending=pending.filter(func(data: Dictionary) -> bool: return not str(data.get("type","")).begins_with("terminal."))

static func request_timed_out(request_data: Dictionary, seconds: float) -> bool:
	# Remote open resolves the catalog (2s), agent (8s), and validates its
	# profile twice (2s each) plus agent (8s): 22s, with startup headroom.
	var deadline := 30.0 if request_data.get("type", "") == "terminal.open" else 8.0
	return not request_data.is_empty() and seconds > deadline
