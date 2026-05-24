use ohmywu_domain::{AgentMode, PolicyMode};
use ohmywu_llm_adapter::LlmConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::permission::PermissionConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(flatten)]
    pub config: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_policy_mode")]
    pub policy_mode: PolicyMode,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_background_solid_color")]
    pub background_solid_color: String,
    #[serde(default = "default_background_preset")]
    pub background_preset: String,
    /// "solid" | "image"
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
    /// follow uploaded image dominant color
    #[serde(default = "default_background_auto_theme")]
    pub background_auto_theme: bool,
    #[serde(default)]
    pub background_theme_color: Option<String>,
    #[serde(default = "default_agent_mode")]
    pub agent_mode: AgentMode,
    #[serde(default)]
    pub active_llm_profile_id: Option<String>,
    #[serde(default)]
    pub llm_profiles: Vec<LlmProfile>,
    #[serde(default)]
    pub compression_llm_profile_id: Option<String>,
    #[serde(default)]
    pub llm_provider: Option<LlmConfig>,
    #[serde(default)]
    pub permissions: PermissionConfig,
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
fn default_background_solid_color() -> String {
    "#111827".into()
}
fn default_background_preset() -> String {
    "noctis".into()
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
fn default_background_auto_theme() -> bool {
    true
}
fn default_agent_mode() -> AgentMode {
    AgentMode::Agent
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            policy_mode: default_policy_mode(),
            theme: default_theme(),
            accent: default_accent(),
            background_solid_color: default_background_solid_color(),
            background_preset: default_background_preset(),
            background_mode: default_background_mode(),
            surface_opacity: default_surface_opacity(),
            background_scale: default_bg_scale(),
            background_blur: default_bg_blur(),
            background_mask_opacity: default_bg_mask(),
            background_auto_theme: default_background_auto_theme(),
            background_theme_color: None,
            agent_mode: default_agent_mode(),
            active_llm_profile_id: None,
            llm_profiles: Vec::new(),
            compression_llm_profile_id: None,
            llm_provider: None,
            permissions: PermissionConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn llm_config_by_id(&self, profile_id: &str) -> Option<LlmConfig> {
        let trimmed = profile_id.trim();
        if trimmed.is_empty() {
            return None;
        }
        self.llm_profiles
            .iter()
            .find(|item| item.id == trimmed)
            .map(|item| item.config.clone())
    }

    pub fn active_llm_config(&self) -> Option<LlmConfig> {
        if let Some(active_id) = &self.active_llm_profile_id {
            if let Some(profile) = self.llm_profiles.iter().find(|item| &item.id == active_id) {
                return Some(profile.config.clone());
            }
        }
        if self.active_llm_profile_id.is_none() {
            return None;
        }
        self.llm_profiles
            .first()
            .map(|profile| profile.config.clone())
            .or_else(|| self.llm_provider.clone())
    }

    pub fn compression_llm_config(&self) -> Option<LlmConfig> {
        self
            .compression_llm_profile_id
            .as_deref()
            .and_then(|profile_id| self.llm_config_by_id(profile_id))
            .or_else(|| self.active_llm_config())
    }

    pub fn normalized(mut self) -> Self {
        let migrating_legacy = self.llm_profiles.is_empty() && self.llm_provider.is_some();
        if migrating_legacy
            && let Some(config) = self.llm_provider.clone()
        {
            self.llm_profiles.push(LlmProfile {
                id: "default".into(),
                name: default_profile_name(&config),
                config,
            });
        }

        for (index, profile) in self.llm_profiles.iter_mut().enumerate() {
            if profile.id.trim().is_empty() {
                profile.id = format!("profile-{}", index + 1);
            }
            if profile.name.trim().is_empty() {
                profile.name = default_profile_name(&profile.config);
            }
        }

        if self.llm_profiles.is_empty() {
            self.active_llm_profile_id = None;
            self.compression_llm_profile_id = None;
            self.llm_provider = None;
            return self;
        }

        self.active_llm_profile_id = match self.active_llm_profile_id.clone() {
            Some(id) if self.llm_profiles.iter().any(|profile| profile.id == id) => Some(id),
            Some(_) => Some(self.llm_profiles[0].id.clone()),
            None if migrating_legacy => Some(self.llm_profiles[0].id.clone()),
            None => None,
        };
        self.compression_llm_profile_id = match self.compression_llm_profile_id.clone() {
            Some(id) if self.llm_profiles.iter().any(|profile| profile.id == id) => Some(id),
            Some(_) => self.active_llm_profile_id.clone(),
            None => self.active_llm_profile_id.clone(),
        };
        self.llm_provider = self.active_llm_config();
        self
    }
}

fn default_profile_name(config: &LlmConfig) -> String {
    let provider = if config.provider_type.trim().is_empty() {
        "model"
    } else {
        config.provider_type.trim()
    };
    let model = config.model.trim();
    if model.is_empty() {
        provider.to_string()
    } else {
        format!("{} · {}", provider, model)
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
    let config: AppConfig =
        serde_json::from_str(&content).map_err(|e| format!("Parse config: {}", e))?;
    Ok(config.normalized())
}

pub fn save_config(data_dir: &Path, config: &AppConfig) -> Result<(), String> {
    let config_path = data_dir.join("config.json");
    let tmp = data_dir.join("config.json.tmp");
    let normalized = config.clone().normalized();
    let json = serde_json::to_string_pretty(&normalized)
        .map_err(|e| format!("Serialize config: {}", e))?;
    std::fs::write(&tmp, json).map_err(|e| format!("Write config tmp: {}", e))?;
    std::fs::rename(&tmp, &config_path).map_err(|e| format!("Rename config: {}", e))?;
    Ok(())
}
