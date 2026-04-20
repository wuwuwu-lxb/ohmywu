use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    Sandbox,
    Danger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    ReadOnly,
    ControlledWrite,
    HighRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum ActionTarget {
    Tool(String),
    Workflow(String),
    Skill(String),
    AgentTemplate(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSpec {
    pub name: String,
    pub title: String,
    pub summary: String,
    pub domain: String,
    pub risk_level: RiskLevel,
    pub target: ActionTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub title: String,
    pub summary: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    pub name: String,
    pub title: String,
    pub summary: String,
    pub action_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLifecycleState {
    Draft,
    Trial,
    Active,
    Frozen,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub name: String,
    pub title: String,
    pub summary: String,
    pub default_actions: Vec<String>,
    pub lifecycle_state: AgentLifecycleState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    pub id: Uuid,
    pub template_name: String,
    pub lifecycle_state: AgentLifecycleState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub actor: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub id: Uuid,
    pub summary: String,
    pub steps: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedReference {
    pub raw: String,
    pub kind: String,
    pub name: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceResolutionRequest {
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceResolutionResponse {
    pub input: String,
    pub references: Vec<ResolvedReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub app: String,
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub policy_mode: PolicyMode,
    pub default_domain: String,
    pub allow_agent_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            policy_mode: PolicyMode::Sandbox,
            default_domain: "system-management".to_string(),
            allow_agent_mode: false,
        }
    }
}

// ── M2: Process ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u64,
    pub name: String,
    pub state: String,
    pub cpu_percent: f32,
    pub memory_kb: u64,
    pub user: String,
    pub command: String,
}

// ── M2: Service ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub description: String,
    pub enabled: bool,
}

// ── M2: Storage ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEntry {
    pub path: String,
    pub size_bytes: u64,
    pub human_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageScanResult {
    pub root_path: String,
    pub total_bytes: u64,
    pub entries: Vec<StorageEntry>,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageScanQuery {
    pub path: Option<String>,
    pub depth: Option<u32>,
}

// ── M2: Journal ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Utc>,
    pub unit: String,
    pub priority: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalQuery {
    pub unit: Option<String>,
    pub priority: Option<i32>,
    pub lines: Option<usize>,
    pub since: Option<DateTime<Utc>>,
}
