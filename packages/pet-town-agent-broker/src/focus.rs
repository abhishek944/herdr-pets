use crate::{
    application_focus::{activate_application, focus_codex},
    command, herdr, FocusRoute,
};
use serde::Deserialize;
use std::ffi::OsString;

#[derive(Deserialize)]
struct AgentGetEnvelope {
    result: AgentGetResult,
}
#[derive(Deserialize)]
struct AgentGetResult {
    agent: CurrentAgent,
}
#[derive(Deserialize)]
struct CurrentAgent {
    agent_session: Option<CurrentSession>,
    tab_id: String,
    workspace_id: String,
}
#[derive(Deserialize)]
struct CurrentSession {
    value: String,
}
#[derive(PartialEq, Eq)]
struct VerifiedAgent {
    tab_id: String,
    workspace_id: String,
}

fn verify(
    binary: &OsString,
    pane_id: &str,
    socket: Option<&str>,
    machine: Option<&str>,
    expected_session: &str,
) -> Result<VerifiedAgent, String> {
    let arguments = vec!["agent".to_string(), "get".to_string(), pane_id.to_string()];
    let text = command::run_on_machine(binary, socket, machine, &arguments)
        .ok_or_else(|| "agent is no longer available".to_string())?;
    let current = serde_json::from_str::<AgentGetEnvelope>(&text)
        .map_err(|_| "Herdr returned invalid agent details".to_string())?
        .result
        .agent;
    let current_session = current.agent_session.map(|session| session.value);
    let ephemeral = expected_session == format!("ephemeral:{pane_id}") && current_session.is_none();
    if !ephemeral && current_session.as_deref() != Some(expected_session) {
        return Err("agent changed since the latest poll".to_string());
    }
    Ok(VerifiedAgent {
        tab_id: current.tab_id,
        workspace_id: current.workspace_id,
    })
}

fn focus_target(
    binary: &OsString,
    socket: Option<&str>,
    machine: Option<&str>,
    kind: &str,
    id: &str,
    error: &str,
) -> Result<(), String> {
    let arguments = vec![kind.to_string(), "focus".to_string(), id.to_string()];
    command::run_on_machine(binary, socket, machine, &arguments)
        .map(|_| ())
        .ok_or_else(|| error.to_string())
}

pub(crate) fn focus_herdr(
    binary: &OsString,
    pane_id: &str,
    socket: Option<&str>,
    machine: Option<&str>,
    expected_session: &str,
) -> Result<(), String> {
    let verified = verify(binary, pane_id, socket, machine, expected_session)?;
    focus_target(
        binary,
        socket,
        machine,
        "workspace",
        &verified.workspace_id,
        "Herdr could not focus that workspace",
    )?;
    focus_target(
        binary,
        socket,
        machine,
        "tab",
        &verified.tab_id,
        "Herdr could not focus that tab",
    )?;
    if verify(binary, pane_id, socket, machine, expected_session)? != verified {
        return Err("agent moved while focus was in progress".to_string());
    }
    focus_target(
        binary,
        socket,
        machine,
        "agent",
        pane_id,
        "Herdr could not focus that agent",
    )?;
    #[cfg(all(target_os = "macos", not(test)))]
    if machine.is_none() {
        let _ = crate::macos_activation::activate(binary, socket);
    }
    Ok(())
}

pub fn focus_route(route: &FocusRoute) -> Result<(), String> {
    focus_route_with_codex_activation(route).map(|_| ())
}
/// Returns whether Codex Desktop itself was activated (rather than a fallback app).
pub fn focus_route_with_codex_activation(route: &FocusRoute) -> Result<bool, String> {
    match route {
        FocusRoute::Herdr {
            pane_id,
            socket,
            machine,
            agent_session_id,
        } => focus_herdr(
            &herdr::binary(),
            pane_id,
            socket.as_deref(),
            machine.as_deref(),
            agent_session_id,
        )
        .map(|_| false),
        FocusRoute::Application { bundle_id } => activate_application(bundle_id).map(|_| false),
        FocusRoute::Codex {
            thread_id,
            fallback_bundle_id,
        } => focus_codex(thread_id.as_deref(), fallback_bundle_id.as_deref()),
    }
}
