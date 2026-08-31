use crate::agents::{parse_agent_list, unavailable_snapshot, AgentSnapshot, AgentView};
use crate::focus::{public_agent_id, FocusTarget, FocusTargets};
use crate::herdr_command::run_herdr_command;
use crate::labels::labels_for_session;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CONCURRENT_SESSION_QUERIES: usize = 8;

fn herdr_binary() -> OsString {
    std::env::var_os("HERDR_PETS_HERDR_BIN")
        .or_else(|| std::env::var_os("HERDR_BIN_PATH"))
        .unwrap_or_else(|| "herdr".into())
}

fn registered_sessions() -> Vec<Option<String>> {
    let Some(directory) = std::env::var_os("HERDR_PETS_SESSION_REGISTRY") else {
        return vec![None];
    };
    let mut paths: Vec<PathBuf> = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|item| item.path()))
            .collect(),
        Err(_) => return vec![None],
    };
    paths.sort();

    let mut sockets: Vec<String> = paths
        .into_iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .map(|socket| socket.trim().to_string())
        .filter(|socket| !socket.is_empty() && Path::new(socket).exists())
        .collect();
    sockets.sort();
    sockets.dedup();

    if sockets.is_empty() {
        vec![None]
    } else {
        sockets.into_iter().map(Some).collect()
    }
}

fn query_session(
    herdr: &OsString,
    socket: Option<String>,
) -> Option<Vec<(AgentView, FocusTarget)>> {
    let arguments = vec!["agent".to_string(), "list".to_string()];
    let text = run_herdr_command(herdr, socket.as_deref(), &arguments)?;
    let parsed = parse_agent_list(&text).ok()?;
    let mut workspaces: Vec<String> = parsed
        .iter()
        .filter_map(|agent| agent.workspace_id.clone())
        .collect();
    workspaces.sort();
    workspaces.dedup();
    let labels = labels_for_session(herdr, socket.as_deref(), &workspaces);

    Some(
        parsed
            .into_iter()
            .map(|mut agent| {
                agent.view.label = labels
                    .pane_labels
                    .get(&agent.view.id)
                    .cloned()
                    .or_else(|| {
                        agent
                            .tab_id
                            .as_ref()
                            .and_then(|id| labels.tab_labels.get(id).cloned())
                    })
                    .unwrap_or_else(|| agent.view.label.clone());
                let pane_id = agent.view.id.clone();
                let agent_session_id = agent.agent_session_id;
                agent.view.id = public_agent_id(socket.as_deref(), &pane_id);
                (
                    agent.view,
                    FocusTarget {
                        pane_id,
                        socket: socket.clone(),
                        agent_session_id,
                    },
                )
            })
            .collect(),
    )
}

fn query_agents() -> (AgentSnapshot, HashMap<String, FocusTarget>) {
    let herdr = herdr_binary();
    let sessions = registered_sessions();
    let mut available = false;
    let mut agents = Vec::new();
    for batch in sessions.chunks(MAX_CONCURRENT_SESSION_QUERIES) {
        let workers: Vec<_> = batch
            .iter()
            .cloned()
            .map(|socket| {
                let herdr = herdr.clone();
                std::thread::spawn(move || query_session(&herdr, socket))
            })
            .collect();
        for worker in workers {
            if let Ok(Some(mut session_agents)) = worker.join() {
                available = true;
                agents.append(&mut session_agents);
            }
        }
    }
    agents.sort_by(|left, right| left.0.id.cmp(&right.0.id));
    agents.dedup_by(|left, right| left.0.id == right.0.id);
    let targets = agents
        .iter()
        .map(|(view, target)| (view.id.clone(), target.clone()))
        .collect();
    let views = agents.into_iter().map(|(view, _)| view).collect();
    (
        AgentSnapshot {
            available,
            agents: views,
        },
        targets,
    )
}

pub fn snapshot_json() -> String {
    serde_json::to_string(&query_agents().0)
        .unwrap_or_else(|_| r#"{"available":false,"agents":[]}"#.to_string())
}

#[tauri::command]
pub(crate) async fn list_agents(
    targets: tauri::State<'_, FocusTargets>,
) -> Result<AgentSnapshot, String> {
    let (snapshot, next_targets) = tauri::async_runtime::spawn_blocking(query_agents)
        .await
        .unwrap_or_else(|_| (unavailable_snapshot(), HashMap::new()));
    targets.replace(next_targets);
    Ok(snapshot)
}
