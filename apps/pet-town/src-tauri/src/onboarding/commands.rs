use super::{model::Context, window, Onboarding};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

#[tauri::command]
pub(crate) fn get_onboarding_context(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Onboarding>,
) -> Result<Context, String> {
    window::require(&window)?;
    Ok(state.context(&app))
}

#[tauri::command]
pub(crate) fn save_onboarding(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Onboarding>,
    step: u8,
    tool: Option<String>,
    skipped: bool,
) -> Result<Context, String> {
    window::require(&window)?;
    window::require_installation()?;
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let operation = state.begin()?;
    if step > 7 {
        return Err("This setup step is not available.".into());
    }
    if let Some(id) = &tool {
        super::tools::spec(id)?;
    }
    state.store.update(|p| {
        if skipped && p.resume_step.is_none() {
            p.resume_step = Some(p.step);
        } else if !skipped && step > p.step && p.resume_step == Some(p.step) {
            p.resume_step = None;
        }
        p.step = step;
        if tool.is_some() {
            p.tool = tool;
        }
    })?;
    drop(operation);
    Ok(state.context(&app))
}

#[tauri::command]
pub(crate) fn set_onboarding_appearance(
    app: AppHandle,
    window: WebviewWindow,
    theme: String,
) -> Result<(), String> {
    window::require(&window)?;
    window::require_installation()?;
    let state = app.state::<Onboarding>();
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let _operation = state.begin()?;
    let theme = match theme.as_str() {
        "standard" | "ocean" | "rainforest" | "snowy" | "desert" | "rollercoaster" | "goldmine"
        | "railway" => serde_json::from_value(serde_json::Value::String(theme))
            .map_err(|_| "Unsupported scenery.")?,
        _ => return Err("Choose one of the available desktop looks.".into()),
    };
    let store = app.state::<crate::preferences::PreferencesStore>();
    let mut snapshot = store.snapshot();
    snapshot.preferences.app.strip_theme = theme;
    let snapshot = store.apply(snapshot.preferences, snapshot.revision)?;
    app.emit_to("main", "preferences-applied", &snapshot)
        .map_err(|_| {
            "The look was saved, but the desktop could not be refreshed. Reopen Pet Town.".into()
        })
}

#[tauri::command]
pub(crate) async fn check_onboarding(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Context, String> {
    window::require(&window)?;
    window::require_installation()?;
    let state = app.state::<Onboarding>();
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let _operation = state.begin()?;
    state.runtime().dependency = super::model::Dependency::new("checking", "Checking Herdr…");
    let target = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = target.state::<Onboarding>();
        let dependency = super::dependency::check();
        let selected = state.store.get().tool;
        let tools = super::tools::list(selected.as_deref());
        let mut runtime = state.runtime();
        runtime.dependency = dependency;
        runtime.tools = tools;
    })
    .await
    .map_err(|_| "Could not finish checking setup.")?;
    drop(_operation);
    Ok(state.context(&app))
}

#[tauri::command]
pub(crate) async fn install_onboarding_herdr(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<(), String> {
    window::require(&window)?;
    window::require_installation()?;
    let state = app.state::<Onboarding>();
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let _operation = state.begin()?;
    let result = super::install::run(&app).await;
    if let Err(error) = &result {
        state.runtime().dependency = super::model::Dependency::new("error", error);
    }
    result
}

#[tauri::command]
pub(crate) fn finish_onboarding(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Onboarding>,
) -> Result<(), String> {
    window::require(&window)?;
    window::require_installation()?;
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let _operation = state.begin()?;
    state.store.update(|p| {
        p.completed = true;
        p.step = p.resume_step.unwrap_or(p.step);
    })?;
    crate::village_visibility::set(&app, true)?;
    window
        .close()
        .map_err(|_| "Setup was saved, but the welcome window could not close.".into())
}

#[tauri::command]
pub(crate) fn open_onboarding(app: AppHandle) -> Result<(), String> {
    window::open(&app)
}
