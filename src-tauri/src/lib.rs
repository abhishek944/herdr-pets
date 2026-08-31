mod agents;
mod focus;
mod herdr_command;
mod labels;
mod sessions;
mod window;

pub use agents::{AgentSnapshot, AgentView};
pub use sessions::snapshot_json;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(window::HitRegions::default())
        .manage(focus::FocusTargets::default())
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = webview.window().show();
            }
        })
        .setup(|app| {
            window::create_village_window(app)?;
            window::configure_window(app)?;
            #[cfg(target_os = "macos")]
            window::start_hit_test_loop(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sessions::list_agents,
            focus::focus_agent,
            window::set_hit_regions,
            window::show_village
        ])
        .run(tauri::generate_context!())
        .expect("failed to run herdr-pets");
}
