use super::{model::Context, window, Onboarding};
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[tauri::command]
pub(crate) async fn run_onboarding_sample(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<(), String> {
    window::require(&window)?;
    window::require_installation()?;
    let state = app.state::<Onboarding>();
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let _operation = state.begin()?;
    let tool = state
        .store
        .get()
        .tool
        .ok_or("Choose a coding tool before running the sample.")?;
    super::handoff::prepare_observation(&app)?;
    let target = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        super::sample::start(&target.state::<Onboarding>(), &tool)
    })
    .await
    .map_err(|_| "The sample launch was interrupted. Check Herdr before retrying.")?;
    if let Err(error) = &result {
        let owned = state.store.get().sample.is_some();
        state.runtime().sample = super::model::Sample {
            phase: "error".into(),
            message: Some(error.clone()),
            owned,
            ..Default::default()
        };
    }
    result
}

#[tauri::command]
pub(crate) async fn stop_onboarding_sample(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<(), String> {
    window::require(&window)?;
    let state = app.state::<Onboarding>();
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let _operation = state.begin()?;
    let target = app.clone();
    tauri::async_runtime::spawn_blocking(move || super::sample::stop(&target.state::<Onboarding>()))
        .await
        .map_err(|_| "Could not confirm the sample stopped. Check Herdr.")?
}

#[tauri::command]
pub(crate) async fn poll_onboarding_sample(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Context, String> {
    window::require(&window)?;
    let state = app.state::<Onboarding>();
    let Ok(operation) = state.begin() else {
        return Ok(state.context(&app));
    };
    let _ = crate::street_windows::emit_to_streets(app.clone(), "onboarding-presence-request", ());
    let target = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        super::sample_poll::poll(&target.state::<Onboarding>())
    })
    .await
    .map_err(|_| "Could not check your companion.")?;
    drop(operation);
    Ok(app.state::<Onboarding>().context(&app))
}

#[tauri::command]
pub(crate) async fn onboarding_handoff(
    app: AppHandle,
    window: WebviewWindow,
    destination: String,
) -> Result<(), String> {
    window::require(&window)?;
    if !matches!(destination.as_str(), "applications" | "installGuide") {
        window::require_installation()?;
    }
    let state = app.state::<Onboarding>();
    let updates = app.state::<crate::app_updates::AppUpdates>();
    let _update_admission = updates.window_opening()?;
    let _operation = state.begin()?;
    if destination == "sample" {
        let id = app
            .state::<Onboarding>()
            .runtime()
            .sample
            .agent_id
            .clone()
            .ok_or("The sample agent is not available yet.")?;
        return crate::focus::focus_agent(id).await;
    }
    let target = app.clone();
    tauri::async_runtime::spawn_blocking(move || super::handoff::run(&target, &destination))
        .await
        .map_err(|_| "Could not open this setup step.")?
}

#[tauri::command]
pub(crate) fn report_onboarding_pets(
    window: WebviewWindow,
    state: State<'_, Onboarding>,
    ids: Vec<String>,
) -> Result<(), String> {
    let url = window.url().map_err(|_| "The strip URL is unavailable.")?;
    let local = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"))
        || (cfg!(debug_assertions)
            && url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
            && url.port() == Some(1420));
    if !crate::street_windows::is_street_label(window.label())
        || !local
        || !matches!(url.path(), "/" | "/index.html")
        || ids.len() > 256
        || ids.iter().any(|id| id.len() > 256)
    {
        return Err("Pet presence is available only from Pet Street.".into());
    }
    let mut runtime = state.runtime();
    runtime.rendered_ids = if window.is_visible().unwrap_or(false) {
        ids
    } else {
        Vec::new()
    };
    runtime.rendered_at = Some(std::time::Instant::now());
    Ok(())
}
