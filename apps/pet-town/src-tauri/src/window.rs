use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tauri::{Manager, Monitor, WebviewWindow};

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(PetPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HitRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Default)]
pub(crate) struct HitRegions(std::sync::Mutex<HashMap<String, Vec<HitRegion>>>);

#[tauri::command]
pub(crate) fn set_hit_regions(
    window: WebviewWindow,
    regions: Vec<HitRegion>,
    state: tauri::State<'_, HitRegions>,
) {
    if let Ok(mut stored) = state.0.lock() {
        stored.insert(window.label().to_string(), regions);
    }
}

pub(crate) fn show_window(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let target = window.clone();
        window
            .run_on_main_thread(move || unsafe {
                use objc2_app_kit::NSWindow;
                if let Ok(native_window) = target.ns_window() {
                    let native_window: &NSWindow = &*native_window.cast();
                    native_window.orderFrontRegardless();
                }
            })
            .map_err(|error| error.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.show().map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
fn refresh_mouse_passthrough(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use objc2_app_kit::{NSEvent, NSWindow};

    let regions = app
        .state::<HitRegions>()
        .0
        .lock()
        .map(|stored| stored.clone())
        .unwrap_or_default();

    unsafe {
        let mouse = NSEvent::mouseLocation();
        for window in crate::street_windows::street_windows(app) {
            let native_window: &NSWindow = &*window.ns_window()?.cast();
            let frame = native_window.frame();
            let local_x = mouse.x - frame.origin.x;
            let local_y = frame.size.height - (mouse.y - frame.origin.y);
            let over_citizen = regions
                .get(window.label())
                .map(|list| list.as_slice())
                .unwrap_or(&[])
                .iter()
                .any(|region| {
                    local_x >= region.x
                        && local_x <= region.x + region.width
                        && local_y >= region.y
                        && local_y <= region.y + region.height
                });
            native_window.setIgnoresMouseEvents(!over_citizen);
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn start_hit_test_loop(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(16));
        let target = app.clone();
        if app
            .run_on_main_thread(move || {
                let _ = refresh_mouse_passthrough(&target);
            })
            .is_err()
        {
            break;
        }
    });
}

pub(crate) fn create_street_window(
    app: &tauri::AppHandle,
    label: &str,
) -> Result<WebviewWindow, Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    let window =
        tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("index.html".into()))
            .title("Pet Street")
            // Additional transparent height lets Ocean water sit below boat hulls.
            // Standard pets retain their existing bottom-anchored on-screen position.
            .inner_size(1100.0, 290.0)
            .min_inner_size(320.0, 290.0)
            .resizable(true)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .focusable(false)
            .focused(false)
            .accept_first_mouse(true)
            .skip_taskbar(false)
            .shadow(false)
            .visible(false)
            .visible_on_all_workspaces(true)
            .build()?;
    Ok(window)
}

pub(crate) fn configure_street_window(
    window: &WebviewWindow,
    monitor: &Monitor,
) -> Result<(), Box<dyn std::error::Error>> {
    window.set_ignore_cursor_events(true)?;

    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::{StyleMask, WebviewWindowExt};
        let panel = window.to_panel::<PetPanel>()?;
        panel.add_style_mask(StyleMask::empty().nonactivating_panel().into())?;
        // Class conversion skips NSPanel initialization. Synchronize its
        // WindowServer activation flag as well as its AppKit style mask.
        unsafe {
            use tauri_nspanel::objc2::{msg_send, sel};
            let native = panel.as_panel();
            let supported: bool =
                msg_send![native, respondsToSelector: sel!(_setPreventsActivation:)];
            if supported {
                let _: () = msg_send![native, _setPreventsActivation: true];
            }
        }
        panel.set_hides_on_deactivate(false);
    }

    #[cfg(target_os = "macos")]
    unsafe {
        use objc2_app_kit::{NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior};

        let native_window: &NSWindow = &*window.ns_window()?.cast();
        // Full-screen apps occupy their own Space above the status-bar level.
        // Use the screen-saver level so pets remain visible over those windows.
        native_window.setLevel(NSScreenSaverWindowLevel);
        // Joining all Spaces alone does not join other applications' Spaces
        // (including Stage Manager and full-screen apps).
        let behavior = native_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::CanJoinAllApplications
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary;
        native_window.setCollectionBehavior(behavior);
    }

    crate::street_windows::place_street_window(window, monitor)
        .map_err(|error| error.to_string())?;

    Ok(())
}
