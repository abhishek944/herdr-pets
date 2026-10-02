//! Saved SSH machine discovery is asynchronous: local pets never wait on SSH.
use crate::{agents, command, herdr, AdapterAgent};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const STALE_AFTER: Duration = Duration::from_secs(10);
const EXPIRE_AFTER: Duration = Duration::from_secs(30);
const MAX_CONCURRENT_QUERIES: usize = 8;

#[derive(Clone, Deserialize)]
pub(super) struct Machine {
    pub(super) id: String,
    pub(super) label: String,
    enabled: bool,
}

pub(super) fn parse_machines(text: &str) -> Option<Vec<Machine>> {
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

#[derive(Default)]
struct Cache {
    started: Option<Instant>,
    running: bool,
    machines: HashMap<String, Record>,
}

static CACHE: OnceLock<Mutex<HashMap<OsString, Cache>>> = OnceLock::new();

fn update(binary: &OsString, update: impl FnOnce(&mut Cache)) {
    if let Ok(mut caches) = CACHE.get_or_init(Default::default).lock() {
        update(caches.entry(binary.clone()).or_default());
    }
}

pub(crate) fn refresh(binary: &OsString) {
    update(binary, |cache| cache.started = Some(Instant::now()));
    let args = ["machine", "list", "--json"].map(String::from);
    let machines = command::run(binary, None, &args).and_then(|text| parse_machines(&text));
    let Some(machines) = machines else {
        // An old CLI or malformed catalog must not break local monitoring.
        // Existing remote records expire rather than being treated as live forever.
        return;
    };
    update(binary, |cache| {
        cache
            .machines
            .retain(|id, _| machines.iter().any(|machine| &machine.id == id));
    });
    for batch in machines.chunks(MAX_CONCURRENT_QUERIES) {
        let workers: Vec<_> = batch
            .iter()
            .cloned()
            .map(|machine| {
                let binary = binary.clone();
                std::thread::spawn(move || {
                    let result = herdr::query_session(
                        &binary,
                        None,
                        Some((machine.id.clone(), machine.label)),
                        agents::parse_agent_list,
                    );
                    update(&binary, |cache| {
                        if let Some(agents) = result {
                            cache.machines.insert(
                                machine.id,
                                Record {
                                    updated: Instant::now(),
                                    agents,
                                },
                            );
                        } else if let Some(record) = cache.machines.get_mut(&machine.id) {
                            // Keep a briefly disconnected pet visible, explicitly unknown.
                            for agent in &mut record.agents {
                                agent.view.status = "unknown".to_string();
                            }
                        }
                    });
                })
            })
            .collect();
        for worker in workers {
            let _ = worker.join();
        }
    }
}

pub(crate) fn snapshot(binary: &OsString) -> Vec<AdapterAgent> {
    let Ok(mut caches) = CACHE.get_or_init(Default::default).lock() else {
        return Vec::new();
    };
    let cache = caches.entry(binary.clone()).or_default();
    cache
        .machines
        .retain(|_, record| record.updated.elapsed() < EXPIRE_AFTER);
    let agents = cache
        .machines
        .values()
        .flat_map(|record| {
            record.agents.iter().cloned().map(|mut agent| {
                if record.updated.elapsed() >= STALE_AFTER {
                    agent.view.status = "unknown".to_string();
                }
                agent
            })
        })
        .collect();
    if !cache.running
        && cache
            .started
            .is_none_or(|started| started.elapsed() >= POLL_INTERVAL)
    {
        cache.running = true;
        cache.started = Some(Instant::now());
        let binary = binary.clone();
        // No overlapping poll rounds, even when an SSH command times out.
        std::thread::spawn(move || {
            let _ = std::panic::catch_unwind(|| refresh(&binary));
            update(&binary, |cache| cache.running = false);
        });
    }
    agents
}

#[cfg(test)]
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
