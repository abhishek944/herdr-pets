use crate::preferences::PreferencesStore;
use crate::settings_window_lifecycle::{arm_readiness_timeout, cleanup_failed_window};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
#[derive(Default)]
struct SessionState {
    open: bool,
    ready: bool,
    generation: u64,
    selected_pet_id: Option<String>,
    initial_tab: Option<String>,
}
impl SessionState {
    fn allows_reload_resume(&self, generation: u64) -> bool {
        !self.open && self.generation == generation
    }
}
#[derive(Default)]
pub struct SettingsSession(Mutex<SessionState>);
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsContext {
    selected_pet_id: Option<String>,
    initial_tab: Option<String>,
}
impl SettingsSession {
    fn lock(&self) -> std::sync::MutexGuard<'_, SessionState> {
        self.0.lock().unwrap_or_else(|error| error.into_inner())
    }
    fn begin(
        &self,
        app: &AppHandle,
        requested: Option<String>,
        initial_tab: Option<String>,
    ) -> SettingsContext {
        let store = app.state::<PreferencesStore>();
        let selected = requested.filter(|id| store.contains_id(id));
        let mut session = self.lock();
        if !session.open {
            session.ready = false;
        }
        session.open = true;
        session.generation = session.generation.wrapping_add(1);
        session.selected_pet_id = selected.clone();
        session.initial_tab = initial_tab.clone();
        SettingsContext {
            selected_pet_id: selected,
            initial_tab,
        }
    }
    fn context(&self) -> SettingsContext {
        let session = self.lock();
        SettingsContext {
            selected_pet_id: session.selected_pet_id.clone(),
            initial_tab: session.initial_tab.clone(),
        }
    }
    pub fn is_open(&self) -> bool {
        self.lock().open
    }
    fn is_ready(&self) -> bool {
        self.lock().ready
    }
    fn readiness_token(&self) -> u64 {
        self.lock().generation
    }
    pub(crate) fn is_waiting(&self, token: u64) -> bool {
        let session = self.lock();
        session.open && !session.ready && session.generation == token
    }
    fn mark_ready(&self) {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .ready = true;
    }
    pub fn pause_for_reload(&self, app: &AppHandle) -> Option<u64> {
        let session = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if session.open {
            return None;
        }
        let _ = crate::street_windows::emit_to_streets(app.clone(), "village-pause", ());
        Some(session.generation)
    }
    pub fn resume_after_reload(&self, app: &AppHandle, generation: u64) {
        let session = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if session.allows_reload_resume(generation) {
            let _ = crate::street_windows::emit_to_streets(app.clone(), "village-resume", ());
        }
    }
    fn finish(&self) -> bool {
        let mut session = self.lock();
        let was_open = session.open;
        session.open = false;
        session.ready = false;
        if was_open {
            session.generation = session.generation.wrapping_add(1);
        }
        was_open
    }
}
pub fn open_internal(
    app: &AppHandle,
    pet_id: Option<String>,
    initial_tab: Option<String>,
) -> Result<(), String> {
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _opening = updates.window_opening()?;
    let context = app
        .state::<SettingsSession>()
        .begin(app, pet_id, initial_tab);
    arm_readiness_timeout(app, app.state::<SettingsSession>().readiness_token());
    let result = if let Some(window) = app.get_webview_window("settings") {
        match window.emit("settings-selection", context) {
            Err(error) => Err(error.to_string()),
            Ok(()) if app.state::<SettingsSession>().is_ready() => window
                .show()
                .and_then(|_| window.center())
                .and_then(|_| window.set_focus())
                .map_err(|error| error.to_string()),
            Ok(()) => Ok(()),
        }
    } else {
        WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
            .title("Pet Town Settings")
            .inner_size(880.0, 720.0)
            .min_inner_size(700.0, 560.0)
            .resizable(true)
            .decorations(true)
            .transparent(false)
            .focusable(true)
            .visible(false)
            .center()
            .build()
            .map(|_| ())
            .map_err(|error| error.to_string())
    };
    if result.is_err() {
        cleanup_failed_window(app);
    }
    result
}
#[tauri::command]
pub fn open_preferences(app: AppHandle, pet_id: Option<String>) -> Result<(), String> {
    open_internal(&app, pet_id, None)
}
#[tauri::command]
pub fn show_settings(app: AppHandle) -> Result<(), String> {
    if !app.state::<SettingsSession>().is_open() {
        return Err("Settings session is no longer open".to_string());
    }
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "Settings window is unavailable".to_string())?;
    let result = window
        .show()
        .and_then(|_| window.center())
        .and_then(|_| window.set_focus())
        .map_err(|error| error.to_string());
    if result.is_err() {
        cleanup_failed_window(&app);
    } else {
        app.state::<SettingsSession>().mark_ready();
    }
    result
}
#[tauri::command]
pub fn is_settings_open(state: tauri::State<'_, SettingsSession>) -> bool {
    state.is_open()
}
#[tauri::command]
pub fn get_settings_context(state: tauri::State<'_, SettingsSession>) -> SettingsContext {
    state.context()
}
pub fn close(app: &AppHandle) {
    app.state::<SettingsSession>().finish();
}
