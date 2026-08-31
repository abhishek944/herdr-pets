use crate::herdr_command::run_herdr_command;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

const MAX_PROCESS_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Deserialize)]
struct SessionList {
    sessions: Vec<SessionRecord>,
}

#[derive(Deserialize)]
struct SessionRecord {
    name: String,
    socket_path: String,
}

#[derive(Clone, Debug)]
struct ProcessRecord {
    pid: i32,
    parent_pid: i32,
    arguments: Vec<String>,
}

fn session_name(herdr: &OsString, socket: Option<&str>) -> Option<String> {
    let Some(socket) = socket else {
        return Some("default".to_string());
    };
    let arguments = vec![
        "session".to_string(),
        "list".to_string(),
        "--json".to_string(),
    ];
    let text = run_herdr_command(herdr, None, &arguments)?;
    serde_json::from_str::<SessionList>(&text)
        .ok()?
        .sessions
        .into_iter()
        .find(|session| session.socket_path == socket)
        .map(|session| session.name)
}

fn process_snapshot() -> Result<Vec<ProcessRecord>, String> {
    let output = Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid=,command="])
        .output()
        .map_err(|_| "could not inspect the Herdr UI host".to_string())?;
    if !output.status.success() || output.stdout.len() > MAX_PROCESS_OUTPUT_BYTES {
        return Err("could not inspect the Herdr UI host".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_process)
        .collect())
}

fn parse_process(line: &str) -> Option<ProcessRecord> {
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let parent_pid = fields.next()?.parse().ok()?;
    let arguments = fields.map(str::to_string).collect();
    Some(ProcessRecord {
        pid,
        parent_pid,
        arguments,
    })
}

fn is_client(record: &ProcessRecord, binary_name: &str, session: &str) -> bool {
    let Some(executable) = record.arguments.first() else {
        return false;
    };
    if Path::new(executable)
        .file_name()
        .and_then(|name| name.to_str())
        != Some(binary_name)
    {
        return false;
    }
    let arguments: Vec<&str> = record
        .arguments
        .iter()
        .skip(1)
        .map(String::as_str)
        .collect();
    arguments.is_empty() && session == "default"
        || arguments == ["--session", session]
        || arguments == ["session", "attach", session]
}

fn unique_host_pid<F>(
    processes: &[ProcessRecord],
    binary_name: &str,
    session: &str,
    is_gui: F,
) -> Option<i32>
where
    F: Fn(i32) -> bool,
{
    let by_pid: HashMap<i32, &ProcessRecord> =
        processes.iter().map(|item| (item.pid, item)).collect();
    let mut hosts = HashSet::new();
    for client in processes
        .iter()
        .filter(|item| is_client(item, binary_name, session))
    {
        let mut next = client.parent_pid;
        for _ in 0..32 {
            if is_gui(next) {
                hosts.insert(next);
                break;
            }
            let Some(parent) = by_pid.get(&next) else {
                break;
            };
            next = parent.parent_pid;
        }
    }
    (hosts.len() == 1).then(|| *hosts.iter().next().unwrap())
}

pub(crate) fn activate_herdr_host(herdr: &OsString, socket: Option<&str>) -> Result<(), String> {
    let session = session_name(herdr, socket)
        .ok_or_else(|| "could not identify the Herdr session".to_string())?;
    let binary_name = Path::new(herdr)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("herdr");
    let processes = process_snapshot()?;
    let host_pid = unique_host_pid(&processes, binary_name, &session, |pid| {
        NSRunningApplication::runningApplicationWithProcessIdentifier(pid).is_some()
    })
    .ok_or_else(|| "could not identify one Herdr UI application".to_string())?;
    let application = NSRunningApplication::runningApplicationWithProcessIdentifier(host_pid)
        .ok_or_else(|| "the Herdr UI application exited".to_string())?;
    let options = NSApplicationActivationOptions::ActivateAllWindows;
    application.unhide();
    application
        .activateWithOptions(options)
        .then_some(())
        .ok_or_else(|| "macOS refused to activate the Herdr UI application".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: i32, parent_pid: i32, arguments: &[&str]) -> ProcessRecord {
        ProcessRecord {
            pid,
            parent_pid,
            arguments: arguments.iter().map(|value| value.to_string()).collect(),
        }
    }

    #[test]
    fn selects_only_the_matching_session_client_host() {
        let processes = vec![
            process(10, 1, &["/opt/herdr", "server"]),
            process(20, 30, &["herdr"]),
            process(21, 31, &["herdr", "--session", "other"]),
            process(30, 40, &["-zsh"]),
            process(31, 41, &["-zsh"]),
            process(40, 1, &["terminal-a"]),
            process(41, 1, &["terminal-b"]),
        ];
        assert_eq!(
            unique_host_pid(&processes, "herdr", "default", |pid| pid == 40),
            Some(40)
        );
        assert_eq!(
            unique_host_pid(&processes, "herdr", "other", |pid| pid == 41),
            Some(41)
        );
    }

    #[test]
    fn rejects_ambiguous_ui_hosts() {
        let processes = vec![process(20, 40, &["herdr"]), process(21, 41, &["herdr"])];
        assert_eq!(
            unique_host_pid(&processes, "herdr", "default", |pid| pid >= 40),
            None
        );
    }
}
