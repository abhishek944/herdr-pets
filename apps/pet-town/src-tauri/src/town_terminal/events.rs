use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum TerminalEvent {
    Status {
        state: String,
        control: bool,
        message: String,
    },
    Frame {
        seq: u64,
        width: u16,
        height: u16,
        full: bool,
        bytes: String,
    },
}

pub(super) fn status(channel: &Channel<TerminalEvent>, state: &str, control: bool, message: &str) {
    let _ = channel.send(TerminalEvent::Status {
        state: state.into(),
        control,
        message: message.into(),
    });
}

pub(super) fn frame(value: &Value) -> Option<TerminalEvent> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    if value["type"] != "terminal.frame" || value["encoding"] != "ansi" {
        return None;
    }
    let width = u16::try_from(value["width"].as_u64()?).ok()?;
    let height = u16::try_from(value["height"].as_u64()?).ok()?;
    let bytes = value["bytes"].as_str()?;
    if !(2..=512).contains(&width)
        || !(1..=256).contains(&height)
        || bytes.len() > 1_398_104
        || STANDARD.decode(bytes).ok()?.len() > 1_048_576
    {
        return None;
    }
    Some(TerminalEvent::Frame {
        seq: value["seq"].as_u64()?,
        width,
        height,
        full: value["full"].as_bool()?,
        bytes: bytes.into(),
    })
}

pub(super) fn closed_state(reason: &str) -> (&'static str, &'static str) {
    let reason = reason.to_ascii_lowercase();
    if reason.contains("permission denied") || reason.contains("host key") {
        ("disconnected", "SSH authentication or host verification failed. Connect to the saved machine in a terminal, then retry Observe.")
    } else if reason.contains("not found") || reason.contains("not recognized") {
        (
            "disconnected",
            "Herdr must be available on the remote SSH PATH to observe this terminal.",
        )
    } else if reason.contains("control")
        || reason.contains("takeover")
        || reason.contains("attached")
    {
        (
            "conflict",
            "Another window controls this terminal. Watch it or deliberately take control.",
        )
    } else {
        (
            "closed",
            "The terminal view ended. The agent may still be running in Herdr.",
        )
    }
}
