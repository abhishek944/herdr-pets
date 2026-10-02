use crate::{command, herdr, FocusRoute};
use serde_json::Value;
use std::ffi::OsString;
use std::process::{Command, Stdio};

/// Private binding retained by the native host, never serialized to a renderer.
pub struct TerminalTarget {
    binary: OsString,
    pane: String,
    socket: Option<String>,
    session: String,
}

pub fn terminal_target(id: &str) -> Result<TerminalTarget, String> {
    if id.is_empty() || id.len() > 256 {
        return Err("Choose a current Herdr companion.".into());
    }
    let route = crate::collect().focus_routes.remove(id);
    let Some(FocusRoute::Herdr {
        pane_id,
        socket,
        machine,
        agent_session_id,
    }) = route
    else {
        return Err("This companion has no available Herdr terminal.".into());
    };
    if machine.is_some() {
        return Err(
            "Remote companions are monitored in Pet Town. Use Herdr to view their terminals."
                .into(),
        );
    }
    if agent_session_id.starts_with("ephemeral:") {
        return Err("Herdr has not reported a stable session for this companion.".into());
    }
    let target = TerminalTarget {
        binary: herdr::binary(),
        pane: pane_id,
        socket: socket.or_else(|| std::env::var("HERDR_SOCKET_PATH").ok()),
        session: agent_session_id,
    };
    target.verify()?;
    Ok(target)
}

impl TerminalTarget {
    pub fn return_to_live(&self) -> Result<(), String> {
        let socket = self.socket.as_deref().ok_or("Herdr has not reported this view's API socket. Open in Herdr to return to live output.")?;
        crate::terminal_api::live(socket, &self.pane)
    }

    pub fn verify(&self) -> Result<(), String> {
        let args = vec!["agent".into(), "get".into(), self.pane.clone()];
        let text = command::run(&self.binary, self.socket.as_deref(), &args)
            .ok_or("The Herdr session is unavailable. Reconnect to watch again.")?;
        let value: Value =
            serde_json::from_str(&text).map_err(|_| "Herdr returned invalid session details.")?;
        if value["result"]["agent"]["agent_session"]["value"].as_str()
            != Some(self.session.as_str())
        {
            return Err(
                "This agent session ended or changed. Choose its current companion.".into(),
            );
        }
        Ok(())
    }

    pub fn stream_command(&self, control: bool, takeover: bool, cols: u16, rows: u16) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .env_remove("OPENAI_API_KEY")
            .env_remove("HERDR_SOCKET_PATH")
            .args([
                "terminal",
                "session",
                if control { "control" } else { "observe" },
            ])
            .arg(&self.pane)
            .args(["--cols", &cols.to_string(), "--rows", &rows.to_string()])
            .stdin(if control {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(socket) = &self.socket {
            command.env("HERDR_SOCKET_PATH", socket);
        }
        if control && takeover {
            command.arg("--takeover");
        }
        command
    }
}
