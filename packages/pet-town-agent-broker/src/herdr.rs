use crate::agents::ParsedAgent;
use crate::{command, labels, remote, state, AdapterAgent, AdapterSnapshot, FocusRoute};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CONCURRENT_SESSION_QUERIES: usize = 8;

type Parser = fn(&str) -> Result<Vec<ParsedAgent>, serde_json::Error>;

pub(crate) fn binary() -> OsString {
    if let Some(path) =
        std::env::var_os("PET_TOWN_HERDR_BIN").or_else(|| std::env::var_os("HERDR_BIN_PATH"))
    {
        return path;
    }
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/herdr"),
        PathBuf::from("/opt/homebrew/opt/herdr/bin/herdr"),
        PathBuf::from("/usr/local/bin/herdr"),
        PathBuf::from("/Applications/Herdr.app/Contents/MacOS/herdr"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(&home).join(".local/bin/herdr"));
        candidates.push(PathBuf::from(home).join(".cargo/bin/herdr"));
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(OsString::from)
        .unwrap_or_else(|| "herdr".into())
}

fn opaque(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn public_id(socket: Option<&str>, session: &str) -> String {
    let namespace = socket.map(opaque).unwrap_or_else(|| "default".to_string());
    format!("herdr:{namespace}:{}", opaque(session))
}

fn owner_key(socket: Option<&str>, session: &str) -> String {
    let namespace = socket.map(opaque).unwrap_or_else(|| "default".to_string());
    format!("herdr-session:{namespace}:{}", opaque(session))
}

fn registered_sessions() -> Vec<Option<String>> {
    let Some(directory) = state::session_registry() else {
        return vec![std::env::var("HERDR_SOCKET_PATH").ok()];
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

pub(crate) fn query_session(
    herdr: &OsString,
    socket: Option<String>,
    machine: Option<(String, String)>,
    parse: Parser,
) -> Option<Vec<AdapterAgent>> {
    let arguments = vec!["agent".to_string(), "list".to_string()];
    let text = command::run_on_machine(
        herdr,
        socket.as_deref(),
        machine.as_ref().map(|(id, _)| id.as_str()),
        &arguments,
    )?;
    let parsed = parse(&text).ok()?;
    let mut workspaces: Vec<String> = parsed
        .iter()
        .filter_map(|agent| agent.workspace_id.clone())
        .collect();
    workspaces.sort();
    workspaces.dedup();
    let session_labels = if machine.is_some() {
        labels::SessionLabels::default()
    } else {
        labels::for_session(herdr, socket.as_deref(), &workspaces)
    };

    Some(
        parsed
            .into_iter()
            .map(|mut agent| {
                agent.view.label = session_labels
                    .pane_labels
                    .get(&agent.view.id)
                    .cloned()
                    .or_else(|| {
                        agent
                            .tab_id
                            .as_ref()
                            .and_then(|id| session_labels.tab_fallback_labels.get(id).cloned())
                    })
                    .unwrap_or_else(|| agent.view.label.clone());
                let pane_id = agent.view.id.clone();
                let agent_session_id = agent
                    .agent_session_id
                    .unwrap_or_else(|| format!("ephemeral:{pane_id}"));
                let (public_id, owner_key) = if let Some((id, label)) = &machine {
                    agent.view.label = format!("{label}: {}", agent.view.label);
                    remote_identity(id, &agent_session_id)
                } else {
                    (
                        public_id(socket.as_deref(), &agent_session_id),
                        owner_key(socket.as_deref(), &agent_session_id),
                    )
                };
                agent.view.id = public_id;
                agent.view.source = "herdr".to_string();
                AdapterAgent {
                    owner_key,
                    hosted_owner_key: None,
                    view: agent.view,
                    focus_route: Some(FocusRoute::Herdr {
                        pane_id,
                        socket: socket.clone().filter(|_| machine.is_none()),
                        machine: machine.as_ref().map(|(id, _)| id.clone()),
                        agent_session_id,
                    }),
                }
            })
            .collect(),
    )
}

pub(crate) fn snapshot(parse: Parser) -> AdapterSnapshot {
    let herdr = binary();
    // Remote SSH work never blocks the local discovery path.
    let mut agents = remote::snapshot(&herdr);
    let sessions = registered_sessions();
    let mut available = !agents.is_empty();
    for batch in sessions.chunks(MAX_CONCURRENT_SESSION_QUERIES) {
        let workers: Vec<_> = batch
            .iter()
            .cloned()
            .map(|socket| {
                let herdr = herdr.clone();
                std::thread::spawn(move || query_session(&herdr, socket, None, parse))
            })
            .collect();
        for worker in workers {
            if let Ok(Some(mut session_agents)) = worker.join() {
                available = true;
                agents.append(&mut session_agents);
            }
        }
    }
    agents.sort_by(|left, right| left.view.id.cmp(&right.view.id));
    agents.dedup_by(|left, right| left.owner_key == right.owner_key);
    AdapterSnapshot { available, agents }
}

pub(crate) fn remote_identity(machine: &str, session: &str) -> (String, String) {
    let namespace = format!("machine:{}", opaque(machine));
    (
        format!("herdr:{namespace}:{}", opaque(session)),
        format!("herdr-session:{namespace}:{}", opaque(session)),
    )
}
