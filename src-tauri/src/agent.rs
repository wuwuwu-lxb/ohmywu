use std::time::Instant;

use ohmywu_domain::chrono_now;
use ohmywu_llm_adapter::types::{ChatMessage, ChatResponse, ChatStreamChunk, ToolCall};
use ohmywu_llm_adapter::{create_provider, LlmConfig, LlmError, LlmProvider};
use ohmywu_session::{ExecutionRecord, SessionMessage};
use tauri::Emitter;

use crate::tools::{self, active_tool_defs, ExecuteRequest};
use crate::AppState;

const SYSTEM_PROMPT: &str = "\
你是 OhMyWu，一个帮助用户管理电脑的桌面 AI 助手。\
你可以执行 shell 命令（bash）和读取文件（read）来帮助用户解决问题。\
\
核心原则：\
1. 在执行可能有破坏性的操作前，先向用户确认。\
2. 默认使用中文回复，保持简洁友好。\
3. 在执行命令前，简要解释你在做什么。\
4. 如果用户的请求不需要工具，直接文字回复。\
5. 你直接跑在用户的电脑上，拥有本地执行能力。\
";

const MAX_ITERATIONS: usize = 10;

pub struct AgentResponse {
    pub content: String,
    pub executions: Vec<ExecutionRecord>,
    pub task_id: Option<String>,
}

/// Run the agent conversation loop.
/// If `app_handle` is provided, streams content deltas via "chat-stream" events.
pub async fn agent_loop(
    state: &AppState,
    session_id: &str,
    user_message: &str,
    llm_config: &LlmConfig,
    app_handle: Option<&tauri::AppHandle>,
) -> std::result::Result<AgentResponse, LlmError> {
    let provider = create_provider(llm_config)?;

    // Step 1: Health check — fast fail if unreachable
    provider.health_check().await?;

    // Step 2: Probe capabilities — detect whether tools/streaming are supported
    let caps = provider.probe_capabilities().await;
    let tools = if caps.supports_streaming_with_tools {
        active_tool_defs(state)
    } else {
        // Model doesn't support tools (e.g. DeepSeek), fall back to pure text
        vec![]
    };

    let mut executions: Vec<ExecutionRecord> = Vec::new();
    let mut last_task_id: Option<String> = None;

    // build initial messages
    let mut messages: Vec<ChatMessage> = Vec::new();
    messages.push(ChatMessage::system(SYSTEM_PROMPT));

    // include recent session history (last 20 messages)
    let history = state.session.load_session(session_id).unwrap_or_default();
    for msg in history.iter().rev().take(20).rev() {
        messages.push(ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // add current user message
    messages.push(ChatMessage::user(user_message));

    // conversation loop
    for _iteration in 0..MAX_ITERATIONS {
        let response: ChatResponse = if let Some(handle) = app_handle {
            // streaming mode: collect all content and tool calls
            chat_with_streaming(provider.as_ref(), &messages, &tools, handle).await?
        } else {
            provider.chat(&messages, &tools).await?
        };

        // If the assistant responds with content only (no tool calls), return it
        if response.tool_calls.is_none() || response.tool_calls.as_ref().is_none_or(|tc| tc.is_empty()) {
            let content = response.content.unwrap_or_default();
            return Ok(AgentResponse {
                content,
                executions,
                task_id: last_task_id,
            });
        }

        // Process tool calls
        let tool_calls = response.tool_calls.as_ref().unwrap();
        let assistant_content = response.content.unwrap_or_default();

        // Add assistant message with tool calls
        messages.push(ChatMessage {
            role: "assistant".into(),
            content: assistant_content,
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
        });

        for tc in tool_calls {
            let name = &tc.function.name;
            if name.is_empty() {
                continue;
            }

            let params: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                .unwrap_or(serde_json::Value::Null);

            // Tool name = capability name. Unknown tools fall back to bash.
            let capability = if state.capabilities.contains(name) {
                name.clone()
            } else {
                "bash".to_string()
            };

            let req = ExecuteRequest {
                capability: capability.clone(),
                params: params.clone(),
            };

            let start = Instant::now();
            let result = tools::dispatch_tool(state, req).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            let exec_record = ExecutionRecord {
                capability: capability.clone(),
                input: tc.function.arguments.clone(),
                output: result.output.clone(),
                error: result.error.clone(),
                status: result.status.clone(),
                duration_ms,
            };

            last_task_id = Some(result.task_id.clone());
            executions.push(exec_record);

            let tool_result = match result.status.as_str() {
                "success" => result.output.unwrap_or_else(|| "(empty)".into()),
                "denied" => format!("权限不足：{}", result.error.unwrap_or_default()),
                status => format!("{}：{}", status, result.error.unwrap_or_default()),
            };

            messages.push(ChatMessage::tool(&tool_result, &tc.id));
        }
    }

    // Max iterations reached — return whatever we have
    Ok(AgentResponse {
        content: "已执行操作，但达到最大推理轮次。".into(),
        executions,
        task_id: last_task_id,
    })
}

/// Streaming chat: emit chunks via Tauri events, return the final ChatResponse.
async fn chat_with_streaming(
    provider: &dyn LlmProvider,
    messages: &[ChatMessage],
    tools: &[ohmywu_llm_adapter::types::ToolDef],
    app_handle: &tauri::AppHandle,
) -> std::result::Result<ChatResponse, LlmError> {
    use futures::StreamExt;

    let mut stream = provider.chat_stream(messages, tools).await?;
    let mut full_content = String::new();
    let mut full_tool_calls: Vec<ToolCall> = Vec::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                if let Some(delta) = &c.content_delta {
                    full_content.push_str(delta);
                }

                // Accumulate tool call deltas
                if let Some(ref delta) = c.tool_call_delta
                    && let Some(ref args) = delta.arguments_delta
                {
                    // Try parsing as a complete tool call
                    if let Ok(tcs) = serde_json::from_str::<Vec<ToolCall>>(args) {
                        full_tool_calls.extend(tcs);
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
                        tool_call_delta: None,
                        done: true,
                    },
                );
                return Err(e);
            }
        }
    }

    Ok(ChatResponse {
        role: "assistant".into(),
        content: if full_content.is_empty() {
            None
        } else {
            Some(full_content)
        },
        tool_calls: if full_tool_calls.is_empty() {
            None
        } else {
            Some(full_tool_calls)
        },
    })
}

/// Save a message to the session with execution records.
pub fn build_agent_message(
    response: &AgentResponse,
) -> SessionMessage {
    SessionMessage {
        role: "agent".into(),
        content: response.content.clone(),
        executions: if response.executions.is_empty() {
            None
        } else {
            Some(response.executions.clone())
        },
        task_id: response.task_id.clone(),
        timestamp: chrono_now(),
    }
}
