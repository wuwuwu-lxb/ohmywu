use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::adapters::buffered_line_stream;
use crate::error::LlmError;
use crate::format::openai_chat;
use crate::types::*;
use crate::{HealthStatus, LlmProvider, ProviderCapabilities};

/// OpenAI-compatible provider — works with OpenAI, DeepSeek, Moonshot, etc.
pub struct OpenAiChatProvider {
    endpoint: String,
    model: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiChatProvider {
    pub fn new(endpoint: String, model: String, api_key: String) -> Self {
        Self {
            endpoint: Self::normalize_endpoint(endpoint),
            model,
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn normalize_endpoint(endpoint: String) -> String {
        let e = endpoint.trim_end_matches('/').to_string();
        if e.ends_with("/v1/chat/completions") || e.ends_with("/chat/completions") {
            e
        } else if e.ends_with("/v1") {
            format!("{}/chat/completions", e)
        } else {
            format!("{}/v1/chat/completions", e)
        }
    }

    fn request(
        &self,
        body: &serde_json::Value,
    ) -> reqwest::RequestBuilder {
        self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
    }
}

#[async_trait]
impl LlmProvider for OpenAiChatProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> std::result::Result<ChatResponse, LlmError> {
        let body = openai_chat::build_request(&self.model, messages, tools, false);

        let resp = self
            .request(&body)
            .send()
            .await
            .map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            eprintln!(
                "openai-compatible chat failed: status={} endpoint={} model={} request={} response={}",
                status,
                self.endpoint,
                self.model,
                body,
                text
            );
            return Err(LlmError::from_http_status(status, &text, !tools.is_empty()));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::Protocol(e.to_string()))?;

        openai_chat::parse_response(&data)
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
            .request(&body)
            .send()
            .await
            .map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            eprintln!(
                "openai-compatible chat_stream failed: status={} endpoint={} model={} request={} response={}",
                status,
                self.endpoint,
                self.model,
                body,
                text
            );
            return Err(LlmError::from_http_status(status, &text, !tools.is_empty()));
        }

        Ok(buffered_line_stream(resp, openai_chat::parse_stream_line))
    }

    async fn health_check(&self) -> std::result::Result<HealthStatus, LlmError> {
        let start = std::time::Instant::now();
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": "ping"}],
            "max_tokens": 1,
            "stream": false,
        });

        let resp = self.request(&body).send().await.map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let err = LlmError::from_http_status(status, &text, false);
            return Err(err);
        }

        let latency = start.elapsed().as_millis() as u64;
        Ok(HealthStatus::Ok {
            model: self.model.clone(),
            latency_ms: latency,
        })
    }

    async fn probe_capabilities(&self) -> ProviderCapabilities {
        // 1. Test basic chat
        let basic = self
            .chat(&[ChatMessage::user("hi")], &[])
            .await;
        if basic.is_err() {
            return ProviderCapabilities {
                supports_tools: false,
                supports_streaming: false,
                supports_streaming_with_tools: false,
            };
        }

        // 2. Test streaming
        let stream_test = self
            .chat_stream(&[ChatMessage::user("hi")], &[])
            .await;
        let supports_streaming = stream_test.is_ok();

        // 3. Test tools (non-streaming to be safe)
        let tool_test = self
            .chat(
                &[ChatMessage::user("say hello back, no tool needed")],
                &[ToolDef {
                    tool_type: "function".into(),
                    function: FunctionDef {
                        name: "say_hello".into(),
                        description: "say hello".into(),
                        parameters: serde_json::json!({"type": "object", "properties": {}}),
                    },
                }],
            )
            .await;
        let supports_tools = tool_test.is_ok();

        ProviderCapabilities {
            supports_tools,
            supports_streaming,
            supports_streaming_with_tools: supports_streaming && supports_tools,
        }
    }
}
