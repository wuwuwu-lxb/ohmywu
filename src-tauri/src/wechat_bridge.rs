use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub const WECHAT_BRIDGE_BIND_ADDR: &str = "127.0.0.1:31918";
pub const WECHAT_BRIDGE_MESSAGE_PATH: &str = "/ohmywu/wechat-bridge/message";
pub const WECHAT_BRIDGE_QR_PATH: &str = "/ohmywu/wechat-bridge/qr";
pub const WECHAT_BRIDGE_HEADER_TOKEN: &str = "x-ohmywu-bridge-token";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatBridgeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_profile_id: Option<String>,
    #[serde(default = "default_bridge_token")]
    pub bridge_token: String,
    #[serde(default = "default_wechat_api_base_url")]
    pub api_base_url: String,
    #[serde(default = "default_wechat_bot_type")]
    pub bot_type: String,
    #[serde(default = "default_qr_poll_interval")]
    pub qr_poll_interval: u64,
    #[serde(default = "default_long_poll_timeout_ms")]
    pub long_poll_timeout_ms: u64,
    #[serde(default = "default_api_timeout_ms")]
    pub api_timeout_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sync_buf: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context_tokens: BTreeMap<String, String>,
}

impl Default for WechatBridgeConfig {
    fn default() -> Self {
        Self {
            agent_id: None,
            llm_profile_id: None,
            bridge_token: default_bridge_token(),
            api_base_url: default_wechat_api_base_url(),
            bot_type: default_wechat_bot_type(),
            qr_poll_interval: default_qr_poll_interval(),
            long_poll_timeout_ms: default_long_poll_timeout_ms(),
            api_timeout_ms: default_api_timeout_ms(),
            bot_token: None,
            account_id: None,
            user_id: None,
            sync_buf: String::new(),
            context_tokens: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatBridgeView {
    pub config: WechatBridgeConfig,
    pub bridge_running: bool,
    pub bridge_last_error: Option<String>,
    pub bridge_header_name: String,
    pub message_endpoint: String,
    pub qr_endpoint: String,
    pub latest_qr_content: Option<String>,
    pub latest_qr_updated_at: Option<String>,
    pub login_status: Option<String>,
    pub connected: bool,
    pub connected_account_id: Option<String>,
    pub connected_user_id: Option<String>,
    pub qr_session_key: Option<String>,
    pub qr_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WechatBridgeServerStatus {
    pub running: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WechatBridgeQrState {
    pub latest_qr_content: Option<String>,
    pub latest_qr_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WechatBridgeLoginSession {
    pub session_key: String,
    pub qrcode: String,
    pub qrcode_img_content: String,
    pub started_at: String,
    pub updated_at: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct WechatBridgeStore {
    file_path: PathBuf,
    server_status: Arc<Mutex<WechatBridgeServerStatus>>,
    qr_state: Arc<Mutex<WechatBridgeQrState>>,
    login_session: Arc<Mutex<Option<WechatBridgeLoginSession>>>,
}

impl WechatBridgeStore {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let dir = data_dir.join("integrations");
        std::fs::create_dir_all(&dir).map_err(|e| format!("Create integrations dir: {}", e))?;
        let file_path = dir.join("wechat_bridge.json");
        if !file_path.exists() {
            let default = WechatBridgeConfig::default();
            save_config_file(&file_path, &default)?;
        }
        Ok(Self {
            file_path,
            server_status: Arc::new(Mutex::new(WechatBridgeServerStatus::default())),
            qr_state: Arc::new(Mutex::new(WechatBridgeQrState::default())),
            login_session: Arc::new(Mutex::new(None)),
        })
    }

    pub fn get_config(&self) -> Result<WechatBridgeConfig, String> {
        load_config_file(&self.file_path)
    }

    pub fn save_config(&self, config: &WechatBridgeConfig) -> Result<WechatBridgeConfig, String> {
        let normalized = normalize_config(config.clone());
        save_config_file(&self.file_path, &normalized)?;
        Ok(normalized)
    }

    pub async fn get_view(&self) -> Result<WechatBridgeView, String> {
        let config = self.get_config()?;
        let server_status = self.server_status.lock().await.clone();
        let qr_state = self.qr_state.lock().await.clone();
        let login_session = self.login_session.lock().await.clone();
        Ok(WechatBridgeView {
            connected: config.bot_token.is_some(),
            connected_account_id: config.account_id.clone(),
            connected_user_id: config.user_id.clone(),
            config,
            bridge_running: server_status.running,
            bridge_last_error: server_status.last_error,
            bridge_header_name: WECHAT_BRIDGE_HEADER_TOKEN.into(),
            message_endpoint: format!("http://{}{}", WECHAT_BRIDGE_BIND_ADDR, WECHAT_BRIDGE_MESSAGE_PATH),
            qr_endpoint: format!("http://{}{}", WECHAT_BRIDGE_BIND_ADDR, WECHAT_BRIDGE_QR_PATH),
            latest_qr_content: qr_state.latest_qr_content,
            latest_qr_updated_at: qr_state.latest_qr_updated_at,
            login_status: login_session.as_ref().map(|item| item.status.clone()),
            qr_session_key: login_session.as_ref().map(|item| item.session_key.clone()),
            qr_error: login_session.as_ref().and_then(|item| item.error.clone()),
        })
    }

    pub async fn set_server_status(&self, running: bool, last_error: Option<String>) {
        let mut status = self.server_status.lock().await;
        status.running = running;
        status.last_error = last_error;
    }

    pub async fn update_qr_content(&self, content: String, updated_at: String) {
        let mut qr_state = self.qr_state.lock().await;
        qr_state.latest_qr_content = Some(content);
        qr_state.latest_qr_updated_at = Some(updated_at);
    }

    pub async fn set_login_session(&self, session: WechatBridgeLoginSession) {
        let mut login_session = self.login_session.lock().await;
        *login_session = Some(session);
    }

    pub async fn clear_login_session(&self) {
        let mut login_session = self.login_session.lock().await;
        *login_session = None;
    }

    pub async fn get_login_session(&self) -> Option<WechatBridgeLoginSession> {
        self.login_session.lock().await.clone()
    }
}

fn default_bridge_token() -> String {
    "ohmywu-local-bridge".into()
}

fn default_wechat_api_base_url() -> String {
    "https://ilinkai.weixin.qq.com".into()
}

fn default_wechat_bot_type() -> String {
    "3".into()
}

fn default_qr_poll_interval() -> u64 {
    1
}

fn default_long_poll_timeout_ms() -> u64 {
    35_000
}

fn default_api_timeout_ms() -> u64 {
    15_000
}

fn normalize_config(mut config: WechatBridgeConfig) -> WechatBridgeConfig {
    config.agent_id = config
        .agent_id
        .take()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    config.llm_profile_id = config
        .llm_profile_id
        .take()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    if config.bridge_token.trim().is_empty() {
        config.bridge_token = default_bridge_token();
    } else {
        config.bridge_token = config.bridge_token.trim().to_string();
    }
    if config.api_base_url.trim().is_empty() {
        config.api_base_url = default_wechat_api_base_url();
    } else {
        config.api_base_url = config.api_base_url.trim().trim_end_matches('/').to_string();
    }
    if config.bot_type.trim().is_empty() {
        config.bot_type = default_wechat_bot_type();
    } else {
        config.bot_type = config.bot_type.trim().to_string();
    }
    config.qr_poll_interval = config.qr_poll_interval.max(1);
    config.long_poll_timeout_ms = config.long_poll_timeout_ms.max(1_000);
    config.api_timeout_ms = config.api_timeout_ms.max(1_000);
    config.bot_token = config
        .bot_token
        .take()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    config.account_id = config
        .account_id
        .take()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    config.user_id = config
        .user_id
        .take()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    config.sync_buf = config.sync_buf.trim().to_string();
    config.context_tokens = config
        .context_tokens
        .into_iter()
        .map(|(user_id, token)| (user_id.trim().to_string(), token.trim().to_string()))
        .filter(|(user_id, token)| !user_id.is_empty() && !token.is_empty())
        .collect();
    config
}

fn load_config_file(path: &Path) -> Result<WechatBridgeConfig, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Read wechat bridge config: {}", e))?;
    let config: WechatBridgeConfig =
        serde_json::from_str(&content).map_err(|e| format!("Parse wechat bridge config: {}", e))?;
    Ok(normalize_config(config))
}

fn save_config_file(path: &Path, config: &WechatBridgeConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Serialize wechat bridge config: {}", e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("Write wechat bridge config: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Rename wechat bridge config: {}", e))?;
    Ok(())
}
