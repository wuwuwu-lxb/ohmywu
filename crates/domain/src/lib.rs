use serde::{Deserialize, Serialize};

/// Atomic capability — the lowest execution unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
}

impl Capability {
    pub fn new(name: &str, description: &str, risk_level: RiskLevel) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            risk_level,
        }
    }
}

/// Stable action — a unified entry point wrapping one or more capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source: ActionSource,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    pub available: bool,
}

impl Action {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            title: id.to_string(),
            description: description.to_string(),
            source: ActionSource::Builtin,
            capabilities: Vec::new(),
            tags: Vec::new(),
            path: None,
            entry: None,
            available: true,
        }
    }

    pub fn builtin(
        id: &str,
        title: &str,
        description: &str,
        capabilities: &[&str],
        tags: &[&str],
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            source: ActionSource::Builtin,
            capabilities: capabilities.iter().map(|item| item.to_string()).collect(),
            tags: tags.iter().map(|item| item.to_string()).collect(),
            path: None,
            entry: None,
            available: true,
        }
    }

    pub fn skill(
        id: &str,
        title: &str,
        description: &str,
        path: &str,
        entry: &str,
        tags: &[String],
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            source: ActionSource::Skill,
            capabilities: Vec::new(),
            tags: tags.to_vec(),
            path: Some(path.to_string()),
            entry: Some(entry.to_string()),
            available: true,
        }
    }

    pub fn user(
        id: &str,
        title: &str,
        description: &str,
        capabilities: &[String],
        tags: &[String],
        available: bool,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            source: ActionSource::User,
            capabilities: capabilities.to_vec(),
            tags: tags.to_vec(),
            path: None,
            entry: None,
            available,
        }
    }

    pub fn sort_key(&self) -> (u8, String, String) {
        let source_rank = match self.source {
            ActionSource::Builtin => 0,
            ActionSource::User => 1,
            ActionSource::Skill => 2,
        };
        (source_rank, self.title.to_lowercase(), self.id.to_lowercase())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSource {
    Builtin,
    User,
    Skill,
}

impl ActionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::User => "user",
            Self::Skill => "skill",
        }
    }
}

/// Risk level for capability and action classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    ReadOnly,
    ControlledWrite,
    HighRisk,
}

/// Policy mode — determines what operations are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyMode {
    Sandbox,
    Danger,
}

/// Agent mode — determines how the runtime exposes and executes tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Plan,
    Agent,
    Auto,
}

/// A task — represents a tracked execution unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub target: String,
    pub status: TaskStatus,
    pub detail: Option<String>,
    pub created_at: String,
}

/// Task execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Current UTC timestamp in ISO 8601 format.
pub fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Audit event — records every significant operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub actor: String,
    pub action: String,
    pub target: String,
    pub risk_level: RiskLevel,
    pub status: String,
    pub detail: Option<String>,
    pub timestamp: String,
}
