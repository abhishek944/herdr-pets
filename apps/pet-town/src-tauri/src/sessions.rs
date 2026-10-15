use pet_town_agent_broker::{AgentSnapshot, BrokerSnapshot, FocusRoute};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct MayorSessionOwner {
    pane: String,
    log: PathBuf,
}

fn active_mayor_owner() -> Option<MayorSessionOwner> {
    let path = crate::preferences_io::preferences_path().ok()?;
    let firstmate_active = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .is_some_and(|preferences| preferences["app"]["orchestrator"]["enabled"] == true);
    if !firstmate_active {
        return None;
    }
    let owner = std::fs::read(path.with_file_name("firstmate-session.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<MayorSessionOwner>(&bytes).ok())?;
    (!owner.pane.is_empty() && owner.log.is_absolute()).then_some(owner)
}

fn is_mayor_primary(route: &FocusRoute, owner: &MayorSessionOwner) -> bool {
    matches!(route, FocusRoute::Herdr { pane_id, agent_session_id, machine: None, .. }
        if pane_id == &owner.pane && owner.log.as_path() == Path::new(agent_session_id))
}

pub(crate) fn mayor_primary_route() -> Option<FocusRoute> {
    let owner = active_mayor_owner()?;
    pet_town_agent_broker::collect()
        .focus_routes
        .into_values()
        .find(|route| is_mayor_primary(route, &owner))
}

pub(crate) fn collect_visible() -> BrokerSnapshot {
    let mut collected = pet_town_agent_broker::collect();
    let Some(owner) = active_mayor_owner() else {
        return collected;
    };
    let owned_ids: Vec<String> = collected
        .focus_routes
        .iter()
        .filter(|&(_, route)| is_mayor_primary(route, &owner))
        .map(|(id, _)| id.clone())
        .collect();
    collected
        .snapshot
        .agents
        .retain(|agent| !owned_ids.contains(&agent.id));
    for id in owned_ids {
        collected.focus_routes.remove(&id);
    }
    collected
}

fn collect_with_activity() -> BrokerSnapshot {
    let mut collected = collect_visible();
    // Pet Street must not depend on Pet Town being open for Codex telemetry.
    let usage = if collected.snapshot.agents.iter().any(|agent| {
        agent.status == "working" && matches!(agent.source.as_str(), "codex" | "herdr")
    }) {
        crate::codex_usage::refresh()
    } else {
        crate::codex_usage::Snapshot::default()
    };
    crate::codex_usage::attach_activity(&mut collected.snapshot.agents, &usage);
    collected
}

pub fn snapshot_json() -> String {
    pet_town_agent_broker::refresh_remote();
    let collected = collect_visible();
    serde_json::to_string(&collected.snapshot)
        .unwrap_or_else(|_| r#"{"available":false,"agents":[]}"#.to_string())
}

#[tauri::command]
pub(crate) async fn list_agents() -> Result<AgentSnapshot, String> {
    let collected = tauri::async_runtime::spawn_blocking(collect_with_activity)
        .await
        .unwrap_or_else(|_| BrokerSnapshot {
            snapshot: AgentSnapshot {
                available: false,
                agents: Vec::new(),
            },
            focus_routes: HashMap::new(),
        });
    Ok(collected.snapshot)
}
