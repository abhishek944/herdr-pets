extends RefCounted
const Capabilities = preload("capabilities.gd")
var host: Node
var terminal_token := ""
var generation := 0
var terminal_control_requested := false
var terminal := preload("terminal.gd").new()

func dispatch(id: String, action: String, payload: Dictionary) -> void:
	match action:
		"follow": host.follow(id,bool(payload.get("keep_panel",false)))
		"leave": host.leave()
		"control": host.set_control(bool(payload.get("active",not host.controlled)))
		"camera":
			host.town.rig.first_person=bool(payload.get("first",payload.get("first_person",not host.town.rig.first_person)))
			host._refresh_selection()
		"pet": host.change_pet(id,str(payload.get("petId",payload.get("pet",payload.get("id","")))))
		"focus": _action("focusAgent",{"id":id})
		"observe","terminal_watch","interact","terminal_takeover":
			_open_terminal(id, action not in ["observe","terminal_watch"], action=="terminal_takeover")
		"terminal_reconnect":
			if host.town.hud.live.dock.terminal.get("state", "closed") in ["paused", "disconnected", "error", "stale", "conflict"]: _open_terminal(id, terminal_control_requested)
		"terminal_disconnect":
			if id == host.selected_id: suspend_terminal("disconnected")
		"terminal_release": close_terminal()
		"terminal_input": _terminal({"type":"terminal.input","text":_input(payload)})
		"terminal_live": _terminal({"type":"terminal.live"})
		"terminal_resize","terminal_scroll","terminal_mouse":
			var command:=payload.duplicate()
			command.type=action.replace("_",".")
			_terminal(command)
		"mayor_invoke","mayor_call": _action("invokeMayor")
		"mayor_start_talking": _action("mayorTalk",{"active":true})
		"mayor_stop_talking": _action("mayorTalk",{"active":false})
		"mayor_live_start": _action("invokeMayor")
		"mayor_live_stop": _action("stopMayor")
		"mayor_talk": _action("mayorTalk",{"active":bool(payload.get("active",false))})
		"mayor_stop": _action("stopMayor")
		"mayor_retry": _action("mayorRetryVoice")
		"mayor_settings": _action("openMayorSettings")
		"update_check","update_download","update_install": host.transport.request({"type":"update.action","action":action})
		"mayor_mode": _action("setMayorMode",{"mode":str(payload.get("mode","firstmate"))})

func _open_terminal(id: String, control: bool, takeover := false) -> void:
	if id.is_empty() or id != host.selected_id: return
	var record: Dictionary = {}
	for entry in host.entries:
		if str(entry.get("id", "")) == id: record = entry
	if not Capabilities.can_observe(record) or (control and not Capabilities.can_control(record)):
		host.town.hud.show_toast("Remote terminals support Observe only." if record.get("remoteMachine") else "This companion has no available terminal.")
		return
	var view := _cached_view()
	close_terminal(true)
	terminal_control_requested = control
	view.merge({"state":"connecting", "control":false, "viewGeneration":generation, "message":"Connecting to the existing terminal."}, true)
	terminal.state = view
	host.town.hud.set_terminal(view)
	host.transport.request({"type":"terminal.open","id":id,"control":control,"takeover":takeover,"cols":80,"rows":24,"viewGeneration":generation})

func _cached_view() -> Dictionary:
	var view: Dictionary = host.town.hud.live.dock.terminal.duplicate(true)
	for field in ["rawFrame", "error", "reconnectFailed"]: view.erase(field)
	return view

func _action(name: String, payload: Dictionary={}) -> void:
	var request:=payload.duplicate()
	request.type="action"
	request.action=name
	host.transport.request(request)

func _terminal(command: Dictionary) -> void:
	if terminal_token.is_empty() or (command.get("type","")=="terminal.input" and str(command.get("text","")).is_empty()): return
	host.transport.request({"type":"terminal.send","session":terminal_token,"command":command,"viewGeneration":generation})

func release_hidden_ui(open := false) -> void:
	if not open and host.town.hud.visible and host.town.hud.root.visible: return
	if host.town.hud.live.dock.terminal_visible or terminal.state.get("state", "closed") != "closed" or not terminal_token.is_empty(): close_terminal()

func suspend_terminal(state := "paused") -> void:
	var view := _cached_view()
	if view.get("state", "closed") == "closed": return
	if view.get("state") in ["paused", "disconnected", "error", "stale"] and terminal_token.is_empty(): return
	close_terminal(true)
	view.merge({"state":state, "control":false, "viewGeneration":generation, "message":""}, true)
	terminal.state = view
	host.town.hud.set_terminal(view)

func close_terminal(keep_view := false) -> void:
	generation+=1
	host.transport.cancel_terminal_pending()
	if host.transport.connected: host.transport.request({"type":"terminal.close","viewGeneration":generation})
	terminal_token=""
	terminal=preload("terminal.gd").new()
	terminal.state.viewGeneration=generation
	if not keep_view:
		terminal_control_requested = false
		host.town.hud.set_terminal(terminal.state)

func response(request: Dictionary, data: Dictionary) -> void:
	var kind := str(request.get("type", ""))
	if kind.begins_with("terminal.") and int(request.get("viewGeneration", -1)) != generation: return
	if kind == "terminal.close":
		if not data.get("ok", false): host.town.hud.show_toast(str(data.get("error", "Terminal release could not complete.")))
		return
	if kind == "terminal.open" and request.get("id", "") != host.selected_id: return
	if not data.get("ok",false):
		var message:=str(data.get("error","Desktop action could not complete."))
		host.town.hud.show_toast(message)
		if kind.begins_with("terminal."):
			var view := _cached_view()
			close_terminal(true)
			view.merge({"state":"error", "control":false, "error":message, "message":message, "reconnectFailed":kind=="terminal.open", "viewGeneration":generation}, true)
			terminal.state = view
			host.town.hud.set_terminal(view)
		return
	if request.get("type","")=="terminal.open":
		if int(request.get("viewGeneration",-1))!=generation or request.get("id","")!=host.selected_id: return
		terminal_token=str(data.get("result",{}).get("token",""))

func events(data: Array) -> void:
	var changed := false
	for event_data in data:
		if event_data is Dictionary and int(event_data.get("viewGeneration",-1))==generation and event_data.get("id","")==host.selected_id:
			# Consume every ordered delta; publish only the final view in this batch.
			host.town.hud.set_terminal(terminal.event(event_data), true)
			changed = true
	if changed: host.town.hud.refresh_terminal()

func _input(payload: Dictionary) -> String:
	if payload.has("text"):
		var text:=str(payload.text)
		if payload.get("paste",false):
			if text.contains("\u001b[200~") or text.contains("\u001b[201~"):
				host.town.hud.show_toast("Clipboard text contains paste controls. Nothing was pasted.")
				return ""
			text="\u001b[200~"+text+"\u001b[201~"
			if text.to_utf8_buffer().size()>48000:
				host.town.hud.show_toast("This paste exceeds 48 KB. Use a smaller selection or Herdr.")
				return ""
		return text
	var key:=int(payload.get("keycode",0))
	var unicode:=int(payload.get("unicode",0))
	var special:={KEY_ENTER:"\r",KEY_KP_ENTER:"\r",KEY_BACKSPACE:"\u007f",KEY_TAB:"\t",KEY_ESCAPE:"\u001b",KEY_UP:"\u001b[A",KEY_DOWN:"\u001b[B",KEY_RIGHT:"\u001b[C",KEY_LEFT:"\u001b[D",KEY_HOME:"\u001b[H",KEY_END:"\u001b[F",KEY_DELETE:"\u001b[3~",KEY_PAGEUP:"\u001b[5~",KEY_PAGEDOWN:"\u001b[6~"}
	var text: String=special.get(key,String.chr(unicode) if unicode>0 else "")
	if payload.get("ctrl",false) and key>=KEY_A and key<=KEY_Z: text=String.chr(key-KEY_A+1)
	if payload.get("alt",false) and not text.is_empty(): text="\u001b"+text
	return text
