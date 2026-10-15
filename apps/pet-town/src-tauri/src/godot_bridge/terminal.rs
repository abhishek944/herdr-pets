use super::GodotState;
use crate::town_terminal::{input::TerminalCommand, session::Session, TerminalEvent};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::atomic::Ordering,
    sync::{Arc, Mutex},
};
use tauri::{
    ipc::{Channel, InvokeResponseBody},
    AppHandle, Manager,
};

pub(super) type Queue = Arc<Mutex<VecDeque<Value>>>;
pub(super) fn queue() -> Queue {
    Arc::new(Mutex::new(VecDeque::new()))
}
pub(super) fn drain(queue: &Queue) -> Value {
    Value::Array(
        queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect(),
    )
}
pub(super) fn open(app: &AppHandle, request: &Value, queue: Queue) -> Result<Value, String> {
    if !super::focused(app) {
        return Err("Focus the town before connecting a terminal.".into());
    }
    let initial_epoch = app
        .state::<GodotState>()
        .terminal_epoch
        .load(Ordering::SeqCst);
    let target = pet_town_agent_broker::terminal_target(
        request["id"].as_str().ok_or("Choose a companion.")?,
    )?;
    let control = request["control"].as_bool().unwrap_or(false);
    let takeover = request["takeover"].as_bool().unwrap_or(false);
    if takeover && !control {
        return Err("Takeover requires interactive control.".into());
    }
    target.validate_mode(control, takeover)?;
    let cols = request["cols"].as_u64().unwrap_or(80);
    let rows = request["rows"].as_u64().unwrap_or(24);
    if !(2..=512).contains(&cols) || !(2..=256).contains(&rows) {
        return Err("Terminal dimensions are out of range.".into());
    }
    if app
        .state::<GodotState>()
        .terminal_epoch
        .load(Ordering::SeqCst)
        != initial_epoch
        || !super::focused(app)
    {
        return Err("The town changed while connecting. Reconnect to continue.".into());
    }
    release(app);
    drain(&queue);
    let epoch = initial_epoch.wrapping_add(1);
    let channel_app = app.clone();
    let generation = request["viewGeneration"].clone();
    let id = request["id"].clone();
    let channel: Channel<TerminalEvent> = Channel::new(move |body| {
        let InvokeResponseBody::Json(text) = body else {
            return Ok(());
        };
        if let Ok(mut value) = serde_json::from_str::<Value>(&text) {
            let closed = value["type"] == "status"
                && matches!(
                    value["state"].as_str(),
                    Some("disconnected" | "closed" | "stale" | "conflict")
                );
            if channel_app
                .state::<GodotState>()
                .terminal_epoch
                .load(Ordering::SeqCst)
                != epoch
                && !closed
            {
                return Ok(());
            }
            value["viewGeneration"] = generation.clone();
            value["id"] = id.clone();
            let mut pending = queue.lock().unwrap_or_else(|e| e.into_inner());
            // Never silently discard a delta; force a reconnect if rendering falls behind.
            let queued_bytes: usize = pending
                .iter()
                .map(|event| event["bytes"].as_str().map_or(0, str::len))
                .sum();
            if pending.len() >= 32
                || queued_bytes + value["bytes"].as_str().map_or(0, str::len) > 8_000_000
            {
                pending.clear();
                pending.push_back(
                    json!({"type":"status","state":"disconnected","control":false,
                    "message":"Terminal output fell behind. Reconnect to refresh it.", "viewGeneration":generation,"id":id}),
                );
                return Err(tauri::Error::Io(std::io::Error::other(
                    "Godot terminal queue exceeded its bound",
                )));
            }
            pending.push_back(value);
        }
        Ok(())
    });
    let state = app.state::<GodotState>();
    let mut current = state.terminal.lock().unwrap_or_else(|e| e.into_inner());
    if state.terminal_epoch.load(Ordering::SeqCst) != epoch || !super::focused(app) {
        return Err("The town changed while connecting. Reconnect to continue.".into());
    }
    let session = Session::start(target, control, takeover, cols as u16, rows as u16, channel)?;
    let result = json!({"token":session.token});
    if state.terminal_epoch.load(Ordering::SeqCst) != epoch || !super::focused(app) {
        session.stop();
        return Err("Town lost focus while connecting.".into());
    }
    *current = Some(session);
    Ok(result)
}
pub(super) fn send(app: &AppHandle, request: &Value) -> Result<(), String> {
    if !super::focused(app) {
        return Err("Terminal input is disabled while the town is unfocused.".into());
    }
    let session = app
        .state::<GodotState>()
        .terminal
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .filter(|s| request["session"].as_str() == Some(s.token.as_str()))
        .cloned()
        .ok_or("This terminal view is no longer active.")?;
    let command: TerminalCommand = serde_json::from_value(request["command"].clone())
        .map_err(|_| "Terminal command is invalid.")?;
    session.send(command)
}
pub(super) fn release(app: &AppHandle) {
    app.state::<GodotState>()
        .terminal_epoch
        .fetch_add(1, Ordering::SeqCst);
    let session = app
        .state::<GodotState>()
        .terminal
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    if let Some(session) = session {
        session.finish("disconnected", "Terminal detached. Reconnect to continue.");
    }
}
