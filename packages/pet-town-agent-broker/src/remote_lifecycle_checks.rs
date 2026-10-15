use crate::{command, remote_checks::FakeHerdr, RemoteMonitor};
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

fn hanging_remote(fake: &FakeHerdr) {
    fs::write(
        &fake.binary,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$0.log"
if [ "$1 $2 $3" = 'machine list --json' ]; then
    printf '[{"id":"m1","label":"Remote","enabled":true}]'
elif [ "$1" = '--machine' ]; then
    printf '%s' "$$" > "$0.remote-pid"
    exec sleep 60
elif [ "$1 $2" = 'agent list' ]; then
    sleep 0.3
    printf '{"result":{"agents":[]}}'
else
    printf '{"result":{"workspaces":[]}}'
fi
"#,
    )
    .unwrap();
}

fn remote_pid(fake: &FakeHerdr) -> i32 {
    fs::read_to_string(fake.directory.join("herdr.remote-pid"))
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn helper_entry() {
    if std::env::var_os("PET_TOWN_TEST_COLLECT").is_some() {
        crate::collect();
    }
}

#[test]
fn short_lived_collection_does_not_start_remote_work() {
    let fake = FakeHerdr::new();
    hanging_remote(&fake);
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "remote_lifecycle_checks::helper_entry"])
        .env("PET_TOWN_TEST_COLLECT", "1")
        .env("PET_TOWN_HERDR_BIN", &fake.binary)
        .env("PET_TOWN_SESSION_REGISTRY", fake.directory.join("sessions"))
        .env_remove("HERDR_SOCKET_PATH")
        .output()
        .unwrap();
    assert!(result.status.success());
    let log = fs::read_to_string(fake.directory.join("herdr.log")).unwrap();
    assert!(
        !log.contains("machine"),
        "helper started background discovery: {log}"
    );
    assert!(!fake.directory.join("herdr.remote-pid").exists());
}

#[test]
fn stopping_monitor_cancels_and_reaps_hanging_remote_commands() {
    let fake = FakeHerdr::new();
    hanging_remote(&fake);
    let monitor = RemoteMonitor::with_binary(fake.binary.clone());
    let deadline = Instant::now() + Duration::from_secs(3);
    while !fake.directory.join("herdr.remote-pid").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    let pid = remote_pid(&fake);
    let started = Instant::now();
    monitor.stop();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "remote child survived shutdown"
    );
    monitor.stop();
}

#[test]
fn remote_command_timeout_reaps_the_child() {
    let fake = FakeHerdr::new();
    hanging_remote(&fake);
    let started = Instant::now();
    assert!(command::run_on_machine(
        &fake.binary,
        None,
        Some("m1"),
        &["agent".into(), "list".into()]
    )
    .is_none());
    assert!(started.elapsed() >= Duration::from_secs(8));
    assert!(started.elapsed() < Duration::from_secs(11));
    assert_eq!(unsafe { libc::kill(remote_pid(&fake), 0) }, -1);
}
