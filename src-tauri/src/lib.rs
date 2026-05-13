mod agent;
mod config;
mod data_dir;
mod executor;
mod tools;

use std::sync::Arc;
use std::sync::RwLock;

use ohmywu_action_registry::ActionRegistry;
use ohmywu_audit::AuditLog;
use ohmywu_capability_registry::CapabilityRegistry;
use ohmywu_domain::*;
use ohmywu_policy_engine::PolicyEngine;
use ohmywu_session::{
    ExecutionRecord, SessionManager, SessionMessage, SessionSummary,
};
use ohmywu_task_engine::TaskEngine;
use tauri::Manager;

use config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub capabilities: Arc<CapabilityRegistry>,
    pub actions: Arc<ActionRegistry>,
    pub policy: Arc<PolicyEngine>,
    pub tasks: Arc<TaskEngine>,
    pub audit: Arc<AuditLog>,
    pub session: Arc<SessionManager>,
    pub config: Arc<RwLock<AppConfig>>,
    pub data_dir: std::path::PathBuf,
}

impl AppState {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        let capabilities = Arc::new(CapabilityRegistry::new());
        let actions = Arc::new(ActionRegistry::new());
        let policy = Arc::new(PolicyEngine::new());
        let tasks = Arc::new(TaskEngine::new());
        let audit = Arc::new(AuditLog::new());
        let session = Arc::new(SessionManager::new(data_dir.join("sessions")));

        // load config, apply policy mode
        let config: AppConfig = config::load_config(&data_dir).unwrap_or_default();
        policy.set_mode(config.policy_mode);

        let config = Arc::new(RwLock::new(config));

        // register initial capabilities
        register_capabilities(&capabilities);
        // register initial actions
        register_actions(&actions);

        Self {
            capabilities,
            actions,
            policy,
            tasks,
            audit,
            session,
            config,
            data_dir,
        }
    }
}

fn register_capabilities(registry: &CapabilityRegistry) {
    registry.register(Capability::new(
        "bash",
        "Execute a shell command. Subject to policy control.",
        RiskLevel::HighRisk,
    ));
    registry.register(Capability::new(
        "read",
        "Read file contents from the filesystem.",
        RiskLevel::ReadOnly,
    ));
}

fn register_actions(registry: &ActionRegistry) {
    registry.register(Action::new("shell.exec", "Execute a shell command"));
    registry.register(Action::new("fs.read", "Read a file"));
    registry.register(Action::new("system.info", "Get system information"));
}

// ── Tauri Commands: queries ──────────────────────────────────────

#[tauri::command]
fn get_capabilities(state: tauri::State<AppState>) -> Vec<Capability> {
    state.capabilities.list()
}

#[tauri::command]
fn get_actions(state: tauri::State<AppState>) -> Vec<Action> {
    state.actions.list()
}

#[tauri::command]
fn get_policy_mode(state: tauri::State<AppState>) -> PolicyMode {
    state.policy.current_mode()
}

#[tauri::command]
fn get_tasks(state: tauri::State<AppState>) -> Vec<Task> {
    state.tasks.list()
}

#[tauri::command]
fn get_audits(state: tauri::State<AppState>) -> Vec<AuditEvent> {
    state.audit.list(100)
}

// ── Tauri Commands: execution ────────────────────────────────────

#[tauri::command]
async fn execute_capability(
    request: executor::ExecuteRequest,
    state: tauri::State<'_, AppState>,
) -> Result<executor::ExecuteResult, String> {
    Ok(executor::execute_capability(&state, request).await)
}

// ── Tauri Commands: policy ───────────────────────────────────────

#[tauri::command]
async fn set_policy_mode(
    mode: PolicyMode,
    state: tauri::State<'_, AppState>,
) -> Result<PolicyMode, String> {
    state.policy.set_mode(mode);
    let mut config = state.config.write().map_err(|e| format!("Lock: {}", e))?;
    config.policy_mode = mode;
    config::save_config(&state.data_dir, &config)?;
    Ok(mode)
}

// ── Tauri Commands: session ──────────────────────────────────────

#[tauri::command]
async fn create_session(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<SessionSummary, String> {
    state.session.create_session(&name)
}

#[tauri::command]
async fn list_sessions(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SessionSummary>, String> {
    state.session.list_sessions()
}

#[tauri::command]
async fn load_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SessionMessage>, String> {
    state.session.load_session(&session_id)
}

#[tauri::command]
async fn delete_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.session.delete_session(&session_id)
}

// ── Tauri Commands: chat ─────────────────────────────────────────

#[tauri::command]
async fn send_message(
    session_id: String,
    content: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<SessionMessage, String> {
    let now = chrono_now();

    // save user message
    let user_msg = SessionMessage {
        role: "user".into(),
        content: content.clone(),
        executions: None,
        task_id: None,
        timestamp: now,
    };
    state.session.append_message(&session_id, &user_msg)?;

    // load LLM config
    let llm_config = {
        let cfg_guard = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        cfg_guard.llm_provider.clone()
    };

    // Phase 2: LLM agent loop (with fallback to Phase 1 mock if no LLM configured)
    let agent_response = if let Some(llm_cfg) = llm_config {
        let llm_adapter_config = ohmywu_llm_adapter::LlmConfig {
            provider_type: llm_cfg.provider_type,
            endpoint: llm_cfg.endpoint,
            model: llm_cfg.model,
            api_key: llm_cfg.api_key,
        };

        agent::agent_loop(
            &state,
            &session_id,
            &content,
            &llm_adapter_config,
            Some(&app_handle),
        )
        .await
        .unwrap_or_else(|e| {
            let context = format!(
                "（endpoint: {}, model: {}）",
                llm_adapter_config.endpoint, llm_adapter_config.model
            );
            let friendly = match e.as_str() {
                s if s.contains("400") => format!(
                    "请求格式错误 {}。\n请检查 Model 名称是否正确（如 qwen2.5、llama3.2）。\n用 `ollama list` 查看已下载的模型。\n\n本地指令：`read <路径>` / `run <命令>`",
                    context
                ),
                s if s.contains("401") => "API Key 无效，请在设置中更新。".into(),
                s if s.contains("404") => format!(
                    "Endpoint 无法访问 {}。\n确认 Ollama 已在 {} 启动。",
                    context, llm_adapter_config.endpoint
                ),
                s if s.contains("timeout") || s.contains("Timeout") => "请求超时，请检查网络或模型服务状态。".into(),
                s if s.contains("connection") || s.contains("Connect") => format!(
                    "无法连接到 {}。\n请确认 Ollama 已启动（ollama serve）。",
                    llm_adapter_config.endpoint
                ),
                _ => format!(
                    "LLM 暂时不可用。{}\n\n本地指令：`read <路径>` / `run <命令>`",
                    e
                ),
            };
            agent::AgentResponse {
                content: friendly,
                executions: vec![],
                task_id: None,
            }
        })
    } else {
        // Phase 1 fallback: simple command parsing
        if let Some(path) = parse_read_cmd(&content) {
            let req = executor::ExecuteRequest {
                capability: "read".into(),
                params: serde_json::json!({ "path": path }),
            };
            let result = executor::execute_capability(&state, req).await;
            let err_msg = result.error.clone().unwrap_or_default();
            let reply = match result.status.as_str() {
                "success" => result.output.clone().unwrap_or_else(|| "(empty)".into()),
                "denied" => format!("权限不足：{}", err_msg),
                _ => format!("读取失败：{}", err_msg),
            };
            agent::AgentResponse {
                content: reply,
                executions: vec![ExecutionRecord {
                    capability: "read".into(),
                    input: path.to_string(),
                    output: result.output.clone(),
                    error: result.error.clone(),
                    status: result.status.clone(),
                    duration_ms: result.duration_ms,
                }],
                task_id: Some(result.task_id),
            }
        } else if let Some(cmd) = parse_run_cmd(&content) {
            let req = executor::ExecuteRequest {
                capability: "bash".into(),
                params: serde_json::json!({ "command": cmd }),
            };
            let result = executor::execute_capability(&state, req).await;
            let err_msg = result.error.clone().unwrap_or_default();
            let reply = match result.status.as_str() {
                "success" => result.output.clone().unwrap_or_else(|| "(empty)".into()),
                "denied" => format!("权限不足：{}", err_msg),
                _ => format!("执行失败：{}", err_msg),
            };
            agent::AgentResponse {
                content: reply,
                executions: vec![ExecutionRecord {
                    capability: "bash".into(),
                    input: cmd.to_string(),
                    output: result.output.clone(),
                    error: result.error.clone(),
                    status: result.status.clone(),
                    duration_ms: result.duration_ms,
                }],
                task_id: Some(result.task_id),
            }
        } else {
            let reply = format!(
                "你好！我是 OhMyWu。\n\n当前未配置 LLM。设置页中配置 Ollama 或 OpenAI 端点即可开启 AI 对话。\n\n本地指令：\n- `read <路径>` — 读取文件\n- `run <命令>` — 执行 shell 命令\n\n当前策略模式：{:?}",
                state.policy.current_mode()
            );
            agent::AgentResponse {
                content: reply,
                executions: vec![],
                task_id: None,
            }
        }
    };

    let agent_msg = agent::build_agent_message(&agent_response);
    state.session.append_message(&session_id, &agent_msg)?;

    Ok(agent_msg)
}

fn parse_read_cmd(input: &str) -> Option<&str> {
    for prefix in &["read ", "cat "] {
        if let Some(rest) = input.strip_prefix(prefix) {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

fn parse_run_cmd(input: &str) -> Option<&str> {
    for prefix in &["run ", "bash "] {
        if let Some(rest) = input.strip_prefix(prefix) {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

// ── Tauri Commands: LLM test ─────────────────────────────────────

#[tauri::command]
async fn test_llm_connection(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let config = {
        let cfg = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        cfg.llm_provider.clone()
    };

    let llm_cfg = match config {
        Some(c) => c,
        None => return Err("未配置 LLM。".into()),
    };

    // Try a minimal chat without tools
    let adapter_cfg = ohmywu_llm_adapter::LlmConfig {
        provider_type: llm_cfg.provider_type,
        endpoint: llm_cfg.endpoint.clone(),
        model: llm_cfg.model.clone(),
        api_key: llm_cfg.api_key,
    };

    let provider = ohmywu_llm_adapter::create_provider(&adapter_cfg)?;
    let messages = vec![
        ohmywu_llm_adapter::types::ChatMessage::user("ping"),
    ];
    let tools = vec![];

    // Use non-streaming for test
    match provider.chat(&messages, &tools).await {
        Ok(resp) => Ok(format!(
            "连接成功！Model: {}, Response: {}",
            llm_cfg.model,
            resp.content.unwrap_or_else(|| "(no content)".into())
        )),
        Err(e) => Err(format!(
            "连接失败 — {}:{} — {}",
            llm_cfg.endpoint, llm_cfg.model, e
        )),
    }
}

// ── Tauri Commands: config ───────────────────────────────────────

#[tauri::command]
async fn get_config(
    state: tauri::State<'_, AppState>,
) -> Result<AppConfig, String> {
    let config = state.config.read().map_err(|e| format!("Lock: {}", e))?;
    Ok(config.clone())
}

#[tauri::command]
async fn save_config(
    config: AppConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // update policy engine
    state.policy.set_mode(config.policy_mode);
    // persist
    config::save_config(&state.data_dir, &config)?;
    // update in-memory
    let mut current = state.config.write().map_err(|e| format!("Lock: {}", e))?;
    *current = config;
    Ok(())
}

// ── Tauri App Entry ──────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let data_dir = data_dir::ensure_data_dirs()
                .expect("Failed to initialize data directory");
            let state = AppState::new(data_dir);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_capabilities,
            get_actions,
            get_policy_mode,
            get_tasks,
            get_audits,
            execute_capability,
            set_policy_mode,
            create_session,
            list_sessions,
            load_session,
            delete_session,
            send_message,
            get_config,
            save_config,
            test_llm_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
