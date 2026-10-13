extends SceneTree

const Capabilities = preload("res://scripts/desktop/capabilities.gd")
const Toolbar = preload("res://ui/companion_profile_toolbar.gd")
const Actions = preload("res://scripts/desktop/actions.gd")

class FakeTransport extends RefCounted:
	var connected := true
	var requests: Array = []
	func request(value: Dictionary) -> void: requests.append(value)
	func cancel_terminal_pending() -> void: pass

class FakeHud extends RefCounted:
	var live := {"dock": {"terminal": {"state": "closed"}, "terminal_visible": false}}
	var notices: Array = []
	var refreshes := 0
	func show_toast(value: String) -> void: notices.append(value)
	func set_terminal(value: Dictionary, _defer := false) -> void: live.dock.terminal = value
	func refresh_terminal() -> void: refreshes += 1

class FakeHost extends Node:
	var selected_id := "remote"
	var entries: Array = []
	var transport := FakeTransport.new()
	var town := {"hud": FakeHud.new()}

func _initialize() -> void:
	var remote := {"id": "remote", "source": "herdr", "remoteMachine": "Build", "supportsObserve": true, "supportsTerminal": false}
	var local := {"id": "local", "source": "herdr", "supportsObserve": true, "supportsTerminal": true}
	assert(Capabilities.can_observe(remote))
	assert(not Capabilities.can_control(remote))
	assert(Capabilities.action(remote) == "observe")
	assert(Capabilities.label(remote) == "Observe")
	assert(Capabilities.can_control(local))
	assert(Capabilities.action(local) == "interact")
	assert(not Capabilities.can_observe({"source": "codex"}))
	assert(not Capabilities.can_observe({"source": "herdr", "supportsObserve": false, "supportsTerminal": false}))
	var toolbar := Toolbar.new()
	var button := Button.new()
	toolbar.record = remote
	toolbar.buttons = {"interact": button}
	var emitted: Array = []
	toolbar.action_requested.connect(func(_id: String, action: String, _payload: Dictionary): emitted.append(action))
	toolbar._activate("interact")
	assert(emitted == ["observe"], "The existing toolbar action must dispatch remote Observe")
	toolbar.record = local
	toolbar._activate("interact")
	assert(emitted.back() == "interact")
	toolbar.free()
	button.free()
	var host := FakeHost.new()
	host.entries = [remote, local]
	var actions := Actions.new()
	actions.host = host
	actions.dispatch("remote", "interact", {})
	assert(host.transport.requests.is_empty(), "Remote input must be rejected before opening a terminal")
	actions.dispatch("remote", "observe", {})
	var opening: Dictionary = host.transport.requests.back()
	assert(opening.type == "terminal.open" and not opening.control and not opening.takeover)
	actions.response(opening, {"ok": true, "result": {"token": "test-observer"}})
	var frame := {"id": "remote", "viewGeneration": actions.generation, "type": "frame", "seq": 1, "full": true, "width": 80, "height": 24, "bytes": "SGVsbG8="}
	actions.events([frame])
	assert(host.town.hud.live.dock.terminal.text.begins_with("Hello"))
	assert(host.town.hud.refreshes == 1)
	actions.suspend_terminal("disconnected")
	actions.dispatch("remote", "terminal_reconnect", {})
	assert(not host.transport.requests.back().control, "Reconnect must stay read-only")
	var stale := frame.duplicate()
	stale.viewGeneration = actions.generation - 1
	actions.events([stale])
	assert(host.town.hud.refreshes == 1, "An old stream must not update a new terminal view")
	host.selected_id = "local"
	actions.dispatch("local", "interact", {})
	assert(host.transport.requests.back().control, "Local input remains supported")
	actions.close_terminal()
	assert(actions.terminal_token.is_empty())
	host.free()
	print("Godot remote Observe checks: pass")
	quit(0)
