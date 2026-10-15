use crate::{agents, command, herdr, AdapterAgent};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(5);
const STALE_AFTER: Duration = Duration::from_secs(10);
const EXPIRE_AFTER: Duration = Duration::from_secs(30);
pub(crate) const MAX_CONCURRENT_QUERIES: usize = 8;

#[derive(Clone, Deserialize)]
pub(crate) struct Machine {
    pub(crate) id: String,
    pub(crate) label: String,
    enabled: bool,
}

pub(crate) fn parse_machines(text: &str) -> Option<Vec<Machine>> {
    let mut machines = serde_json::from_str::<Vec<Machine>>(text).ok()?;
    machines.retain(|machine| machine.enabled && !machine.id.trim().is_empty());
    machines.sort_by(|left, right| left.id.cmp(&right.id));
    machines.dedup_by(|left, right| left.id == right.id);
    for machine in &mut machines {
        machine.label = agents::safe_display_label(Some(&machine.label))
            .unwrap_or_else(|| "Remote".to_string());
    }
    Some(machines)
}

struct Record {
    updated: Instant,
    agents: Vec<AdapterAgent>,
}

type Cache = HashMap<String, Record>;
static CACHE: OnceLock<Mutex<HashMap<OsString, Cache>>> = OnceLock::new();

fn update(binary: &OsString, update: impl FnOnce(&mut Cache)) {
    if let Ok(mut caches) = CACHE.get_or_init(Default::default).lock() {
        update(caches.entry(binary.clone()).or_default());
    }
}

pub(crate) fn catalog(binary: &OsString, cancelled: Option<&AtomicBool>) -> Option<Vec<Machine>> {
    let args = ["machine", "list", "--json"].map(String::from);
    let text = command::run_cancellable(binary, None, None, &args, cancelled)?;
    parse_machines(&text)
}

pub(crate) fn retain(binary: &OsString, machines: &[Machine]) {
    update(binary, |cache| {
        cache.retain(|id, _| machines.iter().any(|machine| &machine.id == id));
    });
}

pub(crate) fn query(
    binary: &OsString,
    machine: &Machine,
    cancelled: Option<&AtomicBool>,
) -> Option<Vec<AdapterAgent>> {
    herdr::query_session(
        binary,
        None,
        Some((machine.id.clone(), machine.label.clone())),
        agents::parse_agent_list,
        cancelled,
    )
}

pub(crate) fn record(binary: &OsString, id: String, result: Option<Vec<AdapterAgent>>) {
    update(binary, |cache| {
        if let Some(agents) = result {
            cache.insert(
                id,
                Record {
                    updated: Instant::now(),
                    agents,
                },
            );
        } else if let Some(record) = cache.get_mut(&id) {
            for agent in &mut record.agents {
                agent.view.status = "unknown".to_string();
            }
        }
    });
}

pub(crate) fn refresh(binary: &OsString) {
    let Some(machines) = catalog(binary, None) else {
        return;
    };
    retain(binary, &machines);
    let queue = Mutex::new(machines.into_iter());
    std::thread::scope(|scope| {
        for _ in 0..MAX_CONCURRENT_QUERIES {
            let queue = &queue;
            scope.spawn(move || loop {
                let machine = queue.lock().unwrap().next();
                let Some(machine) = machine else {
                    break;
                };
                let result = query(binary, &machine, None);
                record(binary, machine.id, result);
            });
        }
    });
}

pub(crate) fn snapshot(binary: &OsString) -> Vec<AdapterAgent> {
    let mut agents = Vec::new();
    update(binary, |cache| {
        cache.retain(|_, record| record.updated.elapsed() < EXPIRE_AFTER);
        for record in cache.values() {
            agents.extend(record.agents.iter().cloned().map(|mut agent| {
                if record.updated.elapsed() >= STALE_AFTER {
                    agent.view.status = "unknown".to_string();
                }
                agent
            }));
        }
    });
    agents
}

#[cfg(all(test, unix))]
#[path = "remote_cache_checks.rs"]
mod cache_checks;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_profiles_only_and_labels_are_sanitized() {
        let machines = parse_machines(
            r#"[
            {"id":"m1","label":"Build\nbox","enabled":true,"target":"ignored","selected":false},
            {"id":"m2","label":"Disabled","enabled":false},
            {"id":"","label":"Invalid","enabled":true}
        ]"#,
        )
        .unwrap();
        assert_eq!(machines.len(), 1);
        assert_eq!(machines[0].id, "m1");
        assert_eq!(machines[0].label, "Buildbox");
        assert!(parse_machines("not json").is_none());
        assert!(parse_machines(r#"[{"id":"m1","label":"missing enabled"}]"#).is_none());
    }

    #[test]
    fn machine_identity_separates_identical_remote_sessions() {
        let first = herdr::remote_identity("m1", "same-session");
        assert_ne!(first, herdr::remote_identity("m2", "same-session"));
        assert_eq!(first, herdr::remote_identity("m1", "same-session"));
        assert!(first.0.starts_with("herdr:machine:"));
    }
}
