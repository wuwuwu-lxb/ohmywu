mod agent;
mod agent_catalog;
mod action_catalog;
mod capabilities;
mod config;
mod data_dir;
mod external_sessions;
mod permission;
mod runtime;
mod tools;
mod wechat_bridge;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ohmywu_action_registry::ActionRegistry;
use ohmywu_audit::AuditLog;
use ohmywu_capability_registry::CapabilityRegistry;
use ohmywu_domain::*;
use ohmywu_policy_engine::PolicyEngine;
use ohmywu_session::{
    ExecutionRecord, SessionManager, SessionMessage, SessionSummary,
};
use ohmywu_task_engine::TaskEngine;
use tauri::Emitter;
use tauri::Manager;
use ohmywu_domain::AuditEvent;
use ohmywu_llm_adapter::{HealthStatus, LlmConfig, ProviderMetadata};
use ohmywu_wiki::WikiEngine;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use agent_catalog::{AgentCatalog, AgentUpsertInput, AgentView};
use action_catalog::{ActionBlueprint, ActionCatalog, ActionUpsertInput, ActionView};
use capabilities::{CapabilityCatalog, CapabilityUpsertInput, CapabilityView};
use config::AppConfig;
use external_sessions::{ExternalRouteInput, ExternalRouteStateView, ExternalSessionStore};
use runtime::{RuntimeStore, RuntimeThreadView};
use wechat_bridge::{
    WechatBridgeConfig, WechatBridgeLoginSession, WechatBridgeStore, WechatBridgeView,
    WECHAT_BRIDGE_BIND_ADDR, WECHAT_BRIDGE_HEADER_TOKEN, WECHAT_BRIDGE_MESSAGE_PATH,
    WECHAT_BRIDGE_QR_PATH,
};

#[derive(Clone)]
pub struct AppState {
    pub capabilities: Arc<CapabilityRegistry>,
    pub capability_catalog: Arc<RwLock<CapabilityCatalog>>,
    pub actions: Arc<ActionRegistry>,
    pub action_catalog: Arc<RwLock<ActionCatalog>>,
    pub agent_catalog: Arc<RwLock<AgentCatalog>>,
    pub session_agents: Arc<RwLock<HashMap<String, Vec<agent::AgentInvocationProfile>>>>,
    pub policy: Arc<PolicyEngine>,
    pub tasks: Arc<TaskEngine>,
    pub audit: Arc<AuditLog>,
    pub session: Arc<SessionManager>,
    pub config: Arc<RwLock<AppConfig>>,
    pub data_dir: std::path::PathBuf,
    pub wiki: Arc<RwLock<WikiEngine>>,
    pub runtime: Arc<RuntimeStore>,
    pub external_sessions: Arc<ExternalSessionStore>,
    pub wechat_bridge: Arc<WechatBridgeStore>,
    pub cancel_token: Arc<AtomicBool>,
    pub app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
}

impl AppState {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        let capabilities = Arc::new(CapabilityRegistry::new());
        let capability_catalog = Arc::new(RwLock::new(
            CapabilityCatalog::load(&data_dir)
                .expect("failed to initialize capability catalog"),
        ));
        let actions = Arc::new(ActionRegistry::new());
        let action_catalog = Arc::new(RwLock::new(
            ActionCatalog::load(&data_dir)
                .expect("failed to initialize action catalog"),
        ));
        let agent_catalog = Arc::new(RwLock::new(
            AgentCatalog::load(&data_dir)
                .expect("failed to initialize agent catalog"),
        ));
        let session_agents = Arc::new(RwLock::new(HashMap::new()));
        let policy = Arc::new(PolicyEngine::new());
        let tasks = Arc::new(TaskEngine::new());
        let audit = Arc::new(
            AuditLog::load(&data_dir.join("audit"))
                .expect("failed to initialize audit log"),
        );
        let session = Arc::new(SessionManager::new(data_dir.join("sessions")));
        let runtime = Arc::new(
            RuntimeStore::new(data_dir.join("runtime"))
                .expect("failed to initialize runtime store"),
        );
        let external_sessions = Arc::new(
            ExternalSessionStore::load(&data_dir)
                .expect("failed to initialize external session store"),
        );
        let wechat_bridge = Arc::new(
            WechatBridgeStore::load(&data_dir)
                .expect("failed to initialize wechat bridge"),
        );
        let cancel_token = Arc::new(AtomicBool::new(false));
        let app_handle = Arc::new(RwLock::new(None));

        // load config, apply policy mode
        let config: AppConfig = config::load_config(&data_dir).unwrap_or_default();
        policy.set_mode(config.policy_mode);

        let config = Arc::new(RwLock::new(config));

        // init wiki
        let wiki = Arc::new(RwLock::new(
            WikiEngine::new(data_dir.join("wiki")),
        ));
        {
            let w = wiki.write().unwrap();
            w.init().unwrap_or_else(|e| eprintln!("wiki init: {}", e));
        }

        {
            let catalog = capability_catalog.read().unwrap();
            catalog.sync_registry(&capabilities);
        }
        {
            let capability_names = capability_catalog
                .read()
                .unwrap()
                .active_names()
                .into_iter()
                .collect();
            let catalog = action_catalog.read().unwrap();
            catalog.sync_registry(&actions, &capability_names);
        }

        Self {
            capabilities,
            capability_catalog,
            actions,
            action_catalog,
            agent_catalog,
            session_agents,
            policy,
            tasks,
            audit,
            session,
            config,
            data_dir,
            wiki,
            runtime,
            external_sessions,
            wechat_bridge,
            cancel_token,
            app_handle,
        }
    }

    pub fn set_session_agents(
        &self,
        session_id: &str,
        profiles: Vec<agent::AgentInvocationProfile>,
    ) {
        if let Ok(mut store) = self.session_agents.write() {
            store.insert(session_id.to_string(), profiles);
        }
    }

    pub fn get_session_agents(
        &self,
        session_id: &str,
    ) -> Vec<agent::AgentInvocationProfile> {
        if let Some(profiles) = self
            .session_agents
            .read()
            .ok()
            .and_then(|store| store.get(session_id).cloned())
        {
            return profiles;
        }
        self.agent_catalog
            .read()
            .map(|catalog| catalog.list_profiles())
            .unwrap_or_default()
    }

    pub fn get_delegatable_session_agents(
        &self,
        session_id: &str,
    ) -> Vec<agent::AgentInvocationProfile> {
        let mut items = self
            .get_session_agents(session_id)
            .into_iter()
            .filter(|profile| profile.delegatable)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| {
            b.delegate_priority
                .cmp(&a.delegate_priority)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase()))
        });
        items
    }

    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        if let Ok(mut slot) = self.app_handle.write() {
            *slot = Some(handle);
        }
    }

    pub fn get_app_handle(&self) -> Option<tauri::AppHandle> {
        self.app_handle
            .read()
            .ok()
            .and_then(|slot| slot.clone())
    }
}

fn sync_action_registry(state: &AppState) -> Result<(), String> {
    let capability_names = state
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
    catalog.sync_registry(&state.actions, &capability_names);
    Ok(())
}

// ── Tauri Commands: queries ──────────────────────────────────────

#[tauri::command]
fn get_capabilities(state: tauri::State<AppState>) -> Result<Vec<CapabilityView>, String> {
    let catalog = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    Ok(catalog.list_views())
}

#[tauri::command]
fn upsert_capability(
    input: CapabilityUpsertInput,
    state: tauri::State<AppState>,
) -> Result<Vec<CapabilityView>, String> {
    let mut catalog = state
        .capability_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.upsert(input)?;
    catalog.sync_registry(&state.capabilities);
    drop(catalog);
    sync_action_registry(&state)?;
    let catalog = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    Ok(catalog.list_views())
}

#[tauri::command]
fn set_capability_enabled(
    name: String,
    enabled: bool,
    state: tauri::State<AppState>,
) -> Result<Vec<CapabilityView>, String> {
    let mut catalog = state
        .capability_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.set_enabled(&name, enabled)?;
    catalog.sync_registry(&state.capabilities);
    drop(catalog);
    sync_action_registry(&state)?;
    let catalog = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    Ok(catalog.list_views())
}

#[tauri::command]
fn delete_capability(
    name: String,
    state: tauri::State<AppState>,
) -> Result<Vec<CapabilityView>, String> {
    let mut catalog = state
        .capability_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.delete(&name)?;
    catalog.sync_registry(&state.capabilities);
    drop(catalog);
    sync_action_registry(&state)?;
    let catalog = state
        .capability_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    Ok(catalog.list_views())
}

#[tauri::command]
fn get_actions(state: tauri::State<AppState>) -> Result<Vec<ActionView>, String> {
    let capability_names = state
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
    Ok(catalog.list_views(&capability_names))
}

#[tauri::command]
fn get_agents(state: tauri::State<AppState>) -> Result<Vec<AgentView>, String> {
    let catalog = state
        .agent_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    Ok(catalog.list_views())
}

#[tauri::command]
fn upsert_agent(
    input: AgentUpsertInput,
    state: tauri::State<AppState>,
) -> Result<Vec<AgentView>, String> {
    let known_capabilities = state
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
    Ok(catalog.list_views())
}

#[tauri::command]
fn delete_agent(
    id: String,
    state: tauri::State<AppState>,
) -> Result<Vec<AgentView>, String> {
    let mut catalog = state
        .agent_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.delete(&id)?;
    Ok(catalog.list_views())
}

#[tauri::command]
fn refresh_actions(state: tauri::State<AppState>) -> Result<Vec<ActionView>, String> {
    sync_action_registry(&state)?;
    get_actions(state)
}

#[tauri::command]
fn upsert_action(
    input: ActionUpsertInput,
    state: tauri::State<AppState>,
) -> Result<Vec<ActionView>, String> {
    let known_capabilities = state
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
    sync_action_registry(&state)?;
    get_actions(state)
}

#[tauri::command]
fn set_action_enabled(
    id: String,
    enabled: bool,
    state: tauri::State<AppState>,
) -> Result<Vec<ActionView>, String> {
    let mut catalog = state
        .action_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.set_enabled(&id, enabled)?;
    drop(catalog);
    sync_action_registry(&state)?;
    get_actions(state)
}

#[tauri::command]
fn delete_action(
    id: String,
    state: tauri::State<AppState>,
) -> Result<Vec<ActionView>, String> {
    let mut catalog = state
        .action_catalog
        .write()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.delete(&id)?;
    drop(catalog);
    sync_action_registry(&state)?;
    get_actions(state)
}

#[tauri::command]
fn get_action_blueprint(
    action_id: String,
    state: tauri::State<AppState>,
) -> Result<ActionBlueprint, String> {
    let catalog = state
        .action_catalog
        .read()
        .map_err(|e| format!("Lock: {}", e))?;
    catalog.get_blueprint(&action_id)
}

#[tauri::command]
fn get_policy_mode(state: tauri::State<AppState>) -> PolicyMode {
    state.policy.current_mode()
}

#[tauri::command]
fn get_agent_mode(state: tauri::State<AppState>) -> Result<AgentMode, String> {
    let config = state.config.read().map_err(|e| format!("Lock: {}", e))?;
    Ok(config.agent_mode)
}

#[tauri::command]
fn get_tasks(state: tauri::State<AppState>) -> Vec<Task> {
    state.tasks.list()
}

#[tauri::command]
fn get_llm_providers() -> Vec<ProviderMetadata> {
    ohmywu_llm_adapter::provider::builtin_providers().to_vec()
}

#[tauri::command]
fn get_audits(state: tauri::State<AppState>) -> Vec<AuditEvent> {
    state.audit.list(200)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditExportResult {
    path: String,
    count: usize,
}

#[tauri::command]
fn clear_audits(state: tauri::State<AppState>) -> Result<(), String> {
    state.audit.clear()
}

#[tauri::command]
fn export_audits(
    session_id: Option<String>,
    state: tauri::State<AppState>,
) -> Result<AuditExportResult, String> {
    let events = match session_id.as_deref() {
        Some("__system__") => state.audit.list_by_session(None),
        Some(id) => state.audit.list_by_session(Some(id)),
        None => state.audit.list_all(),
    };
    let export_dir = state.data_dir.join("exports").join("audits");
    fs::create_dir_all(&export_dir)
        .map_err(|e| format!("Create export dir {}: {}", export_dir.display(), e))?;

    let label = if let Some(session_id) = session_id.as_deref() {
        if session_id == "__system__" {
            "system".to_string()
        } else {
        state
            .session
            .get_session_summary(session_id)?
            .map(|summary| summary.name)
            .unwrap_or_else(|| session_id.to_string())
        }
    } else {
        "all".to_string()
    };
    let sanitized = sanitize_filename(&label);
    let timestamp = chrono_now()
        .replace(':', "-")
        .replace('.', "-");
    let path = export_dir.join(format!("audit-{}-{}.json", sanitized, timestamp));
    let content = serde_json::to_string_pretty(&events)
        .map_err(|e| format!("Serialize audit export: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Write audit export {}: {}", path.display(), e))?;
    Ok(AuditExportResult {
        path: path.display().to_string(),
        count: events.len(),
    })
}

// ── Tauri Commands: execution ────────────────────────────────────

#[tauri::command]
async fn execute_capability(
    request: tools::ExecuteRequest,
    state: tauri::State<'_, AppState>,
) -> Result<tools::ExecuteResult, String> {
    Ok(tools::dispatch_tool(&state, request).await)
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

#[tauri::command]
async fn set_agent_mode(
    mode: AgentMode,
    state: tauri::State<'_, AppState>,
) -> Result<AgentMode, String> {
    let mut config = state.config.write().map_err(|e| format!("Lock: {}", e))?;
    config.agent_mode = mode;
    config::save_config(&state.data_dir, &config)?;
    Ok(mode)
}

// ── Tauri Commands: session ──────────────────────────────────────

#[tauri::command]
async fn create_session(
    name: String,
    category: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<SessionSummary, String> {
    state.session.create_session(&name, category.as_deref())
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
        .and_then(|_| state.external_sessions.remove_session_references(&session_id, &state.session))
        .and_then(|_| state.runtime.delete_thread(&session_id))
}

#[tauri::command]
async fn update_session_meta(
    session_id: String,
    name: Option<String>,
    category: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<SessionSummary, String> {
    state
        .session
        .update_session_meta(&session_id, name.as_deref(), category.as_deref())
}

#[tauri::command]
async fn load_runtime_thread(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<RuntimeThreadView>, String> {
    state.runtime.load_thread_view(&session_id)
}

// ── Tauri Commands: chat ─────────────────────────────────────────

#[tauri::command]
fn cancel_agent(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.cancel_token.store(true, Ordering::SeqCst);
    Ok(())
}

async fn process_message(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    session_id: &str,
    content: &str,
    agent_profile: Option<agent::AgentInvocationProfile>,
    agent_profiles: Option<Vec<agent::AgentInvocationProfile>>,
    llm_profile_override_id: Option<&str>,
    stream_chat: bool,
    emit_session_update: bool,
) -> Result<SessionMessage, String> {
    state.cancel_token.store(false, Ordering::SeqCst);
    state.set_app_handle(app_handle.clone());

    let now = chrono_now();
    let mut known_profiles = agent_profiles.unwrap_or_default();
    if let Some(profile) = &agent_profile
        && !known_profiles.iter().any(|item| item.id == profile.id)
    {
        known_profiles.push(profile.clone());
    }
    if !known_profiles.is_empty() {
        state.set_session_agents(&session_id, known_profiles);
    }
    let agent_mode = {
        let cfg_guard = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        cfg_guard.agent_mode
    };
    let turn = state.runtime.start_turn(session_id, agent_mode, content)?;
    let _ = app_handle.emit(
        "runtime-event",
        serde_json::json!({
            "sessionId": session_id,
            "kind": "turn.started",
            "turnId": turn.id,
            "summary": "开始新回合",
            "status": "running",
            "payload": {
              "agentMode": agent_mode,
              "userContent": content,
              "agentName": agent_profile.as_ref().map(|profile| profile.name.clone()),
            },
            "timestamp": now,
        }),
    );

    // save user message
    let user_msg = SessionMessage {
        role: "user".into(),
        content: content.to_string(),
        agent_id: agent_profile.as_ref().map(|profile| profile.id.clone()),
        agent_name: agent_profile.as_ref().map(|profile| profile.name.clone()),
        turn_id: Some(turn.id.clone()),
        reasoning_content: None,
        executions: None,
        task_id: None,
        timestamp: now,
    };
    state.session.append_message(session_id, &user_msg)?;

    let command_response = handle_slash_command(state, content)?;

    let llm_config = {
        let cfg_guard = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        resolve_llm_config(&cfg_guard, llm_profile_override_id)
    };

    let agent_response = if let Some(response) = command_response {
        response
    } else if let Some(llm_config) = llm_config {
        agent::agent_loop(
            state,
            session_id,
            &turn.id,
            content,
            agent_profile.as_ref(),
            &llm_config,
            Some(app_handle),
            stream_chat,
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("agent loop failed: {}", e);
            let friendly = format!(
                "{}\n\n详细信息：{}\nendpoint: {}\nmodel: {}\n\n本地指令：`read <路径>` / `run <命令>`",
                e.user_friendly(),
                e,
                llm_config.endpoint,
                llm_config.model,
            );
            agent::AgentResponse {
                content: friendly,
                reasoning_content: None,
                executions: vec![],
                task_id: None,
            }
        })
    } else if let Some(path) = parse_read_cmd(content) {
        let req = tools::ExecuteRequest {
            capability: "read".into(),
            params: serde_json::json!({ "path": path }),
            session_id: Some(session_id.to_string()),
            turn_id: Some(turn.id.clone()),
        };
        let result = tools::dispatch_tool(state, req).await;
        let err_msg = result.error.clone().unwrap_or_default();
        let reply = match result.status.as_str() {
            "success" => result.output.clone().unwrap_or_else(|| "(empty)".into()),
            "denied" => format!("权限不足：{}", err_msg),
            _ => format!("读取失败：{}", err_msg),
        };
        agent::AgentResponse {
            content: reply,
            reasoning_content: None,
            executions: vec![ExecutionRecord {
                capability: "read".into(),
                input: path.to_string(),
                output: result.output.clone(),
                artifact_id: result.artifact_id.clone(),
                artifact_path: result.artifact_path.clone(),
                verification_hint: None,
                error: result.error.clone(),
                status: result.status.clone(),
                duration_ms: result.duration_ms,
            }],
            task_id: Some(result.task_id),
        }
    } else if let Some(cmd) = parse_run_cmd(content) {
        let req = tools::ExecuteRequest {
            capability: "bash".into(),
            params: serde_json::json!({ "command": cmd }),
            session_id: Some(session_id.to_string()),
            turn_id: Some(turn.id.clone()),
        };
        let result = tools::dispatch_tool(state, req).await;
        let err_msg = result.error.clone().unwrap_or_default();
        let reply = match result.status.as_str() {
            "success" => result.output.clone().unwrap_or_else(|| "(empty)".into()),
            "denied" => format!("权限不足：{}", err_msg),
            _ => format!("执行失败：{}", err_msg),
        };
        agent::AgentResponse {
            content: reply,
            reasoning_content: None,
            executions: vec![ExecutionRecord {
                capability: "bash".into(),
                input: cmd.to_string(),
                output: result.output.clone(),
                artifact_id: result.artifact_id.clone(),
                artifact_path: result.artifact_path.clone(),
                verification_hint: Some(
                    "if the command changed files, git state, or environment assumptions, use a read-only tool to verify the specific side effect before making stronger claims"
                        .to_string(),
                ),
                error: result.error.clone(),
                status: result.status.clone(),
                duration_ms: result.duration_ms,
            }],
            task_id: Some(result.task_id),
        }
    } else {
        let reply = format!(
            "你好！我是 OhMyWu。\n\n当前未配置 LLM。设置页中配置 Ollama 或 OpenAI 端点即可开启 AI 对话。\n\n本地指令：\n- `read <路径>` — 读取文件\n- `run <命令>` — 执行 shell 命令\n- `/profile <配置名>` — 切换模型配置\n- `/provider <供应商>` — 修改当前 provider\n- `/model <模型名>` — 修改当前 model\n\n当前策略模式：{:?}",
            state.policy.current_mode()
        );
        agent::AgentResponse {
            content: reply,
            reasoning_content: None,
            executions: vec![],
            task_id: None,
        }
    };

    let agent_msg = agent::build_agent_message(&agent_response, &turn.id, agent_profile.as_ref());
    state.session.append_message(session_id, &agent_msg)?;
    let completed_turn = state.runtime.finish_turn(
        session_id,
        &turn.id,
        &agent_response.content,
        agent_response.executions.len(),
    )?;
    let _ = app_handle.emit(
        "runtime-event",
        serde_json::json!({
            "sessionId": session_id,
            "kind": "turn.completed",
            "turnId": completed_turn.id,
            "summary": format!("回合完成，执行 {} 个工具", completed_turn.execution_count),
            "status": completed_turn.status,
            "payload": {
              "executionCount": completed_turn.execution_count,
              "assistantContent": agent_response.content,
            },
            "timestamp": completed_turn.finished_at,
        }),
    );

    if emit_session_update {
        let _ = app_handle.emit(
            "session-updated",
            serde_json::json!({
                "sessionId": session_id,
                "turnId": turn.id,
                "source": "background",
            }),
        );
    }

    Ok(agent_msg)
}

#[tauri::command]
async fn send_message(
    session_id: String,
    content: String,
    agent_profile: Option<agent::AgentInvocationProfile>,
    agent_profiles: Option<Vec<agent::AgentInvocationProfile>>,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<SessionMessage, String> {
    process_message(
        &state,
        &app_handle,
        &session_id,
        &content,
        agent_profile,
        agent_profiles,
        None,
        true,
        false,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalMessageInput {
    provider: String,
    account_id: String,
    chat_type: String,
    peer_id: String,
    peer_name: Option<String>,
    content: String,
    agent_profile: Option<agent::AgentInvocationProfile>,
    agent_profiles: Option<Vec<agent::AgentInvocationProfile>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExternalMessageResult {
    session_id: String,
    session_name: String,
    created_session: bool,
    control_only: bool,
    reply: Option<SessionMessage>,
}

#[tauri::command]
async fn receive_external_message(
    input: ExternalMessageInput,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ExternalMessageResult, String> {
    handle_external_message_input(&state, &app_handle, input).await
}

async fn handle_external_message_input(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    input: ExternalMessageInput,
) -> Result<ExternalMessageResult, String> {
    let bridge_config = state.wechat_bridge.get_config().ok();
    let route = ExternalRouteInput {
        provider: input.provider,
        account_id: input.account_id,
        chat_type: input.chat_type,
        peer_id: input.peer_id,
        peer_name: input.peer_name,
    };
    let reply_provider = route.provider.clone();
    let reply_account_id = route.account_id.clone();
    let reply_chat_type = route.chat_type.clone();
    let reply_peer_id = route.peer_id.clone();

    if let Some(name) = parse_external_new_command(&input.content) {
        let summary = state
            .external_sessions
            .create_new_session(route, name.as_deref(), &state.session)?;
        let _ = app_handle.emit(
            "session-updated",
            serde_json::json!({
                "sessionId": summary.id,
                "source": "external-control",
            }),
        );
        let reply = SessionMessage {
            role: "agent".into(),
            content: format!(
                "已新建对话「{}」。后续这个微信会话的消息会进入该对话，直到再次执行 `/new`。",
                summary.name
            ),
            agent_id: None,
            agent_name: Some("OhMyWu".into()),
            turn_id: None,
            reasoning_content: None,
            executions: None,
            task_id: None,
            timestamp: chrono_now(),
        };
        if reply_provider == "wechat"
            && let Err(error) = send_wechat_text_reply(
                state,
                app_handle,
                &reply_account_id,
                &reply_chat_type,
                &reply_peer_id,
                &reply.content,
            )
            .await
        {
            let _ = app_handle.emit(
                "wechat-bridge-event",
                serde_json::json!({
                    "kind": "wechat.outbound.error",
                    "error": error,
                }),
            );
        }
        return Ok(ExternalMessageResult {
            session_id: summary.id,
            session_name: summary.name,
            created_session: true,
            control_only: true,
            reply: Some(reply),
        });
    }

    let configured_agent_profile = if input.agent_profile.is_none() {
        bridge_config
            .as_ref()
            .and_then(|config| config.agent_id.as_deref())
            .and_then(|id| state.agent_catalog.read().ok().and_then(|catalog| catalog.get_profile(id)))
    } else {
        None
    };
    let llm_profile_override_id = bridge_config
        .as_ref()
        .and_then(|config| config.llm_profile_id.as_deref());

    let (summary, created_session) = state
        .external_sessions
        .resolve_or_create_session(route, &state.session)?;
    let reply = process_message(
        &state,
        &app_handle,
        &summary.id,
        &input.content,
        input.agent_profile.or(configured_agent_profile),
        input.agent_profiles,
        llm_profile_override_id,
        false,
        true,
    )
    .await?;

    if reply_provider == "wechat"
        && let Err(error) = send_wechat_text_reply(
            state,
            app_handle,
            &reply_account_id,
            &reply_chat_type,
            &reply_peer_id,
            &reply.content,
        )
        .await
    {
        let _ = app_handle.emit(
            "wechat-bridge-event",
            serde_json::json!({
                "kind": "wechat.outbound.error",
                "error": error,
            }),
        );
    }

    Ok(ExternalMessageResult {
        session_id: summary.id,
        session_name: summary.name,
        created_session,
        control_only: false,
        reply: Some(reply),
    })
}

#[tauri::command]
fn get_external_route_state(
    provider: String,
    account_id: String,
    chat_type: String,
    peer_id: String,
    peer_name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Option<ExternalRouteStateView>, String> {
    state.external_sessions.get_route_state(
        ExternalRouteInput {
            provider,
            account_id,
            chat_type,
            peer_id,
            peer_name,
        },
        &state.session,
    )
}

#[tauri::command]
fn list_external_route_states(
    provider: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ExternalRouteStateView>, String> {
    state
        .external_sessions
        .list_route_states(provider.as_deref(), &state.session)
}

fn resolve_llm_config(config: &AppConfig, override_profile_id: Option<&str>) -> Option<LlmConfig> {
    override_profile_id
        .and_then(|profile_id| config.llm_config_by_id(profile_id))
        .or_else(|| config.active_llm_config())
}

#[tauri::command]
async fn render_qr_svg(content: String) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("二维码内容不能为空".into());
    }

    let output = tokio::process::Command::new("qrencode")
        .arg("-t")
        .arg("SVG")
        .arg("-m")
        .arg("2")
        .arg("-o")
        .arg("-")
        .arg(trimmed)
        .output()
        .await
        .map_err(|e| format!("启动 qrencode 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("qrencode 失败，退出码 {:?}", output.status.code())
        } else {
            format!("qrencode 失败: {}", stderr)
        });
    }

    String::from_utf8(output.stdout).map_err(|e| format!("解析二维码 SVG 失败: {}", e))
}

#[tauri::command]
async fn generate_memory_candidate(
    session_id: String,
    turn_id: Option<String>,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<agent::MemoryCandidateDraft, String> {
    let source = {
        let messages = state.session.load_session(&session_id)?;
        agent::resolve_turn_memory_source(&messages, turn_id.as_deref())?
    };

    let llm_config = {
        let cfg_guard = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        cfg_guard
            .active_llm_config()
            .ok_or_else(|| "当前未配置模型，无法生成记忆候选".to_string())?
    };

    let candidate = agent::generate_memory_candidate(&llm_config, &source)
        .await
        .map_err(|err| format!("生成记忆候选失败: {}", err))?;

    if let Ok(event) = state.runtime.record_event(
        &session_id,
        Some(&source.turn_id),
        "memory.candidate.generated",
        if candidate.should_save {
            "已生成记忆候选"
        } else {
            "已生成候选，但建议忽略"
        },
        serde_json::json!({
            "title": candidate.title,
            "folder": candidate.folder,
            "tags": candidate.tags,
            "shouldSave": candidate.should_save,
            "reason": candidate.reason,
        }),
    ) {
        let _ = app_handle.emit("runtime-event", &event);
    }

    Ok(candidate)
}

#[tauri::command]
fn save_memory_candidate(
    session_id: String,
    turn_id: Option<String>,
    candidate: agent::MemoryCandidateDraft,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ohmywu_wiki::WikiNote, String> {
    let title = candidate.title.trim();
    let body = candidate.body.trim();
    if title.is_empty() {
        return Err("记忆标题不能为空".into());
    }
    if body.is_empty() {
        return Err("记忆正文不能为空".into());
    }

    let folder = agent::normalize_memory_folder(&candidate.folder);
    let tags = candidate
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();

    let note = {
        let wiki = state.wiki.read().map_err(|e| format!("Lock: {}", e))?;
        wiki.write_note(title, title, body, &tags, &folder)?
    };

    if let Some(turn_id) = turn_id.as_deref()
        && let Ok(event) = state.runtime.record_event(
            &session_id,
            Some(turn_id),
            "memory.saved",
            "已写入知识库",
            serde_json::json!({
                "slug": note.slug,
                "title": note.title,
                "folder": note.folder,
                "tags": note.tags,
            }),
        )
    {
        let _ = app_handle.emit("runtime-event", &event);
    }

    Ok(note)
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

fn parse_slash_command(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    let raw = trimmed.strip_prefix('/')?;
    if raw.is_empty() {
        return Some(("", ""));
    }
    if let Some((command, rest)) = raw.split_once(char::is_whitespace) {
        return Some((command.trim(), rest.trim()));
    }
    Some((raw.trim(), ""))
}

fn handle_slash_command(
    state: &AppState,
    input: &str,
) -> Result<Option<agent::AgentResponse>, String> {
    let Some((command, args)) = parse_slash_command(input) else {
        return Ok(None);
    };

    match command {
        "" | "help" => Ok(Some(agent::AgentResponse {
            content: "可用命令：\n- `/profile <配置名或 id>` 切换当前模型配置\n- `/profiles` 查看全部模型配置\n- `/provider <供应商>` 更新当前模型配置的 provider\n- `/model <模型名>` 更新当前模型配置的 model".into(),
            reasoning_content: None,
            executions: vec![],
            task_id: None,
        })),
        "profiles" => {
            let config = state.config.read().map_err(|e| format!("Lock: {}", e))?;
            let active = config.active_llm_profile_id.clone();
            if config.llm_profiles.is_empty() {
                return Ok(Some(agent::AgentResponse {
                    content: "当前还没有模型配置。先去模型设置页创建，或先使用 `/provider <供应商>` 初始化一条配置。".into(),
                    reasoning_content: None,
                    executions: vec![],
                    task_id: None,
                }));
            }
            let mut lines = vec!["当前模型配置：".to_string()];
            for profile in &config.llm_profiles {
                let marker = if active.as_deref() == Some(profile.id.as_str()) { "• 当前" } else { "•" };
                lines.push(format!(
                    "{} {} (`{}`) · {} · {}",
                    marker,
                    profile.name,
                    profile.id,
                    profile.config.provider_type,
                    profile.config.model
                ));
            }
            Ok(Some(agent::AgentResponse {
                content: lines.join("\n"),
                reasoning_content: None,
                executions: vec![],
                task_id: None,
            }))
        }
        "profile" => Ok(Some(switch_llm_profile(state, args)?)),
        "provider" => Ok(Some(update_active_llm_profile(state, args, None)?)),
        "model" => Ok(Some(update_active_llm_profile(state, "", Some(args))?)),
        _ => Ok(Some(agent::AgentResponse {
            content: format!("未知命令 `/{}`。输入 `/help` 查看可用命令。", command),
            reasoning_content: None,
            executions: vec![],
            task_id: None,
        })),
    }
}

fn parse_external_new_command(input: &str) -> Option<Option<String>> {
    let (command, args) = parse_slash_command(input)?;
    if command != "new" {
        return None;
    }
    let trimmed = args.trim();
    if trimmed.is_empty() {
        Some(None)
    } else {
        Some(Some(trimmed.to_string()))
    }
}

fn switch_llm_profile(
    state: &AppState,
    args: &str,
) -> Result<agent::AgentResponse, String> {
    let target = args.trim();
    if target.is_empty() {
        return Ok(agent::AgentResponse {
            content: "用法：`/profile <配置名或 id>`".into(),
            reasoning_content: None,
            executions: vec![],
            task_id: None,
        });
    }

    let mut config = state.config.write().map_err(|e| format!("Lock: {}", e))?;
    if config.llm_profiles.is_empty() {
        return Ok(agent::AgentResponse {
            content: "当前没有可切换的模型配置。先到模型设置页创建配置。".into(),
            reasoning_content: None,
            executions: vec![],
            task_id: None,
        });
    }

    let found = config
        .llm_profiles
        .iter()
        .find(|profile| {
            profile.id.eq_ignore_ascii_case(target)
                || profile.name.eq_ignore_ascii_case(target)
        })
        .map(|profile| (profile.id.clone(), profile.name.clone(), profile.config.provider_type.clone(), profile.config.model.clone()));

    let Some((id, name, provider, model)) = found else {
        return Ok(agent::AgentResponse {
            content: format!("未找到模型配置 `{}`。可先输入 `/profiles` 查看列表。", target),
            reasoning_content: None,
            executions: vec![],
            task_id: None,
        });
    };

    config.active_llm_profile_id = Some(id);
    config::save_config(&state.data_dir, &config)?;
    *config = config.clone().normalized();
    Ok(agent::AgentResponse {
        content: format!("已切换到模型配置「{}」\nprovider: `{}`\nmodel: `{}`", name, provider, model),
        reasoning_content: None,
        executions: vec![],
        task_id: None,
    })
}

fn update_active_llm_profile(
    state: &AppState,
    provider_arg: &str,
    model_arg: Option<&str>,
) -> Result<agent::AgentResponse, String> {
    let mut config = state.config.write().map_err(|e| format!("Lock: {}", e))?;
    ensure_default_llm_profile(&mut config);
    let active_id = config
        .active_llm_profile_id
        .clone()
        .or_else(|| config.llm_profiles.first().map(|profile| profile.id.clone()))
        .ok_or_else(|| "当前没有可修改的模型配置".to_string())?;

    let profile = config
        .llm_profiles
        .iter_mut()
        .find(|item| item.id == active_id)
        .ok_or_else(|| "当前激活模型配置不存在".to_string())?;

    let mut changed = false;
    let provider = provider_arg.trim();
    if !provider.is_empty() {
        profile.config.provider_type = provider.to_string();
        if let Some(meta) = ohmywu_llm_adapter::provider::builtin_providers()
            .iter()
            .find(|item| item.id.eq_ignore_ascii_case(provider))
        {
            profile.config.api_format = meta.api_format.as_str().to_string();
            if profile.config.endpoint.trim().is_empty() {
                if let Some(default_endpoint) = default_endpoint_for_provider(&meta.id) {
                    profile.config.endpoint = default_endpoint.to_string();
                }
            }
        }
        changed = true;
    }

    if let Some(model_arg) = model_arg {
        let model = model_arg.trim();
        if !model.is_empty() {
            profile.config.model = model.to_string();
            changed = true;
        }
    }

    if !changed {
        return Ok(agent::AgentResponse {
            content: "用法：`/provider <供应商>` 或 `/model <模型名>`".into(),
            reasoning_content: None,
            executions: vec![],
            task_id: None,
        });
    }

    if profile.name.trim().is_empty() || profile.name == "默认模型配置" {
        profile.name = format!(
            "{} · {}",
            if profile.config.provider_type.trim().is_empty() {
                "custom"
            } else {
                profile.config.provider_type.trim()
            },
            if profile.config.model.trim().is_empty() {
                "未设置模型"
            } else {
                profile.config.model.trim()
            }
        );
    }

    config.active_llm_profile_id = Some(active_id.clone());
    config::save_config(&state.data_dir, &config)?;
    *config = config.clone().normalized();

    let profile = config
        .llm_profiles
        .iter()
        .find(|item| item.id == active_id)
        .ok_or_else(|| "更新后模型配置不存在".to_string())?;

    Ok(agent::AgentResponse {
        content: format!(
            "已更新当前模型配置「{}」\nprovider: `{}`\nmodel: `{}`\nendpoint: `{}`",
            profile.name,
            profile.config.provider_type,
            profile.config.model,
            profile.config.endpoint
        ),
        reasoning_content: None,
        executions: vec![],
        task_id: None,
    })
}

fn ensure_default_llm_profile(config: &mut AppConfig) {
    if !config.llm_profiles.is_empty() {
        if config.active_llm_profile_id.is_none() {
            config.active_llm_profile_id = config.llm_profiles.first().map(|profile| profile.id.clone());
        }
        return;
    }

    config.llm_profiles.push(config::LlmProfile {
        id: "default".into(),
        name: "默认模型配置".into(),
        config: LlmConfig::new("openai", "https://api.openai.com/v1", "", None),
    });
    config.active_llm_profile_id = Some("default".into());
}

fn default_endpoint_for_provider(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("https://api.openai.com/v1"),
        "anthropic" => Some("https://api.anthropic.com"),
        "deepseek" => Some("https://api.deepseek.com"),
        "gemini" => Some("https://generativelanguage.googleapis.com"),
        "ollama" => Some("http://localhost:11434"),
        "moonshot" => Some("https://api.moonshot.cn/v1"),
        "zhipu" => Some("https://open.bigmodel.cn/api/paas/v4"),
        "qwen" => Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
        "minimax" => Some("https://api.minimaxi.com/v1"),
        _ => None,
    }
}

// ── Tauri Commands: LLM test ─────────────────────────────────────

#[tauri::command]
async fn test_llm_connection(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let config = {
        let cfg = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        cfg.active_llm_config()
    };

    let llm_cfg = match config {
        Some(c) => c,
        None => return Err("未配置 LLM。".into()),
    };

    // Try using health_check
    let provider = ohmywu_llm_adapter::create_provider(&llm_cfg)
        .map_err(|e| e.user_friendly().to_string())?;

    match provider.health_check().await {
        Ok(status) => {
            let HealthStatus::Ok { model, latency_ms } = status;
            Ok(format!("连接成功！Model: {}, 延迟: {}ms", model, latency_ms))
        }
        Err(e) => Err(format!(
            "连接失败 — {}:{} — {}",
            llm_cfg.endpoint,
            llm_cfg.model,
            e.user_friendly()
        )),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmTestResult {
    pub success: bool,
    pub message: String,
    pub model: Option<String>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmModelOption {
    pub id: String,
    pub label: String,
}

#[tauri::command]
async fn test_llm_connection_with_config(
    provider_type: String,
    api_format: Option<String>,
    endpoint: String,
    model: String,
    api_key: Option<String>,
) -> Result<LlmTestResult, String> {
    let mut llm_cfg = LlmConfig::new(&provider_type, &endpoint, &model, api_key);
    if let Some(api_format) = api_format
        && !api_format.trim().is_empty()
    {
        llm_cfg.api_format = api_format;
    }
    let provider = ohmywu_llm_adapter::create_provider(&llm_cfg)
        .map_err(|e| e.user_friendly().to_string())?;
    match provider.health_check().await {
        Ok(status) => {
            let HealthStatus::Ok { model, latency_ms } = status;
            Ok(LlmTestResult {
                success: true,
                message: format!("连接成功！Model: {}, 延迟: {}ms", model, latency_ms),
                model: Some(model),
                latency_ms: Some(latency_ms),
            })
        }
        Err(e) => Ok(LlmTestResult {
            success: false,
            message: format!("连接失败 — {} — {}", e.user_friendly(), llm_cfg.endpoint),
            model: None,
            latency_ms: None,
        }),
    }
}

#[tauri::command]
async fn fetch_llm_models(
    provider_type: String,
    api_format: Option<String>,
    endpoint: String,
    api_key: Option<String>,
) -> Result<Vec<LlmModelOption>, String> {
    let mut llm_cfg = LlmConfig::new(&provider_type, &endpoint, "", api_key.clone());
    if let Some(api_format) = api_format
        && !api_format.trim().is_empty()
    {
        llm_cfg.api_format = api_format;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Create client: {}", e))?;

    let endpoint = endpoint.trim_end_matches('/').to_string();
    let options = match llm_cfg.effective_api_format() {
        ohmywu_llm_adapter::provider::ApiFormat::Ollama => {
            let url = if endpoint.ends_with("/api/tags") {
                endpoint
            } else {
                format!("{}/api/tags", endpoint)
            };
            let data = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("获取模型失败: {}", e))?
                .error_for_status()
                .map_err(|e| format!("获取模型失败: {}", e))?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("解析模型列表失败: {}", e))?;
            data.get("models")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|item| item.get("name").and_then(|v| v.as_str()))
                .map(|name| LlmModelOption {
                    id: name.to_string(),
                    label: name.to_string(),
                })
                .collect::<Vec<_>>()
        }
        ohmywu_llm_adapter::provider::ApiFormat::OpenAiChat
        | ohmywu_llm_adapter::provider::ApiFormat::OpenAiResponses => {
            let url = if endpoint.ends_with("/v1/chat/completions") {
                endpoint.replace("/v1/chat/completions", "/v1/models")
            } else if endpoint.ends_with("/chat/completions") {
                endpoint.replace("/chat/completions", "/models")
            } else if endpoint.ends_with("/v1") {
                format!("{}/models", endpoint)
            } else {
                format!("{}/v1/models", endpoint)
            };
            let response = client
                .get(url)
                .header("Authorization", format!("Bearer {}", api_key.unwrap_or_default()))
                .send()
                .await
                .map_err(|e| format!("获取模型失败: {}", e))?
                .error_for_status()
                .map_err(|e| format!("获取模型失败: {}", e))?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("解析模型列表失败: {}", e))?;
            response
                .get("data")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|item| item.get("id").and_then(|v| v.as_str()))
                .map(|id| LlmModelOption {
                    id: id.to_string(),
                    label: id.to_string(),
                })
                .collect::<Vec<_>>()
        }
        ohmywu_llm_adapter::provider::ApiFormat::Anthropic => {
            let url = if endpoint.ends_with("/v1") {
                format!("{}/models", endpoint)
            } else {
                format!("{}/v1/models", endpoint)
            };
            let response = client
                .get(url)
                .header("x-api-key", api_key.unwrap_or_default())
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
                .map_err(|e| format!("获取模型失败: {}", e))?
                .error_for_status()
                .map_err(|e| format!("获取模型失败: {}", e))?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("解析模型列表失败: {}", e))?;
            response
                .get("data")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|item| item.get("id").and_then(|v| v.as_str()))
                .map(|id| LlmModelOption {
                    id: id.to_string(),
                    label: id.to_string(),
                })
                .collect::<Vec<_>>()
        }
        ohmywu_llm_adapter::provider::ApiFormat::Gemini => {
            let base = if endpoint.contains("/v1beta") || endpoint.ends_with("/v1") {
                endpoint
            } else {
                format!("{}/v1beta", endpoint)
            };
            let separator = if base.contains('?') { "&" } else { "?" };
            let url = format!("{}/models{}key={}", base, separator, api_key.unwrap_or_default());
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("获取模型失败: {}", e))?
                .error_for_status()
                .map_err(|e| format!("获取模型失败: {}", e))?
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("解析模型列表失败: {}", e))?;
            response
                .get("models")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|item| item.get("name").and_then(|v| v.as_str()))
                .map(|name| {
                    let label = name.strip_prefix("models/").unwrap_or(name).to_string();
                    LlmModelOption {
                        id: label.clone(),
                        label,
                    }
                })
                .collect::<Vec<_>>()
        }
    };

    if options.is_empty() {
        return Err("未获取到可用模型".into());
    }

    Ok(options)
}

// ── Tauri Commands: config ───────────────────────────────────────

#[tauri::command]
async fn get_config(
    state: tauri::State<'_, AppState>,
) -> Result<AppConfig, String> {
    let config = state.config.read().map_err(|e| format!("Lock: {}", e))?;
    Ok(config.clone().normalized())
}

#[tauri::command]
async fn save_config(
    config: AppConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let normalized = config.normalized();
    state.policy.set_mode(normalized.policy_mode);
    config::save_config(&state.data_dir, &normalized)?;
    let mut current = state.config.write().map_err(|e| format!("Lock: {}", e))?;
    *current = normalized;
    Ok(())
}

// ── Tauri Commands: background ───────────────────────────────────

#[tauri::command]
fn save_background_file(
    data: Vec<u8>,
    filename: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let bg_dir = state.data_dir.join("background");
    std::fs::create_dir_all(&bg_dir).map_err(|e| format!("Create bg dir: {}", e))?;

    let kind = background_kind_from_filename(&filename)?;
    clear_background_dir(&bg_dir)?;

    let path = bg_dir.join(&filename);
    std::fs::write(&path, &data).map_err(|e| format!("Write bg file: {}", e))?;
    save_background_meta(&bg_dir, &filename, kind)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_background_path(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let bg_dir = state.data_dir.join("background");
    if !bg_dir.exists() {
        return Ok(None);
    }

    if let Some(path) = background_path_from_meta(&bg_dir)? {
        return Ok(Some(path));
    }

    Ok(None)
}

#[tauri::command]
fn clear_background_file(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let bg_dir = state.data_dir.join("background");
    if bg_dir.exists() {
        clear_background_dir(&bg_dir)?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct BackgroundMeta {
    filename: String,
    kind: String,
}

fn background_kind_from_filename(filename: &str) -> Result<&'static str, String> {
    if filename.starts_with("bg_image.") {
        Ok("image")
    } else {
        Err("Background filename must start with `bg_image.`".into())
    }
}

fn clear_background_dir(bg_dir: &std::path::Path) -> Result<(), String> {
    if !bg_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(bg_dir).map_err(|e| format!("Read bg dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Read bg entry: {}", e))?;
        let path = entry.path();
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| format!("Remove bg file {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

fn save_background_meta(bg_dir: &std::path::Path, filename: &str, kind: &str) -> Result<(), String> {
    let meta = BackgroundMeta {
        filename: filename.to_string(),
        kind: kind.to_string(),
    };
    let tmp = bg_dir.join("background.json.tmp");
    let final_path = bg_dir.join("background.json");
    let json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Serialize background meta: {}", e))?;
    std::fs::write(&tmp, json).map_err(|e| format!("Write background meta tmp: {}", e))?;
    std::fs::rename(&tmp, &final_path).map_err(|e| format!("Rename background meta: {}", e))?;
    Ok(())
}

fn background_path_from_meta(bg_dir: &std::path::Path) -> Result<Option<String>, String> {
    let meta_path = bg_dir.join("background.json");
    if meta_path.exists() {
        let content = std::fs::read_to_string(&meta_path)
            .map_err(|e| format!("Read background meta: {}", e))?;
        let meta: BackgroundMeta = serde_json::from_str(&content)
            .map_err(|e| format!("Parse background meta: {}", e))?;
        let path = bg_dir.join(&meta.filename);
        if meta.kind == "image" && meta.filename.starts_with("bg_image.") && path.exists() {
            return Ok(Some(path.to_string_lossy().to_string()));
        }
    }

    for entry in std::fs::read_dir(bg_dir).map_err(|e| format!("Read bg dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Read bg entry: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("bg_image.") {
            return Ok(Some(entry.path().to_string_lossy().to_string()));
        }
    }

    Ok(None)
}

// ── Tauri Commands: wiki ──────────────────────────────────────────

#[tauri::command]
fn wiki_list_notes(state: tauri::State<'_, AppState>) -> Result<Vec<ohmywu_wiki::NoteMeta>, String> {
    let wiki = state.wiki.read().map_err(|e| format!("Lock: {}", e))?;
    wiki.list_notes()
}

#[tauri::command]
fn wiki_read_note(
    slug: String,
    state: tauri::State<'_, AppState>,
) -> Result<ohmywu_wiki::WikiNote, String> {
    let wiki = state.wiki.read().map_err(|e| format!("Lock: {}", e))?;
    wiki.read_note(&slug)
}

#[tauri::command]
fn wiki_upsert_note(
    current_slug: Option<String>,
    slug: Option<String>,
    title: String,
    body: String,
    tags: Vec<String>,
    folder: String,
    state: tauri::State<'_, AppState>,
) -> Result<ohmywu_wiki::WikiNote, String> {
    let wiki = state.wiki.read().map_err(|e| format!("Lock: {}", e))?;
    wiki.upsert_note(
        current_slug.as_deref(),
        slug.as_deref(),
        &title,
        &body,
        &tags,
        &folder,
    )
}

#[tauri::command]
fn wiki_delete_note(
    slug: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let wiki = state.wiki.read().map_err(|e| format!("Lock: {}", e))?;
    wiki.delete_note(&slug)
}

#[tauri::command]
fn wiki_search_notes(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ohmywu_wiki::NoteMeta>, String> {
    let wiki = state.wiki.read().map_err(|e| format!("Lock: {}", e))?;
    wiki.search(&query)
}

#[tauri::command]
fn wiki_get_graph(
    state: tauri::State<'_, AppState>,
) -> Result<ohmywu_wiki::GraphData, String> {
    let wiki = state.wiki.read().map_err(|e| format!("Lock: {}", e))?;
    wiki.build_graph()
}

// ── Tauri Commands: wechat bridge ───────────────────────────────

#[tauri::command]
async fn get_wechat_bridge(state: tauri::State<'_, AppState>) -> Result<WechatBridgeView, String> {
    state.wechat_bridge.get_view().await
}

#[tauri::command]
fn save_wechat_bridge_config(
    config: WechatBridgeConfig,
    state: tauri::State<'_, AppState>,
) -> Result<WechatBridgeConfig, String> {
    state.wechat_bridge.save_config(&config)
}

#[tauri::command]
async fn request_wechat_login_qr(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<WechatBridgeView, String> {
    let config = state.wechat_bridge.get_config()?;
    let api_base_url = config.api_base_url.trim_end_matches('/').to_string();
    let bot_type = if config.bot_type.trim().is_empty() {
        "3".to_string()
    } else {
        config.bot_type.trim().to_string()
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(config.api_timeout_ms))
        .build()
        .map_err(|e| format!("Create wechat login client: {}", e))?;
    let url = format!("{}/ilink/bot/get_bot_qrcode", api_base_url);
    let response = client
        .get(url)
        .query(&[("bot_type", bot_type.as_str())])
        .header("Content-Type", "application/json")
        .header("AuthorizationType", "ilink_bot_token")
        .header("X-WECHAT-UIN", build_wechat_uin())
        .send()
        .await
        .map_err(|e| format!("Request wechat login qr: {}", e))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Request wechat login qr failed: {} {}", status, body));
    }
    let data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Parse wechat login qr response: {}", e))?;
    let qrcode = data
        .get("qrcode")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| "微信二维码响应缺少 qrcode".to_string())?
        .to_string();
    let qrcode_img_content = data
        .get("qrcode_img_content")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| "微信二维码响应缺少 qrcode_img_content".to_string())?
        .to_string();
    let now = chrono_now();
    let session = WechatBridgeLoginSession {
        session_key: format!(
            "wxqr-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_millis())
                .unwrap_or_default()
        ),
        qrcode,
        qrcode_img_content: qrcode_img_content.clone(),
        started_at: now.clone(),
        updated_at: now.clone(),
        status: "wait".into(),
        bot_token: None,
        account_id: None,
        base_url: None,
        user_id: None,
        error: None,
    };
    state.wechat_bridge.update_qr_content(qrcode_img_content.clone(), now.clone()).await;
    state.wechat_bridge.set_login_session(session.clone()).await;
    let _ = app_handle.emit(
        "wechat-bridge-event",
        serde_json::json!({
            "kind": "qr.updated",
            "content": qrcode_img_content,
            "updatedAt": now,
            "status": "wait",
        }),
    );
    spawn_wechat_login_poll_loop(state.inner().clone(), app_handle, session.session_key.clone());
    state.wechat_bridge.get_view().await
}

fn build_wechat_uin() -> String {
    let raw = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos().to_string())
        .unwrap_or_else(|_| "0".into());
    simple_base64_encode(raw.as_bytes())
}

fn simple_base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    let mut index = 0;
    while index < input.len() {
        let b0 = input[index];
        let b1 = input.get(index + 1).copied().unwrap_or(0);
        let b2 = input.get(index + 2).copied().unwrap_or(0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
        if index + 1 < input.len() {
            output.push(TABLE[((b1 & 0x0f) << 2 | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if index + 2 < input.len() {
            output.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        index += 3;
    }
    output
}

fn spawn_wechat_login_poll_loop(
    state: AppState,
    app_handle: tauri::AppHandle,
    session_key: String,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let Some(session) = state.wechat_bridge.get_login_session().await else {
                return;
            };
            if session.session_key != session_key {
                return;
            }
            if matches!(session.status.as_str(), "confirmed" | "expired" | "denied" | "cancel" | "canceled") {
                return;
            }

            let config = match state.wechat_bridge.get_config() {
                Ok(config) => config,
                Err(error) => {
                    let _ = app_handle.emit(
                        "wechat-bridge-event",
                        serde_json::json!({
                            "kind": "qr.error",
                            "error": error,
                        }),
                    );
                    return;
                }
            };
            tokio::time::sleep(Duration::from_secs(config.qr_poll_interval)).await;

            let client = match reqwest::Client::builder()
                .timeout(Duration::from_millis(config.long_poll_timeout_ms))
                .build()
            {
                Ok(client) => client,
                Err(error) => {
                    let _ = app_handle.emit(
                        "wechat-bridge-event",
                        serde_json::json!({
                            "kind": "qr.error",
                            "error": format!("Create wechat qr poll client: {}", error),
                        }),
                    );
                    return;
                }
            };
            let url = format!("{}/ilink/bot/get_qrcode_status", config.api_base_url.trim_end_matches('/'));
            let response = match client
                .get(url)
                .query(&[("qrcode", session.qrcode.as_str())])
                .header("Content-Type", "application/json")
                .header("AuthorizationType", "ilink_bot_token")
                .header("X-WECHAT-UIN", build_wechat_uin())
                .header("iLink-App-ClientVersion", "1")
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let mut updated = session.clone();
                    updated.updated_at = chrono_now();
                    updated.error = Some(format!("Poll wechat qr status failed: {}", error));
                    state.wechat_bridge.set_login_session(updated.clone()).await;
                    let _ = app_handle.emit(
                        "wechat-bridge-event",
                        serde_json::json!({
                            "kind": "qr.error",
                            "error": updated.error,
                        }),
                    );
                    continue;
                }
            };
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                let mut updated = session.clone();
                updated.updated_at = chrono_now();
                updated.error = Some(format!("Poll wechat qr status failed: {} {}", status, body));
                state.wechat_bridge.set_login_session(updated.clone()).await;
                let _ = app_handle.emit(
                    "wechat-bridge-event",
                    serde_json::json!({
                        "kind": "qr.error",
                        "error": updated.error,
                    }),
                );
                continue;
            }
            let data = match response.json::<serde_json::Value>().await {
                Ok(data) => data,
                Err(error) => {
                    let mut updated = session.clone();
                    updated.updated_at = chrono_now();
                    updated.error = Some(format!("Parse wechat qr status failed: {}", error));
                    state.wechat_bridge.set_login_session(updated.clone()).await;
                    let _ = app_handle.emit(
                        "wechat-bridge-event",
                        serde_json::json!({
                            "kind": "qr.error",
                            "error": updated.error,
                        }),
                    );
                    continue;
                }
            };

            let mut updated = session.clone();
            updated.updated_at = chrono_now();
            updated.error = None;
            updated.status = data
                .get("status")
                .and_then(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .unwrap_or("wait")
                .to_string();
            if updated.status == "confirmed" {
                updated.bot_token = data.get("bot_token").and_then(|item| item.as_str()).map(str::trim).map(ToString::to_string).filter(|item| !item.is_empty());
                updated.account_id = data.get("ilink_bot_id").and_then(|item| item.as_str()).map(str::trim).map(ToString::to_string).filter(|item| !item.is_empty());
                updated.base_url = data.get("baseurl").and_then(|item| item.as_str()).map(str::trim).map(|item| item.trim_end_matches('/').to_string()).filter(|item| !item.is_empty());
                updated.user_id = data.get("ilink_user_id").and_then(|item| item.as_str()).map(str::trim).map(ToString::to_string).filter(|item| !item.is_empty());

                let mut saved_config = config.clone();
                saved_config.bot_token = updated.bot_token.clone();
                saved_config.account_id = updated.account_id.clone();
                saved_config.user_id = updated.user_id.clone();
                if let Some(base_url) = updated.base_url.clone() {
                    saved_config.api_base_url = base_url;
                }
                let _ = state.wechat_bridge.save_config(&saved_config);
            } else if updated.status == "expired" {
                updated.error = Some("二维码已过期，请重新生成".into());
            } else if matches!(updated.status.as_str(), "cancel" | "canceled" | "denied") {
                updated.error = Some("已取消微信登录".into());
            }

            state.wechat_bridge.set_login_session(updated.clone()).await;
            let _ = app_handle.emit(
                "wechat-bridge-event",
                serde_json::json!({
                    "kind": "qr.status",
                    "status": updated.status,
                    "error": updated.error,
                    "accountId": updated.account_id,
                    "userId": updated.user_id,
                }),
            );
            if updated.status == "confirmed" {
                return;
            }
        }
    });
}

async fn send_wechat_text_reply(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    account_id: &str,
    chat_type: &str,
    peer_id: &str,
    content: &str,
) -> Result<(), String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if chat_type != "dm" {
        return Err("当前微信直连接入暂时只支持私聊文本回复".into());
    }

    let config = state.wechat_bridge.get_config()?;
    let bot_token = config
        .bot_token
        .clone()
        .ok_or_else(|| "微信尚未登录，缺少 bot_token".to_string())?;
    let context_token = config
        .context_tokens
        .get(peer_id)
        .cloned()
        .ok_or_else(|| format!("缺少 {} 的 context_token，请先让对方发一条消息", peer_id))?;
    let target_account_id = if account_id.trim().is_empty() {
        config.account_id.clone().unwrap_or_default()
    } else {
        account_id.trim().to_string()
    };
    if target_account_id.is_empty() {
        return Err("缺少微信 account_id".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(config.api_timeout_ms))
        .build()
        .map_err(|e| format!("Create wechat send client: {}", e))?;
    let url = format!("{}/ilink/bot/sendmessage", config.api_base_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "base_info": {
            "channel_version": "ohmywu",
        },
        "msg": {
            "from_user_id": "",
            "to_user_id": peer_id,
            "client_id": format!("ohmywu-{}", SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_millis()).unwrap_or_default()),
            "message_type": 2,
            "message_state": 2,
            "context_token": context_token,
            "item_list": [{
                "type": 1,
                "text_item": {
                    "text": trimmed
                }
            }]
        }
    });
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("AuthorizationType", "ilink_bot_token")
        .header("Authorization", format!("Bearer {}", bot_token))
        .header("X-WECHAT-UIN", build_wechat_uin())
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Send wechat message failed: {}", e))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Send wechat message failed: {} {}", status, body));
    }
    let data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Parse wechat send response: {}", e))?;
    if !is_successful_wechat_payload(&data) {
        return Err(format!(
            "Send wechat message failed: {}",
            format_wechat_payload_error(&data)
        ));
    }
    let _ = app_handle.emit(
        "wechat-bridge-event",
        serde_json::json!({
            "kind": "wechat.outbound.sent",
            "peerId": peer_id,
            "accountId": target_account_id,
            "content": trimmed,
        }),
    );
    Ok(())
}

fn start_wechat_direct_loop(app: &tauri::AppHandle, state: AppState) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let config = match state.wechat_bridge.get_config() {
                Ok(config) => config,
                Err(error) => {
                    eprintln!("wechat direct config: {}", error);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }
            };
            if config.bot_token.is_none() {
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            if let Err(error) = poll_wechat_updates_once(&state, &app_handle, config).await {
                let _ = app_handle.emit(
                    "wechat-bridge-event",
                    serde_json::json!({
                        "kind": "wechat.inbound.error",
                        "error": error,
                    }),
                );
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    });
}

async fn poll_wechat_updates_once(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    mut config: WechatBridgeConfig,
) -> Result<(), String> {
    let bot_token = config
        .bot_token
        .clone()
        .ok_or_else(|| "微信尚未登录".to_string())?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(config.long_poll_timeout_ms))
        .build()
        .map_err(|e| format!("Create wechat updates client: {}", e))?;
    let url = format!("{}/ilink/bot/getupdates", config.api_base_url.trim_end_matches('/'));
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("AuthorizationType", "ilink_bot_token")
        .header("Authorization", format!("Bearer {}", bot_token))
        .header("X-WECHAT-UIN", build_wechat_uin())
        .json(&serde_json::json!({
            "base_info": {
                "channel_version": "ohmywu",
            },
            "get_updates_buf": config.sync_buf,
        }))
        .send()
        .await
        .map_err(|e| format!("Request wechat updates failed: {}", e))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Request wechat updates failed: {} {}", status, body));
    }
    let data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Parse wechat updates response: {}", e))?;
    if !is_successful_wechat_payload(&data) {
        if wechat_payload_errcode(&data) == -14 {
            config.bot_token = None;
            config.account_id = None;
            config.user_id = None;
            config.sync_buf.clear();
            config.context_tokens.clear();
            let _ = state.wechat_bridge.save_config(&config);
            let _ = app_handle.emit(
                "wechat-bridge-event",
                serde_json::json!({
                    "kind": "wechat.login.expired",
                    "error": format_wechat_payload_error(&data),
                }),
            );
            return Ok(());
        }
        return Err(format!(
            "Wechat getupdates failed: {}",
            format_wechat_payload_error(&data)
        ));
    }

    let mut changed = false;
    if let Some(next_buf) = data.get("get_updates_buf").and_then(|item| item.as_str()) {
        let next_buf = next_buf.trim();
        if !next_buf.is_empty() && next_buf != config.sync_buf {
            config.sync_buf = next_buf.to_string();
            changed = true;
        }
    }

    if let Some(messages) = data.get("msgs").and_then(|item| item.as_array()) {
        for message in messages {
            if process_wechat_update_message(state, app_handle, &mut config, message).await? {
                changed = true;
            }
        }
    }

    if changed {
        let _ = state.wechat_bridge.save_config(&config);
    }
    Ok(())
}

async fn process_wechat_update_message(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    config: &mut WechatBridgeConfig,
    message: &serde_json::Value,
) -> Result<bool, String> {
    let from_user_id = message
        .get("from_user_id")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .unwrap_or_default();
    if from_user_id.is_empty() {
        return Ok(false);
    }
    if config
        .account_id
        .as_deref()
        .is_some_and(|account_id| account_id == from_user_id)
    {
        return Ok(false);
    }

    let mut changed = false;
    if let Some(context_token) = message
        .get("context_token")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if config.context_tokens.get(from_user_id).map(String::as_str) != Some(context_token) {
            config
                .context_tokens
                .insert(from_user_id.to_string(), context_token.to_string());
            changed = true;
        }
    }

    let item_list = message
        .get("item_list")
        .and_then(|item| item.as_array())
        .cloned()
        .unwrap_or_default();
    let text = wechat_message_text_from_item_list(&item_list);
    if text.trim().is_empty() {
        return Ok(changed);
    }

    let input = ExternalMessageInput {
        provider: "wechat".into(),
        account_id: config.account_id.clone().unwrap_or_else(|| "wechat".into()),
        chat_type: if from_user_id.ends_with("@chatroom") {
            "group".into()
        } else {
            "dm".into()
        },
        peer_id: from_user_id.to_string(),
        peer_name: None,
        content: text,
        agent_profile: None,
        agent_profiles: None,
    };
    let _ = app_handle.emit(
        "wechat-bridge-event",
        serde_json::json!({
            "kind": "wechat.inbound.received",
            "peerId": from_user_id,
        }),
    );
    if changed {
        *config = state.wechat_bridge.save_config(config)?;
    }
    handle_external_message_input(state, app_handle, input).await?;
    Ok(changed)
}

fn wechat_message_text_from_item_list(item_list: &[serde_json::Value]) -> String {
    let mut text_parts = Vec::new();
    for item in item_list {
        let item_type = item.get("type").and_then(|value| value.as_i64()).unwrap_or_default();
        match item_type {
            1 => {
                if let Some(text) = item
                    .get("text_item")
                    .and_then(|value| value.get("text"))
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    text_parts.push(text.to_string());
                }
            }
            2 => text_parts.push("[图片]".into()),
            3 => {
                let text = item
                    .get("voice_item")
                    .and_then(|value| value.get("text"))
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "[语音]".into());
                text_parts.push(text);
            }
            4 => text_parts.push("[文件]".into()),
            5 => text_parts.push("[视频]".into()),
            _ => {}
        }
    }
    text_parts.join("\n").trim().to_string()
}

fn is_successful_wechat_payload(payload: &serde_json::Value) -> bool {
    let ret = payload.get("ret").and_then(|item| item.as_i64()).unwrap_or_default();
    let errcode = payload.get("errcode").and_then(|item| item.as_i64()).unwrap_or_default();
    ret == 0 && errcode == 0
}

fn format_wechat_payload_error(payload: &serde_json::Value) -> String {
    let ret = payload.get("ret").and_then(|item| item.as_i64()).unwrap_or_default();
    let errcode = payload.get("errcode").and_then(|item| item.as_i64()).unwrap_or_default();
    let errmsg = payload
        .get("errmsg")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .unwrap_or_default();
    format!("ret={}, errcode={}, errmsg={}", ret, errcode, errmsg)
}

fn wechat_payload_errcode(payload: &serde_json::Value) -> i64 {
    payload.get("errcode").and_then(|item| item.as_i64()).unwrap_or_default()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WechatBridgeHttpPayload {
    provider: Option<String>,
    account_id: String,
    chat_type: String,
    peer_id: String,
    peer_name: Option<String>,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WechatBridgeQrPayload {
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WechatBridgeHttpResponse {
    ok: bool,
    session_id: Option<String>,
    session_name: Option<String>,
    created_session: bool,
    control_only: bool,
    reply: Option<String>,
    error: Option<String>,
}

fn start_wechat_bridge_loop(app: &tauri::AppHandle, state: AppState) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind(WECHAT_BRIDGE_BIND_ADDR).await {
            Ok(listener) => {
                state.wechat_bridge.set_server_status(true, None).await;
                listener
            }
            Err(error) => {
                state
                    .wechat_bridge
                    .set_server_status(false, Some(format!("Bind {} failed: {}", WECHAT_BRIDGE_BIND_ADDR, error)))
                    .await;
                return;
            }
        };

        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(parts) => parts,
                Err(error) => {
                    eprintln!("wechat bridge accept failed: {}", error);
                    continue;
                }
            };

            let state = state.clone();
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let response = match handle_wechat_bridge_http_request(&state, &app_handle, &mut stream).await {
                    Ok(response) => response,
                    Err(error) => http_json_response(
                        500,
                        &serde_json::json!({
                            "ok": false,
                            "error": error,
                        }),
                    ),
                };
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
}

async fn handle_wechat_bridge_http_request(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    stream: &mut tokio::net::TcpStream,
) -> Result<String, String> {
    let (method, path, headers, body) = read_http_request(stream).await?;

    if method != "POST" {
        return Ok(http_json_response(
            405,
            &serde_json::json!({
                "ok": false,
                "error": "Only POST is supported",
            }),
        ));
    }

    if path != WECHAT_BRIDGE_MESSAGE_PATH && path != WECHAT_BRIDGE_QR_PATH {
        return Ok(http_json_response(
            404,
            &serde_json::json!({
                "ok": false,
                "error": "Not found",
            }),
        ));
    }

    let config = state.wechat_bridge.get_config()?;
    let expected_token = config.bridge_token.trim();
    let provided_token = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(WECHAT_BRIDGE_HEADER_TOKEN))
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default();
    if !expected_token.is_empty() && provided_token != expected_token {
        return Ok(http_json_response(
            401,
            &serde_json::json!({
                "ok": false,
                "error": "Invalid bridge token",
            }),
        ));
    }

    if path == WECHAT_BRIDGE_QR_PATH {
        let payload: WechatBridgeQrPayload =
            serde_json::from_slice(&body).map_err(|e| format!("Parse bridge qr payload: {}", e))?;
        let content = payload.content.trim().to_string();
        if content.is_empty() {
            return Ok(http_json_response(
                400,
                &serde_json::json!({
                    "ok": false,
                    "error": "QR content is empty",
                }),
            ));
        }

        let updated_at = chrono_now();
        state
            .wechat_bridge
            .update_qr_content(content.clone(), updated_at.clone())
            .await;
        let _ = app_handle.emit(
            "wechat-bridge-event",
            serde_json::json!({
                "kind": "qr.updated",
                "content": content,
                "updatedAt": updated_at,
            }),
        );
        return Ok(http_json_response(
            200,
            &serde_json::json!({
                "ok": true,
                "updated": true,
            }),
        ));
    }

    let payload: WechatBridgeHttpPayload =
        serde_json::from_slice(&body).map_err(|e| format!("Parse bridge payload: {}", e))?;
    let input = ExternalMessageInput {
        provider: payload.provider.unwrap_or_else(|| "wechat".into()),
        account_id: payload.account_id,
        chat_type: payload.chat_type,
        peer_id: payload.peer_id,
        peer_name: payload.peer_name,
        content: payload.content,
        agent_profile: None,
        agent_profiles: None,
    };

    let _ = app_handle.emit(
        "wechat-bridge-event",
        serde_json::json!({
            "kind": "bridge.inbound",
            "line": format!("收到桥接消息: {} / {}", input.chat_type, input.peer_id),
        }),
    );

    match handle_external_message_input(state, app_handle, input).await {
        Ok(result) => Ok(http_json_response(
            200,
            &WechatBridgeHttpResponse {
                ok: true,
                session_id: Some(result.session_id),
                session_name: Some(result.session_name),
                created_session: result.created_session,
                control_only: result.control_only,
                reply: result.reply.map(|item| item.content),
                error: None,
            },
        )),
        Err(error) => Ok(http_json_response(
            500,
            &WechatBridgeHttpResponse {
                ok: false,
                session_id: None,
                session_name: None,
                created_session: false,
                control_only: false,
                reply: None,
                error: Some(error),
            },
        )),
    }
}

async fn read_http_request(
    stream: &mut tokio::net::TcpStream,
) -> Result<(String, String, Vec<(String, String)>, Vec<u8>), String> {
    let mut buffer = Vec::new();
    let mut header_end = None;
    loop {
        if buffer.len() > 1024 * 1024 {
            return Err("Request header too large".into());
        }
        let mut chunk = [0_u8; 4096];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("Read request: {}", e))?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(index) = find_header_end(&buffer) {
            header_end = Some(index);
            break;
        }
    }

    let header_end = header_end.ok_or_else(|| "Invalid HTTP request".to_string())?;
    let head = String::from_utf8(buffer[..header_end].to_vec())
        .map_err(|e| format!("Decode request header: {}", e))?;
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or_else(|| "Missing request line".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "Missing method".to_string())?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| "Missing path".to_string())?
        .to_string();

    let mut headers = Vec::new();
    let mut content_length = 0_usize;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_string();
        let value = value.trim().to_string();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.parse::<usize>().unwrap_or(0);
        }
        headers.push((name, value));
    }

    let body_start = header_end + 4;
    let mut body = buffer[body_start..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0_u8; content_length - body.len()];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("Read request body: {}", e))?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Ok((method, path, headers, body))
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http_json_response<T: Serialize>(status_code: u16, body: &T) -> String {
    let reason = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{\"ok\":false}".into());
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_code,
        reason,
        json.len(),
        json
    )
}

fn sanitize_filename(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        "audit".to_string()
    } else {
        sanitized
    }
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
            state.set_app_handle(app.handle().clone());
            start_wechat_bridge_loop(app.handle(), state.clone());
            start_wechat_direct_loop(app.handle(), state.clone());
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_capabilities,
            upsert_capability,
            set_capability_enabled,
            delete_capability,
            get_agents,
            upsert_agent,
            delete_agent,
            get_actions,
            refresh_actions,
            upsert_action,
            set_action_enabled,
            delete_action,
            get_action_blueprint,
            get_policy_mode,
            get_agent_mode,
            get_tasks,
            get_audits,
            clear_audits,
            export_audits,
            get_llm_providers,
            execute_capability,
            set_policy_mode,
            set_agent_mode,
            cancel_agent,
            create_session,
            list_sessions,
            load_session,
            delete_session,
            update_session_meta,
            load_runtime_thread,
            send_message,
            receive_external_message,
            get_config,
            save_config,
            test_llm_connection,
            test_llm_connection_with_config,
            fetch_llm_models,
            save_background_file,
            get_background_path,
            clear_background_file,
            generate_memory_candidate,
            save_memory_candidate,
            wiki_list_notes,
            wiki_read_note,
            wiki_upsert_note,
            wiki_delete_note,
            wiki_search_notes,
            wiki_get_graph,
            get_wechat_bridge,
            save_wechat_bridge_config,
            request_wechat_login_qr,
            get_external_route_state,
            list_external_route_states,
            render_qr_svg,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
