use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::types::*;
use crate::{LlmProvider, Result};

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
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDef>>,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
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
    ) -> Result<ChatResponse> {
        let ollama_msgs: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.clone(),
            })
            .collect();

        let body = OllamaChatRequest {
            model: self.model.clone(),
            messages: ollama_msgs,
            stream: false,
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.to_vec())
            },
        };

        let url = format!("{}/api/chat", self.endpoint);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama HTTP {}: {}", status, text));
        }

        let data: OllamaChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("Ollama parse: {}", e))?;

        let content = if data.message.content.is_empty() {
            None
        } else {
            Some(data.message.content)
        };

        Ok(ChatResponse {
            role: data.message.role,
            content,
            tool_calls: data.message.tool_calls,
        })
    }

    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>> {
        let ollama_msgs: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.clone(),
            })
            .collect();

        let body = OllamaChatRequest {
            model: self.model.clone(),
            messages: ollama_msgs,
            stream: true,
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.to_vec())
            },
        };

        let url = format!("{}/api/chat", self.endpoint);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama stream request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama HTTP {}: {}", status, text));
        }

        use futures::StreamExt;
        let stream = resp
            .bytes_stream()
            .map(|item| {
                match item {
                    Err(e) => {
                        vec![Err(format!("Stream error: {}", e))]
                    }
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let mut chunks = Vec::new();
                        for line in text.lines() {
                            let line = line.trim();
                            if line.is_empty() {
                                continue;
                            }
                            match serde_json::from_str::<OllamaStreamChunk>(line) {
                                Ok(chunk) => {
                                    let content_delta = if chunk.message.content.is_empty() {
                                        None
                                    } else {
                                        Some(chunk.message.content)
                                    };
                                    let tool_delta = chunk.message.tool_calls.map(|tc| {
                                        ToolCallDelta {
                                            index: 0,
                                            id: None,
                                            name: None,
                                            arguments_delta: Some(
                                                serde_json::to_string(&tc).unwrap_or_default(),
                                            ),
                                        }
                                    });
                                    chunks.push(Ok(ChatStreamChunk {
                                        content_delta,
                                        tool_call_delta: tool_delta,
                                        done: chunk.done,
                                    }));
                                }
                                Err(e) => {
                                    chunks.push(Err(format!("Parse stream chunk: {}", e)));
                                }
                            }
                        }
                        chunks
                    }
                }
            })
            .map(futures::stream::iter)
            .flatten();

        Ok(Box::pin(stream))
    }
}
