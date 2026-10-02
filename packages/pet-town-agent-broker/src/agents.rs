use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub id: String,
    pub status: String,
    pub label: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_activity: Option<crate::TokenActivity>,
    #[serde(default)]
    pub supports_terminal: bool,
    #[serde(default)]
    pub remote_machine: Option<String>,
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
    agent_session: Option<RawAgentSession>,
    foreground_cwd: Option<String>,
    cwd: Option<String>,
    tab_id: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAgentSession {
    value: String,
}

pub(crate) struct ParsedAgent {
    pub view: AgentView,
    pub tab_id: Option<String>,
    pub workspace_id: Option<String>,
    pub agent_session_id: Option<String>,
}

fn safe_project_name(path: Option<&str>) -> String {
    let name = path
        .and_then(|value| Path::new(value).file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("project");
    safe_display_label(Some(name)).unwrap_or_else(|| "project".to_string())
}

pub(crate) fn safe_display_label(value: Option<&str>) -> Option<String> {
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

pub(crate) fn parse_agent_list(text: &str) -> Result<Vec<ParsedAgent>, serde_json::Error> {
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
                    source: "unknown".to_string(),
                    token_activity: None,
                    supports_terminal: false,
                    remote_machine: None,
                },
                tab_id: agent.tab_id,
                workspace_id: agent.workspace_id,
                agent_session_id: agent.agent_session.map(|session| session.value),
            }
        })
        .collect();
    agents.sort_by(|left, right| left.view.id.cmp(&right.view.id));
    agents.dedup_by(|left, right| left.view.id == right.view.id);
    Ok(agents)
}
