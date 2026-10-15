#[cfg(target_os = "macos")]
pub(crate) fn focus_codex(thread_id: Option<&str>, fallback: Option<&str>) -> Result<bool, String> {
    if let Ok(application_path) = activate_application("com.openai.codex") {
        if let Some(id) = thread_id.filter(|id| valid_codex_thread(id)) {
            let application_path = application_path.ok_or("Codex's bundle path is unavailable")?;
            let url = format!("codex://threads/{id}");
            let opened = std::process::Command::new("/usr/bin/open")
                .args(["-a", &application_path, &url])
                .status()
                .map_err(|error| error.to_string())?;
            if !opened.success() {
                return Err("Codex could not open this local conversation".into());
            }
        }
        return Ok(true);
    }
    fallback
        .map_or_else(|| Err("Codex is not running".into()), activate_application)
        .map(|_| false)
}

fn valid_codex_thread(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn focus_codex(
    _thread_id: Option<&str>,
    _fallback: Option<&str>,
) -> Result<bool, String> {
    Err("Codex focus is unavailable on this platform".into())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn activate_application(_bundle_id: &str) -> Result<Option<String>, String> {
    Err("application focus is unavailable on this platform".to_string())
}

#[cfg(target_os = "macos")]
pub(crate) fn activate_application(bundle_id: &str) -> Result<Option<String>, String> {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    use std::process::Command;

    let output = Command::new("/bin/ps")
        .env_remove("OPENAI_API_KEY")
        .args(["-axo", "pid="])
        .output()
        .map_err(|_| "could not inspect running applications".to_string())?;
    if !output.status.success() || output.stdout.len() > 1024 * 1024 {
        return Err("could not inspect running applications".to_string());
    }
    for pid in String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
    {
        let Some(application) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        else {
            continue;
        };
        if application
            .bundleIdentifier()
            .is_none_or(|identifier| identifier.to_string() != bundle_id)
        {
            continue;
        }
        // Preserve the running installation when apps share a bundle ID.
        let path = application
            .bundleURL()
            .and_then(|url| url.path())
            .map(|path| path.to_string());
        application.unhide();
        return application
            .activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows)
            .then_some(path)
            .ok_or_else(|| "macOS refused to activate the agent application".to_string());
    }
    Err("the agent application is not running".to_string())
}
