mod agent;
mod agent_catalog;
mod action_catalog;
mod capabilities;
mod config;
mod data_dir;
mod permission;
mod runtime;
mod tools;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
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
use tauri::Emitter;
use tauri::Manager;
use ohmywu_domain::AuditEvent;
use ohmywu_llm_adapter::{HealthStatus, LlmConfig, ProviderMetadata};
use ohmywu_wiki::WikiEngine;
use serde::{Deserialize, Serialize};

use agent_catalog::{AgentCatalog, AgentUpsertInput, AgentView};
use action_catalog::{ActionBlueprint, ActionCatalog, ActionUpsertInput, ActionView};
use capabilities::{CapabilityCatalog, CapabilityUpsertInput, CapabilityView};
use config::AppConfig;
use runtime::{RuntimeStore, RuntimeThreadView};

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
        let audit = Arc::new(AuditLog::new());
        let session = Arc::new(SessionManager::new(data_dir.join("sessions")));
        let runtime = Arc::new(
            RuntimeStore::new(data_dir.join("runtime"))
                .expect("failed to initialize runtime store"),
        );

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
            cancel_token: Arc::new(AtomicBool::new(false)),
            app_handle: Arc::new(RwLock::new(None)),
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

#[tauri::command]
async fn send_message(
    session_id: String,
    content: String,
    agent_profile: Option<agent::AgentInvocationProfile>,
    agent_profiles: Option<Vec<agent::AgentInvocationProfile>>,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<SessionMessage, String> {
    // reset cancel token for this new message
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
    let turn = state.runtime.start_turn(&session_id, agent_mode, &content)?;
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
        content: content.clone(),
        agent_id: agent_profile.as_ref().map(|profile| profile.id.clone()),
        agent_name: agent_profile.as_ref().map(|profile| profile.name.clone()),
        turn_id: Some(turn.id.clone()),
        reasoning_content: None,
        executions: None,
        task_id: None,
        timestamp: now,
    };
    state.session.append_message(&session_id, &user_msg)?;

    // load LLM config
    let llm_config = {
        let cfg_guard = state.config.read().map_err(|e| format!("Lock: {}", e))?;
        cfg_guard.active_llm_config()
    };

    // Phase 2: LLM agent loop (with fallback to Phase 1 mock if no LLM configured)
    let agent_response = if let Some(llm_config) = llm_config {
        agent::agent_loop(
            &state,
            &session_id,
            &turn.id,
            &content,
            agent_profile.as_ref(),
            &llm_config,
            Some(&app_handle),
            true,
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
    } else {
        // Phase 1 fallback: simple command parsing
        if let Some(path) = parse_read_cmd(&content) {
            let req = tools::ExecuteRequest {
                capability: "read".into(),
                params: serde_json::json!({ "path": path }),
            };
            let result = tools::dispatch_tool(&state, req).await;
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
                    error: result.error.clone(),
                    status: result.status.clone(),
                    duration_ms: result.duration_ms,
                }],
                task_id: Some(result.task_id),
            }
        } else if let Some(cmd) = parse_run_cmd(&content) {
            let req = tools::ExecuteRequest {
                capability: "bash".into(),
                params: serde_json::json!({ "command": cmd }),
            };
            let result = tools::dispatch_tool(&state, req).await;
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
                reasoning_content: None,
                executions: vec![],
                task_id: None,
            }
        }
    };

    let agent_msg = agent::build_agent_message(&agent_response, &turn.id, agent_profile.as_ref());
    state.session.append_message(&session_id, &agent_msg)?;
    let completed_turn = state.runtime.finish_turn(
        &session_id,
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

    Ok(agent_msg)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
