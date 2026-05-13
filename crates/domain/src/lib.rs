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
    pub description: String,
}

impl Action {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
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
