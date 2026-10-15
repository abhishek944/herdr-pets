mod adapter_events;
mod adapter_setup;
mod agents;
mod app_commands;
mod app_events;
mod app_menu;
mod app_singleton;
mod app_updates;
mod codex_usage;
mod control;
#[cfg(target_os = "macos")]
mod dock_menu;
mod focus;
mod focus_id;
mod focus_trace;
#[cfg(target_os = "macos")]
mod global_mayor_shortcut;
mod godot_bridge;
mod herdr_command;
mod herdr_state;
#[cfg(target_os = "macos")]
mod macos_activation;
#[cfg(target_os = "macos")]
mod macos_app_focus;
#[cfg(unix)]
mod mayor_retry_signal;
mod onboarding;
mod orchestrator;
mod pet_studio;
mod preferences;
mod preferences_commands;
mod preferences_defaults;
mod preferences_io;
mod preferences_lock;
mod preferences_model;
mod preferences_permissions;
mod preferences_registration;
mod preferences_state;
mod preferences_validation;
mod sessions;
mod settings_window;
mod settings_window_lifecycle;
mod street_windows;
mod town_commands;
#[cfg(unix)]
mod town_launch_cli;
mod town_process;
mod town_snapshot;
mod town_terminal;
mod town_voice;
mod village_visibility;
mod window;

pub use adapter_events::{record_from_stdin, relay_from_stdin};
pub use adapter_setup::{configure_adapter, setups_json};
pub use agents::{AgentSnapshot, AgentView};
pub use preferences_commands::startup_enabled_from_disk;
pub use sessions::snapshot_json;

#[cfg(unix)]
pub use town_launch_cli::open as open_town_from_cli;

pub fn open_onboarding_herdr_from_cli() -> Result<(), String> {
    onboarding::herdr_handoff_from_cli()
}

pub fn codex_usage_snapshot_from_cli() -> String {
    codex_usage::snapshot_from_cli()
}

pub fn focus_agent_from_cli(id: &str) -> Result<(), String> {
    focus::focus_current_agent(id)
}

pub fn focus_native_town_from_cli(pid: u32, parent: u32) -> Result<(), String> {
    godot_bridge::focus_from_cli(pid, parent)
}

pub fn verify_existing_herdr_agent(pane_id: String) -> Result<(), String> {
    focus::verify_existing_herdr_agent(pane_id)
}

pub fn verify_existing_application(bundle_id: String) -> Result<(), String> {
    macos_app_focus::activate_running_application(&bundle_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    control::install_signal_handlers();
    let app_lock = match app_singleton::acquire_or_notify() {
        Ok(Some(lock)) => lock,
        Ok(None) => return,
        Err(error) => {
            eprintln!("pet-town: could not acquire application lock: {error}");
            return;
        }
    };
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    builder
        .manage(app_lock)
        .manage(app_updates::AppUpdates::default())
        .manage(onboarding::Onboarding::default())
        .manage(window::HitRegions::default())
        .manage(village_visibility::VillageVisibility::default())
        .manage(preferences::PreferencesStore::load_default())
        .manage(pet_studio::PetStudioState::load())
        .manage(orchestrator::OrchestratorState::default())
        .manage(orchestrator::firstmate::FirstmateState::default())
        .manage(orchestrator::firstmate_audio::FirstmateAudioState::default())
        .manage(settings_window::SettingsSession::default())
        .manage(town_process::TownWindowState::default())
        .manage(godot_bridge::GodotState::default())
        .manage(town_terminal::TownTerminalState::default())
        .on_window_event(app_events::window_event)
        .setup(|app| {
            app_menu::install(app)?;
            street_windows::reconcile_street_windows(app.handle())?;
            onboarding::window::start(app.handle());
            control::start(app.handle().clone());
            #[cfg(unix)]
            if let Err(error) = town_launch_cli::start(app.handle()) {
                eprintln!("Pet Town launch command unavailable: {error}");
            }
            app_updates::start(app.handle().clone());
            #[cfg(target_os = "macos")]
            global_mayor_shortcut::start();
            orchestrator::commands::preferences_changed(app.handle(), None);
            street_windows::start_street_watcher(app.handle().clone());
            #[cfg(target_os = "macos")]
            window::start_hit_test_loop(app.handle().clone());
            Ok(())
        })
        .invoke_handler(app_commands::handler())
        .build(tauri::generate_context!())
        .expect("failed to build pet-town")
        .run(app_events::run_event);
}
