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

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::task;

use ohmywu_domain::{AgentMode, RiskLevel};

use crate::AppState;

// ── Shared types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub capability: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub capability: String,
    pub status: String,
    pub output: Option<String>,
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
pub fn active_tool_defs(state: &AppState) -> Vec<ToolDef> {
    let caps = state.capabilities.list();
    let agent_mode = state
        .config
        .read()
        .map(|cfg| cfg.agent_mode)
        .unwrap_or(AgentMode::Agent);
    let mut tools: Vec<ToolDef> = caps
        .iter()
        .filter(|cap| tool_visible_in_mode(&cap.name, cap.risk_level, agent_mode))
        .filter_map(|cap| {
            let params = tool_params(&cap.name)?;
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
        a_score.cmp(&b_score)
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
    let cap = match state.capabilities.get(&cap_name) {
        Some(c) => c,
        None => {
            return ExecuteResult {
                capability: cap_name.clone(),
                status: "not_found".into(),
                output: None,
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
    if !tool_allowed_in_mode(&cap_name, cap.risk_level, agent_mode) {
        return ExecuteResult {
            capability: cap_name,
            status: "denied".into(),
            output: None,
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
            &cap_name,
            &params,
            Some(ToolKind::from_risk(cap.risk_level)),
            agent_mode,
        )
    };

    match permission_result {
        crate::permission::PermissionCheck::Denied(msg) => {
            state.audit.record(
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
                error: None,
                task_id: String::new(),
                duration_ms: 0,
                policy_decision: "needs_confirm".into(),
            };
        }
        crate::permission::PermissionCheck::Allowed => {}
    }

    // 4. task creation
    let target = describe_target(&cap_name, &params);
    let task = state.tasks.create(&cap_name, &target);
    let task_id = task.id.clone();

    // 4. execute (with spawn_blocking + timeout)
    let start = Instant::now();

    let exec_result: Result<ExecOutput, String> = if cap_name == "thinking" {
        // thinking is instant, no blocking needed
        thinking::execute(&params)
    } else if cap_name == "checklist_write" {
        checklist::write(&params, &state.runtime)
    } else if cap_name.starts_with("wiki_") {
        // wiki tools: read/write/search/list/graph — fast file I/O
        let wiki_lock = state.wiki.read().unwrap();
        match cap_name.as_str() {
            "wiki_read" => wiki::read(&params, &wiki_lock),
            "wiki_write" => wiki::write(&params, &wiki_lock),
            "wiki_search" => wiki::search(&params, &wiki_lock),
            "wiki_list" => wiki::list(&params, &wiki_lock),
            "wiki_graph" => wiki::graph(&params, &wiki_lock),
            other => Err(format!("Unknown capability: {}", other)),
        }
    } else {
        let cap_name_clone = cap_name.clone();
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
            let output = Some(truncate(&output_raw, LARGE_RESULT_THRESHOLD));
            let status = if exec_out.exit_code == 0 {
                "success"
            } else {
                "failed"
            };
            let detail = output.as_deref().unwrap_or("(empty)");
            state.tasks.complete(&task_id, detail);
            state.audit.record(
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
                output,
                error: exec_out.stderr.filter(|s| !s.is_empty()),
                task_id,
                duration_ms,
                policy_decision: "allowed".into(),
            }
        }
        Err(err_msg) => {
            state.tasks.fail(&task_id, &err_msg);
            state.audit.record(
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
        _ => format!("{:?}", params),
    }
}

fn tool_visible_in_mode(name: &str, risk: RiskLevel, mode: AgentMode) -> bool {
    if name == "checklist_write" {
        return true;
    }
    match mode {
        AgentMode::Plan => matches!(risk, RiskLevel::ReadOnly),
        AgentMode::Agent | AgentMode::Auto => true,
    }
}

fn tool_allowed_in_mode(name: &str, risk: RiskLevel, mode: AgentMode) -> bool {
    if name == "checklist_write" {
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

pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}\n... [truncated {} chars]",
            truncated, s.chars().count())
    }
}
