use ohmywu_domain::PolicyMode;
use ohmywu_llm_adapter::LlmConfig;
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
    /// "solid" | "image" | "video"
    #[serde(default = "default_background_mode")]
    pub background_mode: String,
    /// surface translucency 35-88
    #[serde(default = "default_surface_opacity")]
    pub surface_opacity: u8,
    /// background image scale (1.0 = native)
    #[serde(default = "default_bg_scale")]
    pub background_scale: f32,
    /// background blur px
    #[serde(default = "default_bg_blur")]
    pub background_blur: u8,
    /// mask opacity 0-100
    #[serde(default = "default_bg_mask")]
    pub background_mask_opacity: u8,
    #[serde(default)]
    pub llm_provider: Option<LlmConfig>,
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
fn default_background_mode() -> String {
    "solid".into()
}
fn default_surface_opacity() -> u8 {
    72
}
fn default_bg_scale() -> f32 {
    1.0
}
fn default_bg_blur() -> u8 {
    0
}
fn default_bg_mask() -> u8 {
    30
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            policy_mode: default_policy_mode(),
            theme: default_theme(),
            accent: default_accent(),
            background_mode: default_background_mode(),
            surface_opacity: default_surface_opacity(),
            background_scale: default_bg_scale(),
            background_blur: default_bg_blur(),
            background_mask_opacity: default_bg_mask(),
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
