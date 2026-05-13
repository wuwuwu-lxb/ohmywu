pub mod types;
pub mod ollama;
pub mod openai_compat;

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use types::{ChatMessage, ChatResponse, ChatStreamChunk, ToolDef};

/// Result type for LLM operations.
pub type Result<T> = std::result::Result<T, String>;

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
}

/// Configuration for an LLM provider.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider_type: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
}

/// Create an LLM provider from configuration.
pub fn create_provider(config: &LlmConfig) -> Result<Box<dyn LlmProvider>> {
    match config.provider_type.as_str() {
        "ollama" => {
            let endpoint = if config.endpoint.is_empty() {
                "http://localhost:11434".to_string()
            } else {
                config.endpoint.clone()
            };
            Ok(Box::new(ollama::OllamaProvider::new(
                endpoint,
                config.model.clone(),
            )))
        }
        "openai_compatible" => {
            let api_key = config
                .api_key
                .clone()
                .ok_or_else(|| "API key required for OpenAI-compatible provider".to_string())?;
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(
                config.endpoint.clone(),
                config.model.clone(),
                api_key,
            )))
        }
        other => Err(format!("Unknown provider type: {}", other)),
    }
}
