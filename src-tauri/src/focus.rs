use crate::herdr_command::run_herdr_command;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct FocusTarget {
    pub(crate) pane_id: String,
    pub(crate) socket: Option<String>,
    pub(crate) agent_session_id: Option<String>,
}

#[derive(Default)]
pub(crate) struct FocusTargets(Mutex<HashMap<String, FocusTarget>>);

impl FocusTargets {
    pub(crate) fn replace(&self, targets: HashMap<String, FocusTarget>) {
        if let Ok(mut stored) = self.0.lock() {
            *stored = targets;
        }
    }

    fn get(&self, id: &str) -> Option<FocusTarget> {
        self.0.lock().ok()?.get(id).cloned()
    }
}

fn herdr_binary() -> OsString {
    std::env::var_os("HERDR_PETS_HERDR_BIN")
        .or_else(|| std::env::var_os("HERDR_BIN_PATH"))
        .unwrap_or_else(|| "herdr".into())
}

fn session_namespace(socket: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in socket.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn public_agent_id(socket: Option<&str>, pane_id: &str) -> String {
    socket
        .map(|path| format!("{}:{pane_id}", session_namespace(path)))
        .unwrap_or_else(|| pane_id.to_string())
}

#[derive(Deserialize)]
struct AgentGetEnvelope {
    result: AgentGetResult,
}

#[derive(Deserialize)]
struct AgentGetResult {
    agent: CurrentAgent,
}

#[derive(Deserialize)]
struct CurrentAgent {
    agent_session: Option<CurrentAgentSession>,
}

#[derive(Deserialize)]
struct CurrentAgentSession {
    value: String,
}

fn run_focus_target(herdr: &OsString, target: &FocusTarget) -> Result<(), String> {
    if let Some(expected) = target.agent_session_id.as_deref() {
        let arguments = vec![
            "agent".to_string(),
            "get".to_string(),
            target.pane_id.clone(),
        ];
        let text = run_herdr_command(herdr, target.socket.as_deref(), &arguments)
            .ok_or_else(|| "agent is no longer available".to_string())?;
        let current = serde_json::from_str::<AgentGetEnvelope>(&text)
            .ok()
            .and_then(|envelope| envelope.result.agent.agent_session)
            .map(|session| session.value);
        if current.as_deref() != Some(expected) {
            return Err("agent changed since the latest poll".to_string());
        }
    }
    let arguments = vec![
        "agent".to_string(),
        "focus".to_string(),
        target.pane_id.clone(),
    ];
    run_herdr_command(herdr, target.socket.as_deref(), &arguments)
        .map(|_| ())
        .ok_or_else(|| "Herdr could not focus that agent".to_string())
}

#[tauri::command]
pub(crate) async fn focus_agent(
    id: String,
    targets: tauri::State<'_, FocusTargets>,
) -> Result<(), String> {
    let target = targets
        .get(&id)
        .ok_or_else(|| "agent is no longer available".to_string())?;
    let herdr = herdr_binary();
    tauri::async_runtime::spawn_blocking(move || {
        run_focus_target(&herdr, &target)?;
        #[cfg(target_os = "macos")]
        crate::macos_activation::activate_herdr_host(&herdr, target.socket.as_deref())?;
        Ok(())
    })
    .await
    .map_err(|_| "agent focus task failed".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_ids_preserve_the_focus_route() {
        assert_eq!(public_agent_id(None, "w1:p2"), "w1:p2");
        let first = public_agent_id(Some("/tmp/one.sock"), "w1:p2");
        let second = public_agent_id(Some("/tmp/two.sock"), "w1:p2");
        assert_ne!(first, second);
        assert!(first.ends_with(":w1:p2"));
    }

    #[test]
    fn targets_accept_only_exact_polled_ids() {
        let targets = FocusTargets::default();
        targets.replace(HashMap::from([(
            "public:w1:p2".to_string(),
            FocusTarget {
                pane_id: "w1:p2".to_string(),
                socket: Some("/tmp/session.sock".to_string()),
                agent_session_id: Some("session-one".to_string()),
            },
        )]));
        assert_eq!(targets.get("public:w1:p2").unwrap().pane_id, "w1:p2");
        assert!(targets.get("w1:p2").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn focus_uses_raw_pane_and_matching_socket() {
        use std::os::unix::fs::PermissionsExt;
        let directory =
            std::env::temp_dir().join(format!("herdr-pets-focus-{}", std::process::id()));
        let executable = directory.join("fake-herdr");
        let output = directory.join("arguments.txt");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nif [ \"$2\" = get ]; then\n  printf '%s' '{{\"result\":{{\"agent\":{{\"agent_session\":{{\"value\":\"session-one\"}}}}}}}}'\n  exit 0\nfi\nprintf '%s\\n' \"$HERDR_SOCKET_PATH\" \"$@\" > '{}'\nprintf '{{}}'\n",
                output.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let executable = executable.into_os_string();
        assert!(run_focus_target(
            &executable,
            &FocusTarget {
                pane_id: "w9:p4".to_string(),
                socket: Some("/tmp/right-session.sock".to_string()),
                agent_session_id: Some("replaced-session".to_string()),
            },
        )
        .is_err());
        run_focus_target(
            &executable,
            &FocusTarget {
                pane_id: "w9:p4".to_string(),
                socket: Some("/tmp/right-session.sock".to_string()),
                agent_session_id: Some("session-one".to_string()),
            },
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            "/tmp/right-session.sock\nagent\nfocus\nw9:p4\n"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
