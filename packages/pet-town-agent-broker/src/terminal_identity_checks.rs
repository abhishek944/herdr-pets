use crate::{agents, herdr, remote_checks::FakeHerdr, terminal::bind_route, FocusRoute};
use std::fs;
use std::path::PathBuf;

#[test]
fn remote_observe_without_native_session_binds_and_revalidates_terminal() {
    let fake = FakeHerdr::new();
    let path = PathBuf::from(&fake.binary);
    let native = fs::read_to_string(&path).unwrap();
    let fallback = native.replace(
        r#""agent_session":{"value":"shared-session"}"#,
        r#""terminal_id":"terminal-one""#,
    );
    fs::write(&path, &fallback).unwrap();
    let agent = herdr::query_session(
        &fake.binary,
        None,
        Some(("m1".into(), "Build".into())),
        agents::parse_agent_list,
        None,
    )
    .unwrap()
    .remove(0);
    assert!(agent.view.supports_observe);
    assert!(!agent.view.supports_terminal);
    let route = agent.focus_route.unwrap();
    let target = bind_route(fake.binary.clone(), route.clone()).unwrap();
    assert!(target.stream_command(false, false, 80, 24).is_ok());
    assert!(target.stream_command(true, false, 80, 24).is_err());
    assert!(target.stream_command(false, true, 80, 24).is_err());
    // Reconnect must bind to the exact terminal from the latest discovery.
    fs::write(&path, fallback.replace("terminal-one", "terminal-two")).unwrap();
    assert!(target.verify().is_err());
    assert!(bind_route(fake.binary.clone(), route.clone()).is_err());
    // A missing identity must not compare equal to a missing response field.
    fs::write(
        &path,
        fallback.replace(r#""terminal_id":"terminal-one""#, r#""terminal_id":null"#),
    )
    .unwrap();
    assert!(target.verify().is_err());
    let mut missing = route.clone();
    if let FocusRoute::Herdr { terminal_id, .. } = &mut missing {
        *terminal_id = None;
    }
    assert!(bind_route(fake.binary.clone(), missing).is_err());
    // A newly reported native session changes the companion identity.
    fs::write(&path, native).unwrap();
    assert!(target.verify().is_err());
    // Fallback never unlocks the local input path.
    let mut local = route;
    if let FocusRoute::Herdr { machine, .. } = &mut local {
        *machine = None;
    }
    assert!(bind_route(fake.binary.clone(), local).is_err());
}
