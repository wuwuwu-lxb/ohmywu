pub mod artifact;
pub mod bash;
pub mod checklist;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod read;
pub mod thinking;
pub mod web_fetch;
pub mod wiki;
pub mod write;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::task;

use ohmywu_domain::{AgentMode, RiskLevel};

use crate::action_catalog::ActionUpsertInput;
use crate::agent_catalog::AgentUpsertInput;
use crate::capabilities::CapabilityUpsertInput;
use crate::AppState;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentDelegateInput {
    target_agent_id: String,
    task: String,
}

// ── Shared types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub capability: String,
    pub params: serde_json::Value,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub capability: String,
    pub status: String,
    pub output: Option<String>,
    pub artifact_id: Option<String>,
    pub artifact_path: Option<String>,
    pub error: Option<String>,
    pub task_id: String,
    pub duration_ms: u64,
    pub policy_decision: String,
}

// ── Tool metadata ────────────────────────────────────────────────

/// Runtime classification of a tool for permission and execution routing.
/// TODO: used in Phase 2 permission system
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolKind {
    ReadOnly,
    ControlledWrite,
    HighRisk,
    None_,
}

#[allow(dead_code)]
impl ToolKind {
    pub fn from_risk(level: RiskLevel) -> Self {
        match level {
            RiskLevel::ReadOnly => Self::ReadOnly,
            RiskLevel::ControlledWrite => Self::ControlledWrite,
            RiskLevel::HighRisk => Self::HighRisk,
        }
    }

    pub fn is_concurrency_safe(&self) -> bool {
        matches!(self, Self::ReadOnly | Self::None_)
    }

    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::HighRisk)
    }
}

/// Look up a capability by tool name to check its risk level.
#[allow(dead_code)]
pub fn tool_kind(state: &AppState, name: &str) -> Option<ToolKind> {
    state
        .capabilities
        .get(name)
        .map(|c| ToolKind::from_risk(c.risk_level))
}

// ── Tool definition generation ───────────────────────────────────

use ohmywu_llm_adapter::types::{FunctionDef, ToolDef};

/// Generate LLM tool definitions from all registered capabilities.
pub fn active_tool_defs(state: &AppState, allowed_tools: Option<&[String]>) -> Vec<ToolDef> {
    let caps = state
        .capability_catalog
        .read()
        .map(|catalog| catalog.active_entries())
        .unwrap_or_default();
    let agent_mode = state
        .config
        .read()
        .map(|cfg| cfg.agent_mode)
        .unwrap_or(AgentMode::Agent);
    let mut tools: Vec<ToolDef> = caps
        .iter()
        .filter(|cap| {
            if is_system_capability(&cap.name) || is_system_capability(&cap.implementation) {
                return true;
            }
            allowed_tools.is_none_or(|tools| {
                tools.is_empty()
                    || tools.iter().any(|item| item == &cap.name || item == &cap.implementation)
            })
        })
        .filter(|cap| tool_visible_in_mode(&cap.implementation, cap.risk_level, agent_mode))
        .filter_map(|cap| {
            let params = tool_params(&cap.implementation)?;
            Some(ToolDef {
                tool_type: "function".into(),
                function: FunctionDef {
                    name: cap.name.clone(),
                    description: cap.description.clone(),
                    parameters: params,
                },
            })
        })
        .collect();

    // Sort: read-only tools first, then write/high-risk
    tools.sort_by(|a, b| {
        let a_risk = caps.iter().find(|c| c.name == a.function.name);
        let b_risk = caps.iter().find(|c| c.name == b.function.name);
        let a_score = a_risk.map_or(0, |c| risk_sort_key(c.risk_level));
        let b_score = b_risk.map_or(0, |c| risk_sort_key(c.risk_level));
        a_score
            .cmp(&b_score)
            .then_with(|| a.function.name.cmp(&b.function.name))
    });

    tools
}

fn risk_sort_key(level: RiskLevel) -> u8 {
    match level {
        RiskLevel::ReadOnly => 0,
        RiskLevel::ControlledWrite => 1,
        RiskLevel::HighRisk => 2,
    }
}

fn is_system_capability(name: &str) -> bool {
    matches!(name, "thinking" | "checklist_write")
}

/// Return the JSON Schema parameters for a given tool/capability name.
pub fn tool_params(name: &str) -> Option<serde_json::Value> {
    match name {
        "bash" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })),
        "read" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to read"
                }
            },
            "required": ["path"]
        })),
        "artifact_read" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "artifactId": {
                    "type": ["string", "null"],
                    "description": "Stable artifact id emitted by a previous tool execution"
                },
                "path": {
                    "type": ["string", "null"],
                    "description": "Optional absolute artifact path from a previous tool execution"
                },
                "offset": {
                    "type": "integer",
                    "description": "Character offset to start reading from",
                    "minimum": 0
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of characters to read for this chunk",
                    "minimum": 200,
                    "maximum": 8000
                }
            }
        })),
        "write" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["path", "content"]
        })),
        "edit" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to find (must match uniquely)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })),
        "glob" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g. '**/*.rs')"
                },
                "path": {
                    "type": "string",
                    "description": "Root directory to search in (defaults to home)"
                }
            },
            "required": ["pattern"]
        })),
        "grep" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern (regex or plain text)"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in"
                },
                "include": {
                    "type": "string",
                    "description": "Optional file glob filter (e.g. '*.rs')"
                }
            },
            "required": ["pattern"]
        })),
        "web_fetch" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                }
            },
            "required": ["url"]
        })),
        "thinking" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your internal reasoning about the current task"
                }
            },
            "required": ["thought"]
        })),
        "checklist_write" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Optional checklist title"
                },
                "items": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Ordered task items for the current turn"
                }
            },
            "required": ["items"]
        })),
        "wiki_read" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "slug": { "type": "string", "description": "The note slug to read" }
            },
            "required": ["slug"]
        })),
        "wiki_write" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "slug": { "type": "string", "description": "URL-friendly identifier (e.g. rust-ownership)" },
                "title": { "type": "string", "description": "Human-readable title" },
                "body": { "type": "string", "description": "Markdown body. Use [[slug]] for interlinking." },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "Tags for categorization" },
                "folder": { "type": "string", "description": "Folder: concepts, notes, daily, or profile" }
            },
            "required": ["slug", "title", "body"]
        })),
        "wiki_search" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search keywords" }
            },
            "required": ["query"]
        })),
        "wiki_list" => Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        "wiki_graph" => Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        "capability_list" => Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        "capability_register" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "existingName": { "type": ["string", "null"], "description": "Existing capability name when updating" },
                "name": { "type": "string", "description": "New capability name" },
                "title": { "type": "string", "description": "Human-readable capability title" },
                "description": { "type": "string", "description": "Capability usage description" },
                "riskLevel": { "type": "string", "enum": ["ReadOnly", "ControlledWrite", "HighRisk"] },
                "implementation": { "type": "string", "description": "Underlying builtin capability to wrap" },
                "enabled": { "type": "boolean", "description": "Whether to enable immediately" }
            },
            "required": ["name", "title", "description", "riskLevel", "implementation", "enabled"]
        })),
        "action_list" => Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        "action_register" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "existingId": { "type": ["string", "null"], "description": "Existing action id when updating" },
                "id": { "type": "string", "description": "Stable action id" },
                "title": { "type": "string", "description": "Human-readable title" },
                "description": { "type": "string", "description": "One-line action summary" },
                "capabilities": { "type": "array", "items": { "type": "string" }, "description": "Capability names used by this action" },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional tags" },
                "prompt": { "type": "string", "description": "Compiled action prompt or transformed skill body" },
                "supportingFiles": { "type": "array", "items": { "type": "string" }, "description": "Relevant files that support the action" },
                "sourceHint": { "type": ["string", "null"], "description": "Original skill path, repo url, or note" },
                "enabled": { "type": "boolean", "description": "Whether to enable immediately" }
            },
            "required": ["id", "title", "description", "capabilities", "prompt", "enabled"]
        })),
        "agent_list" => Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        "agent_delegate" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "targetAgentId": { "type": "string", "description": "Target agent id to delegate to" },
                "task": { "type": "string", "description": "Bounded subtask for the target agent" }
            },
            "required": ["targetAgentId", "task"]
        })),
        "agent_register" => Some(serde_json::json!({
            "type": "object",
            "properties": {
                "existingId": { "type": ["string", "null"], "description": "Existing agent id when updating" },
                "id": { "type": "string", "description": "Stable agent id" },
                "name": { "type": "string", "description": "Human-readable agent name" },
                "role": { "type": "string", "description": "Agent responsibility boundary" },
                "persona": { "type": "string", "description": "Execution persona and style" },
                "memoryScope": { "type": "string", "description": "Serialized memory scope JSON string" },
                "tools": { "type": "array", "items": { "type": "string" }, "description": "Capability names this agent may use" },
                "delegateTags": { "type": "array", "items": { "type": "string" }, "description": "Short keywords used to recommend this agent for delegation" },
                "delegateNote": { "type": "string", "description": "Human-readable note describing when this agent should be delegated to" },
                "delegatable": { "type": "boolean", "description": "Whether this agent should appear in agent_list and be available for agent_delegate" },
                "delegatePriority": { "type": "integer", "description": "Delegation priority from 0 to 100. Higher values appear earlier in candidate lists." }
            },
            "required": ["id", "name", "role", "persona", "memoryScope", "tools", "delegatable", "delegatePriority"]
        })),
        _ => None,
    }
}

// ── Main dispatch ────────────────────────────────────────────────

/// Dispatch a tool call by name. Routes to the appropriate handler.
pub async fn dispatch_tool(
    state: &AppState,
    request: ExecuteRequest,
) -> ExecuteResult {
    let cap_name = request.capability.clone();
    let params = request.params.clone();

    // 1. capability lookup
    let cap = match state
        .capability_catalog
        .read()
        .ok()
        .and_then(|catalog| catalog.resolve_active(&cap_name))
    {
        Some(c) => c,
        None => {
            return ExecuteResult {
                capability: cap_name.clone(),
                status: "not_found".into(),
                output: None,
                artifact_id: None,
                artifact_path: None,
                error: Some(format!("Capability '{}' not registered", cap_name)),
                task_id: String::new(),
                duration_ms: 0,
                policy_decision: "denied".into(),
            };
        }
    };

    let agent_mode = state
        .config
        .read()
        .map(|cfg| cfg.agent_mode)
        .unwrap_or(AgentMode::Agent);
    if !tool_allowed_in_mode(&cap.implementation, cap.risk_level, agent_mode) {
        return ExecuteResult {
            capability: cap_name,
            status: "denied".into(),
            output: None,
            artifact_id: None,
            artifact_path: None,
            error: Some(format!("Tool is disabled in {:?} mode", agent_mode)),
            task_id: String::new(),
            duration_ms: 0,
            policy_decision: "denied".into(),
        };
    }

    // 2. policy gate
    let decision = state.policy.check(cap.risk_level);
    if !decision.allowed {
        state.audit.record(
            request.session_id.as_deref(),
            request.turn_id.as_deref(),
            "user",
            &cap_name,
            "(denied by policy)",
            cap.risk_level,
            "denied",
            Some(&format!("mode={:?}", decision.mode)),
        );
        return ExecuteResult {
            capability: cap_name,
            status: "denied".into(),
            output: None,
            artifact_id: None,
            artifact_path: None,
            error: Some(format!("Policy denied in {:?} mode", decision.mode)),
            task_id: String::new(),
            duration_ms: 0,
            policy_decision: "denied".into(),
        };
    }

    // 3. permission check (Claude Code style — deny wins, no popups)
    let permission_result = {
        let cfg = state.config.read().unwrap();
        crate::permission::check_permission(
            &cfg.permissions,
            &cap.implementation,
            &params,
            Some(ToolKind::from_risk(cap.risk_level)),
            agent_mode,
        )
    };

    match permission_result {
        crate::permission::PermissionCheck::Denied(msg) => {
            state.audit.record(
                request.session_id.as_deref(),
                request.turn_id.as_deref(),
                "user",
                &cap_name,
                "(denied by permission rules)",
                cap.risk_level,
                "denied",
                Some(&msg),
            );
            return ExecuteResult {
                capability: cap_name,
                status: "denied".into(),
                output: None,
                artifact_id: None,
                artifact_path: None,
                error: Some(msg),
                task_id: String::new(),
                duration_ms: 0,
                policy_decision: "denied".into(),
            };
        }
        crate::permission::PermissionCheck::NeedsConfirm(msg) => {
            // Return a "needs confirmation" message — the model handles it conversationally
            return ExecuteResult {
                capability: cap_name,
                status: "needs_confirm".into(),
                output: Some(msg),
                artifact_id: None,
                artifact_path: None,
                error: None,
                task_id: String::new(),
                duration_ms: 0,
                policy_decision: "needs_confirm".into(),
            };
        }
        crate::permission::PermissionCheck::Allowed => {}
    }

    // 4. task creation
    let target = describe_target(&cap.implementation, &params);
    let task = state.tasks.create(&cap_name, &target);
    let task_id = task.id.clone();

    // 4. execute (with spawn_blocking + timeout)
    let start = Instant::now();

    let exec_result: Result<ExecOutput, String> = if cap.implementation == "thinking" {
        // thinking is instant, no blocking needed
        thinking::execute(&params)
    } else if cap.implementation == "artifact_read" {
        artifact::execute(&params, &state.data_dir)
    } else if cap.implementation == "checklist_write" {
        checklist::write(&params, &state.runtime)
    } else if cap.implementation == "capability_list" {
        execute_capability_list(state)
    } else if cap.implementation == "capability_register" {
        execute_capability_register(state, &params)
    } else if cap.implementation == "action_list" {
        execute_action_list(state)
    } else if cap.implementation == "action_register" {
        execute_action_register(state, &params)
    } else if cap.implementation == "agent_list" {
        execute_agent_list(state, &params)
    } else if cap.implementation == "agent_delegate" {
        execute_agent_delegate(state, &params).await
    } else if cap.implementation == "agent_register" {
        execute_agent_register(state, &params)
    } else if cap.implementation.starts_with("wiki_") {
        // wiki tools: read/write/search/list/graph — fast file I/O
        let wiki_lock = state.wiki.read().unwrap();
        match cap.implementation.as_str() {
            "wiki_read" => wiki::read(&params, &wiki_lock),
            "wiki_write" => wiki::write(&params, &wiki_lock),
            "wiki_search" => wiki::search(&params, &wiki_lock),
            "wiki_list" => wiki::list(&params, &wiki_lock),
            "wiki_graph" => wiki::graph(&params, &wiki_lock),
            other => Err(format!("Unknown capability: {}", other)),
        }
    } else {
        let cap_name_clone = cap.implementation.clone();
        let params_clone = params.clone();
        let future = task::spawn_blocking(move || -> Result<ExecOutput, String> {
            match cap_name_clone.as_str() {
                "bash" => bash::execute(&params_clone),
                "read" => read::execute(&params_clone),
                "write" => write::execute(&params_clone),
                "edit" => edit::execute(&params_clone),
                "glob" => glob::execute(&params_clone),
                "grep" => grep::execute(&params_clone),
                "web_fetch" => web_fetch::execute(&params_clone),
                "checklist_write" => Err("checklist_write should execute inline".into()),
                other => Err(format!("Unknown capability: {}", other)),
            }
        });

        match tokio::time::timeout(Duration::from_secs(30), future).await {
            Ok(Ok(result)) => result,
            Ok(Err(join_err)) => Err(format!("Task join error: {}", join_err)),
            Err(_elapsed) => Err("Execution timed out after 30s".to_string()),
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // 5. task update & audit
    match exec_result {
        Ok(exec_out) => {
            let output_raw = exec_out.output.unwrap_or_default();
            let finalized =
                finalize_tool_output(state, &cap_name, &cap.implementation, &task_id, &output_raw);
            let status = if exec_out.exit_code == 0 {
                "success"
            } else {
                "failed"
            };
            let detail = finalized.output.as_deref().unwrap_or("(empty)");
            state.tasks.complete(&task_id, detail);
            state.audit.record(
                request.session_id.as_deref(),
                request.turn_id.as_deref(),
                "user",
                &cap_name,
                &target,
                cap.risk_level,
                status,
                Some(detail),
            );
            ExecuteResult {
                capability: cap_name,
                status: status.into(),
                output: finalized.output,
                artifact_id: finalized.artifact_id,
                artifact_path: finalized.artifact_path,
                error: exec_out.stderr.filter(|s| !s.is_empty()),
                task_id,
                duration_ms,
                policy_decision: "allowed".into(),
            }
        }
        Err(err_msg) => {
            state.tasks.fail(&task_id, &err_msg);
            state.audit.record(
                request.session_id.as_deref(),
                request.turn_id.as_deref(),
                "user",
                &cap_name,
                &target,
                cap.risk_level,
                "failed",
                Some(&err_msg),
            );
            ExecuteResult {
                capability: cap_name,
                status: "failed".into(),
                output: None,
                artifact_id: None,
                artifact_path: None,
                error: Some(err_msg),
                task_id,
                duration_ms,
                policy_decision: "allowed".into(),
            }
        }
    }
}

fn describe_target(cap: &str, params: &serde_json::Value) -> String {
    match cap {
        "bash" => params
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("(no command)")
            .to_string(),
        "read" => params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("(no path)")
            .to_string(),
        "artifact_read" => params
            .get("artifactId")
            .and_then(|v| v.as_str())
            .or_else(|| params.get("path").and_then(|v| v.as_str()))
            .unwrap_or("(no artifact)")
            .to_string(),
        "write" => params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("(no path)")
            .to_string(),
        "edit" => params
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("(no path)")
            .to_string(),
        "checklist_write" => params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("current checklist")
            .to_string(),
        "capability_register" => params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("(no name)")
            .to_string(),
        "action_register" => params
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("(no id)")
            .to_string(),
        "agent_delegate" => params
            .get("targetAgentId")
            .and_then(|v| v.as_str())
            .unwrap_or("(no target)")
            .to_string(),
        "agent_register" => params
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("(no id)")
            .to_string(),
        _ => format!("{:?}", params),
    }
}

fn tool_visible_in_mode(implementation: &str, risk: RiskLevel, mode: AgentMode) -> bool {
    if implementation == "checklist_write" {
        return true;
    }
    match mode {
        AgentMode::Plan => matches!(risk, RiskLevel::ReadOnly),
        AgentMode::Agent | AgentMode::Auto => true,
    }
}

fn tool_allowed_in_mode(implementation: &str, risk: RiskLevel, mode: AgentMode) -> bool {
    if implementation == "checklist_write" {
        return true;
    }
    match mode {
        AgentMode::Plan => matches!(risk, RiskLevel::ReadOnly),
        AgentMode::Agent | AgentMode::Auto => true,
    }
}

// ── Internal types & utilities ───────────────────────────────────

pub struct ExecOutput {
    pub output: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: i32,
}

const LARGE_RESULT_THRESHOLD: usize = 10_000;
const LARGE_RESULT_SUMMARY_LIMIT: usize = 1_800;

struct FinalizedToolOutput {
    output: Option<String>,
    artifact_id: Option<String>,
    artifact_path: Option<String>,
}

struct ArtifactHandle {
    id: String,
    path: PathBuf,
}

pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}\n... [truncated {} chars]",
            truncated, s.chars().count())
    }
}

fn finalize_tool_output(
    state: &AppState,
    capability: &str,
    implementation: &str,
    task_id: &str,
    output_raw: &str,
) -> FinalizedToolOutput {
    if output_raw.is_empty() {
        return FinalizedToolOutput {
            output: None,
            artifact_id: None,
            artifact_path: None,
        };
    }

    if implementation == "agent_delegate" || output_raw.chars().count() <= LARGE_RESULT_THRESHOLD {
        return FinalizedToolOutput {
            output: Some(output_raw.to_string()),
            artifact_id: None,
            artifact_path: None,
        };
    }

    match persist_output_artifact(state, capability, task_id, output_raw) {
        Ok(handle) => {
            let summary = format!(
                "输出较长，完整结果已保存到 artifact `{}`。\n路径：{}\n\n{}",
                handle.id,
                handle.path.display(),
                truncate(output_raw, LARGE_RESULT_SUMMARY_LIMIT)
            );
            FinalizedToolOutput {
                output: Some(summary),
                artifact_id: Some(handle.id),
                artifact_path: Some(handle.path.display().to_string()),
            }
        }
        Err(err) => {
            let fallback = format!(
                "{}\n\n[artifact 保存失败：{}]",
                truncate(output_raw, LARGE_RESULT_THRESHOLD),
                err
            );
            FinalizedToolOutput {
                output: Some(fallback),
                artifact_id: None,
                artifact_path: None,
            }
        }
    }
}

fn persist_output_artifact(
    state: &AppState,
    capability: &str,
    task_id: &str,
    output_raw: &str,
) -> Result<ArtifactHandle, String> {
    let dir = state.data_dir.join("runtime").join("artifacts");
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Create artifact dir {}: {}", dir.display(), e))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Artifact timestamp: {}", e))?
        .as_millis();
    let artifact_id = format!(
        "{}-{}-{}",
        sanitize_artifact_segment(task_id),
        sanitize_artifact_segment(capability),
        timestamp
    );
    let filename = format!("{}.txt", artifact_id);
    let path = dir.join(filename);
    fs::write(&path, output_raw)
        .map_err(|e| format!("Write artifact {}: {}", path.display(), e))?;
    Ok(ArtifactHandle {
        id: artifact_id,
        path,
    })
}

fn sanitize_artifact_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "artifact".to_string()
    } else {
        trimmed.to_string()
    }
}

fn execute_capability_list(state: &AppState) -> Result<ExecOutput, String> {
    let catalog = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    let views = catalog.list_views();
    let output = serde_json::to_string_pretty(&views)
        .map_err(|e| format!("Serialize capabilities: {}", e))?;
    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}

fn execute_capability_register(state: &AppState, params: &serde_json::Value) -> Result<ExecOutput, String> {
    let input: CapabilityUpsertInput = serde_json::from_value(params.clone())
        .map_err(|e| format!("Parse capability_register params: {}", e))?;
    let mut catalog = state
        .capability_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.upsert(input)?;
    catalog.sync_registry(&state.capabilities);
    drop(catalog);
    crate::sync_action_registry(state)?;
    let catalog = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    let output = serde_json::to_string_pretty(&catalog.list_views())
        .map_err(|e| format!("Serialize capabilities: {}", e))?;
    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}

fn execute_action_list(state: &AppState) -> Result<ExecOutput, String> {
    let active_capabilities: std::collections::HashSet<String> = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?
        .active_names()
        .into_iter()
        .collect();
    let catalog = state
        .action_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    let views = catalog.list_views(&active_capabilities);
    let output = serde_json::to_string_pretty(&views)
        .map_err(|e| format!("Serialize actions: {}", e))?;
    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}

fn execute_action_register(state: &AppState, params: &serde_json::Value) -> Result<ExecOutput, String> {
    let input: ActionUpsertInput = serde_json::from_value(params.clone())
        .map_err(|e| format!("Parse action_register params: {}", e))?;
    let known_capabilities: std::collections::HashSet<String> = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?
        .all_names()
        .into_iter()
        .collect();
    let mut catalog = state
        .action_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.upsert(input, &known_capabilities)?;
    drop(catalog);
    crate::sync_action_registry(state)?;
    let active_capabilities: std::collections::HashSet<String> = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?
        .active_names()
        .into_iter()
        .collect();
    let catalog = state
        .action_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    let output = serde_json::to_string_pretty(&catalog.list_views(&active_capabilities))
        .map_err(|e| format!("Serialize actions: {}", e))?;
    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}

fn execute_agent_list(state: &AppState, params: &serde_json::Value) -> Result<ExecOutput, String> {
    let session_id = params
        .get("session_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "agent_list 缺少 session_id".to_string())?;
    let current_agent_id = params
        .get("currentAgentId")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let profiles = state
        .get_delegatable_session_agents(session_id)
        .into_iter()
        .filter(|profile| profile.id != current_agent_id)
        .collect::<Vec<_>>();
    let output = serde_json::to_string_pretty(&profiles)
        .map_err(|e| format!("Serialize agent profiles: {}", e))?;
    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}

async fn execute_agent_delegate(state: &AppState, params: &serde_json::Value) -> Result<ExecOutput, String> {
    let session_id = params
        .get("session_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "agent_delegate 缺少 session_id".to_string())?;
    let turn_id = params
        .get("turn_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "agent_delegate 缺少 turn_id".to_string())?;
    let input: AgentDelegateInput = serde_json::from_value(params.clone())
        .map_err(|e| format!("Parse agent_delegate params: {}", e))?;
    let current_agent_id = params
        .get("currentAgentId")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if !current_agent_id.is_empty() && current_agent_id == input.target_agent_id {
        return Err("不能把任务委派给当前 agent 自己".into());
    }

    let target = state
        .get_delegatable_session_agents(session_id)
        .into_iter()
        .find(|profile| profile.id == input.target_agent_id)
        .ok_or_else(|| format!("未找到 agent '{}'", input.target_agent_id))?;

    let llm_config = {
        let cfg = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        cfg.active_llm_config()
            .ok_or_else(|| "当前未配置模型，无法委派子 Agent".to_string())?
    };

    let payload = crate::agent::delegate_to_agent(
        state,
        session_id,
        turn_id,
        &target,
        &input.task,
        &llm_config,
    )
    .await?;
    let output = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Serialize delegated agent result: {}", e))?;
    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}

fn execute_agent_register(state: &AppState, params: &serde_json::Value) -> Result<ExecOutput, String> {
    let input: AgentUpsertInput = serde_json::from_value(params.clone())
        .map_err(|e| format!("Parse agent_register params: {}", e))?;
    let known_capabilities: std::collections::HashSet<String> = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?
        .all_names()
        .into_iter()
        .collect();
    let mut catalog = state
        .agent_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.upsert(input, &known_capabilities)?;
    let output = serde_json::to_string_pretty(&catalog.list_views())
        .map_err(|e| format!("Serialize agents: {}", e))?;
    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}
