use crate::{agents, focus, herdr, remote, FocusRoute};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct FakeHerdr {
    pub(super) directory: PathBuf,
    pub(super) binary: OsString,
}
impl FakeHerdr {
    pub(super) fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "pet-town-remote-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("herdr");
        fs::write(&path, r#"#!/bin/sh
printf '%s\n' "$*" >> "$0.log"
if [ "$1 $2 $3" = 'machine list --json' ]; then
    sleep 0.2
    if [ -f "$0.removed" ]; then printf '[]'; exit; fi
    printf '[{"id":"m1","label":"Build","enabled":true},{"id":"m2","label":"Other","enabled":true},{"id":"offline","label":"Offline","enabled":true},{"id":"disabled","label":"Disabled","enabled":false}]'
    exit
fi
[ "$1" = '--machine' ] || exit 4
machine=$2
shift 2
[ "${HERDR_SOCKET_PATH-unset}" = unset ] || exit 5
[ "$machine" != offline ] || exit 6
case "$1 $2" in
    'agent list') printf '{"result":{"agents":[{"pane_id":"w1:p1","agent_status":"working","agent_session":{"value":"shared-session"},"cwd":"/srv/project","tab_id":"w1:t1","workspace_id":"w1"}]}}' ;;
    'agent get') printf '{"result":{"agent":{"agent_session":{"value":"shared-session"},"tab_id":"w1:t1","workspace_id":"w1"}}}' ;;
    'workspace focus'|'tab focus'|'agent focus') printf '{"result":{}}' ;;
    *) exit 7 ;;
esac
"#).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory,
            binary: path.into_os_string(),
        }
    }
    fn log(&self) -> String {
        fs::read_to_string(self.directory.join("herdr.log")).unwrap()
    }
}
impl Drop for FakeHerdr {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn remote_discovery_preserves_machine_routes_and_duplicate_session_ids() {
    let fake = FakeHerdr::new();
    let query = |machine: &str| {
        herdr::query_session(
            &fake.binary,
            Some("/wrong/local/socket".into()),
            Some((machine.into(), "Build".into())),
            agents::parse_agent_list,
            None,
        )
        .unwrap()
        .remove(0)
    };
    let first = query("m1");
    let second = query("m2");
    assert_ne!(first.owner_key, second.owner_key);
    assert_ne!(first.view.id, second.view.id);
    assert_eq!(first.view.label, "Build: project");
    assert_eq!(first.view.status, "working");
    assert!(!first.view.supports_terminal);
    assert_eq!(first.view.remote_machine.as_deref(), Some("Build"));
    assert_eq!(
        first.focus_route,
        Some(FocusRoute::Herdr {
            pane_id: "w1:p1".into(),
            socket: None,
            machine: Some("m1".into()),
            agent_session_id: "shared-session".into()
        })
    );
    assert!(fake.log().contains("--machine m2 agent list"));
}

#[test]
fn remote_focus_routes_every_command_and_rejects_replaced_agents() {
    let fake = FakeHerdr::new();
    focus::focus_herdr(
        &fake.binary,
        "w1:p1",
        Some("/wrong/local/socket"),
        Some("m2"),
        "shared-session",
    )
    .unwrap();
    let log = fake.log();
    for command in [
        "agent get w1:p1",
        "workspace focus w1",
        "tab focus w1:t1",
        "agent focus w1:p1",
    ] {
        assert!(log.contains(&format!("--machine m2 {command}")));
    }
    let lines_before = log.lines().count();
    assert!(focus::focus_herdr(&fake.binary, "w1:p1", None, Some("m2"), "old-session").is_err());
    assert_eq!(fake.log().lines().count(), lines_before + 1);
    assert!(focus::focus_herdr(
        &fake.binary,
        "w1:p1",
        None,
        Some("offline"),
        "shared-session"
    )
    .is_err());
}

#[test]
fn background_poll_returns_immediately_and_ignores_failed_or_disabled_machines() {
    let fake = FakeHerdr::new();
    let monitor = crate::RemoteMonitor::with_binary(fake.binary.clone());
    let started = Instant::now();
    assert!(remote::snapshot(&fake.binary).is_empty());
    assert!(started.elapsed() < Duration::from_millis(100));
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let agents = remote::snapshot(&fake.binary);
        if agents.len() == 2 {
            break;
        }
        assert!(Instant::now() < deadline, "remote pets were not discovered");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!fake.log().contains("--machine disabled"));
    monitor.stop();
}

#[test]
fn old_local_focus_routes_remain_deserializable() {
    let route: FocusRoute = serde_json::from_str(
        r#"{"kind":"herdr","pane_id":"w1:p1","socket":null,"agent_session_id":"local"}"#,
    )
    .unwrap();
    assert!(matches!(route, FocusRoute::Herdr { machine: None, .. }));
}

#[test]
fn cold_focus_helper_resolves_only_the_selected_machine_and_session() {
    let fake = FakeHerdr::new();
    let id = herdr::remote_identity("m2", "shared-session").0;
    let route = crate::remote_route::resolve(&fake.binary, &id).unwrap();
    assert!(matches!(route, FocusRoute::Herdr { machine: Some(machine), .. } if machine == "m2"));
    assert!(!fake.log().contains("--machine m1"));
    let old_id = herdr::remote_identity("m2", "old-session").0;
    assert!(crate::remote_route::resolve(&fake.binary, &old_id).is_none());
    assert!(crate::remote_route::resolve(&fake.binary, "herdr:default:local").is_none());
}
