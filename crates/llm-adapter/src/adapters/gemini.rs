use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

use crate::error::LlmError;
use crate::format::gemini;
use crate::types::*;
use crate::{HealthStatus, LlmProvider, ProviderCapabilities};

/// Google Gemini API provider.
pub struct GeminiProvider {
    endpoint: String,
    model: String,
    api_key: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(endpoint: String, model: String, api_key: String) -> Self {
        let endpoint = endpoint.trim_end_matches('/').to_string();
        let endpoint = if endpoint.contains("googleapis.com") {
            endpoint
        } else {
            "https://generativelanguage.googleapis.com".to_string()
        };
        Self {
            endpoint,
            model,
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn request(
        &self,
        url_path: &str,
        body: &serde_json::Value,
    ) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{}", self.endpoint, url_path))
            .query(&[("key", &self.api_key)])
            .header("Content-Type", "application/json")
            .json(body)
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> std::result::Result<ChatResponse, LlmError> {
        let (_url_path, body) = gemini::build_request(
            &self.model, None, messages, tools, false,
        );

        let resp = self
            .request(&_url_path, &body)
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

        gemini::parse_response(&data)
    }

    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> std::result::Result<
        Pin<Box<dyn Stream<Item = std::result::Result<ChatStreamChunk, LlmError>> + Send>>,
        LlmError,
    > {
        let (_url_path, body) = gemini::build_request(
            &self.model, None, messages, tools, true,
        );

        let resp = self
            .request(&_url_path, &body)
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
                        match gemini::parse_stream_line(line) {
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
        let (_url_path, body) = gemini::build_request(
            &self.model,
            None,
            &[ChatMessage::user("ping")],
            &[],
            false,
        );

        let resp = self.request(&_url_path, &body).send().await.map_err(|e| LlmError::from_reqwest_error(&e))?;

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
        let ok = self.health_check().await.is_ok();
        // Gemini supports tools — just verify connectivity
        ProviderCapabilities {
            supports_tools: ok,
            supports_streaming: ok,
            supports_streaming_with_tools: ok,
        }
    }
}
