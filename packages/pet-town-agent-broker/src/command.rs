use std::ffi::OsString;
use std::io::Read;
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
#[cfg(not(unix))]
use wait_timeout::ChildExt;

const HERDR_TIMEOUT: Duration = Duration::from_secs(2);
const REMOTE_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

fn terminate(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        let _ = libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn collect_output(
    child: &mut Child,
    mut stdout: ChildStdout,
    timeout: Duration,
) -> Option<(ExitStatus, Vec<u8>)> {
    use std::os::fd::AsRawFd;

    let descriptor = stdout.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        terminate(child);
        return None;
    }

    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.len() > MAX_OUTPUT_BYTES {
                        terminate(child);
                        return None;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    terminate(child);
                    return None;
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                unsafe {
                    let _ = libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                loop {
                    match stdout.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => {
                            bytes.extend_from_slice(&buffer[..count]);
                            if bytes.len() > MAX_OUTPUT_BYTES {
                                return None;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                return Some((status, bytes));
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                terminate(child);
                return None;
            }
        }
    }
}

#[cfg(not(unix))]
fn collect_output(
    child: &mut Child,
    mut stdout: ChildStdout,
    timeout: Duration,
) -> Option<(ExitStatus, Vec<u8>)> {
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((MAX_OUTPUT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let status = match child.wait_timeout(timeout) {
        Ok(Some(status)) => status,
        _ => {
            terminate(child);
            return None;
        }
    };
    let bytes = reader.join().ok()?.ok()?;
    (bytes.len() <= MAX_OUTPUT_BYTES).then_some((status, bytes))
}

pub(crate) fn run(herdr: &OsString, socket: Option<&str>, arguments: &[String]) -> Option<String> {
    run_on_machine(herdr, socket, None, arguments)
}

pub(crate) fn run_on_machine(
    herdr: &OsString,
    socket: Option<&str>,
    machine: Option<&str>,
    arguments: &[String],
) -> Option<String> {
    let mut command = Command::new(herdr);
    if let Some(id) = machine {
        command.args(["--machine", id]);
        // Never let a local pane or socket influence a remote request.
        for name in [
            "HERDR_SOCKET_PATH",
            "HERDR_SESSION",
            "HERDR_PANE_ID",
            "HERDR_TAB_ID",
            "HERDR_WORKSPACE_ID",
        ] {
            command.env_remove(name);
        }
    }
    command
        .env_remove("OPENAI_API_KEY")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(path) = socket.filter(|_| machine.is_none()) {
        command.env("HERDR_SOCKET_PATH", path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let (status, bytes) = collect_output(
        &mut child,
        stdout,
        if machine.is_some() {
            REMOTE_TIMEOUT
        } else {
            HERDR_TIMEOUT
        },
    )?;
    status
        .success()
        .then(|| String::from_utf8(bytes).ok())
        .flatten()
}
