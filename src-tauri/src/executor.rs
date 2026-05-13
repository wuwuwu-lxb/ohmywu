use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::task;

use crate::AppState;

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

pub async fn execute_capability(
    state: &AppState,
    request: ExecuteRequest,
) -> ExecuteResult {
    // 1. capability lookup
    let cap = state.capabilities.get(&request.capability);
    let cap = match cap {
        Some(c) => c,
        None => {
            return ExecuteResult {
                capability: request.capability,
                status: "not_found".into(),
                output: None,
                error: Some("Capability not registered".into()),
                task_id: String::new(),
                duration_ms: 0,
                policy_decision: "denied".into(),
            };
        }
    };

    // 2. policy gate
    let decision = state.policy.check(cap.risk_level);
    if !decision.allowed {
        state.audit.record(
            "user",
            &request.capability,
            "(denied by policy)",
            cap.risk_level,
            "denied",
            Some(&format!("mode={:?}", decision.mode)),
        );
        return ExecuteResult {
            capability: request.capability,
            status: "denied".into(),
            output: None,
            error: Some(format!("Policy denied in {:?} mode", decision.mode)),
            task_id: String::new(),
            duration_ms: 0,
            policy_decision: "denied".into(),
        };
    }

    // 3. task creation
    let target = match request.capability.as_str() {
        "bash" => request
            .params
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("(no command)")
            .to_string(),
        "read" => request
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("(no path)")
            .to_string(),
        _ => format!("{:?}", request.params),
    };

    let task = state.tasks.create(&request.capability, &target);
    let task_id = task.id.clone();

    // 4. execution (in spawn_blocking with timeout to avoid hanging)
    let cap_name = request.capability.clone();
    let params = request.params.clone();

    let start = Instant::now();
    let exec_future = task::spawn_blocking(move || {
        match cap_name.as_str() {
            "bash" => execute_bash(&params),
            "read" => execute_read(&params),
            _ => Err(format!("Unknown capability: {}", cap_name)),
        }
    });

    let exec_result: Result<ExecOutput, String> = match tokio::time::timeout(
        Duration::from_secs(30),
        exec_future,
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(join_err)) => Err(format!("Task join error: {}", join_err)),
        Err(_elapsed) => Err("Execution timed out after 30s".to_string()),
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // 5. task update & audit
    match exec_result {
        Ok(exec_out) => {
            let output_raw = exec_out.output.unwrap_or_default();
            let output = Some(truncate(&output_raw, 10_000));
            let status = if exec_out.exit_code == 0 {
                "success"
            } else {
                "failed"
            };
            let detail = output.as_deref().unwrap_or("(empty)");
            state.tasks.complete(&task_id, detail);
            state.audit.record(
                "user",
                &request.capability,
                &target,
                cap.risk_level,
                status,
                Some(detail),
            );
            ExecuteResult {
                capability: request.capability,
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
                &request.capability,
                &target,
                cap.risk_level,
                "failed",
                Some(&err_msg),
            );
            ExecuteResult {
                capability: request.capability,
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

// ── capability implementations ──────────────────────────────────

struct ExecOutput {
    output: Option<String>,
    stderr: Option<String>,
    exit_code: i32,
}

fn execute_bash(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'command' parameter for bash".to_string())?;

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")))
        .output()
        .map_err(|e| format!("Bash execution failed: {}", e))?;

    Ok(ExecOutput {
        output: Some(String::from_utf8_lossy(&output.stdout).into_owned()),
        stderr: Some(String::from_utf8_lossy(&output.stderr).into_owned()),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

fn execute_read(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let path_str = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'path' parameter for read".to_string())?;

    let content =
        std::fs::read_to_string(path_str).map_err(|e| format!("Read failed: {}", e))?;

    Ok(ExecOutput {
        output: Some(content),
        stderr: None,
        exit_code: 0,
    })
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}\n... [truncated]", truncated)
    }
}
