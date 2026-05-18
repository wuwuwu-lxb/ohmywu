use std::collections::HashMap;
use std::sync::atomic::Ordering;

use futures::FutureExt;

use ohmywu_domain::{chrono_now, AgentMode, RiskLevel};
use ohmywu_llm_adapter::types::{ChatMessage, ChatResponse, ChatStreamChunk, ToolCall};
use ohmywu_llm_adapter::{create_provider, LlmConfig, LlmError, LlmProvider};
use ohmywu_session::{ExecutionRecord, SessionMessage};
use tauri::Emitter;

use crate::tools::{self, active_tool_defs, ExecuteRequest};
use crate::AppState;

const SYSTEM_PROMPT: &str = "\
你是 OhMyWu，一个帮助用户管理电脑的桌面 AI 助手。

## 可用工具

- `bash` — 执行 shell 命令
- `read` — 读取文件内容
- `write` — 写入文件（自动创建目录）
- `edit` — 精确替换文件中的文本（old_string 必须唯一匹配）
- `glob` — 搜索匹配模式的文件
- `grep` — 搜索文件内容
- `web_fetch` — 获取 URL 内容
- `thinking` — 内部推理和规划
- `checklist_write` — 写入当前回合的执行清单

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
";

const MAX_ITERATIONS: usize = 10;

pub struct AgentResponse {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub executions: Vec<ExecutionRecord>,
    pub task_id: Option<String>,
}

/// Run the agent conversation loop.
/// If `app_handle` is provided, streams content deltas via "chat-stream" events.
pub async fn agent_loop(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    user_message: &str,
    llm_config: &LlmConfig,
    app_handle: Option<&tauri::AppHandle>,
) -> std::result::Result<AgentResponse, LlmError> {
    let provider = create_provider(llm_config)?;

    // Step 1: Health check — fast fail if unreachable
    provider.health_check().await?;

    // Step 2: Build the active toolset directly.
    // Claude Code style: prefer a single real execution path over pre-probe gating.
    let tools = active_tool_defs(state);

    let mut executions: Vec<ExecutionRecord> = Vec::new();
    let mut last_task_id: Option<String> = None;

    // build initial messages
    let mut messages: Vec<ChatMessage> = Vec::new();
    messages.push(ChatMessage::system(&build_system_prompt(state)));

    // include recent session history (last 20 messages)
    let history = state.session.load_session(session_id).unwrap_or_default();
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
            state,
        ).await {
            Ok(ok) => ok,
            Err(LlmError::Incompatible(_)) if !tools.is_empty() => {
                // Retry once in plain-text mode only after a real tool request fails.
                chat_once(provider.as_ref(), &messages, &[], app_handle, state).await?
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

        // Build dispatch futures for all tool calls
        let mut futures = Vec::new();
        let mut exec_meta: Vec<(String, String, String)> = Vec::new();
        // (tc.id, capability, input)

        for tc in tool_calls {
            if tc.function.name.is_empty() {
                continue;
            }

            let params: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                .unwrap_or(serde_json::Value::Null);
            let params = augment_tool_params(&tc.function.name, params, session_id, turn_id);

            // Tool name = capability name. Unknown tools fall back to bash.
            let capability = if state.capabilities.contains(&tc.function.name) {
                tc.function.name.clone()
            } else {
                "bash".to_string()
            };

            exec_meta.push((
                tc.id.clone(),
                capability.clone(),
                tc.function.arguments.clone(),
            ));

            // Use cached result if early-executed during streaming
            if let Some(cached) = early_cache.remove(&tc.id) {
                futures.push(futures::future::ready(cached).boxed());
            } else {
                let state_clone = state.clone();
                futures.push(
                    async move {
                        tools::dispatch_tool(
                            &state_clone,
                            ExecuteRequest {
                                capability,
                                params,
                            },
                        )
                        .await
                    }
                    .boxed(),
                );
            }
        }

        // Run all tool dispatches concurrently
        let results = futures::future::join_all(futures).await;

        // Process results in order
        for ((tc_id, capability, input), result) in exec_meta.iter().zip(results.iter()) {
            let exec_record = ExecutionRecord {
                capability: capability.clone(),
                input: input.clone(),
                output: result.output.clone(),
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
                let _ = state.runtime.append_tool_event(session_id, turn_id, capability, &result.status);
                let _ = handle.emit(
                    "runtime-event",
                    serde_json::json!({
                        "kind": "tool.completed",
                        "turnId": turn_id,
                        "sessionId": session_id,
                        "summary": format!("{} -> {}", capability, result.status),
                        "capability": capability,
                        "status": result.status,
                        "timestamp": chrono_now(),
                    }),
                );
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
    tools: &[ohmywu_llm_adapter::types::ToolDef],
    app_handle: Option<&tauri::AppHandle>,
    state: &AppState,
) -> std::result::Result<(ChatResponse, HashMap<String, tools::ExecuteResult>), LlmError> {
    if let Some(handle) = app_handle {
        match chat_with_streaming(provider, messages, tools, handle, state).await {
            Ok(ok) => Ok(ok),
            Err(LlmError::Incompatible(_)) => Err(LlmError::Incompatible(
                "streaming tools incompatible".into(),
            )),
            Err(_) if !tools.is_empty() => {
                let resp = provider.chat(messages, tools).await?;
                Ok((resp, HashMap::new()))
            }
            Err(err) => Err(err),
        }
    } else {
        let resp = provider.chat(messages, tools).await?;
        Ok((resp, HashMap::new()))
    }
}

fn build_system_prompt(state: &AppState) -> String {
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
    format!("{}\n\n{}", SYSTEM_PROMPT, mode_note)
}

fn augment_tool_params(
    tool_name: &str,
    mut params: serde_json::Value,
    session_id: &str,
    turn_id: &str,
) -> serde_json::Value {
    if tool_name != "checklist_write" {
        return params;
    }
    if let Some(obj) = params.as_object_mut() {
        obj.insert("session_id".into(), serde_json::Value::String(session_id.to_string()));
        obj.insert("turn_id".into(), serde_json::Value::String(turn_id.to_string()));
    }
    params
}

/// Streaming chat: emit chunks via Tauri events, execute read-only tools during streaming.
/// Returns (ChatResponse, early_cache) where early_cache maps tool_call_id → ExecuteResult
/// for read-only tools that were executed before streaming finished.
async fn chat_with_streaming(
    provider: &dyn LlmProvider,
    messages: &[ChatMessage],
    tools: &[ohmywu_llm_adapter::types::ToolDef],
    app_handle: &tauri::AppHandle,
    state: &AppState,
) -> std::result::Result<(ChatResponse, HashMap<String, tools::ExecuteResult>), LlmError> {
    use futures::StreamExt;
    use std::collections::{HashMap as StdHashMap, HashSet};

    let mut stream = provider.chat_stream(messages, tools).await?;
    let mut full_content = String::new();
    let mut full_reasoning = String::new();
    let mut tool_call_parts: StdHashMap<usize, ToolCall> = StdHashMap::new();
    let mut early_started: HashSet<String> = HashSet::new();

    // Track early-executed tool calls spawned during streaming
    let mut early_handles: Vec<(String, tokio::task::JoinHandle<tools::ExecuteResult>)> = Vec::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
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
                        let is_readonly = state
                            .capabilities
                            .get(&entry.function.name)
                            .map(|c| matches!(c.risk_level, RiskLevel::ReadOnly))
                            .unwrap_or(false);

                        if is_readonly && early_started.insert(entry.id.clone()) {
                            let state_clone = state.clone();
                            let cap_name = entry.function.name.clone();
                            let tc_id = entry.id.clone();
                            let handle = tokio::spawn(async move {
                                tools::dispatch_tool(
                                    &state_clone,
                                    ExecuteRequest {
                                        capability: cap_name,
                                        params,
                                    },
                                )
                                .await
                            });
                            early_handles.push((tc_id, handle));
                        }
                    }
                }

                // Emit to frontend (content only)
                let _ = app_handle.emit("chat-stream", &c);

                if c.done {
                    break;
                }
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "chat-stream",
                    &ChatStreamChunk {
                        content_delta: Some(format!("\n[错误: {}]", e)),
                        reasoning_delta: None,
                        tool_call_delta: None,
                        done: true,
                    },
                );
                return Err(e);
            }
        }
    }

    // Collect early execution results (tasks ran concurrently with streaming, likely done)
    let mut early_cache: HashMap<String, tools::ExecuteResult> = HashMap::new();
    for (tc_id, handle) in early_handles {
        if let Ok(result) = handle.await {
            early_cache.insert(tc_id, result);
        }
    }

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
) -> SessionMessage {
    SessionMessage {
        role: "agent".into(),
        content: response.content.clone(),
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
