use async_trait::async_trait;
use futures::Stream;
use serde::Deserialize;
use std::pin::Pin;

use crate::adapters::buffered_line_stream;
use crate::error::LlmError;
use crate::format::openai_chat;
use crate::types::*;
use crate::LlmProvider;

/// Ollama provider — connects to a local Ollama instance.
pub struct OllamaProvider {
    endpoint: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.endpoint)
    }
}

#[derive(Deserialize)]
struct OllamaStreamChunk {
    message: OllamaStreamMessage,
    done: bool,
}

#[derive(Deserialize)]
struct OllamaStreamMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> std::result::Result<ChatResponse, LlmError> {
        // Use the openai_chat format to build the body (Ollama is compatible)
        let body = openai_chat::build_request(&self.model, messages, tools, false);

        let resp = self
            .client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::from_http_status(status, &text, !tools.is_empty()));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::Protocol(e.to_string()))?;

        // Ollama /api/chat returns a different format than OpenAI
        // { model, created_at, message: { role, content, tool_calls }, done }
        let msg = data
            .get("message")
            .ok_or_else(|| LlmError::Protocol("no message in response".to_string()))?;

        let role = msg
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("assistant")
            .to_string();

        let content = msg
            .get("content")
            .and_then(|c| c.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let tool_calls = msg.get("tool_calls").and_then(|tc| tc.as_array()).map(
            |tc_array| {
                tc_array
                    .iter()
                    .filter_map(|tc| {
                        let name = tc
                            .get("name")
                            .or_else(|| tc.get("function").and_then(|f| f.get("name")))
                            .and_then(|n| n.as_str())?;
                        let args = tc
                            .get("arguments")
                            .or_else(|| tc.get("function").and_then(|f| f.get("arguments")));
                        Some(ToolCall {
                            id: format!("ollama_{}", name),
                            call_type: "function".to_string(),
                            function: ToolCallFunction {
                                name: name.to_string(),
                                arguments: args
                                    .map(|a| serde_json::to_string(a).unwrap_or_default())
                                    .unwrap_or_default(),
                            },
                        })
                    })
                    .collect()
            },
        );

        Ok(ChatResponse {
            role,
            content,
            reasoning_content: None,
            tool_calls: tool_calls.filter(|tc: &Vec<ToolCall>| !tc.is_empty()),
        })
    }

    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> std::result::Result<
        Pin<Box<dyn Stream<Item = std::result::Result<ChatStreamChunk, LlmError>> + Send>>,
        LlmError,
    > {
        let body = openai_chat::build_request(&self.model, messages, tools, true);

        let resp = self
            .client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::from_http_status(status, &text, !tools.is_empty()));
        }

        Ok(buffered_line_stream(resp, |line| {
            let line = line.trim();
            if line.is_empty() {
                return Ok(None);
            }
            let chunk: OllamaStreamChunk =
                serde_json::from_str(line).map_err(|e| LlmError::Protocol(e.to_string()))?;
            let content_delta = if chunk.message.content.is_empty() {
                None
            } else {
                Some(chunk.message.content)
            };
            let tool_call_delta = chunk
                .message
                .tool_calls
                .and_then(|mut tc| tc.drain(..).next())
                .map(|call| ToolCallDelta {
                    index: 0,
                    id: Some(call.id),
                    name: Some(call.function.name),
                    arguments_delta: Some(call.function.arguments),
                });
            Ok(Some(ChatStreamChunk {
                content_delta,
                reasoning_delta: None,
                tool_call_delta,
                done: chunk.done,
            }))
        }))
    }

    async fn health_check(&self) -> std::result::Result<crate::HealthStatus, LlmError> {
        let start = std::time::Instant::now();
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": "ping"}],
            "stream": false,
        });

        let resp = self
            .client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::from_http_status(status, &text, false));
        }

        let latency = start.elapsed().as_millis() as u64;
        Ok(crate::HealthStatus::Ok {
            model: self.model.clone(),
            latency_ms: latency,
        })
    }

    async fn probe_capabilities(&self) -> crate::ProviderCapabilities {
        // Ollama generally supports both tools and streaming
        // Try streaming first
        let stream_test = self
            .chat_stream(
                &[ChatMessage::user("ping")],
                &[],
            )
            .await;
        let supports_streaming = stream_test.is_ok();

        // Try tools
        let tool_test = self
            .chat(
                &[ChatMessage::user("use a tool")],
                &[ToolDef {
                    tool_type: "function".into(),
                    function: FunctionDef {
                        name: "test".into(),
                        description: "test".into(),
                        parameters: serde_json::json!({"type": "object", "properties": {}}),
                    },
                }],
            )
            .await;
        let supports_tools = tool_test.is_ok();

        crate::ProviderCapabilities {
            supports_tools,
            supports_streaming,
            supports_streaming_with_tools: supports_streaming && supports_tools,
        }
    }
}
