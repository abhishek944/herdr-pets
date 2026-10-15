extends RefCounted
## Observe and input are independent; remote observation never grants input.
static func can_control(record: Dictionary) -> bool:
	return str(record.get("source", "")).to_lower() == "herdr" and not record.get("remoteMachine") and bool(record.get("supportsTerminal", true))

static func can_observe(record: Dictionary) -> bool:
	return str(record.get("source", "")).to_lower() == "herdr" and bool(record.get("supportsObserve", can_control(record)))

static func action(record: Dictionary) -> String:
	return "interact" if can_control(record) else "observe"

static func label(record: Dictionary) -> String:
	return "Observe" if record.get("remoteMachine") else "Terminal"
