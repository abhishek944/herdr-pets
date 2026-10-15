use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub(crate) struct VillageVisibility {
    requested: AtomicBool,
    ready: std::sync::Mutex<HashSet<String>>,
    town_active: AtomicBool,
}

impl Default for VillageVisibility {
    fn default() -> Self {
        Self {
            requested: AtomicBool::new(true),
            ready: std::sync::Mutex::new(HashSet::new()),
            town_active: AtomicBool::new(false),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VillageVisibilityChanged {
    visible: bool,
}

fn apply(app: &AppHandle, visible: bool) -> Result<(), String> {
    for window in crate::street_windows::street_windows(app) {
        if visible {
            crate::window::show_window(&window)?;
        } else {
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn any_ready(state: &VillageVisibility) -> bool {
    state
        .ready
        .lock()
        .map(|ready| !ready.is_empty())
        .unwrap_or(false)
}

pub(crate) fn set(app: &AppHandle, visible: bool) -> Result<bool, String> {
    let state = app.state::<VillageVisibility>();
    state.requested.store(visible, Ordering::SeqCst);
    if any_ready(&state) {
        apply(app, visible && !state.town_active.load(Ordering::SeqCst))?;
    }
    let _ = app.emit(
        "village-visibility-changed",
        VillageVisibilityChanged { visible },
    );
    Ok(visible)
}

pub(crate) fn set_town_active(app: &AppHandle, active: bool) -> Result<(), String> {
    let state = app.state::<VillageVisibility>();
    state.town_active.store(active, Ordering::SeqCst);
    if any_ready(&state) {
        let visible = state.requested.load(Ordering::SeqCst) && !active;
        apply(app, visible)?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn renderer_ready(window: WebviewWindow, app: AppHandle) -> Result<bool, String> {
    let state = app.state::<VillageVisibility>();
    if let Ok(mut ready) = state.ready.lock() {
        ready.insert(window.label().to_string());
    }
    let visible =
        state.requested.load(Ordering::SeqCst) && !state.town_active.load(Ordering::SeqCst);
    if visible {
        crate::window::show_window(&window)?;
    }
    Ok(visible)
}

#[tauri::command]
pub(crate) fn set_village_visible(app: AppHandle, visible: bool) -> Result<bool, String> {
    set(&app, visible)
}

#[tauri::command]
pub(crate) fn village_visible(state: tauri::State<'_, VillageVisibility>) -> bool {
    state.requested.load(Ordering::SeqCst)
}
