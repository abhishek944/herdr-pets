//! Herdr's --machine API forwarding does not forward terminal session streams.
use crate::command;
use serde::Deserialize;
use std::ffi::OsString;
use std::process::{Command, Stdio};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Observer {
    pub(crate) id: String,
    target: String,
    session: String,
    enabled: bool,
}

impl Observer {
    pub(crate) fn load(binary: &OsString, id: &str) -> Result<Self, String> {
        let args = ["machine", "list", "--json"].map(String::from);
        let text = command::run(binary, None, &args)
            .ok_or("Herdr's saved machines are unavailable. Reconnect to observe again.")?;
        Self::parse(&text, id)
    }

    fn parse(text: &str, id: &str) -> Result<Self, String> {
        let profiles: Vec<Self> = serde_json::from_str(text)
            .map_err(|_| "Herdr returned invalid saved machine details.")?;
        let profile = profiles
            .into_iter()
            .find(|p| p.id == id && p.enabled)
            .ok_or("This remote machine was removed or disabled. Choose a current companion.")?;
        if profile.target.is_empty()
            || profile.session.is_empty()
            || profile.target.chars().any(char::is_control)
            || profile.session.chars().any(char::is_control)
        {
            return Err("Herdr returned invalid SSH target or session details.".into());
        }
        Ok(profile)
    }

    pub(crate) fn verify_profile(&self, binary: &OsString) -> Result<(), String> {
        if Self::load(binary, &self.id)? != *self {
            return Err("This remote machine changed. Reconnect to its current session.".into());
        }
        Ok(())
    }

    pub(crate) fn command(&self, pane: &str, cols: u16, rows: u16) -> Command {
        let mut command = Command::new("ssh");
        command.args([
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=8",
            "-o",
            "ServerAliveInterval=5",
            "-o",
            "ServerAliveCountMax=2",
            "--",
            &self.target,
            &self.script(pane, cols, rows),
        ]);
        for name in [
            "OPENAI_API_KEY",
            "HERDR_SOCKET_PATH",
            "HERDR_SESSION",
            "HERDR_PANE_ID",
            "HERDR_TAB_ID",
            "HERDR_WORKSPACE_ID",
        ] {
            command.env_remove(name);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn script(&self, pane: &str, cols: u16, rows: u16) -> String {
        // The SSH input pipe is a lifetime signal, never terminal input. A shell
        // watchdog terminates/reaps only this observer when the pipe closes,
        // including idle terminals that would otherwise never write again.
        format!(
            "exec 3<&0; herdr --session {} terminal session observe {} --cols {} --rows {} </dev/null & observer=$!; watcher=''; \
             trap 'kill \"$observer\" \"$watcher\" 2>/dev/null; wait \"$observer\" \"$watcher\" 2>/dev/null' 0; \
             trap 'exit 1' HUP INT TERM; \
             (while IFS= read -r ignored <&3; do :; done; kill \"$observer\" 2>/dev/null) & watcher=$!; \
             wait \"$observer\"; status=$?; exit \"$status\"",
            quote(&self.session), quote(pane), cols, rows
        )
    }
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_selection_rejects_disabled_missing_and_malformed_profiles() {
        let catalog = r#"[{"id":"m1","target":"user@host","session":"agents","enabled":true},{"id":"m2","target":"other","session":"default","enabled":false}]"#;
        assert_eq!(Observer::parse(catalog, "m1").unwrap().session, "agents");
        assert!(Observer::parse(catalog, "m2").is_err());
        assert!(Observer::parse(catalog, "missing").is_err());
        assert!(Observer::parse("not json", "m1").is_err());
    }
    #[test]
    fn observer_is_read_only_and_quotes_remote_arguments() {
        let observer = Observer {
            id: "m1".into(),
            target: "user@host".into(),
            session: "a'$(touch /tmp/wrong)".into(),
            enabled: true,
        };
        let command = observer.command("w1:p1", 80, 24);
        assert_eq!(command.get_program(), "ssh");
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"BatchMode=yes".into()));
        assert!(args.contains(&"StrictHostKeyChecking=yes".into()));
        assert!(args
            .last()
            .unwrap()
            .contains("terminal session observe 'w1:p1'"));
        assert!(!args.last().unwrap().contains("control"));
        assert_eq!(quote("a'b"), "'a'\\''b'");
        assert!(command
            .get_envs()
            .any(|(key, value)| key == "HERDR_SOCKET_PATH" && value.is_none()));
    }
}
