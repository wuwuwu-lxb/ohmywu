use crate::error::LlmError;
use crate::types::*;
use serde_json::{json, Value};

/// Build an Anthropic Messages API request body.
pub fn build_request(
    model: &str,
    system_prompt: Option<&str>,
    messages: &[ChatMessage],
    tools: &[ToolDef],
    max_tokens: u32,
    stream: bool,
) -> Value {
    let mut body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": stream,
    });

    // System prompt (Anthropic uses top-level "system" field, not a message role)
    if let Some(sys) = system_prompt {
        body["system"] = json!(sys);
    }

    // Convert messages to Anthropic format
    let anthropic_messages: Vec<Value> = messages
        .iter()
        .filter(|m| m.role != "system") // system is handled above
        .map(|m| {
            let role = match m.role.as_str() {
                "assistant" => "assistant",
                "tool" => "user", // tool results become user messages with tool_result content
                _ => "user",
            };

            if m.role == "tool" {
                // Tool result: content is a list of tool_result blocks
                json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": m.tool_call_id.as_deref().unwrap_or(""),
                        "content": m.content
                    }]
                })
            } else if let Some(ref tcs) = m.tool_calls {
                // Assistant message with tool calls
                let mut content: Vec<Value> = if m.content.is_empty() {
                    vec![]
                } else {
                    vec![json!({"type": "text", "text": m.content})]
                };
                for tc in tcs {
                    let args: Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                    content.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.function.name,
                        "input": args
                    }));
                }
                json!({
                    "role": "assistant",
                    "content": content
                })
            } else {
                json!({
                    "role": role,
                    "content": m.content
                })
            }
        })
        .collect();

    body["messages"] = json!(anthropic_messages);

    // Tools
    if !tools.is_empty() {
        let anthropic_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.function.name,
                    "description": t.function.description,
                    "input_schema": t.function.parameters,
                })
            })
            .collect();
        body["tools"] = json!(anthropic_tools);
    }

    body
}

/// Parse a non-streaming Anthropic Messages API response.
pub fn parse_response(data: &serde_json::Value) -> std::result::Result<ChatResponse, LlmError> {
    let role = data
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("assistant")
        .to_string();

    let content_blocks = data.get("content").and_then(|v| v.as_array());

    let text_content: String = content_blocks
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    let content = if text_content.is_empty() {
        None
    } else {
        Some(text_content)
    };

    let tool_calls: Option<Vec<ToolCall>> = content_blocks.map(|blocks| {
        blocks
            .iter()
            .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
            .filter_map(|b| {
                let id = b.get("id")?.as_str()?;
                let name = b.get("name")?.as_str()?;
                let input = b.get("input")?;
                Some(ToolCall {
                    id: id.to_string(),
                    call_type: "function".to_string(),
                    function: ToolCallFunction {
                        name: name.to_string(),
                        arguments: serde_json::to_string(input).unwrap_or_default(),
                    },
                })
            })
            .collect()
    });

    Ok(ChatResponse {
        role,
        content,
        reasoning_content: None,
        tool_calls: tool_calls.filter(|tc| !tc.is_empty()),
    })
}

/// Parse a single SSE line from Anthropic streaming response.
/// Returns None for non-content lines (events, ping, etc.).
pub fn parse_stream_line(line: &str) -> std::result::Result<Option<ChatStreamChunk>, LlmError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    // SSE format: "event: <type>\n" or "data: <json>\n"
    if let Some(data_str) = line.strip_prefix("data: ") {
        if data_str == "[DONE]" {
            return Ok(Some(ChatStreamChunk {
                content_delta: None,
                reasoning_delta: None,
                tool_call_delta: None,
                done: true,
            }));
        }

        let data: Value =
            serde_json::from_str(data_str).map_err(|e| LlmError::Protocol(e.to_string()))?;

        match data.get("type").and_then(|t| t.as_str()) {
            Some("content_block_delta") => {
                let delta = data.get("delta").ok_or_else(|| LlmError::Protocol("missing delta in content_block_delta".into()))?;
                match delta.get("type").and_then(|t| t.as_str()) {
                    Some("text_delta") => {
                        let text = delta
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or_default()
                            .to_string();
                        Ok(Some(ChatStreamChunk {
                            content_delta: if text.is_empty() { None } else { Some(text) },
                            reasoning_delta: None,
                            tool_call_delta: None,
                            done: false,
                        }))
                    }
                    Some("input_json_delta") => {
                        let partial = delta
                            .get("partial_json")
                            .and_then(|t| t.as_str())
                            .unwrap_or_default();
                        let index = data
                            .get("index")
                            .and_then(|i| i.as_u64())
                            .unwrap_or(0) as usize;
                        Ok(Some(ChatStreamChunk {
                            content_delta: None,
                            reasoning_delta: None,
                            tool_call_delta: Some(ToolCallDelta {
                                index,
                                id: None,
                                name: None,
                                arguments_delta: Some(partial.to_string()),
                            }),
                            done: false,
                        }))
                    }
                    _ => Ok(None),
                }
            }
            Some("content_block_start") => {
                let block = data.get("content_block").ok_or_else(|| LlmError::Protocol("missing content_block in content_block_start".into()))?;
                match block.get("type").and_then(|t| t.as_str()) {
                    Some("tool_use") => {
                        let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        Ok(Some(ChatStreamChunk {
                            content_delta: None,
                            reasoning_delta: None,
                            tool_call_delta: Some(ToolCallDelta {
                                index: data
                                    .get("index")
                                    .and_then(|i| i.as_u64())
                                    .unwrap_or(0) as usize,
                                id: Some(id.to_string()),
                                name: Some(name.to_string()),
                                arguments_delta: None,
                            }),
                            done: false,
                        }))
                    }
                    _ => Ok(None),
                }
            }
            Some("message_stop") => Ok(Some(ChatStreamChunk {
                content_delta: None,
                reasoning_delta: None,
                tool_call_delta: None,
                done: true,
            })),
            Some("message_start") => parse_stream_line_from_message_start(&data),
            _ => Ok(None), // Skip other event types (ping, etc.)
        }
    } else if line.starts_with("event: ") || line.starts_with(':') {
        // SSE event type line or comment (ignore)
        Ok(None)
    } else {
        Ok(None)
    }
}

/// Parse the initial message_start event to check for pre-existing tool_use blocks.
fn parse_stream_line_from_message_start(
    data: &Value,
) -> std::result::Result<Option<ChatStreamChunk>, LlmError> {
    if let Some(msg) = data.get("message")
        && let Some(content) = msg.get("content").and_then(|c| c.as_array())
    {
            for block in content {
                if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    // Initial tool_use in message_start means the model is using a cached tool
                    return Ok(Some(ChatStreamChunk {
                        content_delta: None,
                        reasoning_delta: None,
                        tool_call_delta: Some(ToolCallDelta {
                            index: 0,
                            id: Some(id.to_string()),
                            name: Some(name.to_string()),
                            arguments_delta: None,
                        }),
                        done: false,
                    }));
                }
        }
    }
    Ok(None)
}
