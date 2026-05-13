use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::types::*;
use crate::{LlmProvider, Result};

/// OpenAI-compatible provider — works with OpenAI, Anthropic proxies, Groq, etc.
pub struct OpenAiCompatProvider {
    endpoint: String,
    model: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiCompatProvider {
    pub fn new(endpoint: String, model: String, api_key: String) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model,
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn chat_url(&self) -> String {
        if self.endpoint.ends_with("/v1") {
            format!("{}/chat/completions", self.endpoint)
        } else if self.endpoint.contains("/v1/chat/completions") {
            self.endpoint.clone()
        } else {
            format!("{}/v1/chat/completions", self.endpoint)
        }
    }
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAiStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiStreamToolCall>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAiStreamToolCall {
    index: usize,
    #[serde(default)]
    id: Option<String>,
    function: Option<OpenAiStreamFunction>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAiStreamFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> Result<ChatResponse> {
        let body = OpenAiChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.to_vec())
            },
            stream: None,
        };

        let resp = self
            .client
            .post(&self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI HTTP {}: {}", status, text));
        }

        let data: OpenAiChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("OpenAI parse: {}", e))?;

        let choice = data
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| "No choices in response".to_string())?;

        Ok(ChatResponse {
            role: choice.message.role,
            content: choice.message.content,
            tool_calls: choice.message.tool_calls,
        })
    }

    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>> {
        let body = OpenAiChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.to_vec())
            },
            stream: Some(true),
        };

        let resp = self
            .client
            .post(&self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI stream request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI HTTP {}: {}", status, text));
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
                            if line.is_empty() || line == "data: [DONE]" {
                                continue;
                            }
                            let json_str = line.strip_prefix("data: ").unwrap_or(line);
                            if let Ok(chunk) =
                                serde_json::from_str::<OpenAiStreamChunk>(json_str)
                            {
                                if let Some(choice) = chunk.choices.into_iter().next() {
                                    let content_delta = choice.delta.content;
                                    let done = choice.finish_reason.as_deref() == Some("stop");
                                    chunks.push(Ok(ChatStreamChunk {
                                        content_delta,
                                        tool_call_delta: None,
                                        done,
                                    }));
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
