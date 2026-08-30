use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{Manager, PhysicalPosition, PhysicalSize};
#[cfg(not(unix))]
use wait_timeout::ChildExt;

const WINDOW_BOTTOM_MARGIN: i32 = 8;
const HERDR_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_HERDR_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_CONCURRENT_SESSION_QUERIES: usize = 8;
const LABEL_CACHE_TTL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub id: String,
    pub status: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSnapshot {
    pub available: bool,
    pub agents: Vec<AgentView>,
}

#[derive(Debug, Deserialize)]
struct AgentListEnvelope {
    result: AgentListResult,
}

#[derive(Debug, Deserialize)]
struct AgentListResult {
    agents: Vec<RawAgent>,
}

#[derive(Debug, Deserialize)]
struct RawAgent {
    pane_id: String,
    #[serde(default)]
    agent_status: String,
    foreground_cwd: Option<String>,
    cwd: Option<String>,
    tab_id: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug)]
struct ParsedAgent {
    view: AgentView,
    tab_id: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaneListEnvelope {
    result: PaneListResult,
}

#[derive(Debug, Deserialize)]
struct PaneListResult {
    panes: Vec<RawPaneLabel>,
}

#[derive(Debug, Deserialize)]
struct RawPaneLabel {
    pane_id: String,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TabListEnvelope {
    result: TabListResult,
}

#[derive(Debug, Deserialize)]
struct TabListResult {
    tabs: Vec<RawTabLabel>,
}

#[derive(Debug, Deserialize)]
struct RawTabLabel {
    tab_id: String,
    label: Option<String>,
    number: Option<u32>,
}

#[derive(Clone, Default)]
struct SessionLabels {
    pane_labels: HashMap<String, String>,
    tab_labels: HashMap<String, String>,
}

#[derive(Default)]
struct CachedSessionLabels {
    updated_at: Option<Instant>,
    labels: SessionLabels,
}

static LABEL_CACHE: OnceLock<Mutex<HashMap<String, CachedSessionLabels>>> = OnceLock::new();

fn safe_project_name(path: Option<&str>) -> String {
    let name = path
        .and_then(|value| Path::new(value).file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("project");

    let cleaned: String = name
        .chars()
        .filter(|character| !character.is_control())
        .take(48)
        .collect();

    if cleaned.trim().is_empty() {
        "project".to_string()
    } else {
        cleaned
    }
}

fn safe_display_label(value: Option<&str>) -> Option<String> {
    let cleaned: String = value?
        .chars()
        .filter(|character| !character.is_control())
        .take(48)
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn normalized_status(status: &str) -> String {
    match status.to_ascii_lowercase().as_str() {
        "working" | "blocked" | "idle" | "done" => status.to_ascii_lowercase(),
        _ => "unknown".to_string(),
    }
}

fn parse_agent_list(text: &str) -> Result<Vec<ParsedAgent>, serde_json::Error> {
    let envelope: AgentListEnvelope = serde_json::from_str(text)?;
    let mut agents: Vec<ParsedAgent> = envelope
        .result
        .agents
        .into_iter()
        .filter(|agent| !agent.pane_id.trim().is_empty())
        .map(|agent| {
            let project_path = agent.foreground_cwd.as_deref().or(agent.cwd.as_deref());
            ParsedAgent {
                view: AgentView {
                    id: agent.pane_id,
                    status: normalized_status(&agent.agent_status),
                    label: safe_project_name(project_path),
                },
                tab_id: agent.tab_id,
                workspace_id: agent.workspace_id,
            }
        })
        .collect();
    agents.sort_by(|left, right| left.view.id.cmp(&right.view.id));
    agents.dedup_by(|left, right| left.view.id == right.view.id);
    Ok(agents)
}

fn unavailable_snapshot() -> AgentSnapshot {
    AgentSnapshot {
        available: false,
        agents: Vec::new(),
    }
}

fn herdr_binary() -> OsString {
    std::env::var_os("HERDR_PETS_HERDR_BIN")
        .or_else(|| std::env::var_os("HERDR_BIN_PATH"))
        .unwrap_or_else(|| "herdr".into())
}

fn registered_sessions() -> Vec<Option<String>> {
    let Some(directory) = std::env::var_os("HERDR_PETS_SESSION_REGISTRY") else {
        return vec![None];
    };
    let mut paths: Vec<PathBuf> = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|item| item.path()))
            .collect(),
        Err(_) => return vec![None],
    };
    paths.sort();

    let mut sockets: Vec<String> = paths
        .into_iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .map(|socket| socket.trim().to_string())
        .filter(|socket| !socket.is_empty() && Path::new(socket).exists())
        .collect();
    sockets.sort();
    sockets.dedup();

    if sockets.is_empty() {
        vec![None]
    } else {
        sockets.into_iter().map(Some).collect()
    }
}

fn session_namespace(socket: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in socket.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn terminate_process_group(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        // Each Herdr command is spawned in its own process group below.
        let _ = libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn collect_child_output(
    child: &mut Child,
    mut stdout: ChildStdout,
) -> Option<(ExitStatus, Vec<u8>)> {
    use std::os::fd::AsRawFd;

    let descriptor = stdout.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        terminate_process_group(child);
        return None;
    }

    let deadline = Instant::now() + HERDR_TIMEOUT;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.len() > MAX_HERDR_OUTPUT_BYTES {
                        terminate_process_group(child);
                        return None;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    terminate_process_group(child);
                    return None;
                }
            }
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                // Stop descendants that inherited stdout, then drain bytes already
                // buffered in the pipe without waiting for EOF from escaped children.
                unsafe {
                    let _ = libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                loop {
                    match stdout.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => {
                            bytes.extend_from_slice(&buffer[..count]);
                            if bytes.len() > MAX_HERDR_OUTPUT_BYTES {
                                return None;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                return Some((status, bytes));
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                terminate_process_group(child);
                return None;
            }
        }
    }
}

#[cfg(not(unix))]
fn collect_child_output(
    child: &mut Child,
    mut stdout: ChildStdout,
) -> Option<(ExitStatus, Vec<u8>)> {
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((MAX_HERDR_OUTPUT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let status = match child.wait_timeout(HERDR_TIMEOUT) {
        Ok(Some(status)) => status,
        _ => {
            terminate_process_group(child);
            return None;
        }
    };
    let bytes = reader.join().ok()?.ok()?;
    (bytes.len() <= MAX_HERDR_OUTPUT_BYTES).then_some((status, bytes))
}

fn run_herdr_command(
    herdr: &OsString,
    socket: Option<&str>,
    arguments: &[String],
) -> Option<String> {
    let mut command = Command::new(herdr);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(path) = socket {
        command.env("HERDR_SOCKET_PATH", path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let (status, bytes) = collect_child_output(&mut child, stdout)?;
    status
        .success()
        .then(|| String::from_utf8(bytes).ok())
        .flatten()
}

fn query_session_labels(
    herdr: &OsString,
    socket: Option<&str>,
    workspaces: &[String],
) -> SessionLabels {
    let mut labels = SessionLabels::default();
    for workspace in workspaces {
        let pane_arguments = vec![
            "pane".to_string(),
            "list".to_string(),
            "--workspace".to_string(),
            workspace.clone(),
        ];
        if let Some(envelope) = run_herdr_command(herdr, socket, &pane_arguments)
            .and_then(|text| serde_json::from_str::<PaneListEnvelope>(&text).ok())
        {
            for pane in envelope.result.panes {
                if let Some(label) = safe_display_label(pane.label.as_deref()) {
                    labels.pane_labels.insert(pane.pane_id, label);
                }
            }
        }

        let tab_arguments = vec![
            "tab".to_string(),
            "list".to_string(),
            "--workspace".to_string(),
            workspace.clone(),
        ];
        if let Some(envelope) = run_herdr_command(herdr, socket, &tab_arguments)
            .and_then(|text| serde_json::from_str::<TabListEnvelope>(&text).ok())
        {
            for tab in envelope.result.tabs {
                if let Some(label) = safe_display_label(tab.label.as_deref()) {
                    let is_default_number =
                        tab.number.is_some_and(|number| label == number.to_string());
                    if !is_default_number {
                        labels.tab_labels.insert(tab.tab_id, label);
                    }
                }
            }
        }
    }
    labels
}

fn labels_for_session(
    herdr: &OsString,
    socket: Option<&str>,
    workspaces: &[String],
) -> SessionLabels {
    let key = format!(
        "{}\0{}",
        socket.unwrap_or("<current-session>"),
        workspaces.join("\0")
    );
    let cache = LABEL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(stored) = cache.lock() {
        if let Some(entry) = stored.get(&key) {
            if entry
                .updated_at
                .is_some_and(|updated| updated.elapsed() < LABEL_CACHE_TTL)
            {
                return entry.labels.clone();
            }
        }
    }

    let labels = query_session_labels(herdr, socket, workspaces);
    if let Ok(mut stored) = cache.lock() {
        stored.insert(
            key,
            CachedSessionLabels {
                updated_at: Some(Instant::now()),
                labels: labels.clone(),
            },
        );
    }
    labels
}

fn query_session(herdr: &OsString, socket: Option<String>) -> Option<Vec<AgentView>> {
    let arguments = vec!["agent".to_string(), "list".to_string()];
    let text = run_herdr_command(herdr, socket.as_deref(), &arguments)?;
    let mut parsed = parse_agent_list(&text).ok()?;
    let mut workspaces: Vec<String> = parsed
        .iter()
        .filter_map(|agent| agent.workspace_id.clone())
        .collect();
    workspaces.sort();
    workspaces.dedup();
    let labels = labels_for_session(herdr, socket.as_deref(), &workspaces);

    for agent in &mut parsed {
        agent.view.label = labels
            .pane_labels
            .get(&agent.view.id)
            .cloned()
            .or_else(|| {
                agent
                    .tab_id
                    .as_ref()
                    .and_then(|tab_id| labels.tab_labels.get(tab_id).cloned())
            })
            .unwrap_or_else(|| agent.view.label.clone());
    }

    let mut agents: Vec<AgentView> = parsed.into_iter().map(|agent| agent.view).collect();
    if let Some(path) = socket.as_deref() {
        let namespace = session_namespace(path);
        for agent in &mut agents {
            agent.id = format!("{namespace}:{}", agent.id);
        }
    }
    Some(agents)
}

fn query_agents() -> AgentSnapshot {
    let herdr = herdr_binary();
    let sessions = registered_sessions();
    let mut available = false;
    let mut agents = Vec::new();

    for batch in sessions.chunks(MAX_CONCURRENT_SESSION_QUERIES) {
        let workers: Vec<_> = batch
            .iter()
            .cloned()
            .map(|socket| {
                let herdr = herdr.clone();
                std::thread::spawn(move || query_session(&herdr, socket))
            })
            .collect();
        for worker in workers {
            if let Ok(Some(mut session_agents)) = worker.join() {
                available = true;
                agents.append(&mut session_agents);
            }
        }
    }
    agents.sort_by(|left, right| left.id.cmp(&right.id));
    agents.dedup_by(|left, right| left.id == right.id);
    AgentSnapshot { available, agents }
}

pub fn snapshot_json() -> String {
    serde_json::to_string(&query_agents())
        .unwrap_or_else(|_| r#"{"available":false,"agents":[]}"#.to_string())
}

#[tauri::command]
async fn list_agents() -> AgentSnapshot {
    tauri::async_runtime::spawn_blocking(query_agents)
        .await
        .unwrap_or_else(|_| unavailable_snapshot())
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HitRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Default)]
struct HitRegions(std::sync::Mutex<Vec<HitRegion>>);

#[tauri::command]
fn set_hit_regions(regions: Vec<HitRegion>, state: tauri::State<'_, HitRegions>) {
    if let Ok(mut stored) = state.0.lock() {
        *stored = regions;
    }
}

#[tauri::command]
fn show_village(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc2_app_kit::NSWindow;
        let native_window: &NSWindow = &*window
            .ns_window()
            .map_err(|error| error.to_string())?
            .cast();
        native_window.orderFrontRegardless();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.show().map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
fn refresh_mouse_passthrough(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use objc2_app_kit::{NSEvent, NSWindow};

    let window = app
        .get_webview_window("main")
        .ok_or("main village window is closed")?;
    let regions = app
        .state::<HitRegions>()
        .0
        .lock()
        .map(|stored| stored.clone())
        .unwrap_or_default();

    unsafe {
        let native_window: &NSWindow = &*window.ns_window()?.cast();
        let frame = native_window.frame();
        let mouse = NSEvent::mouseLocation();
        let local_x = mouse.x - frame.origin.x;
        let local_y = frame.size.height - (mouse.y - frame.origin.y);
        let over_citizen = regions.iter().any(|region| {
            local_x >= region.x
                && local_x <= region.x + region.width
                && local_y >= region.y
                && local_y <= region.y + region.height
        });
        native_window.setIgnoresMouseEvents(!over_citizen);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn start_hit_test_loop(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(33));
        let target = app.clone();
        if app
            .run_on_main_thread(move || {
                let _ = refresh_mouse_passthrough(&target);
            })
            .is_err()
        {
            break;
        }
    });
}

fn create_village_window(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
        .title("Herdr Pets")
        .inner_size(1100.0, 150.0)
        .min_inner_size(320.0, 150.0)
        .resizable(true)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .focusable(false)
        .focused(false)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false)
        .visible_on_all_workspaces(true)
        .build()?;
    Ok(())
}

fn configure_window(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("main village window was not created")?;

    window.set_ignore_cursor_events(true)?;

    #[cfg(target_os = "macos")]
    unsafe {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

        // Tauri's all-workspaces option sets CanJoinAllSpaces. FullScreenAuxiliary
        // is additionally required for a companion window beside fullscreen apps.
        let native_window: &NSWindow = &*window.ns_window()?.cast();
        let behavior = native_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary;
        native_window.setCollectionBehavior(behavior);
    }

    if let Some(monitor) = window.current_monitor()? {
        let area = monitor.work_area();
        let window_height = window.outer_size()?.height;
        window.set_size(PhysicalSize::new(area.size.width, window_height))?;
        let x = area.position.x;
        let y =
            area.position.y + area.size.height as i32 - window_height as i32 - WINDOW_BOTTOM_MARGIN;
        window.set_position(PhysicalPosition::new(x, y))?;
    }

    #[cfg(target_os = "macos")]
    unsafe {
        use objc2_app_kit::NSWindow;
        let native_window: &NSWindow = &*window.ns_window()?.cast();
        native_window.orderFrontRegardless();
    }
    #[cfg(not(target_os = "macos"))]
    window.show()?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(HitRegions::default())
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = webview.window().show();
            }
        })
        .setup(|app| {
            create_village_window(app)?;
            configure_window(app)?;
            #[cfg(target_os = "macos")]
            start_hit_test_loop(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_agents,
            set_hit_regions,
            show_village
        ])
        .run(tauri::generate_context!())
        .expect("failed to run herdr-pets");
}
