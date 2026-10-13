//! Same-user, launch-only control channel; the desktop remains the Godot owner.
use std::{
    fs,
    io::{Read, Write},
    os::unix::{fs::DirBuilderExt, fs::PermissionsExt, net::UnixListener, net::UnixStream},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn directory() -> Result<PathBuf, String> {
    let parent = crate::app_singleton::lock_path()
        .parent()
        .ok_or("Application state directory is unavailable.")?
        .join("launch-control");
    fs::create_dir_all(
        parent
            .parent()
            .ok_or("Application state directory is unavailable.")?,
    )
    .map_err(|error| error.to_string())?;
    match fs::DirBuilder::new().mode(0o700).create(&parent) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.to_string()),
    }
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(&parent).map_err(|error| error.to_string())?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err("Application launch directory must be owned by you and private (0700).".into());
    }
    Ok(parent)
}

pub fn open() -> Result<(), String> {
    let path = directory()?.join("town.sock");
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut started = false;
    let mut stream = loop {
        match UnixStream::connect(&path) {
            Ok(stream) => break stream,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                if !started {
                    Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .map_err(|e| format!("Could not start Pet Town: {e}"))?;
                    started = true;
                }
                if Instant::now() >= deadline {
                    return Err("Pet Town launch command is unavailable. Restart the desktop with the updated app.".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(format!("Could not connect to Pet Town: {error}")),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut stream)
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Pet Town launch reply unavailable: {e}. The request may have run; check the game before retrying."))?;
    if bytes.len() > 4096 {
        return Err("Pet Town launch reply exceeded its limit.".into());
    }
    let reply: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if reply["ok"] == true {
        println!("{reply}");
        Ok(())
    } else {
        Err(reply["error"]
            .as_str()
            .unwrap_or("Pet Town launch failed.")
            .into())
    }
}

pub(crate) fn start(app: &tauri::AppHandle) -> Result<(), String> {
    // Only the singleton lock owner reaches setup, so stale socket removal is safe.
    let path = directory()?.join("town.sock");
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let listener = UnixListener::bind(&path).map_err(|e| e.to_string())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    let app = app.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            // Connecting is the entire launch-only request. No caller data is executed.
            let (send, receive) = std::sync::mpsc::channel();
            let target = app.clone();
            let result = app
                .run_on_main_thread(move || {
                    let _ = send.send(crate::town_process::open(&target));
                })
                .map_err(|e| e.to_string())
                .and_then(|()| {
                    receive.recv_timeout(Duration::from_secs(10)).map_err(|_| {
                        "Pet Town launch timed out; check the game before retrying.".to_string()
                    })?
                });
            let reply = match result {
                Ok(()) => serde_json::json!({"ok":true,"status":"launch-dispatched"}),
                Err(error) => serde_json::json!({"ok":false,"error":error}),
            };
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            let _ = stream.write_all(reply.to_string().as_bytes());
        }
    });
    Ok(())
}
