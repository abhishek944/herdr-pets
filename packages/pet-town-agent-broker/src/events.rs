use crate::agents::AgentView;
use crate::token_activity::now_seconds as now;
use crate::{event_focus, AdapterAgent, AdapterSnapshot};
use fs2::FileExt;
use serde::Deserialize;
use std::fs::{self, OpenOptions};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

const INPUT_LIMIT: usize = 64 * 1024;
const RECORD_VERSION: u8 = 1;
const MAX_AGE_SECONDS: u64 = 24 * 60 * 60;
const CODEX_DONE_SECONDS: u64 = 5 * 60;
const SOURCES: [&str; 6] = ["claude", "codex", "opencode", "pi", "factory", "cursor"];

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EventRecord {
    version: u8,
    source: String,
    session_key: String,
    state: String,
    label: String,
    observed_at_seconds: u64,
    hosted_owner_key: Option<String>,
    focus_app: Option<String>,
    #[serde(default)]
    codex_thread_id: Option<String>,
    #[serde(default)]
    token_activity: Option<crate::TokenActivity>,
    #[serde(default, rename = "stateObservedAtMs")]
    _state_observed_at_ms: u64,
}

fn directory() -> Option<PathBuf> {
    Some(
        PathBuf::from(std::env::var_os("HOME")?)
            .join(".pet-town")
            .join("agent-sessions"),
    )
}

fn disabled(record: &EventRecord) -> bool {
    directory()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .map(|path| path.join("disabled-adapters").join(&record.source))
        .is_some_and(|path| path.is_file())
}

static LAST_RECORDS: OnceLock<Mutex<Option<Vec<EventRecord>>>> = OnceLock::new();

fn cached_records() -> Option<Vec<EventRecord>> {
    LAST_RECORDS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()?
        .clone()
}

fn read_records() -> Option<Vec<EventRecord>> {
    let directory = directory()?;
    if !directory.is_dir() {
        return None;
    }
    #[cfg(unix)]
    if fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).is_err() {
        return cached_records();
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let Ok(lock) = options.open(directory.join(".registry-lock")) else {
        return cached_records();
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    while lock.try_lock_exclusive().is_err() {
        if Instant::now() >= deadline {
            return cached_records();
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let current = now();
    let mut records: Vec<EventRecord> = fs::read_dir(&directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let extension = path.extension().and_then(|value| value.to_str());
            if extension == Some("ended") {
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0);
                if current.saturating_sub(modified) > MAX_AGE_SECONDS {
                    let _ = fs::remove_file(path);
                }
                return None;
            }
            if extension.is_some_and(|value| value.starts_with("tmp-")) {
                let stale = entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|time| time.elapsed().ok())
                    .is_some_and(|age| age.as_secs() > 60 * 60);
                if stale {
                    let _ = fs::remove_file(path);
                }
                return None;
            }
            if extension != Some("json")
                || !entry.file_type().ok().is_some_and(|kind| kind.is_file())
            {
                return None;
            }
            let mut file_options = OpenOptions::new();
            file_options.read(true);
            #[cfg(unix)]
            file_options.custom_flags(libc::O_NOFOLLOW);
            let record = file_options.open(&path).ok().and_then(|file| {
                let mut bytes = Vec::new();
                file.take((INPUT_LIMIT + 1) as u64)
                    .read_to_end(&mut bytes)
                    .ok()?;
                (bytes.len() <= INPUT_LIMIT)
                    .then(|| serde_json::from_slice::<EventRecord>(&bytes).ok())
                    .flatten()
            });
            let valid = record.as_ref().is_some_and(|record| {
                record.version == RECORD_VERSION
                    && SOURCES.contains(&record.source.as_str())
                    && current.saturating_sub(record.observed_at_seconds) <= MAX_AGE_SECONDS
            });
            if !valid {
                let _ = fs::remove_file(path);
            }
            record.filter(|record| valid && !disabled(record))
        })
        .collect();
    records.sort_by(|left, right| left.session_key.cmp(&right.session_key));
    let _ = FileExt::unlock(&lock);
    if let Ok(mut cached) = LAST_RECORDS.get_or_init(|| Mutex::new(None)).lock() {
        *cached = Some(records.clone());
    }
    Some(records)
}

pub(crate) fn snapshot() -> AdapterSnapshot {
    let Some(records) = read_records() else {
        return AdapterSnapshot {
            available: false,
            agents: Vec::new(),
        };
    };
    let current = now();
    let agents = records
        .into_iter()
        .map(|record| {
            let status = if record.source == "codex"
                && record.state == "done"
                && current.saturating_sub(record.observed_at_seconds) >= CODEX_DONE_SECONDS
            {
                "idle".to_string()
            } else {
                record.state
            };
            let id = format!("{}:{}", record.source, record.session_key);
            let focus_route =
                event_focus::route(&record.source, record.codex_thread_id, record.focus_app);
            AdapterAgent {
                owner_key: id.clone(),
                hosted_owner_key: record.hosted_owner_key,
                view: AgentView {
                    id,
                    status,
                    label: record.label,
                    source: record.source,
                    token_activity: record
                        .token_activity
                        .filter(|sample| sample.is_fresh(crate::token_activity::now_ms())),
                    ..AgentView::default()
                },
                focus_route,
            }
        })
        .collect();
    AdapterSnapshot {
        available: true,
        agents,
    }
}
