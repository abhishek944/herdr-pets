mod agents;
mod application_focus;
mod command;
mod event_focus;
mod events;
mod focus;
mod herdr;
mod labels;
#[cfg(all(target_os = "macos", not(test)))]
mod macos_activation;
mod remote;
#[cfg(all(test, unix))]
mod remote_checks;
mod remote_route;
mod state;
mod terminal;
mod terminal_api;
mod terminal_socket;
mod token_activity;

use agents::parse_agent_list;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub use agents::AgentView;
pub use focus::{focus_route, focus_route_with_codex_activation};
pub use terminal::{terminal_target, TerminalTarget};
pub use token_activity::{TokenActivity, TokenMeasurement};

/// Revalidate a remote route in short-lived CLI/focus helper processes.
pub fn remote_focus_route(id: &str) -> Option<FocusRoute> {
    remote_route::resolve(&herdr::binary(), id)
}

/// Explicit synchronous refresh for the one-shot snapshot CLI.
pub fn refresh_remote() {
    remote::refresh(&herdr::binary());
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FocusRoute {
    Herdr {
        pane_id: String,
        socket: Option<String>,
        #[serde(default)]
        machine: Option<String>,
        agent_session_id: String,
    },
    Application {
        bundle_id: String,
    },
    Codex {
        thread_id: Option<String>,
        fallback_bundle_id: Option<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSnapshot {
    pub available: bool,
    pub agents: Vec<AgentView>,
}

#[derive(Debug)]
pub struct BrokerSnapshot {
    pub snapshot: AgentSnapshot,
    pub focus_routes: HashMap<String, FocusRoute>,
}

#[derive(Clone, Debug)]
struct AdapterAgent {
    owner_key: String,
    hosted_owner_key: Option<String>,
    view: AgentView,
    focus_route: Option<FocusRoute>,
}

fn source_priority(source: &str) -> u8 {
    match source {
        "herdr" => 0,
        _ => 1,
    }
}

pub fn collect() -> BrokerSnapshot {
    let snapshots = [herdr::snapshot(parse_agent_list), events::snapshot()];
    let mut available = false;
    let mut by_owner = BTreeMap::<String, AdapterAgent>::new();

    for snapshot in snapshots {
        available |= snapshot.available;
        for agent in snapshot.agents {
            let owner = agent
                .hosted_owner_key
                .as_ref()
                .filter(|candidate| by_owner.contains_key(*candidate))
                .cloned()
                .unwrap_or_else(|| agent.owner_key.clone());
            let replace = by_owner.get(&owner).is_some_and(|current| {
                source_priority(&agent.view.source) < source_priority(&current.view.source)
            });
            if replace || !by_owner.contains_key(&owner) {
                by_owner.insert(owner, agent);
            } else if let Some(current) = by_owner.get_mut(&owner) {
                // Herdr remains authoritative for identity/state/focus. Its validated
                // harness adapter may enrich that same pet with numeric activity.
                if agent.view.token_activity.as_ref().is_some_and(|sample| {
                    current
                        .view
                        .token_activity
                        .as_ref()
                        .is_none_or(|old| sample.observed_at_ms > old.observed_at_ms)
                }) {
                    current.view.token_activity = agent.view.token_activity;
                }
            }
        }
    }

    let mut agents = Vec::with_capacity(by_owner.len());
    let mut focus_routes = HashMap::new();
    for agent in by_owner.into_values() {
        if let Some(route) = agent.focus_route {
            focus_routes.insert(agent.view.id.clone(), route);
        }
        agents.push(agent.view);
    }
    agents.sort_by(|left, right| left.id.cmp(&right.id));
    BrokerSnapshot {
        snapshot: AgentSnapshot { available, agents },
        focus_routes,
    }
}

struct AdapterSnapshot {
    available: bool,
    agents: Vec<AdapterAgent>,
}
