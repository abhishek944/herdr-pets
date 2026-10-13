use crate::terminal_remote::Observer;
use std::process::Command;

#[test]
fn observer_stream_preserves_native_frames_and_session_arguments() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("pet-town-observe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let binary = dir.join("herdr");
    std::fs::write(&binary, r#"#!/bin/sh
if [ "$1 $2 $3" = 'machine list --json' ]; then
 printf '[{"id":"m1","target":"user@host","session":"remote agents","enabled":true}]'; exit
fi
printf '%s\n' "$*" > "$0.args"
printf '{"type":"terminal.frame","seq":1,"encoding":"ansi","width":80,"height":24,"full":true,"bytes":"SGVsbG8="}\n'
printf '{"type":"terminal.closed","reason":"ended"}\n'
"#).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let observer = Observer::load(&binary.clone().into_os_string(), "m1").unwrap();
    let command = observer.command("w1:p1", 80, 24);
    let remote_script = command.get_args().last().unwrap();
    // Simulate SSH's remote shell using a fake remote Herdr, without networking.
    let path = format!("{}:/usr/bin:/bin", dir.display());
    let mut child = Command::new("/bin/sh")
        .args(["-c", remote_script.to_str().unwrap()])
        .env("PATH", path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::BufRead;
    let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut lines = String::new();
    for _ in 0..2 {
        output.read_line(&mut lines).unwrap();
    }
    assert!(child.wait().unwrap().success());
    let frames: Vec<serde_json::Value> = lines
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(frames[0]["bytes"], "SGVsbG8=");
    assert_eq!(frames[1]["type"], "terminal.closed");
    assert_eq!(
        std::fs::read_to_string(dir.join("herdr.args"))
            .unwrap()
            .trim(),
        "--session remote agents terminal session observe w1:p1 --cols 80 --rows 24"
    );
    std::fs::write(&binary, "#!/bin/sh\nprintf '[]'").unwrap();
    assert!(observer.verify_profile(&binary.into_os_string()).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn remote_binding_revalidates_identity_and_rejects_input_modes() {
    use crate::{remote_checks::FakeHerdr, terminal::bind_route, FocusRoute};
    let fake = FakeHerdr::new();
    let route = |session: &str| FocusRoute::Herdr {
        pane_id: "w1:p1".into(),
        socket: Some("/wrong/local/socket".into()),
        machine: Some("m1".into()),
        agent_session_id: session.into(),
        terminal_id: None,
    };
    let target = bind_route(fake.binary.clone(), route("shared-session")).unwrap();
    assert!(target.validate_mode(false, false).is_ok());
    assert!(target.stream_command(true, false, 80, 24).is_err());
    assert!(target.stream_command(false, true, 80, 24).is_err());
    assert_eq!(
        target
            .stream_command(false, false, 80, 24)
            .unwrap()
            .get_program(),
        "ssh"
    );
    assert!(bind_route(fake.binary.clone(), route("old-session")).is_err());
    assert!(bind_route(fake.binary.clone(), route("ephemeral:w1:p1")).is_err());
    let script_path = std::path::PathBuf::from(&fake.binary);
    let script = std::fs::read_to_string(&script_path).unwrap();
    std::fs::write(
        &script_path,
        script.replace("shared-session", "replacement-session"),
    )
    .unwrap();
    assert!(
        target.verify().is_err(),
        "A replaced native session must terminate observation"
    );
    std::fs::write(&script_path, script).unwrap();
    std::fs::write(fake.directory.join("herdr.removed"), "").unwrap();
    assert!(target.verify().is_err());
}

#[test]
fn closing_observer_lifetime_pipe_reaps_an_idle_remote_viewer() {
    use crate::remote_checks::FakeHerdr;
    use std::io::BufRead;
    use std::process::Stdio;
    let fake = FakeHerdr::new();
    let observer = Observer::load(&fake.binary, "m1").unwrap();
    std::fs::write(
        std::path::PathBuf::from(&fake.binary),
        r#"#!/bin/sh
printf '%s' "$$" > "$0.viewer-pid"
printf '{"type":"terminal.frame","seq":1,"full":true}\n'
exec /bin/sleep 60
"#,
    )
    .unwrap();
    let command = observer.command("w1:p1", 80, 24);
    let script = command.get_args().last().unwrap();
    let mut child = Command::new("/bin/sh")
        .args(["-c", script.to_str().unwrap()])
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fake.directory.display()),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut frame = String::new();
    stdout.read_line(&mut frame).unwrap();
    assert!(frame.contains("terminal.frame"));
    drop(child.stdin.take());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            let pid: i32 = std::fs::read_to_string(fake.directory.join("herdr.viewer-pid"))
                .unwrap()
                .parse()
                .unwrap();
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            panic!("Closing Observe left an idle remote viewer running");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let pid: i32 = std::fs::read_to_string(fake.directory.join("herdr.viewer-pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "Remote viewer was not reaped"
    );
}
