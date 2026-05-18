use crate::error::LlmError;
use crate::types::*;
use serde_json::{json, Value};

/// Build a Gemini API request body (streamGenerateContent).
pub fn build_request(
    model: &str,
    system_prompt: Option<&str>,
    messages: &[ChatMessage],
    tools: &[ToolDef],
    stream: bool,
) -> (String, Value) {
    // Gemini URL: POST /v1/models/{model}:streamGenerateContent?alt=sse
    let url = if stream {
        format!(
            "/v1/models/{}:streamGenerateContent?alt=sse",
            model
        )
    } else {
        format!("/v1/models/{}:generateContent", model)
    };

    let mut contents: Vec<Value> = Vec::new();

    // Filter out system messages — Gemini uses system_instruction field
    let chat_messages: Vec<&ChatMessage> = messages
        .iter()
        .filter(|m| m.role != "system")
        .collect();

    // Group messages into Gemini's request/response pairs
    for msg in &chat_messages {
        let role = match msg.role.as_str() {
            "assistant" => "model",
            "tool" => "function", // will be wrapped
            _ => "user",
        };

        let parts = if msg.role == "tool" {
            vec![json!({
                "functionResponse": {
                    "name": msg.tool_call_id.as_deref().unwrap_or("unknown"),
                    "response": {
                        "response": msg.content,
                    }
                }
            })]
        } else if let Some(ref tcs) = msg.tool_calls {
            let mut parts: Vec<Value> = if msg.content.is_empty() {
                vec![]
            } else {
                vec![json!({"text": msg.content})]
            };
            for tc in tcs {
                let args: Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                parts.push(json!({
                    "functionCall": {
                        "name": tc.function.name,
                        "args": args,
                    }
                }));
            }
            parts
        } else {
            vec![json!({"text": msg.content})]
        };

        contents.push(json!({
            "role": role,
            "parts": parts,
        }));
    }

    let mut body = json!({
        "contents": contents,
    });

    // System instruction
    if let Some(sys) = system_prompt {
        body["system_instruction"] = json!({
            "parts": [{"text": sys}]
        });
    }

    // Tools — Gemini uses "function_declarations" format
    if !tools.is_empty() {
        let func_decls: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.function.name,
                    "description": t.function.description,
                    "parameters": t.function.parameters,
                })
            })
            .collect();
        body["tools"] = json!([{
            "function_declarations": func_decls,
        }]);
    }

    (url, body)
}

/// Parse a non-streaming Gemini response.
pub fn parse_response(data: &serde_json::Value) -> std::result::Result<ChatResponse, LlmError> {
    let candidates = data
        .get("candidates")
        .and_then(|c| c.as_array())
        .ok_or_else(|| LlmError::Protocol("no candidates in gemini response".to_string()))?;

    let candidate = candidates
        .first()
        .ok_or_else(|| LlmError::Protocol("empty candidates".to_string()))?;

    let content = candidate
        .get("content")
        .ok_or_else(|| LlmError::Protocol("no content in candidate".to_string()))?;

    let parts = content
        .get("parts")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let text: String = parts
        .iter()
        .filter(|p| p.get("text").and_then(|t| t.as_str()).is_some())
        .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("");

    let content_text = if text.is_empty() { None } else { Some(text) };

    let tool_calls: Vec<ToolCall> = parts
        .iter()
        .filter(|p| p.get("functionCall").is_some())
        .filter_map(|p| {
            let fc = p.get("functionCall")?;
            let name = fc.get("name")?.as_str()?;
            let args = fc.get("args")?;
            Some(ToolCall {
                id: format!("fc_{}", name),
                call_type: "function".to_string(),
                function: ToolCallFunction {
                    name: name.to_string(),
                    arguments: serde_json::to_string(args).unwrap_or_default(),
                },
            })
        })
        .collect();

    Ok(ChatResponse {
        role: "assistant".to_string(),
        content: content_text,
        reasoning_content: None,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
    })
}

/// Parse a single SSE line from Gemini streaming response.
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

    let candidates = match data.get("candidates").and_then(|c| c.as_array()) {
        Some(c) if !c.is_empty() => c,
        _ => return Ok(None),
    };

    let candidate = &candidates[0];
    let content = match candidate.get("content") {
        Some(c) => c,
        None => return Ok(None),
    };

    let parts = match content.get("parts").and_then(|p| p.as_array()) {
        Some(p) => p,
        None => return Ok(None),
    };

    let mut content_delta: Option<String> = None;
    let mut tool_call_delta: Option<ToolCallDelta> = None;

    for part in parts {
        if let Some(text) = part.get("text").and_then(|t| t.as_str())
            && !text.is_empty()
        {
            content_delta = Some(text.to_string());
        }
        if let Some(fc) = part.get("functionCall") {
            let name = fc.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = fc.get("args");
            tool_call_delta = Some(ToolCallDelta {
                index: 0,
                id: None,
                name: Some(name.to_string()),
                arguments_delta: args.map(|a| serde_json::to_string(a).unwrap_or_default()),
            });
        }
    }

    // Check if finish reason indicates completion
    let done = candidate
        .get("finishReason")
        .and_then(|f| f.as_str())
        .map(|f| f == "STOP" || f == "MAX_TOKENS")
        .unwrap_or(false);

    Ok(Some(ChatStreamChunk {
        content_delta,
        reasoning_delta: None,
        tool_call_delta,
        done,
    }))
}
