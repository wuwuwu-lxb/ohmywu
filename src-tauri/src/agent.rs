use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Instant;

use ohmywu_domain::{chrono_now, AgentMode};
use ohmywu_llm_adapter::types::{ChatMessage, ChatResponse, ChatStreamChunk, ToolCall, ToolDef};
use ohmywu_llm_adapter::{create_provider, LlmConfig, LlmError, LlmProvider};
use ohmywu_session::{ExecutionRecord, SessionMessage};
use ohmywu_wiki::RecallHit;
use tauri::Emitter;

use crate::tools::{self, active_tool_defs, ExecuteRequest};
use crate::AppState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocationProfile {
    pub id: String,
    pub name: String,
    pub role: String,
    pub persona: String,
    pub memory_scope: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub delegatable: bool,
    #[serde(default)]
    pub delegate_priority: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCandidateDraft {
    pub title: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub body: String,
    pub should_save: bool,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryScopeConfig {
    label: Option<String>,
    mode: Option<String>,
    folders: Option<Vec<String>>,
    recall_limit: Option<usize>,
    notes: Option<String>,
}

const SYSTEM_PROMPT: &str = "\
你是 OhMyWu，一个帮助用户管理电脑的桌面 AI 助手。

## 权限规则

- 只读工具（read/glob/grep/web_fetch/thinking）始终允许。
- 写入工具（write/edit）需要用户确认。
- 高风险工具（bash）需要用户确认。
- 如果工具返回「需要确认」，先向用户说明要做什么，等待用户同意后再执行。
- 如果工具返回「权限不足」，说明该操作被安全规则禁止，向用户解释原因。
- 管理员可以配置 allow/deny 规则，deny 始终覆盖 allow。

## 核心原则

1. 默认使用中文回复，保持简洁友好。
2. 先简要解释你在做什么，再执行命令。
3. 不需要工具时就文字回复。
4. 你直接跑在用户的电脑上，拥有本地执行能力。
5. 如果任务适合拆分，可以先用 `agent_list` 了解可用 Agent，再用 `agent_delegate` 委派边界清晰的子任务。
6. 如果用户明确要求新增或调整长期角色，可以用 `agent_register` 注册或更新 Agent。
";

const MAX_ITERATIONS: usize = 48;

pub struct AgentResponse {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub executions: Vec<ExecutionRecord>,
    pub task_id: Option<String>,
}

struct ExecutionContext {
    fact_count: usize,
    facts: Vec<ExecutionFact>,
    text: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskStateContext {
    last_user_goal: Option<String>,
    last_agent_summary: Option<String>,
    completed: Vec<String>,
    pending_confirmation: Vec<String>,
    blockers: Vec<String>,
    text: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionFact {
    key: String,
    summary: String,
    source_tool: String,
    sticky: bool,
}

/// Run the agent conversation loop.
/// If `app_handle` is provided, streams content deltas via "chat-stream" events.
pub async fn agent_loop(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    user_message: &str,
    agent_profile: Option<&AgentInvocationProfile>,
    llm_config: &LlmConfig,
    app_handle: Option<&tauri::AppHandle>,
    stream_chat: bool,
) -> std::result::Result<AgentResponse, LlmError> {
    let provider = create_provider(llm_config)?;
    let turn_started = Instant::now();
    let history = state.session.load_session(session_id).unwrap_or_default();

    // Step 1: Health check — fast fail if unreachable
    provider.health_check().await?;

    // Step 2: Build the active toolset directly.
    // Claude Code style: prefer a single real execution path over pre-probe gating.
    let tools = active_tool_defs(state, agent_profile.map(|profile| profile.tools.as_slice()));

    let mut executions: Vec<ExecutionRecord> = Vec::new();
    let mut last_task_id: Option<String> = None;
    let memory_context = build_memory_context(state, agent_profile, user_message);
    let task_state_context = build_task_state_context(&history, turn_id);
    let execution_context = build_execution_context(&history);

    if let Some(handle) = app_handle
        && let Some(memory) = &memory_context
        && let Ok(event) = state.runtime.record_event(
            session_id,
            Some(turn_id),
            "memory.recalled",
            &format!("注入 {} 条知识记忆", memory.hit_count),
            serde_json::json!({
                "scope": memory.scope,
                "hitCount": memory.hit_count,
                "hits": memory.hits,
            }),
        )
    {
        let _ = handle.emit("runtime-event", &event);
    }

    if let Some(handle) = app_handle
        && let Some(task_state) = &task_state_context
        && let Ok(event) = state.runtime.record_event(
            session_id,
            Some(turn_id),
            "task.state.recalled",
            "注入最近任务状态",
            serde_json::json!({
                "lastUserGoal": task_state.last_user_goal,
                "lastAgentSummary": task_state.last_agent_summary,
                "completed": task_state.completed,
                "pendingConfirmation": task_state.pending_confirmation,
                "blockers": task_state.blockers,
            }),
        )
    {
        let _ = handle.emit("runtime-event", &event);
    }

    if let Some(handle) = app_handle
        && let Some(execution) = &execution_context
        && let Ok(event) = state.runtime.record_event(
            session_id,
            Some(turn_id),
            "execution.facts.recalled",
            &format!("注入 {} 条已验证执行事实", execution.fact_count),
            serde_json::json!({
                "factCount": execution.fact_count,
                "facts": execution.facts,
                "preview": execution.text,
            }),
        )
    {
        let _ = handle.emit("runtime-event", &event);
    }

    // build initial messages
    let mut messages: Vec<ChatMessage> = Vec::new();
    messages.push(ChatMessage::system(&build_system_prompt(
        state,
        agent_profile,
        memory_context.as_ref(),
        task_state_context.as_ref(),
        execution_context.as_ref(),
    )));

    // include recent session history (last 20 messages)
    for msg in history.iter().rev().take(20).rev() {
        messages.push(ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
            reasoning_content: msg.reasoning_content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // add current user message
    messages.push(ChatMessage::user(user_message));

    // conversation loop
    for _iteration in 0..MAX_ITERATIONS {
        // check cancellation
        if state.cancel_token.load(Ordering::SeqCst) {
            return Ok(AgentResponse {
                content: "操作已中断。".into(),
                reasoning_content: None,
                executions,
                task_id: last_task_id,
            });
        }

        let (response, mut early_cache) = match chat_once(
            provider.as_ref(),
            &messages,
            &tools,
            app_handle,
            stream_chat,
            state,
            session_id,
            turn_id,
            &turn_started,
        ).await {
            Ok(ok) => ok,
            Err(LlmError::Incompatible(_)) if !tools.is_empty() => {
                // Retry once in plain-text mode only after a real tool request fails.
                chat_once(
                    provider.as_ref(),
                    &messages,
                    &[],
                    app_handle,
                    stream_chat,
                    state,
                    session_id,
                    turn_id,
                    &turn_started,
                )
                .await?
            }
            Err(err) => return Err(err),
        };

        // If the assistant responds with content only (no tool calls), return it
        if response.tool_calls.is_none() || response.tool_calls.as_ref().is_none_or(|tc| tc.is_empty()) {
            let content = response.content.unwrap_or_default();
            return Ok(AgentResponse {
                content,
                reasoning_content: response.reasoning_content,
                executions,
                task_id: last_task_id,
            });
        }

        // Process tool calls concurrently using join_all
        let tool_calls = response.tool_calls.as_ref().unwrap();
        let assistant_content = response.content.unwrap_or_default();

        // Add assistant message with tool calls
        messages.push(ChatMessage {
            role: "assistant".into(),
            content: assistant_content,
            reasoning_content: response.reasoning_content.clone(),
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
        });

        // Execute tool calls in order. This keeps delegation and runtime state predictable.
        let mut results = Vec::new();
        let mut exec_meta: Vec<(String, String, String)> = Vec::new();
        // (tc.id, capability, input)

        for tc in tool_calls {
            if tc.function.name.is_empty() {
                continue;
            }

            let params: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                .unwrap_or(serde_json::Value::Null);
            let params = augment_tool_params(
                &tc.function.name,
                params,
                session_id,
                turn_id,
                agent_profile.map(|profile| profile.id.as_str()),
            );

            let capability = tc.function.name.clone();

            if early_cache.get(&tc.id).is_none()
                && let Some(handle) = app_handle
                && let Ok(event) = state.runtime.record_event(
                    session_id,
                    Some(turn_id),
                    "tool.started",
                    &format!("{} 开始执行", capability),
                    serde_json::json!({
                        "capability": capability,
                        "inputPreview": preview_text(&tc.function.arguments, 256),
                        "toolCallId": tc.id,
                        "early": false,
                    }),
                )
            {
                let _ = handle.emit("runtime-event", &event);
            }

            if capability == "agent_delegate"
                && let Some(handle) = app_handle
                && let Some(mut delegate_meta) = build_delegate_started_payload(&params)
                && let Ok(event) = state.runtime.record_event(
                    session_id,
                    Some(turn_id),
                    "agent.delegate.started",
                    &format!(
                        "委派给 {}",
                        delegate_meta
                            .get("targetAgentName")
                            .and_then(|value| value.as_str())
                            .or_else(|| delegate_meta.get("targetAgentId").and_then(|value| value.as_str()))
                            .unwrap_or("子 Agent")
                    ),
                    {
                        if let Some(obj) = delegate_meta.as_object_mut() {
                            obj.insert("toolCallId".into(), serde_json::Value::String(tc.id.clone()));
                        }
                        delegate_meta
                    },
                )
            {
                let _ = handle.emit("runtime-event", &event);
            }

            exec_meta.push((
                tc.id.clone(),
                capability.clone(),
                tc.function.arguments.clone(),
            ));

            if let Some(cached) = early_cache.remove(&tc.id) {
                results.push(cached);
            } else {
                let state_clone = state.clone();
                let result = tools::dispatch_tool(
                    &state_clone,
                    ExecuteRequest {
                        capability,
                        params,
                    },
                )
                .await;
                results.push(result);
            }
        }

        // Process results in order
        for ((tc_id, capability, input), result) in exec_meta.iter().zip(results.iter()) {
            let exec_record = ExecutionRecord {
                capability: capability.clone(),
                input: input.clone(),
                output: result.output.clone(),
                artifact_path: result.artifact_path.clone(),
                error: result.error.clone(),
                status: result.status.clone(),
                duration_ms: result.duration_ms,
            };

            last_task_id = Some(result.task_id.clone());
            executions.push(exec_record);

            let tool_result = match result.status.as_str() {
                "success" => result.output.clone().unwrap_or_else(|| "(empty)".into()),
                "denied" => format!("权限不足：{}", result.error.as_deref().unwrap_or_default()),
                "needs_confirm" => result.output.clone().unwrap_or_default(),
                status => format!("{}：{}", status, result.error.as_deref().unwrap_or_default()),
            };

            messages.push(ChatMessage::tool(&tool_result, tc_id));

            if let Some(handle) = app_handle {
                let delegated = if capability == "agent_delegate" && result.status == "success" {
                    parse_delegate_payload(result.output.as_deref())
                } else {
                    None
                };
                if let Ok(event) = state.runtime.record_event(
                    session_id,
                    Some(turn_id),
                    "tool.completed",
                    &format!("{} -> {}", capability, result.status),
                    serde_json::json!({
                        "capability": capability,
                        "status": result.status,
                        "toolCallId": tc_id,
                        "inputPreview": preview_text(input, 256),
                        "outputPreview": preview_text(result.output.as_deref().unwrap_or(""), 512),
                        "artifactPath": result.artifact_path,
                        "errorPreview": result.error.as_deref().map(|s| preview_text(s, 256)),
                        "durationMs": result.duration_ms,
                        "taskId": result.task_id,
                        "delegated": delegated,
                    }),
                ) {
                    let _ = handle.emit("runtime-event", &event);
                }

                if let Some(delegated) = delegated
                    && let Ok(event) = state.runtime.record_event(
                        session_id,
                        Some(turn_id),
                        "agent.delegate.completed",
                        &format!(
                            "子 Agent 完成：{}",
                            delegated
                                .get("agentName")
                                .and_then(|value| value.as_str())
                                .or_else(|| delegated.get("agentId").and_then(|value| value.as_str()))
                                .unwrap_or("子 Agent")
                        ),
                        delegated,
                    )
                {
                    let _ = handle.emit("runtime-event", &event);
                }
            }
        }
    }

    // Max iterations reached — return whatever we have
    Ok(AgentResponse {
        content: "已执行操作，但达到最大推理轮次。".into(),
        reasoning_content: None,
        executions,
        task_id: last_task_id,
    })
}

async fn chat_once(
    provider: &dyn LlmProvider,
    messages: &[ChatMessage],
    tools: &[ToolDef],
    app_handle: Option<&tauri::AppHandle>,
    stream_chat: bool,
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    turn_started: &Instant,
) -> std::result::Result<(ChatResponse, HashMap<String, tools::ExecuteResult>), LlmError> {
    if let Some(handle) = app_handle {
        if let Ok(event) = state.runtime.record_event(
            session_id,
            Some(turn_id),
            "provider.request.started",
            "发送流式请求",
            serde_json::json!({
                "messageCount": messages.len(),
                "toolCount": tools.len(),
                "approxContextBytes": approx_context_bytes(messages, tools),
                "elapsedMs": turn_started.elapsed().as_millis() as u64,
            }),
        ) {
            let _ = handle.emit("runtime-event", &event);
        }
        match chat_with_streaming(provider, messages, tools, handle, stream_chat, state, session_id, turn_id, turn_started).await {
            Ok(ok) => Ok(ok),
            Err(LlmError::Incompatible(_)) => Err(LlmError::Incompatible(
                "streaming tools incompatible".into(),
            )),
            Err(_) if !tools.is_empty() => {
                if let Ok(event) = state.runtime.record_event(
                    session_id,
                    Some(turn_id),
                    "provider.request.started",
                    "发送同步请求",
                    serde_json::json!({
                        "messageCount": messages.len(),
                        "toolCount": tools.len(),
                        "approxContextBytes": approx_context_bytes(messages, tools),
                        "elapsedMs": turn_started.elapsed().as_millis() as u64,
                    }),
                ) {
                    let _ = handle.emit("runtime-event", &event);
                }
                let resp = provider.chat(messages, tools).await?;
                if resp.tool_calls.as_ref().is_some_and(|t| !t.is_empty()) {
                    if let Ok(event) = state.runtime.record_event(
                        session_id,
                        Some(turn_id),
                        "tool.call.ready",
                        "收到工具调用",
                        serde_json::json!({
                            "toolCount": resp.tool_calls.as_ref().map(|t| t.len()).unwrap_or(0),
                            "elapsedMs": turn_started.elapsed().as_millis() as u64,
                        }),
                    ) {
                        let _ = handle.emit("runtime-event", &event);
                    }
                }
                Ok((resp, HashMap::new()))
            }
            Err(err) => Err(err),
        }
    } else {
        let resp = provider.chat(messages, tools).await?;
        Ok((resp, HashMap::new()))
    }
}

struct MemoryContext {
    scope: String,
    hit_count: usize,
    hits: Vec<RecallHit>,
    text: String,
}

#[derive(Debug, Clone)]
struct ParsedMemoryScope {
    label: String,
    mode: String,
    folders: Vec<String>,
    recall_limit: usize,
    notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TurnMemorySource {
    pub turn_id: String,
    pub user_content: String,
    pub assistant_content: String,
}

fn build_system_prompt(
    state: &AppState,
    agent_profile: Option<&AgentInvocationProfile>,
    memory_context: Option<&MemoryContext>,
    task_state_context: Option<&TaskStateContext>,
    execution_context: Option<&ExecutionContext>,
) -> String {
    let agent_mode = state
        .config
        .read()
        .map(|cfg| cfg.agent_mode)
        .unwrap_or(AgentMode::Agent);
    let mode_note = match agent_mode {
        AgentMode::Plan => "当前模式：plan。优先调查、阅读、写 checklist。不要尝试写文件或执行 shell。",
        AgentMode::Agent => "当前模式：agent。你可以分步执行，但高风险 shell 操作需要先征得用户确认。",
        AgentMode::Auto => "当前模式：auto。你可以连续执行任务，但仍应保持说明清晰、避免不必要的高风险操作。",
    };
    let agent_note = agent_profile.map(|profile| {
        let scope = parse_memory_scope(&profile.memory_scope);
        format!(
            "## 当前 Agent\n- 名称：{}\n- 角色：{}\n- 人格：{}\n- 记忆模式：{}\n- 记忆范围：{}\n- 召回上限：{}\n- 工具范围：{}{}",
            profile.name,
            profile.role,
            profile.persona,
            human_scope_mode(&scope.mode),
            scope.label,
            scope.recall_limit,
            if profile.tools.is_empty() {
                "全部可用工具".to_string()
            } else {
                profile.tools.join(", ")
            },
            scope
                .notes
                .as_ref()
                .map(|notes| format!("\n- 记忆策略：{}", notes))
                .unwrap_or_default()
        )
    });
    let tool_note = Some(build_tool_prompt(state));
    let memory_note = memory_context.map(|memory| format!("## 已注入记忆\n{}", memory.text));
    let task_state_note = task_state_context.map(|task_state| {
        format!(
            "## 当前任务状态\n{}\n\n优先延续这里的已确认状态，而不是每轮重新猜测任务进度。",
            task_state.text
        )
    });
    let execution_note = execution_context.map(|execution| {
        format!(
            "## 最近已验证执行事实\n{}\n\n这些事实来自最近真实工具执行结果。除非有新的工具结果推翻它们，否则不要忽略、改写或凭空假设不同状态。",
            execution.text
        )
    });

    [Some(SYSTEM_PROMPT.to_string()), tool_note, Some(mode_note.to_string()), agent_note, memory_note, task_state_note, execution_note]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_tool_prompt(state: &AppState) -> String {
    let entries = state
        .capability_catalog
        .read()
        .map(|catalog| catalog.active_entries())
        .unwrap_or_default();
    let mut lines = vec!["## 当前可用工具".to_string()];
    for entry in entries {
        lines.push(format!(
            "- `{}`{} — {}",
            entry.name,
            if entry.title.trim().is_empty() {
                String::new()
            } else {
                format!("（{}）", entry.title)
            },
            entry.description
        ));
    }
    lines.join("\n")
}

fn build_task_state_context(messages: &[SessionMessage], current_turn_id: &str) -> Option<TaskStateContext> {
    let previous_messages = messages
        .iter()
        .filter(|msg| msg.turn_id.as_deref() != Some(current_turn_id))
        .collect::<Vec<_>>();

    if previous_messages.is_empty() {
        return None;
    }

    let last_user_goal = previous_messages
        .iter()
        .rev()
        .find(|msg| msg.role == "user" && !msg.content.trim().is_empty())
        .map(|msg| preview_text(msg.content.trim(), 180));

    let last_agent_summary = previous_messages
        .iter()
        .rev()
        .find(|msg| msg.role == "agent" && !msg.content.trim().is_empty())
        .map(|msg| preview_text(msg.content.trim(), 180));

    let mut completed = Vec::new();
    let mut pending_confirmation = Vec::new();
    let mut blockers = Vec::new();

    for msg in previous_messages.iter().rev() {
        let Some(executions) = &msg.executions else {
            continue;
        };
        for execution in executions.iter().rev() {
            match execution.status.as_str() {
                "success" if completed.len() < 4 => {
                    completed.push(render_execution_fact(execution))
                }
                "needs_confirm" if pending_confirmation.len() < 3 => {
                    pending_confirmation.push(render_execution_fact(execution))
                }
                "failed" | "denied" if blockers.len() < 3 => {
                    blockers.push(render_execution_fact(execution))
                }
                _ => {}
            }
        }
        if completed.len() >= 4 && pending_confirmation.len() >= 3 && blockers.len() >= 3 {
            break;
        }
    }

    completed.reverse();
    pending_confirmation.reverse();
    blockers.reverse();

    let mut sections = Vec::new();
    if let Some(goal) = &last_user_goal {
        sections.push(format!("- 最近目标：{}", goal));
    }
    if let Some(summary) = &last_agent_summary {
        sections.push(format!("- 最近回复摘要：{}", summary));
    }
    if !completed.is_empty() {
        sections.push(format!(
            "- 已完成：\n{}",
            completed
                .iter()
                .map(|item| format!("  - {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !pending_confirmation.is_empty() {
        sections.push(format!(
            "- 待确认：\n{}",
            pending_confirmation
                .iter()
                .map(|item| format!("  - {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !blockers.is_empty() {
        sections.push(format!(
            "- 当前阻塞：\n{}",
            blockers
                .iter()
                .map(|item| format!("  - {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    if sections.is_empty() {
        return None;
    }

    Some(TaskStateContext {
        last_user_goal,
        last_agent_summary,
        completed,
        pending_confirmation,
        blockers,
        text: sections.join("\n"),
    })
}

fn build_execution_context(messages: &[SessionMessage]) -> Option<ExecutionContext> {
    let mut seen = HashMap::<String, ()>::new();
    let mut sticky_facts = Vec::new();
    let mut recent_facts = Vec::new();

    for msg in messages.iter().rev() {
        let Some(executions) = &msg.executions else {
            continue;
        };

        for execution in executions.iter().rev() {
            for fact in extract_execution_facts(execution) {
                if seen.contains_key(&fact.key) {
                    continue;
                }
                seen.insert(fact.key.clone(), ());

                if fact.sticky {
                    sticky_facts.push(fact);
                } else {
                    recent_facts.push(fact);
                }

                if sticky_facts.len() >= 4 && recent_facts.len() >= 4 {
                    break;
                }
            }
        }

        if sticky_facts.len() >= 4 && recent_facts.len() >= 4 {
            break;
        }
    }

    if sticky_facts.is_empty() && recent_facts.is_empty() {
        return None;
    }

    sticky_facts.reverse();
    recent_facts.reverse();
    let facts = sticky_facts
        .into_iter()
        .chain(recent_facts)
        .take(8)
        .collect::<Vec<_>>();

    Some(ExecutionContext {
        fact_count: facts.len(),
        facts: facts.clone(),
        text: facts
            .into_iter()
            .enumerate()
            .map(|(index, fact)| format!("{}. {}", index + 1, fact.summary))
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

fn extract_execution_facts(execution: &ExecutionRecord) -> Vec<ExecutionFact> {
    let capability = execution.capability.trim().to_string();
    let input = execution.input.trim();
    let status = execution.status.as_str();
    let output = execution.output.as_deref().unwrap_or("").trim();

    if capability == "bash" && status == "success" {
        let command = extract_named_arg(input, "command").unwrap_or_else(|| input.to_string());
        let command_trimmed = command.trim();
        let first_line = output.lines().next().unwrap_or("").trim();

        if command_trimmed == "pwd" && !first_line.is_empty() {
            return vec![ExecutionFact {
                key: "env.cwd".into(),
                summary: format!("已验证当前工作目录为 `{}`。", first_line),
                source_tool: capability,
                sticky: true,
            }];
        }

        if matches!(
            command_trimmed,
            "git branch --show-current" | "git rev-parse --abbrev-ref HEAD"
        ) && !first_line.is_empty()
        {
            return vec![ExecutionFact {
                key: "env.git.branch".into(),
                summary: format!("已验证当前 Git 分支为 `{}`。", first_line),
                source_tool: capability,
                sticky: true,
            }];
        }

        if command_trimmed == "git status --short" {
            return vec![ExecutionFact {
                key: "env.git.status.short".into(),
                summary: if output.is_empty() {
                    "已验证当前 Git 工作区没有短格式变更输出。".into()
                } else {
                    format!("已验证当前 Git 工作区状态摘要：{}", preview_text(output, 140))
                },
                source_tool: capability,
                sticky: true,
            }];
        }
    }

    if capability == "read" && status == "success" {
        let path = extract_named_arg(input, "path").unwrap_or_else(|| input.to_string());
        return vec![ExecutionFact {
            key: format!("fs.read.{}", path),
            summary: format!(
                "已成功读取文件 `{}`。内容摘要：{}",
                path,
                if output.is_empty() {
                    "(空文件)".into()
                } else {
                    preview_text(output, 140)
                }
            ),
            source_tool: capability,
            sticky: true,
        }];
    }

    if capability == "write" && status == "success" {
        let path = extract_named_arg(input, "path").unwrap_or_else(|| input.to_string());
        return vec![ExecutionFact {
            key: format!("fs.write.{}", path),
            summary: format!("已成功写入文件 `{}`。", path),
            source_tool: capability,
            sticky: true,
        }];
    }

    if capability == "edit" && status == "success" {
        let path = extract_named_arg(input, "file_path").unwrap_or_else(|| input.to_string());
        return vec![ExecutionFact {
            key: format!("fs.edit.{}", path),
            summary: format!("已成功编辑文件 `{}`。", path),
            source_tool: capability,
            sticky: true,
        }];
    }

    vec![ExecutionFact {
        key: format!("{}::{}::{}", capability, status, input),
        summary: render_execution_fact(execution),
        source_tool: capability,
        sticky: false,
    }]
}

fn render_execution_fact(execution: &ExecutionRecord) -> String {
    let input = preview_text(execution.input.trim(), 96);
    match execution.status.as_str() {
        "success" => {
            let output = execution.output.as_deref().unwrap_or("").trim();
            if output.is_empty() {
                format!("`{}` 已成功执行：`{}`。", execution.capability, input)
            } else {
                format!(
                    "`{}` 已成功执行：`{}`。结果摘要：{}",
                    execution.capability,
                    input,
                    preview_text(output, 140)
                )
            }
        }
        "failed" => format!(
            "`{}` 执行失败：`{}`。错误：{}",
            execution.capability,
            input,
            preview_text(execution.error.as_deref().unwrap_or("未知错误"), 140)
        ),
        "denied" => format!(
            "`{}` 被权限拦截：`{}`。原因：{}",
            execution.capability,
            input,
            preview_text(execution.error.as_deref().unwrap_or("权限不足"), 140)
        ),
        "needs_confirm" => format!(
            "`{}` 尚未真正执行：`{}`。当前状态是等待确认。",
            execution.capability,
            input
        ),
        other => format!(
            "`{}` 最近状态为 `{}`：`{}`。",
            execution.capability,
            other,
            input
        ),
    }
}

fn extract_named_arg(input: &str, key: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(input).ok()?;
    parsed
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub async fn delegate_to_agent(
    state: &AppState,
    session_id: &str,
    parent_turn_id: &str,
    target_profile: &AgentInvocationProfile,
    task: &str,
    llm_config: &LlmConfig,
) -> Result<serde_json::Value, String> {
    let child_turn_id = format!("{}::{}", parent_turn_id, target_profile.id);
    let mut child_profile = target_profile.clone();
    child_profile.tools.retain(|tool| tool != "agent_delegate");
    let app_handle = state.get_app_handle();
    let child_mode = state
        .config
        .read()
        .map(|cfg| cfg.agent_mode)
        .unwrap_or(AgentMode::Agent);

    if let Some(handle) = app_handle.as_ref() {
        let child_turn = state.runtime.start_delegated_turn(
            session_id,
            parent_turn_id,
            &child_turn_id,
            child_mode,
            task,
            &target_profile.name,
        )?;
        let _ = handle.emit(
            "runtime-event",
            serde_json::json!({
                "sessionId": session_id,
                "kind": "turn.started",
                "turnId": child_turn.id,
                "summary": format!("子 Agent 开始：{}", target_profile.name),
                "status": child_turn.status,
                "payload": {
                    "agentMode": child_mode,
                    "userContent": task,
                    "agentName": target_profile.name,
                    "parentTurnId": parent_turn_id,
                    "delegated": true,
                },
                "timestamp": child_turn.started_at,
            }),
        );
    }

    let result = std::pin::Pin::from(Box::new(agent_loop(
        state,
        session_id,
        &child_turn_id,
        task,
        Some(&child_profile),
        llm_config,
        app_handle.as_ref(),
        false,
    )))
    .await
    .map_err(|err| format!("子 Agent 调用失败: {}", err))?;

    if let Some(handle) = app_handle.as_ref() {
        let completed_turn = state.runtime.finish_turn(
            session_id,
            &child_turn_id,
            &result.content,
            result.executions.len(),
        )?;
        let _ = handle.emit(
            "runtime-event",
            serde_json::json!({
                "sessionId": session_id,
                "kind": "turn.completed",
                "turnId": completed_turn.id,
                "summary": format!("子 Agent 完成：{}", target_profile.name),
                "status": completed_turn.status,
                "payload": {
                    "executionCount": completed_turn.execution_count,
                    "assistantContent": result.content,
                    "parentTurnId": parent_turn_id,
                    "delegated": true,
                    "agentName": target_profile.name,
                },
                "timestamp": completed_turn.finished_at,
            }),
        );
    }

    let executions = result
        .executions
        .iter()
        .map(|item| {
            serde_json::json!({
                "capability": item.capability,
                "status": item.status,
                "input": item.input,
                "output": item.output,
                "artifactPath": item.artifact_path,
                "error": item.error,
                "durationMs": item.duration_ms,
            })
        })
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "agentId": target_profile.id,
        "agentName": target_profile.name,
        "role": target_profile.role,
        "task": task,
        "content": result.content,
        "reasoningContent": result.reasoning_content,
        "executionCount": result.executions.len(),
        "executions": executions,
    }))
}

fn build_memory_context(
    state: &AppState,
    agent_profile: Option<&AgentInvocationProfile>,
    user_message: &str,
) -> Option<MemoryContext> {
    let profile = agent_profile?;
    let scope = parse_memory_scope(&profile.memory_scope);
    if scope.folders.is_empty() {
        return None;
    }

    let wiki = state.wiki.read().ok()?;
    let hits = wiki
        .recall(user_message, &scope.folders, scope.recall_limit)
        .ok()?;
    if hits.is_empty() {
        return None;
    }

    let lines = hits
        .iter()
        .enumerate()
        .map(|(index, hit)| {
            format!(
                "{}. [{}] {} | tags: {} | 摘要: {}",
                index + 1,
                hit.folder,
                hit.title,
                if hit.tags.is_empty() {
                    "-".to_string()
                } else {
                    hit.tags.join(", ")
                },
                hit.snippet
            )
        })
        .collect::<Vec<_>>();

    Some(MemoryContext {
        scope: scope.label,
        hit_count: hits.len(),
        hits,
        text: lines.join("\n"),
    })
}

fn parse_memory_scope(scope: &str) -> ParsedMemoryScope {
    if let Ok(config) = serde_json::from_str::<MemoryScopeConfig>(scope) {
        return normalize_memory_scope_config(config);
    }

    let scope_lower = scope.to_lowercase();
    if scope_lower.trim().is_empty() || scope_lower.contains("none") || scope_lower.contains("无记忆") {
        return ParsedMemoryScope {
            label: "禁用长期记忆".into(),
            mode: "none".into(),
            folders: Vec::new(),
            recall_limit: 4,
            notes: None,
        };
    }

    let mut folders = Vec::new();
    for folder in ["concepts", "notes", "daily", "profile"] {
        if scope_lower.contains(folder) {
            folders.push(folder.to_string());
        }
    }

    if scope_lower.contains("all") || scope_lower.contains("全部") {
        folders = vec![
            "concepts".into(),
            "notes".into(),
            "daily".into(),
            "profile".into(),
        ];
    }

    ParsedMemoryScope {
        label: if folders.is_empty() {
            "禁用长期记忆".into()
        } else {
            folders
                .iter()
                .map(|folder| human_folder_label(folder))
                .collect::<Vec<_>>()
                .join(" / ")
        },
        mode: if folders.len() == 4 { "all".into() } else { "focused".into() },
        folders,
        recall_limit: 4,
        notes: None,
    }
}

fn normalize_memory_scope_config(config: MemoryScopeConfig) -> ParsedMemoryScope {
    let mode = config.mode.unwrap_or_else(|| "focused".into());
    let mut folders = config
        .folders
        .unwrap_or_default()
        .into_iter()
        .map(|folder| folder.trim().to_lowercase())
        .filter(|folder| matches!(folder.as_str(), "concepts" | "notes" | "daily" | "profile"))
        .collect::<Vec<_>>();
    folders.sort();
    folders.dedup();

    let mode = if mode == "none" || mode == "all" || mode == "focused" {
        mode
    } else {
        "focused".into()
    };

    if mode == "all" {
        folders = vec![
            "concepts".into(),
            "notes".into(),
            "daily".into(),
            "profile".into(),
        ];
    }
    if mode == "none" {
        folders.clear();
    }

    let recall_limit = config.recall_limit.unwrap_or(4).clamp(1, 8);
    let notes = config.notes.and_then(|notes| {
        let trimmed = notes.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let label = config
        .label
        .map(|label| label.trim().to_string())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| {
            if mode == "none" {
                "禁用长期记忆".into()
            } else if mode == "all" {
                "全部知识".into()
            } else if folders.is_empty() {
                "定向记忆".into()
            } else {
                folders
                    .iter()
                    .map(|folder| human_folder_label(folder))
                    .collect::<Vec<_>>()
                    .join(" / ")
            }
        });

    ParsedMemoryScope {
        label,
        mode,
        folders,
        recall_limit,
        notes,
    }
}

fn human_folder_label(folder: &str) -> String {
    match folder {
        "concepts" => "概念".into(),
        "notes" => "笔记".into(),
        "daily" => "每日".into(),
        "profile" => "画像".into(),
        _ => folder.to_string(),
    }
}

fn human_scope_mode(mode: &str) -> &'static str {
    match mode {
        "none" => "禁用",
        "all" => "全量",
        _ => "定向",
    }
}

fn augment_tool_params(
    tool_name: &str,
    mut params: serde_json::Value,
    session_id: &str,
    turn_id: &str,
    current_agent_id: Option<&str>,
) -> serde_json::Value {
    if let Some(obj) = params.as_object_mut() {
        if matches!(tool_name, "checklist_write" | "agent_list" | "agent_delegate") {
            obj.insert("session_id".into(), serde_json::Value::String(session_id.to_string()));
            obj.insert("turn_id".into(), serde_json::Value::String(turn_id.to_string()));
        }
        if matches!(tool_name, "agent_list" | "agent_delegate")
            && let Some(agent_id) = current_agent_id
        {
            obj.insert("currentAgentId".into(), serde_json::Value::String(agent_id.to_string()));
        }
    }
    params
}

fn approx_context_bytes(messages: &[ChatMessage], tools: &[ToolDef]) -> usize {
    serde_json::to_vec(&serde_json::json!({
        "messages": messages,
        "tools": tools,
    }))
    .map(|bytes| bytes.len())
    .unwrap_or(0)
}

fn preview_text(value: &str, max_chars: usize) -> String {
    let total_chars = value.chars().count();
    if total_chars <= max_chars {
        return value.to_string();
    }
    let preview: String = value.chars().take(max_chars).collect();
    format!("{}… (+{} chars)", preview, total_chars - max_chars)
}

fn build_delegate_started_payload(params: &serde_json::Value) -> Option<serde_json::Value> {
    let target_agent_id = params.get("targetAgentId")?.as_str()?.trim();
    if target_agent_id.is_empty() {
        return None;
    }
    let task = params
        .get("task")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let target_agent_name = params
        .get("targetAgentName")
        .and_then(|value| value.as_str())
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    Some(serde_json::json!({
        "targetAgentId": target_agent_id,
        "targetAgentName": target_agent_name,
        "task": task,
    }))
}

fn parse_delegate_payload(output: Option<&str>) -> Option<serde_json::Value> {
    let output = output?.trim();
    if output.is_empty() {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(output).ok()
}

pub fn resolve_turn_memory_source(
    messages: &[SessionMessage],
    requested_turn_id: Option<&str>,
) -> Result<TurnMemorySource, String> {
    let turn_id = if let Some(turn_id) = requested_turn_id {
        turn_id.to_string()
    } else {
        messages
            .iter()
            .rev()
            .find(|msg| msg.role == "agent" && msg.turn_id.is_some())
            .and_then(|msg| msg.turn_id.clone())
            .ok_or_else(|| "当前会话里还没有可用于记忆沉淀的助手回复".to_string())?
    };

    let mut user_content: Option<String> = None;
    let mut assistant_content: Option<String> = None;

    for msg in messages {
        if msg.turn_id.as_deref() != Some(turn_id.as_str()) {
            continue;
        }
        if msg.role == "user" && user_content.is_none() {
            user_content = Some(msg.content.clone());
        }
        if msg.role == "agent" && assistant_content.is_none() {
            assistant_content = Some(msg.content.clone());
        }
    }

    let user_content = user_content.ok_or_else(|| "未找到该回合对应的用户消息".to_string())?;
    let assistant_content =
        assistant_content.ok_or_else(|| "未找到该回合对应的助手回复".to_string())?;

    Ok(TurnMemorySource {
        turn_id,
        user_content,
        assistant_content,
    })
}

pub async fn generate_memory_candidate(
    llm_config: &LlmConfig,
    source: &TurnMemorySource,
) -> Result<MemoryCandidateDraft, LlmError> {
    let provider = create_provider(llm_config)?;
    let user_excerpt = clip_text_middle(&source.user_content, 1800);
    let assistant_excerpt = clip_text_middle(&source.assistant_content, 4200);
    let prompt = format!(
        "请基于下面这一轮对话，产出一个适合写入长期知识库的记忆候选。\n\n要求：\n1. 只返回 JSON，不要 Markdown，不要解释。\n2. JSON 结构必须是：{{\"title\":\"...\",\"folder\":\"concepts|notes|daily|profile\",\"tags\":[\"...\"],\"body\":\"...\",\"shouldSave\":true,\"reason\":\"...\"}}\n3. `body` 要写成可独立阅读的中文 Markdown，避免口水话，保留真正可复用的信息。\n4. `title` 简洁明确。\n5. `tags` 2 到 6 个即可。\n6. 如果这一轮并不值得长期保存，也要返回同样结构，但将 `shouldSave` 设为 false，并在 `reason` 里说明原因。\n7. `folder` 只能从 `concepts`、`notes`、`daily`、`profile` 中选一个。\n\n[用户消息]\n{}\n\n[助手回复]\n{}",
        user_excerpt, assistant_excerpt
    );

    let response = provider
        .chat(
            &[
                ChatMessage::system(
                    "你是一个严格的知识库整理器，负责把对话回合提炼成长期记忆候选。输出必须是可被 JSON.parse 的纯 JSON。",
                ),
                ChatMessage::user(&prompt),
            ],
            &[],
        )
        .await?;

    let raw = response.content.unwrap_or_default();
    parse_memory_candidate(&raw).map_err(LlmError::Protocol)
}

fn parse_memory_candidate(raw: &str) -> Result<MemoryCandidateDraft, String> {
    let candidate_text = extract_json_object(raw)
        .ok_or_else(|| format!("模型没有返回合法 JSON：{}", preview_text(raw, 180)))?;
    let mut candidate: MemoryCandidateDraft = serde_json::from_str(candidate_text)
        .map_err(|err| format!("解析记忆候选失败: {}", err))?;

    candidate.title = candidate.title.trim().to_string();
    candidate.folder = normalize_memory_folder(&candidate.folder);
    candidate.tags = candidate
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .take(6)
        .collect();
    candidate.body = candidate.body.trim().to_string();
    candidate.reason = candidate.reason.trim().to_string();

    if candidate.title.is_empty() {
        candidate.title = "未命名记忆".into();
    }
    if candidate.reason.is_empty() {
        candidate.reason = if candidate.should_save {
            "包含可复用的长期信息".into()
        } else {
            "信息偏临时，不建议长期沉淀".into()
        };
    }

    Ok(candidate)
}

fn extract_json_object(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let trimmed = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim();
    let trimmed = trimmed.strip_suffix("```").unwrap_or(trimmed).trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    trimmed.get(start..=end)
}

fn clip_text_middle(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_string();
    }

    let head_len = max_chars * 2 / 3;
    let tail_len = max_chars.saturating_sub(head_len + 32);
    let head: String = text.chars().take(head_len).collect();
    let tail: String = text
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}\n\n...[内容已截断，共 {total} 字]...\n\n{tail}")
}

pub fn normalize_memory_folder(folder: &str) -> String {
    match folder.trim().to_lowercase().as_str() {
        "concepts" | "notes" | "daily" | "profile" => folder.trim().to_lowercase(),
        _ => "notes".into(),
    }
}

/// Streaming chat: emit chunks via Tauri events, execute read-only tools during streaming.
/// Returns (ChatResponse, early_cache) where early_cache maps tool_call_id → ExecuteResult
/// for read-only tools that were executed before streaming finished.
async fn chat_with_streaming(
    provider: &dyn LlmProvider,
    messages: &[ChatMessage],
    tools: &[ToolDef],
    app_handle: &tauri::AppHandle,
    stream_chat: bool,
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    turn_started: &Instant,
) -> std::result::Result<(ChatResponse, HashMap<String, tools::ExecuteResult>), LlmError> {
    use futures::StreamExt;
    use std::collections::HashMap as StdHashMap;

    let mut stream = provider.chat_stream(messages, tools).await?;
    let mut full_content = String::new();
    let mut full_reasoning = String::new();
    let mut tool_call_parts: StdHashMap<usize, ToolCall> = StdHashMap::new();
    let mut first_token_recorded = false;
    let mut first_tool_call_recorded = false;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                if !first_token_recorded
                    && (c.content_delta.is_some()
                        || c.reasoning_delta.is_some()
                        || c.tool_call_delta.is_some())
                {
                    first_token_recorded = true;
                    if let Ok(event) = state.runtime.record_event(
                        session_id,
                        Some(turn_id),
                        "provider.first_token",
                        "收到首个流式片段",
                        serde_json::json!({
                            "elapsedMs": turn_started.elapsed().as_millis() as u64,
                        }),
                    ) {
                        let _ = app_handle.emit("runtime-event", &event);
                    }
                }
                if let Some(delta) = &c.content_delta {
                    full_content.push_str(delta);
                }
                if let Some(delta) = &c.reasoning_delta {
                    full_reasoning.push_str(delta);
                }

                // Accumulate tool call deltas — dedup by ID
                if let Some(ref delta) = c.tool_call_delta {
                    let entry = tool_call_parts.entry(delta.index).or_insert_with(|| ToolCall {
                        id: delta.id.clone().unwrap_or_else(|| format!("tool_{}", delta.index)),
                        call_type: "function".into(),
                        function: ohmywu_llm_adapter::types::ToolCallFunction {
                            name: delta.name.clone().unwrap_or_default(),
                            arguments: String::new(),
                        },
                    });

                    if let Some(id) = &delta.id {
                        entry.id = id.clone();
                    }
                    if let Some(name) = &delta.name {
                        entry.function.name = name.clone();
                    }
                    if let Some(arguments_delta) = &delta.arguments_delta {
                        entry.function.arguments.push_str(arguments_delta);
                    }

                    if !entry.function.name.is_empty()
                        && let Ok(params) = serde_json::from_str::<serde_json::Value>(&entry.function.arguments)
                    {
                        if !first_tool_call_recorded {
                            first_tool_call_recorded = true;
                            if let Ok(event) = state.runtime.record_event(
                                session_id,
                                Some(turn_id),
                                "tool.call.ready",
                                "收到工具调用",
                                serde_json::json!({
                                    "capability": entry.function.name,
                                    "toolCallId": entry.id,
                                    "elapsedMs": turn_started.elapsed().as_millis() as u64,
                                }),
                            ) {
                                let _ = app_handle.emit("runtime-event", &event);
                            }
                        }
                        let _ = params;
                    }
                }

                // Emit to frontend (content only)
                if stream_chat {
                    let _ = app_handle.emit("chat-stream", &c);
                }

                if c.done {
                    break;
                }
            }
            Err(e) => {
                if stream_chat {
                    let _ = app_handle.emit(
                        "chat-stream",
                        &ChatStreamChunk {
                            content_delta: Some(format!("\n[错误: {}]", e)),
                            reasoning_delta: None,
                            tool_call_delta: None,
                            done: true,
                        },
                    );
                }
                return Err(e);
            }
        }
    }

    let early_cache: HashMap<String, tools::ExecuteResult> = HashMap::new();

    let mut ordered_parts: Vec<(usize, ToolCall)> = tool_call_parts.into_iter().collect();
    ordered_parts.sort_by_key(|(index, _)| *index);
    let full_tool_calls: Vec<ToolCall> = ordered_parts
        .into_iter()
        .map(|(_, tc)| tc)
        .filter(|tc| !tc.function.name.is_empty())
        .collect();

    Ok((
        ChatResponse {
            role: "assistant".into(),
            content: if full_content.is_empty() {
                None
            } else {
                Some(full_content)
            },
            reasoning_content: if full_reasoning.is_empty() {
                None
            } else {
                Some(full_reasoning)
            },
            tool_calls: if full_tool_calls.is_empty() {
                None
            } else {
                Some(full_tool_calls)
            },
        },
        early_cache,
    ))
}

/// Save a message to the session with execution records.
pub fn build_agent_message(
    response: &AgentResponse,
    turn_id: &str,
    agent_profile: Option<&AgentInvocationProfile>,
) -> SessionMessage {
    SessionMessage {
        role: "agent".into(),
        content: response.content.clone(),
        agent_id: agent_profile.map(|profile| profile.id.clone()),
        agent_name: agent_profile.map(|profile| profile.name.clone()),
        turn_id: Some(turn_id.to_string()),
        reasoning_content: response.reasoning_content.clone(),
        executions: if response.executions.is_empty() {
            None
        } else {
            Some(response.executions.clone())
        },
        task_id: response.task_id.clone(),
        timestamp: chrono_now(),
    }
}
