use ohmywu_domain::PolicyMode;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_policy_mode")]
    pub policy_mode: PolicyMode,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default)]
    pub llm_provider: Option<LlmConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider_type")]
    pub provider_type: String,
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_policy_mode() -> PolicyMode {
    PolicyMode::Sandbox
}
fn default_theme() -> String {
    "midnight".into()
}
fn default_accent() -> String {
    "#3b82f6".into()
}
fn default_provider_type() -> String {
    "ollama".into()
}
fn default_ollama_endpoint() -> String {
    "http://localhost:11434".into()
}
fn default_model() -> String {
    "qwen2.5".into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            policy_mode: default_policy_mode(),
            theme: default_theme(),
            accent: default_accent(),
            llm_provider: None,
        }
    }
}

pub fn load_config(data_dir: &Path) -> Result<AppConfig, String> {
    let config_path = data_dir.join("config.json");
    if !config_path.exists() {
        let config = AppConfig::default();
        save_config(data_dir, &config)?;
        return Ok(config);
    }
    let content =
        std::fs::read_to_string(&config_path).map_err(|e| format!("Read config: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse config: {}", e))
}

pub fn save_config(data_dir: &Path, config: &AppConfig) -> Result<(), String> {
    let config_path = data_dir.join("config.json");
    let tmp = data_dir.join("config.json.tmp");
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("Serialize config: {}", e))?;
    std::fs::write(&tmp, json).map_err(|e| format!("Write config tmp: {}", e))?;
    std::fs::rename(&tmp, &config_path).map_err(|e| format!("Rename config: {}", e))?;
    Ok(())
}
