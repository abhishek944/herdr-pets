use crate::{agents, command, herdr, remote, FocusRoute};
use std::ffi::OsString;

pub(crate) fn resolve(binary: &OsString, id: &str) -> Option<FocusRoute> {
    if !id.starts_with("herdr:machine:") || id.len() > 256 {
        return None;
    }
    let args = ["machine", "list", "--json"].map(String::from);
    let text = command::run(binary, None, &args)?;
    for machine in remote::parse_machines(&text)? {
        let namespace = herdr::remote_identity(&machine.id, "").0;
        let prefix = namespace.rsplit_once(':')?.0;
        if !id.starts_with(&format!("{prefix}:")) {
            continue;
        }
        return herdr::query_session(
            binary,
            None,
            Some((machine.id, machine.label)),
            agents::parse_agent_list,
        )?
        .into_iter()
        .find(|agent| agent.view.id == id)?
        .focus_route;
    }
    None
}
