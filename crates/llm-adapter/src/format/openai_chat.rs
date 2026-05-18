use crate::error::LlmError;
use crate::types::*;
use serde_json::{json, Value};

/// Build an OpenAI Chat Completions API request body.
pub fn build_request(
    model: &str,
    messages: &[ChatMessage],
    tools: &[ToolDef],
    stream: bool,
) -> Value {
    let mut body = json!({
        "model": model,
        "stream": stream,
    });

    let openai_messages: Vec<Value> = messages
        .iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "agent" => "assistant",
                _ => m.role.as_str(),
            };

            if m.role == "tool" {
                json!({
                    "role": "tool",
                    "content": m.content,
                    "tool_call_id": m.tool_call_id,
                })
            } else if let Some(ref tcs) = m.tool_calls {
                let oai_tcs: Vec<Value> = tcs
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.function.name,
                                "arguments": tc.function.arguments,
                            }
                        })
                    })
                    .collect();
                let mut message = json!({
                    "role": "assistant",
                    "content": if m.content.is_empty() { None } else { Some(&m.content) },
                    "tool_calls": oai_tcs,
                });
                if m.reasoning_content.is_some() {
                    message["reasoning_content"] = json!(m.reasoning_content);
                }
                message
            } else {
                let mut message = json!({
                    "role": role,
                    "content": m.content,
                });
                if matches!(role, "assistant") && m.reasoning_content.is_some() {
                    message["reasoning_content"] = json!(m.reasoning_content);
                }
                message
            }
        })
        .collect();

    body["messages"] = json!(openai_messages);

    if !tools.is_empty() {
        let oai_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.function.name,
                        "description": t.function.description,
                        "parameters": t.function.parameters,
                    }
                })
            })
            .collect();
        body["tools"] = json!(oai_tools);
    }

    body
}

/// Parse a non-streaming OpenAI Chat Completions response.
pub fn parse_response(data: &serde_json::Value) -> std::result::Result<ChatResponse, LlmError> {
    let choices = data
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| LlmError::Protocol("no choices in response".to_string()))?;

    let choice = choices
        .first()
        .ok_or_else(|| LlmError::Protocol("empty choices".to_string()))?;

    let msg = choice
        .get("message")
        .ok_or_else(|| LlmError::Protocol("no message in choice".to_string()))?;

    let role = msg
        .get("role")
        .and_then(|r| r.as_str())
        .unwrap_or("assistant")
        .to_string();

    let content = msg.get("content").and_then(|c| match c {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    });
    let reasoning_content = msg
        .get("reasoning_content")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let tool_calls = msg.get("tool_calls").and_then(|tc| tc.as_array()).map(
        |tc_array| {
            tc_array
                .iter()
                .filter_map(|tc| {
                    let id = tc.get("id")?.as_str()?;
                    let func = tc.get("function")?;
                    let name = func.get("name")?.as_str()?;
                    let args = func.get("arguments")?.as_str()?;
                    Some(ToolCall {
                        id: id.to_string(),
                        call_type: "function".to_string(),
                        function: ToolCallFunction {
                            name: name.to_string(),
                            arguments: args.to_string(),
                        },
                    })
                })
                .collect()
        },
    );

    Ok(ChatResponse {
        role,
        content,
        reasoning_content,
        tool_calls: tool_calls.filter(|tc: &Vec<ToolCall>| !tc.is_empty()),
    })
}

/// Parse a single SSE line from OpenAI streaming response.
pub fn parse_stream_line(line: &str) -> std::result::Result<Option<ChatStreamChunk>, LlmError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    let json_str = if let Some(data) = line.strip_prefix("data: ") {
        data.trim()
    } else {
        return Ok(None);
    };

    if json_str == "[DONE]" {
        return Ok(Some(ChatStreamChunk {
            content_delta: None,
            reasoning_delta: None,
            tool_call_delta: None,
            done: true,
        }));
    }

    let data: Value =
        serde_json::from_str(json_str).map_err(|e| LlmError::Protocol(e.to_string()))?;

    let choices = match data.get("choices").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return Ok(None),
    };

    let choice = match choices.first() {
        Some(c) => c,
        None => return Ok(None),
    };

    let delta = match choice.get("delta") {
        Some(d) => d,
        None => return Ok(None),
    };

    let content_delta = delta
        .get("content")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let reasoning_delta = delta
        .get("reasoning_content")
        .or_else(|| delta.get("reasoning"))
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let tool_call_delta = delta.get("tool_calls").and_then(|tc| tc.as_array()).and_then(
        |tc_arr| {
            tc_arr.first().map(|tc| {
                let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                let id = tc.get("id").and_then(|i| i.as_str()).map(|s| s.to_string());
                let name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
                let arguments_delta = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string());
                ToolCallDelta {
                    index,
                    id,
                    name,
                    arguments_delta,
                }
            })
        },
    );

    let finish_reason = choice
        .get("finish_reason")
        .and_then(|f| f.as_str())
        .map(|s| s.to_string());

    let done = finish_reason.as_deref() == Some("stop")
        || finish_reason.as_deref() == Some("tool_calls");

    Ok(Some(ChatStreamChunk {
        content_delta,
        reasoning_delta,
        tool_call_delta,
        done,
    }))
}

#[cfg(test)]
mod tests {
    use super::{build_request, parse_response, parse_stream_line};
    use crate::types::{ChatMessage, ToolCall, ToolCallFunction};

    #[test]
    fn includes_reasoning_content_in_assistant_messages() {
        let messages = vec![
            ChatMessage {
                role: "assistant".into(),
                content: "done".into(),
                reasoning_content: Some("chain".into()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                reasoning_content: Some("call planning".into()),
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".into(),
                    call_type: "function".into(),
                    function: ToolCallFunction {
                        name: "read".into(),
                        arguments: "{\"path\":\"/tmp/a\"}".into(),
                    },
                }]),
                tool_call_id: None,
            },
        ];

        let body = build_request("deepseek-v4-flash", &messages, &[], false);
        let request_messages = body["messages"].as_array().unwrap();
        assert_eq!(request_messages[0]["reasoning_content"], "chain");
        assert_eq!(request_messages[1]["reasoning_content"], "call planning");
    }

    #[test]
    fn parses_reasoning_content_from_response() {
        let data = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "hello",
                    "reasoning_content": "internal"
                }
            }]
        });

        let response = parse_response(&data).unwrap();
        assert_eq!(response.reasoning_content.as_deref(), Some("internal"));
    }

    #[test]
    fn parses_reasoning_delta_from_stream() {
        let chunk = parse_stream_line(
            r#"data: {"choices":[{"delta":{"reasoning_content":"step 1"},"finish_reason":null}]}"#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(chunk.reasoning_delta.as_deref(), Some("step 1"));
        assert!(!chunk.done);
    }
}
