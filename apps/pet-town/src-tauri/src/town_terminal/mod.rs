mod events;
pub(crate) mod input;
mod lifecycle;
pub(crate) mod session;
mod stream;

pub(crate) use events::TerminalEvent;
use session::Session;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tauri::{ipc::Channel, AppHandle, Manager, WebviewWindow};

#[derive(Default)]
pub(crate) struct TownTerminalState {
    current: Mutex<Option<Arc<Session>>>,
    opening: Mutex<()>,
    epoch: AtomicU64,
}

#[tauri::command]
// The parameter list is the IPC contract with the terminal frontend, so it is kept flat.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn town_terminal_open(
    app: AppHandle,
    window: WebviewWindow,
    id: String,
    cols: u16,
    rows: u16,
    control: bool,
    takeover: bool,
    on_event: Channel<TerminalEvent>,
) -> Result<serde_json::Value, String> {
    crate::town_commands::require_town(&window)?;
    input::dimensions(cols, rows)?;
    if takeover && !control {
        return Err("Takeover requires an interactive terminal.".into());
    }
    let epoch = app
        .state::<TownTerminalState>()
        .epoch
        .load(Ordering::SeqCst);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TownTerminalState>();
        let _opening = state
            .opening
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        lifecycle::check(&app, &window, epoch)?;
        let target = pet_town_agent_broker::terminal_target(&id)?;
        target.validate_mode(control, takeover)?;
        // A pending attach must not acquire control after the town loses focus.
        lifecycle::check(&app, &window, epoch)?;
        let mut current = state
            .current
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // No native window queries or slow identity lookup while holding the binding lock.
        if state.epoch.load(Ordering::SeqCst) != epoch {
            return Err("The town changed while connecting. Reconnect to continue.".into());
        }
        if let Some(previous) = current.take() {
            previous.stop();
        }
        let session = Session::start(target, control, takeover, cols, rows, on_event)?;
        let result = serde_json::json!({"token":session.token});
        *current = Some(session.clone());
        drop(current);
        if let Err(error) = lifecycle::check(&app, &window, epoch) {
            session.finish("disconnected", &error);
            return Err(error);
        }
        Ok(result)
    })
    .await
    .map_err(|_| "The terminal view could not connect.".to_string())?
}

#[tauri::command]
pub(crate) async fn town_terminal_send(
    app: AppHandle,
    window: WebviewWindow,
    token: String,
    command: input::TerminalCommand,
) -> Result<(), String> {
    crate::town_commands::require_town(&window)?;
    if !window.is_focused().unwrap_or(false) {
        return Err("Terminal input is disabled while the town is unfocused.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TownTerminalState>();
        let session = state
            .current
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .filter(|session| session.token == token)
            .cloned()
            .ok_or("This terminal view is no longer active.")?;
        session.send(command)
    })
    .await
    .map_err(|_| "Terminal delivery is uncertain. Check Herdr before retrying.".to_string())?
}

#[tauri::command]
pub(crate) async fn town_terminal_close(
    app: AppHandle,
    window: WebviewWindow,
    token: String,
) -> Result<(), String> {
    crate::town_commands::require_town(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<TownTerminalState>();
        let mut current = state
            .current
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if current
            .as_ref()
            .is_some_and(|session| session.token == token)
        {
            if let Some(session) = current.take() {
                session.stop();
            }
        }
    })
    .await
    .map_err(|_| "The terminal view could not detach.".to_string())?;
    Ok(())
}

pub(crate) fn release(app: &AppHandle) {
    let state = app.state::<TownTerminalState>();
    state.epoch.fetch_add(1, Ordering::SeqCst);
    let session = state
        .current
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    if let Some(session) = session {
        // Cancel now, including during app exit; no background cleanup is required.
        session.finish(
            "disconnected",
            "Terminal detached while the town was inactive. Reconnect to continue.",
        );
    }
}
