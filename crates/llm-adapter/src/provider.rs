use serde::{Deserialize, Serialize};

/// Health status returned by health_check.
#[derive(Debug, Clone)]
pub enum HealthStatus {
    Ok {
        model: String,
        latency_ms: u64,
    },
}

/// Provider capabilities reported by probe_capabilities.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_streaming_with_tools: bool,
}

/// API format enum — determines which adapter and request format to use.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ApiFormat {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "openai_chat")]
    OpenAiChat,
    #[serde(rename = "openai_responses")]
    OpenAiResponses,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "ollama")]
    Ollama,
}

impl ApiFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAiChat => "openai_chat",
            Self::OpenAiResponses => "openai_responses",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
        }
    }
}

/// Built-in provider metadata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMetadata {
    pub id: &'static str,
    pub name: &'static str,
    pub api_format: ApiFormat,
    pub icon: Option<&'static str>,
    pub icon_color: Option<&'static str>,
    pub default_model: &'static str,
    pub supports_tools: bool,
    pub website_url: Option<&'static str>,
}

/// All built-in providers.
pub fn builtin_providers() -> &'static [ProviderMetadata] {
    BUILTIN_PROVIDERS
}

const BUILTIN_PROVIDERS: &[ProviderMetadata] = &[
    ProviderMetadata {
        id: "openai",
        name: "OpenAI",
        api_format: ApiFormat::OpenAiChat,
        icon: Some("openai"),
        icon_color: Some("#00A67E"),
        default_model: "gpt-4o",
        supports_tools: true,
        website_url: Some("https://platform.openai.com"),
    },
    ProviderMetadata {
        id: "anthropic",
        name: "Anthropic",
        api_format: ApiFormat::Anthropic,
        icon: Some("anthropic"),
        icon_color: Some("#D4915D"),
        default_model: "claude-sonnet-4-20250514",
        supports_tools: true,
        website_url: Some("https://console.anthropic.com"),
    },
    ProviderMetadata {
        id: "deepseek",
        name: "DeepSeek",
        api_format: ApiFormat::OpenAiChat,
        icon: Some("deepseek"),
        icon_color: Some("#1E88E5"),
        default_model: "deepseek-chat",
        supports_tools: true,
        website_url: Some("https://platform.deepseek.com"),
    },
    ProviderMetadata {
        id: "gemini",
        name: "Google Gemini",
        api_format: ApiFormat::Gemini,
        icon: Some("gemini"),
        icon_color: Some("#4285F4"),
        default_model: "gemini-2.5-flash",
        supports_tools: true,
        website_url: Some("https://ai.google.dev"),
    },
    ProviderMetadata {
        id: "ollama",
        name: "Ollama",
        api_format: ApiFormat::Ollama,
        icon: Some("ollama"),
        icon_color: Some("#000000"),
        default_model: "qwen2.5",
        supports_tools: true,
        website_url: None,
    },
    ProviderMetadata {
        id: "moonshot",
        name: "Moonshot",
        api_format: ApiFormat::OpenAiChat,
        icon: Some("moonshot"),
        icon_color: Some("#6366F1"),
        default_model: "moonshot-v1-8k",
        supports_tools: true,
        website_url: Some("https://platform.moonshot.cn"),
    },
    ProviderMetadata {
        id: "zhipu",
        name: "智谱",
        api_format: ApiFormat::OpenAiChat,
        icon: Some("zhipu"),
        icon_color: Some("#0F62FE"),
        default_model: "glm-4-plus",
        supports_tools: true,
        website_url: Some("https://open.bigmodel.cn"),
    },
    ProviderMetadata {
        id: "qwen",
        name: "通义千问",
        api_format: ApiFormat::OpenAiChat,
        icon: Some("qwen"),
        icon_color: Some("#FF6A00"),
        default_model: "qwen-plus",
        supports_tools: true,
        website_url: Some("https://help.aliyun.com/zh/model-studio"),
    },
    ProviderMetadata {
        id: "minimax",
        name: "MiniMax",
        api_format: ApiFormat::OpenAiChat,
        icon: Some("minimax"),
        icon_color: Some("#FF6B6B"),
        default_model: "abab6.5s-chat",
        supports_tools: true,
        website_url: Some("https://platform.minimaxi.com"),
    },
];

/// Infer API format from a provider type string.
pub fn infer_api_format(provider_type: &str) -> ApiFormat {
    match provider_type {
        "anthropic" => ApiFormat::Anthropic,
        "gemini" => ApiFormat::Gemini,
        "ollama" => ApiFormat::Ollama,
        _ => ApiFormat::OpenAiChat, // default for openai, deepseek, moonshot, etc.
    }
}

/// Find a built-in provider by id.
pub fn find_provider(id: &str) -> Option<&'static ProviderMetadata> {
    BUILTIN_PROVIDERS.iter().find(|p| p.id == id)
}

/// Configuration for an LLM provider (serialized to config.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider_type")]
    pub provider_type: String,
    #[serde(default = "default_api_format_str")]
    pub api_format: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default = "default_model_name")]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

fn default_provider_type() -> String {
    "ollama".into()
}
fn default_api_format_str() -> String {
    "ollama".into()
}
fn default_model_name() -> String {
    "qwen2.5".into()
}

impl LlmConfig {
    pub fn new(provider_type: &str, endpoint: &str, model: &str, api_key: Option<String>) -> Self {
        let api_format = infer_api_format(provider_type).as_str().to_string();
        Self {
            provider_type: provider_type.to_string(),
            api_format,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key,
            max_tokens: None,
        }
    }

    /// Resolve the effective API format, inferring from provider_type if not explicitly set.
    pub fn effective_api_format(&self) -> ApiFormat {
        serde_json::from_value(serde_json::json!(self.api_format))
            .unwrap_or_else(|_| infer_api_format(&self.provider_type))
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider_type: default_provider_type(),
            api_format: default_api_format_str(),
            endpoint: String::new(),
            model: default_model_name(),
            api_key: None,
            max_tokens: None,
        }
    }
}
