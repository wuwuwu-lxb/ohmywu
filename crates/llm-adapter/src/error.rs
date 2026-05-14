use std::time::Duration;

/// Classified LLM error.
#[derive(Debug, Clone)]
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
    pub fn user_friendly(&self) -> &str {
        match self {
            Self::Connection(_) => "无法连接到服务，请检查端点地址",
            Self::Authentication => "认证失败，请检查 API Key",
            Self::ModelNotFound => "模型不存在，请检查模型名称",
            Self::BadRequest(_) => "请求格式错误，请检查参数",
            Self::Incompatible(_) => "该模型不支持工具调用，将使用纯文本模式",
            Self::RateLimited(_) => "请求频率限制，请稍后重试",
            Self::ServerError(_) => "服务端错误，请稍后重试",
            Self::Timeout => "连接超时，请检查网络或端点地址",
            Self::Protocol(_) => "响应解析异常，协议可能不兼容",
        }
    }

    /// Classify an error from HTTP status and response body.
    /// `request_had_tools`: whether the failing request included tool definitions.
    pub fn from_http_status(status: u16, body: &str, request_had_tools: bool) -> Self {
        let body_lower = body.to_lowercase();
        match status {
            400 => {
                // Detect tool incompatibility
                if request_had_tools
                    && (body_lower.contains("tool")
                        || body_lower.contains("function")
                        || body_lower.contains("not supported")
                        || body_lower.contains("not allowed")
                        || body_lower.contains("invalid_request"))
                {
                    Self::Incompatible(body_lower)
                } else if body_lower.contains("not found")
                    || body_lower.contains("does not exist")
                {
                    Self::ModelNotFound
                } else {
                    Self::BadRequest(body_lower)
                }
            }
            401 | 403 => Self::Authentication,
            404 => {
                if body_lower.contains("model") {
                    Self::ModelNotFound
                } else {
                    Self::BadRequest(format!("endpoint not found: {}", body_lower))
                }
            }
            429 => Self::RateLimited(None),
            500..=599 => Self::ServerError(status),
            _ => Self::BadRequest(format!("HTTP {}: {}", status, body_lower)),
        }
    }

    pub fn from_reqwest_error(e: &reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout
        } else if e.is_connect() {
            Self::Connection(e.to_string())
        } else if let Some(status) = e.status() {
            // Can't read body from reqwest::Error directly when it's a transport error
            Self::ServerError(status.as_u16())
        } else {
            Self::Connection(e.to_string())
        }
    }

    pub fn from_timeout(_duration: Duration) -> Self {
        Self::Timeout
    }
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "Connection: {}", msg),
            Self::Authentication => write!(f, "Authentication failed"),
            Self::ModelNotFound => write!(f, "Model not found"),
            Self::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            Self::Incompatible(msg) => write!(f, "Incompatible: {}", msg),
            Self::RateLimited(retry_after) => {
                if let Some(secs) = retry_after {
                    write!(f, "Rate limited, retry after {}s", secs)
                } else {
                    write!(f, "Rate limited")
                }
            }
            Self::ServerError(code) => write!(f, "Server error HTTP {}", code),
            Self::Timeout => write!(f, "Request timed out"),
            Self::Protocol(msg) => write!(f, "Protocol error: {}", msg),
        }
    }
}
