use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

use crate::error::LlmError;
use crate::format::anthropic;
use crate::types::*;
use crate::{HealthStatus, LlmProvider, ProviderCapabilities};

/// Anthropic Messages API provider.
pub struct AnthropicProvider {
    endpoint: String,
    model: String,
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(endpoint: String, model: String, api_key: String) -> Self {
        let endpoint = endpoint.trim_end_matches('/').to_string();
        let endpoint = if endpoint.ends_with("/v1") {
            endpoint
        } else {
            format!("{}/v1", endpoint)
        };
        Self {
            endpoint,
            model,
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn messages_url(&self) -> String {
        format!("{}/messages", self.endpoint)
    }

    fn request(&self, body: &serde_json::Value) -> reqwest::RequestBuilder {
        self.client
            .post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "tools-2024-05-16")
            .header("Content-Type", "application/json")
            .json(body)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> std::result::Result<ChatResponse, LlmError> {
        let body = anthropic::build_request(
            &self.model,
            None,
            messages,
            tools,
            4096,
            false,
        );

        let resp = self
            .request(&body)
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

        anthropic::parse_response(&data)
    }

    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> std::result::Result<
        Pin<Box<dyn Stream<Item = std::result::Result<ChatStreamChunk, LlmError>> + Send>>,
        LlmError,
    > {
        let body = anthropic::build_request(
            &self.model,
            None,
            messages,
            tools,
            4096,
            true,
        );

        let resp = self
            .request(&body)
            .send()
            .await
            .map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::from_http_status(status, &text, !tools.is_empty()));
        }

        let stream = resp
            .bytes_stream()
            .map(|item| match item {
                Err(e) => vec![Err(LlmError::Connection(e.to_string()))],
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut chunks = Vec::new();
                    for line in text.lines() {
                        match anthropic::parse_stream_line(line) {
                            Ok(Some(chunk)) => chunks.push(Ok(chunk)),
                            Ok(None) => {}
                            Err(e) => chunks.push(Err(e)),
                        }
                    }
                    chunks
                }
            })
            .map(futures::stream::iter)
            .flatten();

        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> std::result::Result<HealthStatus, LlmError> {
        let start = std::time::Instant::now();
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "ping"}],
        });

        let resp = self.request(&body).send().await.map_err(|e| LlmError::from_reqwest_error(&e))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::from_http_status(status, &text, false));
        }

        let latency = start.elapsed().as_millis() as u64;
        Ok(HealthStatus::Ok {
            model: self.model.clone(),
            latency_ms: latency,
        })
    }

    async fn probe_capabilities(&self) -> ProviderCapabilities {
        // Anthropic supports everything. Verify connectivity.
        let ok = self.health_check().await.is_ok();
        ProviderCapabilities {
            supports_tools: ok,
            supports_streaming: ok,
            supports_streaming_with_tools: ok,
        }
    }
}
