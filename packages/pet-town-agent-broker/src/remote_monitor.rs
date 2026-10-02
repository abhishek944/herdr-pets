use crate::{herdr, remote, remote_schedule::Schedule, AdapterAgent};
use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub struct RemoteMonitor {
    cancelled: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl RemoteMonitor {
    pub fn start() -> Self {
        Self::with_binary(herdr::binary())
    }

    pub(crate) fn with_binary(binary: OsString) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = cancelled.clone();
        let worker = thread::spawn(move || monitor(binary, stop));
        Self {
            cancelled,
            worker: Mutex::new(Some(worker)),
        }
    }

    pub fn stop(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            let _ = worker.join();
        }
    }
}

impl Drop for RemoteMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

struct Query {
    id: String,
    generation: u64,
    worker: JoinHandle<Option<Vec<AdapterAgent>>>,
}

fn monitor(binary: OsString, cancelled: Arc<AtomicBool>) {
    let mut schedule = Schedule::default();
    let mut queries: Vec<Query> = Vec::new();
    let mut next_catalog = Instant::now();
    while !cancelled.load(Ordering::Relaxed) {
        if Instant::now() >= next_catalog {
            if let Some(machines) = remote::catalog(&binary, Some(&cancelled)) {
                remote::retain(&binary, &machines);
                schedule.reconcile(machines, Instant::now());
            }
            next_catalog = Instant::now() + remote::POLL_INTERVAL;
        }
        let mut index = 0;
        while index < queries.len() {
            if !queries[index].worker.is_finished() {
                index += 1;
                continue;
            }
            let query = queries.swap_remove(index);
            let result = query.worker.join().ok().flatten();
            if schedule.complete(
                &query.id,
                query.generation,
                result.is_some(),
                Instant::now(),
            ) {
                remote::record(&binary, query.id, result);
            }
        }
        while !cancelled.load(Ordering::Relaxed) {
            let Some((machine, generation)) = schedule.next(Instant::now(), queries.len()) else {
                break;
            };
            let binary = binary.clone();
            let cancelled = cancelled.clone();
            queries.push(Query {
                id: machine.id.clone(),
                generation,
                worker: thread::spawn(move || remote::query(&binary, &machine, Some(&cancelled))),
            });
        }
        thread::sleep(Duration::from_millis(20));
    }
    for query in queries {
        let _ = query.worker.join();
    }
}
