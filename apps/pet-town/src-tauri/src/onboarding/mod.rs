pub(crate) mod commands;
mod dependency;
mod handoff;
mod install;
mod model;
mod process;
mod sample;
pub(crate) mod sample_commands;
mod sample_identity;
mod sample_launch;
mod sample_poll;
mod store;
mod tool_readiness;
mod tools;
pub(crate) mod window;
use model::{Context, Dependency, Sample, Tool};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Manager};

pub(crate) struct Onboarding {
    store: store::Store,
    runtime: Mutex<Runtime>,
    pub(crate) busy: AtomicBool,
}
struct Runtime {
    dependency: Dependency,
    tools: Vec<Tool>,
    sample: Sample,
    rendered_ids: Vec<String>,
    rendered_at: Option<std::time::Instant>,
}
impl Default for Onboarding {
    fn default() -> Self {
        Self {
            store: store::Store::default(),
            busy: AtomicBool::new(false),
            runtime: Mutex::new(Runtime {
                dependency: Dependency::new("checking", "Checking Herdr…"),
                tools: Vec::new(),
                sample: Sample {
                    phase: "idle".into(),
                    ..Sample::default()
                },
                rendered_ids: Vec::new(),
                rendered_at: None,
            }),
        }
    }
}
impl Onboarding {
    fn runtime(&self) -> std::sync::MutexGuard<'_, Runtime> {
        self.runtime.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn begin(&self) -> Result<Operation<'_>, String> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "Setup is already working. Wait a moment before trying again.")?;
        Ok(Operation(&self.busy))
    }
    fn context(&self, app: &AppHandle) -> Context {
        let progress = self.store.get();
        let runtime = self.runtime();
        let saved = app
            .state::<crate::preferences::PreferencesStore>()
            .snapshot()
            .preferences
            .app;
        let theme = saved.strip_theme;
        let mut preview_preferences = crate::preferences_model::PreferencesFile::defaults(&[]);
        preview_preferences.app.strip_theme = theme.clone();
        preview_preferences.app.coaster_mode = saved.coaster_mode;
        let mut sample = runtime.sample.clone();
        if progress.own_baseline.is_some() && sample.phase == "idle" {
            sample.phase = "own".into();
        }
        if progress.own_baseline.is_some() {
            sample.owned = false;
        } else if progress.sample.is_some() {
            sample.owned = true;
            if sample.phase == "idle" {
                sample.phase = "waiting".into();
            }
        }
        Context {
            step: progress.step,
            completed: progress.completed,
            tool: progress.tool,
            strip_theme: serde_json::to_value(theme)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_else(|| "standard".into()),
            preview_preferences,
            installation_ready: window::installation_ready(),
            busy: self.busy.load(Ordering::SeqCst),
            dependency: runtime.dependency.clone(),
            tools: runtime.tools.clone(),
            sample,
            has_sample: progress.sample.is_some(),
        }
    }
}
struct Operation<'a>(&'a AtomicBool);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

pub(crate) fn herdr_handoff_from_cli() -> Result<(), String> {
    handoff::open_herdr_from_cli()
}
