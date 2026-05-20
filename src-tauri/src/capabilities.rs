use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ohmywu_capability_registry::CapabilityRegistry;
use ohmywu_domain::{Capability, RiskLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
struct BuiltinCapabilitySpec {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    risk_level: RiskLevel,
}

const BUILTIN_CAPABILITIES: &[BuiltinCapabilitySpec] = &[
    BuiltinCapabilitySpec {
        name: "bash",
        title: "终端命令",
        description: "Execute a shell command. Subject to policy control.",
        risk_level: RiskLevel::HighRisk,
    },
    BuiltinCapabilitySpec {
        name: "read",
        title: "读取文件",
        description: "Read file contents from the filesystem.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "write",
        title: "写入文件",
        description: "Write content to a file, creating parent directories if needed.",
        risk_level: RiskLevel::ControlledWrite,
    },
    BuiltinCapabilitySpec {
        name: "edit",
        title: "精确编辑",
        description: "Edit a file by finding and replacing exact text. Requires unique match.",
        risk_level: RiskLevel::ControlledWrite,
    },
    BuiltinCapabilitySpec {
        name: "glob",
        title: "文件搜索",
        description: "Search for files matching a glob pattern.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "grep",
        title: "内容搜索",
        description: "Search file contents for a pattern.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "web_fetch",
        title: "网页读取",
        description: "Fetch and read content from a URL.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "thinking",
        title: "思考记录",
        description: "Use for internal reasoning and planning steps. Does not execute anything.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "checklist_write",
        title: "任务清单",
        description: "Write or replace the current turn checklist for planning and progress tracking.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "wiki_read",
        title: "知识库读取",
        description: "Read a wiki note by slug. Returns markdown content with metadata.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "wiki_write",
        title: "知识库写入",
        description: "Create or update a wiki note. Specify slug, title, body, tags, and optional folder.",
        risk_level: RiskLevel::ControlledWrite,
    },
    BuiltinCapabilitySpec {
        name: "wiki_search",
        title: "知识库搜索",
        description: "Search wiki notes by keyword. Returns matching notes with relevance scores.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "wiki_list",
        title: "知识库列表",
        description: "List all wiki notes sorted by last updated.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "wiki_graph",
        title: "知识图谱",
        description: "Get the wiki knowledge graph as nodes and edges for visualization.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "capability_list",
        title: "能力目录读取",
        description: "List registered atomic capabilities with runtime metadata.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "capability_register",
        title: "能力目录注册",
        description: "Create or update a user-defined atomic capability wrapper in the capability catalog.",
        risk_level: RiskLevel::ControlledWrite,
    },
    BuiltinCapabilitySpec {
        name: "action_list",
        title: "Action 目录读取",
        description: "List registered actions, including system templates and user-defined actions.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "action_register",
        title: "Action 目录注册",
        description: "Create or update a user-defined action converted from prompts, workflows, or external skills.",
        risk_level: RiskLevel::ControlledWrite,
    },
    BuiltinCapabilitySpec {
        name: "agent_list",
        title: "Agent 目录读取",
        description: "List available agent profiles for the current session, including persona, memory scope, and tool range.",
        risk_level: RiskLevel::ReadOnly,
    },
    BuiltinCapabilitySpec {
        name: "agent_delegate",
        title: "Agent 子任务委派",
        description: "Delegate a bounded subtask to another registered agent and receive its structured result.",
        risk_level: RiskLevel::ControlledWrite,
    },
    BuiltinCapabilitySpec {
        name: "agent_register",
        title: "Agent 目录注册",
        description: "Create or update a registered agent profile, including persona, memory scope, and tool range.",
        risk_level: RiskLevel::ControlledWrite,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySource {
    Builtin,
    User,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityView {
    pub name: String,
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub implementation: String,
    pub source: CapabilitySource,
    pub enabled: bool,
    pub editable: bool,
    pub deletable: bool,
    pub executable: bool,
}

#[derive(Debug, Clone)]
pub struct CapabilityRuntimeEntry {
    pub name: String,
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub implementation: String,
    pub source: CapabilitySource,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityUpsertInput {
    pub existing_name: Option<String>,
    pub name: String,
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub implementation: String,
    pub enabled: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct CapabilityCatalogFile {
    #[serde(default)]
    builtin_overrides: Vec<BuiltinCapabilityOverride>,
    #[serde(default)]
    user_capabilities: Vec<UserCapabilityRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BuiltinCapabilityOverride {
    name: String,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserCapabilityRecord {
    name: String,
    title: String,
    description: String,
    risk_level: RiskLevel,
    implementation: String,
    enabled: bool,
}

pub struct CapabilityCatalog {
    file_path: PathBuf,
    file: CapabilityCatalogFile,
}

impl CapabilityCatalog {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let dir = data_dir.join("capabilities");
        std::fs::create_dir_all(&dir).map_err(|e| format!("Create capabilities dir: {}", e))?;
        let file_path = dir.join("catalog.json");
        let file = if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Read capabilities catalog: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Parse capabilities catalog: {}", e))?
        } else {
            let file = CapabilityCatalogFile::default();
            save_catalog_file(&file_path, &file)?;
            file
        };

        Ok(Self { file_path, file })
    }

    pub fn list_views(&self) -> Vec<CapabilityView> {
        let builtin_enabled = self.builtin_enabled_map();
        let mut items = Vec::new();

        for spec in BUILTIN_CAPABILITIES {
            items.push(CapabilityView {
                name: spec.name.to_string(),
                title: spec.title.to_string(),
                description: spec.description.to_string(),
                risk_level: spec.risk_level,
                implementation: spec.name.to_string(),
                source: CapabilitySource::Builtin,
                enabled: builtin_enabled.get(spec.name).copied().unwrap_or(true),
                editable: false,
                deletable: false,
                executable: true,
            });
        }

        for item in &self.file.user_capabilities {
            items.push(CapabilityView {
                name: item.name.clone(),
                title: item.title.clone(),
                description: item.description.clone(),
                risk_level: item.risk_level,
                implementation: item.implementation.clone(),
                source: CapabilitySource::User,
                enabled: item.enabled,
                editable: true,
                deletable: true,
                executable: builtin_spec(&item.implementation).is_some(),
            });
        }

        items.sort_by(|a, b| {
            let source_rank = match a.source {
                CapabilitySource::Builtin => 0,
                CapabilitySource::User => 1,
            }
            .cmp(&match b.source {
                CapabilitySource::Builtin => 0,
                CapabilitySource::User => 1,
            });
            if source_rank != std::cmp::Ordering::Equal {
                return source_rank;
            }
            a.title
                .to_lowercase()
                .cmp(&b.title.to_lowercase())
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        items
    }

    pub fn active_entries(&self) -> Vec<CapabilityRuntimeEntry> {
        self.list_views()
            .into_iter()
            .filter(|item| item.enabled && item.executable)
            .map(|item| CapabilityRuntimeEntry {
                name: item.name,
                title: item.title,
                description: item.description,
                risk_level: item.risk_level,
                implementation: item.implementation,
                source: item.source,
            })
            .collect()
    }

    pub fn all_names(&self) -> Vec<String> {
        self.list_views().into_iter().map(|item| item.name).collect()
    }

    pub fn active_names(&self) -> Vec<String> {
        self.active_entries().into_iter().map(|item| item.name).collect()
    }

    pub fn resolve_active(&self, name: &str) -> Option<CapabilityRuntimeEntry> {
        self.active_entries().into_iter().find(|item| item.name == name)
    }

    pub fn sync_registry(&self, registry: &CapabilityRegistry) {
        let items = self
            .active_entries()
            .into_iter()
            .map(|item| Capability::new(&item.name, &item.description, item.risk_level))
            .collect();
        registry.replace_all(items);
    }

    pub fn upsert(&mut self, input: CapabilityUpsertInput) -> Result<(), String> {
        let existing_name = input.existing_name.as_deref().map(str::trim).unwrap_or("");
        let name = input.name.trim();
        let title = input.title.trim();
        let description = input.description.trim();
        let implementation = input.implementation.trim();

        validate_capability_name(name)?;
        if title.is_empty() {
            return Err("能力标题不能为空".into());
        }
        if description.is_empty() {
            return Err("能力描述不能为空".into());
        }
        if builtin_spec(implementation).is_none() {
            return Err(format!("基础执行器 '{}' 不存在", implementation));
        }
        if builtin_spec(name).is_some() {
            return Err("不能覆盖内置能力名称".into());
        }
        let implementation_risk = builtin_spec(implementation)
            .map(|item| item.risk_level)
            .unwrap_or(RiskLevel::ReadOnly);
        if risk_rank(input.risk_level) < risk_rank(implementation_risk) {
            return Err("自定义能力的风险等级不能低于底层执行器".into());
        }

        let duplicate = self.file.user_capabilities.iter().any(|item| {
            item.name == name && (existing_name.is_empty() || item.name != existing_name)
        });
        if duplicate {
            return Err(format!("能力名 '{}' 已存在", name));
        }

        if existing_name.is_empty() {
            self.file.user_capabilities.push(UserCapabilityRecord {
                name: name.to_string(),
                title: title.to_string(),
                description: description.to_string(),
                risk_level: input.risk_level,
                implementation: implementation.to_string(),
                enabled: input.enabled,
            });
        } else {
            let Some(item) = self
                .file
                .user_capabilities
                .iter_mut()
                .find(|item| item.name == existing_name)
            else {
                return Err(format!("未找到可编辑能力 '{}'", existing_name));
            };

            item.name = name.to_string();
            item.title = title.to_string();
            item.description = description.to_string();
            item.risk_level = input.risk_level;
            item.implementation = implementation.to_string();
            item.enabled = input.enabled;
        }

        self.file
            .user_capabilities
            .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        self.save()
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<(), String> {
        if builtin_spec(name).is_some() {
            if let Some(override_item) = self
                .file
                .builtin_overrides
                .iter_mut()
                .find(|item| item.name == name)
            {
                override_item.enabled = enabled;
            } else {
                self.file.builtin_overrides.push(BuiltinCapabilityOverride {
                    name: name.to_string(),
                    enabled,
                });
            }
            self.file
                .builtin_overrides
                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            return self.save();
        }

        let Some(item) = self
            .file
            .user_capabilities
            .iter_mut()
            .find(|item| item.name == name)
        else {
            return Err(format!("能力 '{}' 不存在", name));
        };

        item.enabled = enabled;
        self.save()
    }

    pub fn delete(&mut self, name: &str) -> Result<(), String> {
        if builtin_spec(name).is_some() {
            return Err("内置能力不能删除".into());
        }

        let before = self.file.user_capabilities.len();
        self.file.user_capabilities.retain(|item| item.name != name);
        if self.file.user_capabilities.len() == before {
            return Err(format!("能力 '{}' 不存在", name));
        }

        self.save()
    }

    fn save(&self) -> Result<(), String> {
        save_catalog_file(&self.file_path, &self.file)
    }

    fn builtin_enabled_map(&self) -> HashMap<&str, bool> {
        self.file
            .builtin_overrides
            .iter()
            .map(|item| (item.name.as_str(), item.enabled))
            .collect()
    }
}

fn save_catalog_file(path: &Path, file: &CapabilityCatalogFile) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Serialize capabilities catalog: {}", e))?;
    std::fs::write(&tmp, json).map_err(|e| format!("Write capabilities catalog tmp: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Rename capabilities catalog: {}", e))?;
    Ok(())
}

fn builtin_spec(name: &str) -> Option<&'static BuiltinCapabilitySpec> {
    BUILTIN_CAPABILITIES.iter().find(|item| item.name == name)
}

fn validate_capability_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("能力名不能为空".into());
    }
    if name.len() > 64 {
        return Err("能力名不能超过 64 个字符".into());
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        return Err("能力名只允许字母、数字、下划线和短横线".into());
    }
    Ok(())
}

fn risk_rank(risk: RiskLevel) -> u8 {
    match risk {
        RiskLevel::ReadOnly => 0,
        RiskLevel::ControlledWrite => 1,
        RiskLevel::HighRisk => 2,
    }
}
