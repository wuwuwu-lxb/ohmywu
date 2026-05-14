pub mod types;
pub mod error;
pub mod provider;
pub mod format;
pub mod adapters;

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

pub use error::LlmError;
pub use provider::{
    ApiFormat, HealthStatus, LlmConfig, ProviderCapabilities, ProviderMetadata,
};

use types::{ChatMessage, ChatResponse, ChatStreamChunk, ToolDef};

/// Convenience alias using LlmError.
pub type Result<T> = std::result::Result<T, LlmError>;

/// Trait for LLM providers.
/// Both local Ollama and cloud APIs implement this.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request and return the full response.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> Result<ChatResponse>;

    /// Send a chat completion request and return a stream of chunks.
    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>>;

    /// Minimal health check — sends a small request to verify the provider is reachable.
    async fn health_check(&self) -> std::result::Result<HealthStatus, LlmError>;

    /// Probe what this provider can do (tools, streaming) by actually testing.
    async fn probe_capabilities(&self) -> ProviderCapabilities;
}

/// Create an LLM provider from configuration.
pub fn create_provider(config: &LlmConfig) -> Result<Box<dyn LlmProvider>> {
    match config.effective_api_format() {
        provider::ApiFormat::Ollama => {
            let endpoint = if config.endpoint.is_empty() {
                "http://localhost:11434".to_string()
            } else {
                config.endpoint.clone()
            };
            Ok(Box::new(adapters::ollama::OllamaProvider::new(
                endpoint,
                config.model.clone(),
            )))
        }
        provider::ApiFormat::OpenAiChat | provider::ApiFormat::OpenAiResponses => {
            let api_key = config
                .api_key
                .clone()
                .ok_or(LlmError::Authentication)?;
            Ok(Box::new(adapters::openai_chat::OpenAiChatProvider::new(
                config.endpoint.clone(),
                config.model.clone(),
                api_key,
            )))
        }
        provider::ApiFormat::Anthropic => {
            let api_key = config
                .api_key
                .clone()
                .ok_or(LlmError::Authentication)?;
            Ok(Box::new(adapters::anthropic::AnthropicProvider::new(
                config.endpoint.clone(),
                config.model.clone(),
                api_key,
            )))
        }
        provider::ApiFormat::Gemini => {
            let api_key = config
                .api_key
                .clone()
                .ok_or(LlmError::Authentication)?;
            Ok(Box::new(adapters::gemini::GeminiProvider::new(
                config.endpoint.clone(),
                config.model.clone(),
                api_key,
            )))
        }
    }
}
