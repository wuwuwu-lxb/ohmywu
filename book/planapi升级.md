# LLM 适配器升级计划：cc-switch 级别兼容性

> 2026-05-14

## Context

当前 `llm-adapter` crate 只有 Ollama 和 OpenAI-compat 两个 provider，且存在 DeepSeek 400 的已知问题——根源是模型不支持 streaming+tools 组合，但没有 fallback。参考 cc-switch 的方案（协议格式转换 + 错误分类 + 能力探测 + 友好中文提示）进行全面升级。

目标：支持任意 LLM provider（OpenAI、Anthropic、DeepSeek、Gemini、Ollama 以及各类国产模型），并在模型不支持工具调用时优雅降级。

---

## 设计原则

- **增量升级**——不改现有结构，加新模块
- **内部规范格式**——Agent loop 始终使用统一的消息格式，adapter 负责格式转换
- **快速失败**——健康检查先于对话循环，失败直接返回友好错误
- **能力感知**——每个请求前探测模型能力（是否支持 tools / streaming），据此调整请求

---

## 架构

```
Agent Loop (统一消息格式)
    ↓
create_provider(config) → 根据 api_format 选择 adapter
    ↓
Adapter (格式转换 + HTTP 通信)
    ├── AnthropicAdapter    → api/anthropic: Messages API (/v1/messages)
    ├── OpenAiChatAdapter   → api/openai: Chat Completions (/v1/chat/completions)
    ├── GeminiAdapter       → api/gemini: streamGenerateContent
    └── OllamaAdapter       → api/ollama: /api/chat (保持现有)
    ↓
错误分类 LlmError → 友好中文消息
```

---

## 改动清单

### 1. 新增 `error.rs` — 分类错误类型

替代当前到处用的 `String`，按 HTTP status 和错误体内容分类：

| 错误 | 触发条件 | 中文提示 |
|------|---------|---------|
| `Authentication` | HTTP 401 | 认证失败，请检查 API Key |
| `ModelNotFound` | HTTP 404 / body 含 "not found" | 模型不存在，请检查模型名称 |
| `BadRequest` | HTTP 400 | 请求格式错误 |
| `Incompatible` | body 含 "tool" / "function" 相关拒绝 | 该模型不支持工具调用，将使用纯文本模式 |
| `RateLimited` | HTTP 429 | 请求频率限制，请稍后重试 |
| `Timeout` | 连接超时 | 连接超时，请检查网络或端点地址 |
| `Connection` | 连接被拒绝 / DNS 失败 | 无法连接到服务，请检查端点地址 |
| `ServerError` | HTTP 5xx | 服务端错误，请稍后重试 |
| `Protocol` | 响应解析失败 | 响应解析异常，协议不兼容 |

```rust
pub enum LlmError {
    Connection(String),
    Authentication,
    ModelNotFound,
    BadRequest(String),
    Incompatible(String),
    RateLimited(Option<i64>),
    ServerError(u16),
    Timeout,
    Protocol(String),
}

impl LlmError {
    pub fn user_friendly(&self) -> &str;
    pub fn from_http_status(status: u16, body: &str, request_had_tools: bool) -> Self;
}
```

### 2. 新增 `provider.rs` — Provider 元数据注册表

内置 provider 列表，前端据此渲染选择界面：

```rust
pub struct ProviderMetadata {
    pub id: String,              // "deepseek", "openai" ...
    pub name: String,            // "DeepSeek", "OpenAI" ...
    pub api_format: ApiFormat,   // 自动选择对应 adapter
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub default_model: String,
    pub supports_tools: bool,
    pub website_url: Option<String>,
}

pub fn builtin_providers() -> Vec<ProviderMetadata>;
```

参考 cc-switch 的 `DEFAULT_PROVIDER_ICONS`：

| id | name | api_format | 图标色 |
|----|------|-----------|-------|
| `openai` | OpenAI | openai_chat | #00A67E |
| `anthropic` | Anthropic | anthropic | #D4915D |
| `deepseek` | DeepSeek | openai_chat | #1E88E5 |
| `gemini` | Google Gemini | gemini | #4285F4 |
| `ollama` | Ollama | ollama | #000000 |
| `moonshot` | Moonshot | openai_chat | #6366F1 |
| `zhipu` | 智谱 | openai_chat | #0F62FE |
| `qwen` | 通义千问 | openai_chat | #FF6A00 |
| `minimax` | MiniMax | openai_chat | #FF6B6B |

### 3. 新增 `format/` 模块 — 格式转换

轻量级转换函数，不做 trait 抽象，保持简单：

```rust
pub enum ApiFormat {
    Anthropic,
    OpenAiChat,
    OpenAiResponses,
    Gemini,
    Ollama,
}

// format/anthropic.rs — 内部格式 ↔ Anthropic Messages API
pub fn build_anthropic_request(...) -> Value;
pub fn parse_anthropic_response(...) -> ChatResponse;
pub fn parse_anthropic_stream_chunk(...) -> Option<ChatStreamChunk>;

// format/openai_chat.rs — 内部格式 ↔ OpenAI Chat Completions
pub fn build_openai_chat_request(...) -> Value;
pub fn parse_openai_chat_response(...) -> ChatResponse;
pub fn parse_openai_chat_stream_chunk(...) -> Option<ChatStreamChunk>;

// format/gemini.rs — 内部格式 ↔ Gemini
pub fn build_gemini_request(...) -> Value;
pub fn parse_gemini_response(...) -> ChatResponse;
pub fn parse_gemini_stream_chunk(...) -> Option<ChatStreamChunk>;
```

参考 cc-switch: `transform.rs` (anthropic_to_openai)、`transform_gemini.rs` (anthropic_to_gemini)、`transform_responses.rs` (anthropic_to_responses)。

### 4. 新增 adapter 实现

**AnthropicAdapter** (`adapters/anthropic.rs`)
- POST `/v1/messages`
- Header: `anthropic-version: 2023-06-01`, `x-api-key`
- SSE streaming, 支持 extended thinking
- 参考 cc-switch: `ClaudeAdapter`

**GeminiAdapter** (`adapters/gemini.rs`)
- POST `/v1/models/{model}:streamGenerateContent?alt=sse`
- API key via `x-goog-api-key`
- SSE streaming

**OpenAiChatAdapter** (`adapters/openai_chat.rs`) — 重写现有 `openai_compat.rs`
- 逻辑不变，改用 `LlmError` 错误类型
- 收到 400 时解析 body 检测工具调用不兼容
- 端点 URL 拼接逻辑保留

**OllamaAdapter** (`ollama.rs`) — 保留现有，基本不变

### 5. 扩展 `LlmProvider` trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    // 现有方法（错误类型从 String 改为 LlmError）
    async fn chat(&self, messages: &[ChatMessage], tools: &[ToolDef])
        -> std::result::Result<ChatResponse, LlmError>;
    async fn chat_stream(&self, messages: &[ChatMessage], tools: &[ToolDef])
        -> std::result::Result<Pin<Box<dyn Stream<Item = std::result::Result<ChatStreamChunk, LlmError>> + Send>>, LlmError>;

    // 新增
    async fn health_check(&self) -> std::result::Result<HealthStatus, LlmError>;
    async fn probe_capabilities(&self) -> ProviderCapabilities;
}

pub struct ProviderCapabilities {
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_streaming_with_tools: bool,  // DeepSeek 问题所在
}

pub enum HealthStatus {
    Ok { model: String, latency_ms: u64 },
    Degraded { message: String },
}
```

### 6. 扩展 `LlmConfig`

```rust
pub struct LlmConfig {
    pub provider_type: String,     // "openai" | "anthropic" | "deepseek" | "custom"
    pub api_format: String,        // "openai_chat" | "anthropic" | "gemini" | "ollama"
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
}
```

`api_format` 从 `provider_type` 自动推断（通过 `ProviderMetadata`），支持手动覆盖。

### 7. 修改 `agent.rs` — 能力探测 + fallback

```rust
pub async fn agent_loop(state, session_id, user_message, llm_config, app_handle) -> Result {
    let provider = create_provider(llm_config)?;

    // Step 1: 健康检查
    provider.health_check().await.map_err(|e| e.user_friendly())?;

    // Step 2: 能力探测
    let caps = provider.probe_capabilities().await;
    let tools = if caps.supports_streaming_with_tools {
        capabilities_as_tools(state)
    } else {
        vec![]  // 不支持 tools → 纯文本模式
    };

    // Step 3: 对话循环
    for _ in 0..MAX_ITERATIONS {
        let response = provider.chat_stream(&messages, &tools).await?;
        if response.tool_calls.is_none() || tools.is_empty() {
            return Ok(AgentResponse { content, ... });
        }
        // ... 正常工具调用流程
    }
}
```

### 8. 更新 `test_llm_connection` 命令

```rust
#[tauri::command]
async fn test_llm_connection(state, config: LlmConfig) -> ConnectionTestResult {
    match create_provider(&config)?.health_check().await {
        Ok(status) => ConnectionTestResult { success: true, message: "连接成功", ... },
        Err(e) => ConnectionTestResult { success: false, message: e.user_friendly(), ... },
    }
}
```

---

## 分步实施

| Step | 内容 | 文件 |
|------|------|------|
| 1 | 错误类型系统 | `error.rs` |
| 2 | Provider 元数据 + `builtin_providers()` | `provider.rs` |
| 3 | 格式转换函数 | `format/{anthropic,openai_chat,gemini}.rs` |
| 4 | AnthropicAdapter + GeminiAdapter | `adapters/anthropic.rs`, `adapters/gemini.rs` |
| 5 | 重写 OpenAiChatAdapter（用 LlmError） | `adapters/openai_chat.rs` |
| 6 | 扩展 LlmProvider trait + create_provider | `lib.rs` |
| 7 | 扩展 LlmConfig + 能力探测 + fallback | `config.rs`, `agent.rs` |
| 8 | 更新 Tauri 命令 + 前端 | `lib.rs`, `SettingsView.vue` |

---

## 涉及文件

```
crates/llm-adapter/
├── Cargo.toml              # + thiserror
├── src/
│   ├── lib.rs              # 重构：LlmProvider + create_provider
│   ├── types.rs            # 不变
│   ├── error.rs            # NEW
│   ├── provider.rs         # NEW
│   ├── format/
│   │   ├── mod.rs          # NEW
│   │   ├── anthropic.rs    # NEW
│   │   ├── openai_chat.rs  # NEW
│   │   └── gemini.rs       # NEW
│   ├── adapters/
│   │   ├── mod.rs          # NEW
│   │   ├── openai_chat.rs  # 重写 openai_compat.rs
│   │   ├── anthropic.rs    # NEW
│   │   └── gemini.rs       # NEW
│   └── ollama.rs           # 保留

src-tauri/
├── src/
│   ├── config.rs           # 扩展 LlmConfig
│   ├── agent.rs            # 能力探测 + fallback
│   └── lib.rs              # 更新 send_message / test_llm_connection

src/
└── views/
    └── SettingsView.vue     # 更新 provider 选择 UI
```

---

## 验证方法

1. `cargo build` 零 warning 零 error
2. 本地 Ollama 连接正常（现有功能不退化）
3. 配置无效 endpoint → 显示"无法连接到服务，请检查端点地址"
4. 配置有效 endpoint + 错误 API key → 显示"认证失败，请检查 API Key"
5. 配置模型不存在 → 显示"模型不存在，请检查模型名称"
6. DeepSeek 不支持 tools → 能自动降级为纯文本对话，不抛 400
7. 前端 Provider 下拉列表完整（含图标和颜色）
8. 测试连接按钮显示分类后消息
