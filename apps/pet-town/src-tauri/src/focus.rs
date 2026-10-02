mod herdr;

pub(crate) use crate::focus_id::{herdr_owner_key, public_agent_id};
use std::ffi::OsString;

use crate::focus_route::FocusRoute;

fn herdr_binary() -> OsString {
    crate::orchestrator::herdr_binary()
}

pub(crate) fn verify_existing_herdr_agent(pane_id: String) -> Result<(), String> {
    herdr::verify_from_environment(&pane_id)
}

fn run_focus_route(herdr: &OsString, target: &FocusRoute) -> Result<(), String> {
    match target {
        FocusRoute::Herdr {
            pane_id,
            socket,
            machine,
            agent_session_id,
        } => {
            if machine.is_some() {
                pet_town_agent_broker::focus_route(&pet_town_agent_broker::FocusRoute::Herdr {
                    pane_id: pane_id.clone(),
                    socket: None,
                    machine: machine.clone(),
                    agent_session_id: agent_session_id.clone(),
                })
            } else {
                herdr::focus(herdr, pane_id, socket.as_deref(), agent_session_id)
            }
        }
        FocusRoute::Application { bundle_id } => {
            #[cfg(target_os = "macos")]
            return crate::macos_app_focus::activate_running_application(bundle_id);
            #[cfg(not(target_os = "macos"))]
            Err("application focus is unavailable on this platform".to_string())
        }
        FocusRoute::Codex {
            thread_id,
            fallback_bundle_id,
        } => pet_town_agent_broker::focus_route(&pet_town_agent_broker::FocusRoute::Codex {
            thread_id: thread_id.clone(),
            fallback_bundle_id: fallback_bundle_id.clone(),
        }),
    }
}

fn run_user_focus_route(herdr: &OsString, target: &FocusRoute) -> Result<(), String> {
    let FocusRoute::Herdr {
        pane_id,
        socket,
        machine,
        agent_session_id,
    } = target
    else {
        return run_focus_route(herdr, target);
    };

    if machine.is_some() {
        return run_focus_route(herdr, target);
    }
    let verified = herdr::verify(herdr, pane_id, socket.as_deref(), agent_session_id)?;
    herdr::focus_verified(
        herdr,
        pane_id,
        socket.as_deref(),
        agent_session_id,
        &verified,
    )?;
    // Select the pane before showing the terminal, then select it again after
    // macOS activates the host. Activation can raise a different host window.
    #[cfg(target_os = "macos")]
    {
        let activation = crate::macos_activation::activate_herdr_host(herdr, socket.as_deref());
        herdr::focus_verified(
            herdr,
            pane_id,
            socket.as_deref(),
            agent_session_id,
            &verified,
        )?;
        activation?;
    }
    Ok(())
}

pub(crate) fn focus_current_agent(id: &str) -> Result<(), String> {
    let route = if matches!(id, "pet-town-assistant" | "pet-town-mayor") {
        crate::sessions::mayor_primary_route()
    } else {
        crate::sessions::collect_visible()
            .focus_routes
            .remove(id)
            .or_else(|| pet_town_agent_broker::remote_focus_route(id))
    };
    let target = route
        .map(FocusRoute::from)
        .ok_or_else(|| "agent is no longer available".to_string())?;
    crate::focus_trace::record(&format!(
        "route {id}: {}",
        match &target {
            FocusRoute::Herdr { .. } => "herdr",
            FocusRoute::Codex { thread_id, .. } =>
                if thread_id.is_some() {
                    "codex thread"
                } else {
                    "codex app"
                },
            FocusRoute::Application { bundle_id } => bundle_id,
        }
    ));
    if let FocusRoute::Codex {
        thread_id,
        fallback_bundle_id,
    } = &target
    {
        let activated = pet_town_agent_broker::focus_route_with_codex_activation(
            &pet_town_agent_broker::FocusRoute::Codex {
                thread_id: thread_id.clone(),
                fallback_bundle_id: fallback_bundle_id.clone(),
            },
        )?;
        if activated {
            crate::adapter_events::mark_codex_idle(id)?;
        }
        return Ok(());
    }
    run_user_focus_route(&herdr_binary(), &target)
}

#[tauri::command]
pub(crate) async fn focus_agent(id: String) -> Result<(), String> {
    crate::focus_trace::record("focus requested");
    #[cfg(target_os = "macos")]
    crate::focus_trace::frontmost("before focus");
    // Both renderers use this isolated, headless focus boundary. AppKit focus
    // requests must not compete with the nonactivating overlay's GUI process.
    let result = tauri::async_runtime::spawn_blocking(move || {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let output = std::process::Command::new(executable)
            .env_remove("OPENAI_API_KEY")
            .args(["--focus-agent", &id])
            .output()
            .map_err(|error| format!("could not start agent focus: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    })
    .await
    .map_err(|_| "agent focus task failed".to_string())?;
    crate::focus_trace::record(&format!("focus result: {result:?}"));
    #[cfg(target_os = "macos")]
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(800));
        crate::focus_trace::frontmost("settled focus");
    });
    result
}
