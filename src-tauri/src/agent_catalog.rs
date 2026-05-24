use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent::AgentInvocationProfile;

const DEFAULT_CORE_SCOPE: &str = r#"{"version":1,"label":"禁用长期记忆","mode":"none","folders":[],"recallLimit":4,"notes":"适合纯即时任务，不注入历史知识。"}"#;
const DEFAULT_MEMORY_SCOPE: &str = r#"{"version":1,"label":"长期偏好与知识沉淀","mode":"focused","folders":["concepts","profile","daily"],"recallLimit":6,"notes":"优先召回长期偏好、项目决策和近期沉淀。"}"#;
const DEFAULT_CODER_SCOPE: &str = r#"{"version":1,"label":"工程上下文","mode":"focused","folders":["notes","concepts"],"recallLimit":4,"notes":"优先使用项目笔记、技术概念和实现约定。"}"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub id: String,
    pub name: String,
    pub role: String,
    pub persona: String,
    pub memory_scope: String,
    pub tools: Vec<String>,
    pub delegate_tags: Vec<String>,
    pub delegate_note: String,
    pub delegatable: bool,
    pub delegate_priority: i32,
    pub primary: bool,
    pub editable: bool,
    pub deletable: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpsertInput {
    pub existing_id: Option<String>,
    pub id: String,
    pub name: String,
    pub role: String,
    pub persona: String,
    pub memory_scope: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub delegate_tags: Vec<String>,
    #[serde(default)]
    pub delegate_note: String,
    #[serde(default)]
    pub delegatable: bool,
    #[serde(default)]
    pub delegate_priority: i32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct AgentCatalogFile {
    #[serde(default)]
    agents: Vec<AgentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentRecord {
    id: String,
    name: String,
    role: String,
    persona: String,
    memory_scope: String,
    tools: Vec<String>,
    #[serde(default)]
    delegate_tags: Vec<String>,
    #[serde(default)]
    delegate_note: String,
    #[serde(default)]
    delegatable: bool,
    #[serde(default)]
    delegate_priority: i32,
    primary: bool,
}

pub struct AgentCatalog {
    file_path: PathBuf,
    file: AgentCatalogFile,
}

impl AgentCatalog {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let dir = data_dir.join("agents");
        std::fs::create_dir_all(&dir).map_err(|e| format!("Create agents dir: {}", e))?;
        let file_path = dir.join("catalog.json");
        let mut file = if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Read agents catalog: {}", e))?;
            serde_json::from_str(&content).map_err(|e| format!("Parse agents catalog: {}", e))?
        } else {
            default_catalog_file()
        };

        file.agents = file
            .agents
            .into_iter()
            .map(normalize_agent_record)
            .collect();
        file.agents = ensure_primary(file.agents);
        if !file_path.exists() {
            save_catalog_file(&file_path, &file)?;
        }

        Ok(Self { file_path, file })
    }

    pub fn list_views(&self) -> Vec<AgentView> {
        let mut items = self
            .file
            .agents
            .iter()
            .map(|item| AgentView {
                id: item.id.clone(),
                name: item.name.clone(),
                role: item.role.clone(),
                persona: item.persona.clone(),
                memory_scope: item.memory_scope.clone(),
                tools: item.tools.clone(),
                delegate_tags: item.delegate_tags.clone(),
                delegate_note: item.delegate_note.clone(),
                delegatable: item.delegatable,
                delegate_priority: item.delegate_priority,
                primary: item.primary,
                editable: true,
                deletable: !item.primary,
            })
            .collect::<Vec<_>>();

        items.sort_by(|a, b| {
            primary_rank(a.primary)
                .cmp(&primary_rank(b.primary))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase()))
        });
        items
    }

    pub fn list_profiles(&self) -> Vec<AgentInvocationProfile> {
        self.file
            .agents
            .iter()
            .map(|item| AgentInvocationProfile {
                id: item.id.clone(),
                name: item.name.clone(),
                role: item.role.clone(),
                persona: item.persona.clone(),
                memory_scope: item.memory_scope.clone(),
                tools: item.tools.clone(),
                delegatable: item.delegatable,
                delegate_priority: item.delegate_priority,
            })
            .collect()
    }

    pub fn get_profile(&self, id: &str) -> Option<AgentInvocationProfile> {
        self.file
            .agents
            .iter()
            .find(|item| item.id == id)
            .map(|item| AgentInvocationProfile {
                id: item.id.clone(),
                name: item.name.clone(),
                role: item.role.clone(),
                persona: item.persona.clone(),
                memory_scope: item.memory_scope.clone(),
                tools: item.tools.clone(),
                delegatable: item.delegatable,
                delegate_priority: item.delegate_priority,
            })
    }

    pub fn upsert(
        &mut self,
        input: AgentUpsertInput,
        known_capabilities: &HashSet<String>,
    ) -> Result<(), String> {
        let existing_id = input.existing_id.as_deref().map(str::trim).unwrap_or("");
        let id = input.id.trim();
        let name = input.name.trim();
        let role = input.role.trim();
        let persona = input.persona.trim();
        let memory_scope = input.memory_scope.trim();
        let tools = normalize_tools(&input.tools);
        let delegate_tags = normalize_tags(&input.delegate_tags);
        let delegate_note = input.delegate_note.trim().to_string();
        let delegate_priority = input.delegate_priority.clamp(0, 100);

        validate_agent_id(id)?;
        if name.is_empty() {
            return Err("agent 名称不能为空".into());
        }
        if role.is_empty() {
            return Err("agent 角色不能为空".into());
        }
        if persona.is_empty() {
            return Err("agent 人格不能为空".into());
        }
        if memory_scope.is_empty() {
            return Err("agent 记忆范围不能为空".into());
        }
        for tool in &tools {
            if !known_capabilities.contains(tool) {
                return Err(format!("引用了未注册 capability '{}'", tool));
            }
        }

        let duplicate = self.file.agents.iter().any(|item| {
            item.id == id && (existing_id.is_empty() || item.id != existing_id)
        });
        if duplicate {
            return Err(format!("agent id '{}' 已存在", id));
        }

        if existing_id.is_empty() {
            self.file.agents.push(AgentRecord {
                id: id.to_string(),
                name: name.to_string(),
                role: role.to_string(),
                persona: persona.to_string(),
                memory_scope: memory_scope.to_string(),
                tools,
                delegate_tags,
                delegate_note,
                delegatable: input.delegatable,
                delegate_priority,
                primary: false,
            });
        } else {
            let Some(item) = self.file.agents.iter_mut().find(|item| item.id == existing_id) else {
                return Err(format!("未找到可编辑 agent '{}'", existing_id));
            };
            item.id = id.to_string();
            item.name = name.to_string();
            item.role = role.to_string();
            item.persona = persona.to_string();
            item.memory_scope = memory_scope.to_string();
            item.tools = tools;
            item.delegate_tags = delegate_tags;
            item.delegate_note = delegate_note;
            item.delegatable = input.delegatable;
            item.delegate_priority = delegate_priority;
        }

        self.file.agents = ensure_primary(std::mem::take(&mut self.file.agents));
        self.save()
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        if self.file.agents.iter().any(|item| item.id == id && item.primary) {
            return Err("主 agent 不能删除".into());
        }
        let before = self.file.agents.len();
        self.file.agents.retain(|item| item.id != id);
        if self.file.agents.len() == before {
            return Err(format!("agent '{}' 不存在", id));
        }
        self.file.agents = ensure_primary(std::mem::take(&mut self.file.agents));
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        save_catalog_file(&self.file_path, &self.file)
    }
}

fn default_catalog_file() -> AgentCatalogFile {
    AgentCatalogFile {
        agents: vec![
            AgentRecord {
                id: "core".into(),
                name: "主 Agent（无记忆）".into(),
                role: "通用执行 / 零记忆".into(),
                persona: "稳健、可审计、优先把任务拆清楚再执行，不主动携带长期记忆，适合做默认入口和调度。".into(),
                memory_scope: DEFAULT_CORE_SCOPE.into(),
                tools: vec![
                    "read".into(),
                    "artifact_read".into(),
                    "grep".into(),
                    "glob".into(),
                    "bash".into(),
                    "wiki_read".into(),
                    "wiki_search".into(),
                ],
                delegate_tags: vec![
                    "通用".into(),
                    "调度".into(),
                    "拆解".into(),
                    "执行".into(),
                ],
                delegate_note: "默认入口。适合先理解需求、拆任务、串联其他 agent，不适合大量长期记忆召回。".into(),
                delegatable: false,
                delegate_priority: 0,
                primary: true,
            },
            AgentRecord {
                id: "memory".into(),
                name: "记忆 Agent（大量记忆）".into(),
                role: "知识整理 / 长期记忆".into(),
                persona: "更偏总结、归档、提炼长期上下文，减少噪音，强调结构化知识沉淀与记忆召回。".into(),
                memory_scope: DEFAULT_MEMORY_SCOPE.into(),
                tools: vec![
                    "read".into(),
                    "artifact_read".into(),
                    "wiki_read".into(),
                    "wiki_search".into(),
                    "wiki_write".into(),
                ],
                delegate_tags: vec![
                    "记忆".into(),
                    "知识库".into(),
                    "归档".into(),
                    "总结".into(),
                    "复盘".into(),
                ],
                delegate_note: "适合总结、长期知识沉淀、个人偏好整理、记忆候选和复盘归档。".into(),
                delegatable: true,
                delegate_priority: 70,
                primary: false,
            },
            AgentRecord {
                id: "coder".into(),
                name: "编码 Agent（纯编码）".into(),
                role: "纯编码 / 工程实现".into(),
                persona: "偏工程实现，优先读代码、改代码、跑构建，避免无关发散。".into(),
                memory_scope: DEFAULT_CODER_SCOPE.into(),
                tools: vec![
                    "read".into(),
                    "artifact_read".into(),
                    "grep".into(),
                    "glob".into(),
                    "edit".into(),
                    "write".into(),
                    "bash".into(),
                ],
                delegate_tags: vec![
                    "代码".into(),
                    "修复".into(),
                    "构建".into(),
                    "测试".into(),
                    "前端".into(),
                    "后端".into(),
                ],
                delegate_note: "适合读代码、改代码、构建检查、测试失败排查和工程实现。".into(),
                delegatable: true,
                delegate_priority: 90,
                primary: false,
            },
        ],
    }
}

fn ensure_primary(mut agents: Vec<AgentRecord>) -> Vec<AgentRecord> {
    if agents.is_empty() {
        return default_catalog_file().agents;
    }
    if !agents.iter().any(|item| item.primary) {
        if let Some(first) = agents.first_mut() {
            first.primary = true;
        }
    }
    agents.sort_by(|a, b| {
        primary_rank(a.primary)
            .cmp(&primary_rank(b.primary))
            .then_with(|| b.delegate_priority.cmp(&a.delegate_priority))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase()))
    });
    agents
}

fn normalize_agent_record(mut item: AgentRecord) -> AgentRecord {
    if item.tools.iter().any(|tool| tool == "read")
        && !item.tools.iter().any(|tool| tool == "artifact_read")
    {
        item.tools.push("artifact_read".into());
    }

    if item.id == "core" {
        if item.delegate_tags.is_empty() {
            item.delegate_tags = vec![
                "通用".into(),
                "调度".into(),
                "拆解".into(),
                "执行".into(),
            ];
        }
        if item.delegate_note.trim().is_empty() {
            item.delegate_note =
                "默认入口。适合先理解需求、拆任务、串联其他 agent，不适合大量长期记忆召回。".into();
        }
        item.delegatable = false;
        return item;
    }

    if item.id == "memory" {
        if item.delegate_tags.is_empty() {
            item.delegate_tags = vec![
                "记忆".into(),
                "知识库".into(),
                "归档".into(),
                "总结".into(),
                "复盘".into(),
            ];
        }
        if item.delegate_note.trim().is_empty() {
            item.delegate_note =
                "适合总结、长期知识沉淀、个人偏好整理、记忆候选和复盘归档。".into();
        }
        if item.delegate_priority == 0 {
            item.delegate_priority = 70;
        }
        item.delegatable = true;
        return item;
    }

    if item.id == "coder" {
        if item.delegate_tags.is_empty() {
            item.delegate_tags = vec![
                "代码".into(),
                "修复".into(),
                "构建".into(),
                "测试".into(),
                "前端".into(),
                "后端".into(),
            ];
        }
        if item.delegate_note.trim().is_empty() {
            item.delegate_note =
                "适合读代码、改代码、构建检查、测试失败排查和工程实现。".into();
        }
        if item.delegate_priority == 0 {
            item.delegate_priority = 90;
        }
        item.delegatable = true;
        return item;
    }

    if item.delegate_tags.is_empty() {
        item.delegate_tags = vec!["自定义".into()];
    }
    if item.delegate_priority == 0 {
        item.delegate_priority = 50;
    }
    if !item.primary {
        item.delegatable = true;
    }
    item
}

fn primary_rank(primary: bool) -> u8 {
    if primary { 0 } else { 1 }
}

fn normalize_tools(tools: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for tool in tools {
        let trimmed = tool.trim().to_string();
        if trimmed.is_empty() || !seen.insert(trimmed.clone()) {
            continue;
        }
        out.push(trimmed);
    }
    out
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for tag in tags {
        let trimmed = tag.trim().to_string();
        if trimmed.is_empty() || !seen.insert(trimmed.clone()) {
            continue;
        }
        out.push(trimmed);
    }
    out
}

fn validate_agent_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("agent id 不能为空".into());
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err("agent id 只能包含字母、数字、下划线和中划线".into());
    }
    Ok(())
}

fn save_catalog_file(path: &Path, file: &AgentCatalogFile) -> Result<(), String> {
    let tmp_path = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Serialize agents catalog: {}", e))?;
    std::fs::write(&tmp_path, content).map_err(|e| format!("Write agents catalog tmp: {}", e))?;
    std::fs::rename(&tmp_path, path).map_err(|e| format!("Rename agents catalog: {}", e))?;
    Ok(())
}
