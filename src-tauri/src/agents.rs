use serde::{Deserialize, Serialize};
use std::path::Path;

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
    agent_session: Option<RawAgentSession>,
    foreground_cwd: Option<String>,
    cwd: Option<String>,
    pub(crate) tab_id: Option<String>,
    pub(crate) workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAgentSession {
    value: String,
}

#[derive(Debug)]
pub(crate) struct ParsedAgent {
    pub(crate) view: AgentView,
    pub(crate) tab_id: Option<String>,
    pub(crate) workspace_id: Option<String>,
    pub(crate) agent_session_id: Option<String>,
}

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

pub(crate) fn unavailable_snapshot() -> AgentSnapshot {
    AgentSnapshot {
        available: false,
        agents: Vec::new(),
    }
}
