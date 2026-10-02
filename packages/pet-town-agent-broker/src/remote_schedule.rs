use crate::remote::{Machine, MAX_CONCURRENT_QUERIES, POLL_INTERVAL};
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct Entry {
    machine: Machine,
    next: Instant,
    failures: u32,
    healthy: bool,
    running: bool,
    generation: u64,
}

#[derive(Default)]
pub(crate) struct Schedule {
    entries: HashMap<String, Entry>,
    generation: u64,
}

impl Schedule {
    pub(crate) fn reconcile(&mut self, machines: Vec<Machine>, now: Instant) {
        self.entries
            .retain(|id, _| machines.iter().any(|machine| &machine.id == id));
        for machine in machines {
            self.generation += 1;
            let entry = self
                .entries
                .entry(machine.id.clone())
                .or_insert_with(|| Entry {
                    machine: machine.clone(),
                    next: now,
                    failures: 0,
                    healthy: false,
                    running: false,
                    generation: self.generation,
                });
            entry.machine = machine;
        }
    }

    pub(crate) fn next(&mut self, now: Instant, active: usize) -> Option<(Machine, u64)> {
        if active >= MAX_CONCURRENT_QUERIES {
            return None;
        }
        let reserve = self.entries.values().any(|entry| entry.healthy)
            && active >= MAX_CONCURRENT_QUERIES - 1;
        let entry = self
            .entries
            .values_mut()
            .filter(|entry| !entry.running && entry.next <= now && (!reserve || entry.healthy))
            .min_by(|left, right| {
                (!left.healthy, left.next, &left.machine.id).cmp(&(
                    !right.healthy,
                    right.next,
                    &right.machine.id,
                ))
            })?;
        entry.running = true;
        entry.next = now + POLL_INTERVAL;
        Some((entry.machine.clone(), entry.generation))
    }

    pub(crate) fn complete(
        &mut self,
        id: &str,
        generation: u64,
        success: bool,
        now: Instant,
    ) -> bool {
        let Some(entry) = self
            .entries
            .get_mut(id)
            .filter(|entry| entry.generation == generation)
        else {
            return false;
        };
        entry.running = false;
        entry.healthy = success;
        entry.failures = if success {
            0
        } else {
            (entry.failures + 1).min(4)
        };
        if !success {
            let delay = (POLL_INTERVAL * 2_u32.pow(entry.failures)).min(Duration::from_secs(60));
            entry.next = now + delay;
        }
        true
    }
}

#[cfg(test)]
#[path = "remote_schedule_checks.rs"]
mod checks;
