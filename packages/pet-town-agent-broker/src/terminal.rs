use crate::{command, herdr, terminal_remote::Observer, FocusRoute};
use serde_json::Value;
use std::ffi::OsString;
use std::process::{Command, Stdio};

/// Private binding retained by the native host, never serialized to a renderer.
pub struct TerminalTarget {
    binary: OsString,
    pane: String,
    socket: Option<String>,
    session: String,
    remote: Option<Observer>,
}

pub fn terminal_target(id: &str) -> Result<TerminalTarget, String> {
    if id.is_empty() || id.len() > 256 {
        return Err("Choose a current Herdr companion.".into());
    }
    let binary = herdr::binary();
    let route = crate::collect()
        .focus_routes
        .remove(id)
        .or_else(|| crate::remote_route::resolve(&binary, id));
    bind_route(
        binary,
        route.ok_or("This companion has no available Herdr terminal.")?,
    )
}

pub(crate) fn bind_route(binary: OsString, route: FocusRoute) -> Result<TerminalTarget, String> {
    let FocusRoute::Herdr {
        pane_id,
        socket,
        machine,
        agent_session_id,
    } = route
    else {
        return Err("This companion has no available Herdr terminal.".into());
    };
    if agent_session_id.starts_with("ephemeral:") {
        return Err("Herdr has not reported a stable session for this companion.".into());
    }
    let remote = machine
        .as_deref()
        .map(|id| Observer::load(&binary, id))
        .transpose()?;
    let target = TerminalTarget {
        binary,
        remote,
        pane: pane_id,
        socket: if machine.is_some() {
            None
        } else {
            socket.or_else(|| std::env::var("HERDR_SOCKET_PATH").ok())
        },
        session: agent_session_id,
    };
    target.verify()?;
    Ok(target)
}

impl TerminalTarget {
    pub fn validate_mode(&self, control: bool, takeover: bool) -> Result<(), String> {
        if self.remote.is_some() && (control || takeover) {
            return Err("Remote terminals support Observe only. Use Herdr for input.".into());
        }
        Ok(())
    }
    pub fn return_to_live(&self) -> Result<(), String> {
        let socket = self.socket.as_deref().ok_or("Herdr has not reported this view's API socket. Open in Herdr to return to live output.")?;
        crate::terminal_api::live(socket, &self.pane)
    }

    pub fn verify(&self) -> Result<(), String> {
        if let Some(remote) = &self.remote {
            remote.verify_profile(&self.binary)?;
        }
        let args = vec!["agent".into(), "get".into(), self.pane.clone()];
        let text = command::run_on_machine(
            &self.binary,
            self.socket.as_deref(),
            self.remote.as_ref().map(|r| r.id.as_str()),
            &args,
        )
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

    pub fn stream_command(
        &self,
        control: bool,
        takeover: bool,
        cols: u16,
        rows: u16,
    ) -> Result<Command, String> {
        self.validate_mode(control, takeover)?;
        if let Some(remote) = &self.remote {
            return Ok(remote.command(&self.pane, cols, rows));
        }
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
        Ok(command)
    }
}
