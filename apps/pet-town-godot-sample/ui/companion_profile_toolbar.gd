extends PanelContainer

signal action_requested(id: String, action: String, payload: Dictionary)

const Capabilities = preload("res://scripts/desktop/capabilities.gd")
const Style = preload("res://ui/hud_style.gd")

var record: Dictionary = {}
var terminal_mode := false
var portrait: TextureRect
var heading: Label
var status: Label
var action_scroll: ScrollContainer
var action_row: HBoxContainer
var hint_layer: Control
var buttons: Dictionary = {}
var hints: Dictionary = {}

func _ready() -> void:
	var chrome: Dictionary = preload("res://ui/companion_toolbar_chrome.gd").build(
		self, Callable(self, "_activate"), Callable(self, "_show_hint"), Callable(self, "_sync_hint"))
	portrait = chrome.portrait
	heading = chrome.heading
	status = chrome.status
	action_scroll = chrome.action_scroll
	action_row = chrome.action_row
	hint_layer = chrome.hint_layer
	buttons = chrome.buttons
	hints = chrome.hints
	action_scroll.get_h_scroll_bar().value_changed.connect(_on_action_scroll_changed)
	get_viewport().size_changed.connect(layout)
	layout()
	_update_record()

func layout() -> void:
	var viewport := get_viewport_rect().size
	var width := minf(560.0, maxf(0.0, viewport.x))
	position = Vector2((viewport.x - width) * 0.5, 0)
	size = Vector2(width, minf(76.0, maxf(0.0, viewport.y)))
	action_scroll.custom_minimum_size.x = minf(248, maxf(44, width - 176))
	_refresh_hints()

func set_record(value: Dictionary) -> void:
	record = value
	if is_node_ready():
		_update_record()

func set_terminal_mode(active: bool) -> void:
	# Presentation only: do not reserve world width, close a session, or change focus.
	terminal_mode = active
	visible = _has_companion() and not terminal_mode
	if active:
		for hint in hints.values():
			hint.hide()

func _has_companion() -> bool:
	return not str(record.get("id", "")).is_empty() and not _is_mayor(record)

func _update_record() -> void:
	var available := _has_companion()
	visible = available and not terminal_mode
	if not available:
		for button in buttons.values():
			button.disabled = true
		return
	var label := Style.text_or(record.get("label", record.get("displayLabel")), "Companion")
	heading.text = label
	portrait.texture = Style.portrait_texture(record)
	var source := str(record.get("source", "")).strip_edges()
	var status_text := _status_text(str(record.get("status", "")))
	var source_text := "Herdr" if source.to_lower() == "herdr" else source.capitalize() if not source.is_empty() else "Agent"
	status.text = "%s · %s" % [source_text, status_text]
	_update_action("focus", "Select remote pane" if record.get("remoteMachine") else "Open in Herdr" if source.to_lower() == "herdr" else "Open agent", true, false)
	_update_action("interact", Capabilities.label(record), Capabilities.can_observe(record), false)
	var controlled := bool(record.get("controlled", false))
	var first_person := bool(record.get("firstPerson", false))
	_update_action("control", "Release control" if controlled else "Control companion", true, controlled)
	_update_action("camera", "Third person" if first_person else "First person", true, first_person)
	_update_action("leave", "Leave companion", true, false)
	layout()

func _update_action(key: String, label: String, enabled: bool, selected: bool) -> void:
	var button: Button = buttons[key]
	button.disabled = not enabled
	button.accessibility_name = label if enabled else "%s (Herdr only)" % label
	button.accessibility_description = _description(key, selected, enabled)
	button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND if enabled else Control.CURSOR_ARROW
	for state in ["normal", "hover", "pressed", "disabled"]:
		var fill := "e4ebd4" if selected else "fff9e9"
		var border := "aebc8b" if selected else "dec49a"
		if state == "hover" and not selected and enabled:
			fill = "ffffff"
		var box := Style.panel(fill, 10, border, false)
		box.set_content_margin_all(0)
		if state == "disabled":
			box.bg_color.a = 0.56
			box.border_color.a = 0.56
		button.add_theme_stylebox_override(state, box)
	var hint: Label = hints[key]
	hint.text = label if enabled else "%s · Herdr only" % label
	hint.size = hint.get_combined_minimum_size()
	_position_hint(key)

func _show_hint(key: String) -> void:
	var hint: Label = hints[key]
	hint.size = hint.get_combined_minimum_size()
	_position_hint(key)
	hint.show()

func _position_hint(key: String) -> void:
	if not is_instance_valid(buttons.get(key)) or not is_instance_valid(hints.get(key)):
		return
	var hint: Label = hints[key]
	var bounds: Vector2 = get_viewport_rect().size
	var button_rect: Rect2 = buttons[key].get_global_rect()
	var left := clampf(button_rect.position.x + (button_rect.size.x - hint.size.x) * 0.5, 8.0, maxf(8.0, bounds.x - hint.size.x - 8.0))
	var top: float = button_rect.position.y + button_rect.size.y + 6.0
	hint.position = Vector2(left - global_position.x, top - global_position.y)

func _on_action_scroll_changed(_value: float) -> void:
	call_deferred("_refresh_hints")

func _refresh_hints() -> void:
	if not is_visible_in_tree():
		return
	for key in hints:
		_sync_hint(key)
		if hints[key].visible:
			_position_hint(key)

func _sync_hint(key: String) -> void:
	hints[key].visible = buttons[key].is_hovered()

func _activate(key: String) -> void:
	var id := str(record.get("id", ""))
	if id.is_empty() or buttons[key].disabled:
		return
	match key:
		"focus": action_requested.emit(id, "focus", {})
		"interact": action_requested.emit(id, Capabilities.action(record), {})
		"control": action_requested.emit(id, "control", {})
		"camera": action_requested.emit(id, "camera", {"first_person": not bool(record.get("firstPerson", false))})
		"leave": action_requested.emit(id, "leave", {})

func _description(key: String, selected: bool, enabled: bool) -> String:
	if not enabled:
		return "Available only for Herdr companions."
	match key:
		"focus": return "Select its remote pane; open Herdr and select that machine to view it." if record.get("remoteMachine") else "Open this companion using Pet Town's existing scoped focus action."
		"interact": return "Observe this remote terminal without taking control of its input." if record.get("remoteMachine") else "Deliberately request terminal input; existing ownership checks still apply."
		"control": return "Release local companion control." if selected else "Take local control of this companion."
		"camera": return "Switch to third person." if selected else "Switch to first person."
		"leave": return "Return to exploring. The agent continues running."
	return ""

func _status_text(value: String) -> String:
	return {
		"working": "Working",
		"blocked": "Needs you",
		"done": "Completed",
		"completed": "Completed",
		"idle": "Ready",
	}.get(value, "Status unavailable")

func _is_mayor(value: Dictionary) -> bool:
	return bool(value.get("isMayor", false)) or str(value.get("source", "")).to_lower() == "mayor" or str(value.get("id", "")) == "pet-town-mayor"
