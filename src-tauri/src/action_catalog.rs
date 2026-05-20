use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ohmywu_action_registry::ActionRegistry;
use ohmywu_domain::Action;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
struct SystemActionSpec {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    mode: &'static str,
    capabilities: &'static [&'static str],
    tags: &'static [&'static str],
    prompt: &'static str,
}

const ACTION_SPEC_PROMPT: &str = "\
你负责把外部 skill、内部经验或重复任务，沉淀成可审计的 action。

## Action 规范

1. action 必须有稳定 id、清晰标题和一句话描述。
2. action 的 prompt 要聚焦执行策略，而不是重复 capability 文档。
3. capabilities 只声明真正要暴露给执行层的原子能力。
4. 如果来源是外部 skill，需要保留来源说明、关键 supporting files 和迁移备注。
5. 如果动作还不稳定，不要塞太多 capability，先保持单一职责。
6. 默认使用中文描述，但 id 保持英文 / 下划线风格，便于程序引用。

## 转化原则

- 市面上的 skill、SKILL.md、工作流 prompt，都可以转成 action。
- 先提炼目标、边界、依赖能力，再整理为 prompt。
- 避免把整份 skill 原样堆进去，应该抽出有效规范和执行步骤。
- supporting files 只保留真正影响行为的文件。
";

const CAPABILITY_REGISTRY_PROMPT: &str = "\
你负责把重复出现的操作模式，注册成新的原子化能力包装层。

## 何时注册新能力

- 用户反复要求某一类固定动作，并希望模型有更明确的工具语义。
- 现有 capability 太底层，不利于模型稳定选择。
- 需要把某个执行器包装成更业务化的名字和说明。

## 注册原则

1. 优先复用现有底层执行器，不要凭空发明不可执行的能力。
2. 能力名要短、稳定、机器可读，例如 `project_read_docs`。
3. 风险等级不能低于底层执行器。
4. 描述要说明调用时机，而不是泛泛写“执行任务”。
5. 注册后要检查是否真的改善了工具选择，而不是制造噪音。
";

const AGENT_REGISTRY_PROMPT: &str = "\
你负责把稳定的人格、记忆策略和工具范围，注册成可复用的 agent。

## Agent 规范

1. agent id 必须稳定、机器可读，建议用英文和下划线。
2. name 面向人读，role 说明职责边界，persona 说明执行风格。
3. memory scope 必须明确：禁用、全量或定向；不要写模糊描述。
4. tools 只保留该 agent 真正需要的能力，避免默认全开。
5. `delegateTags` 要简短，适合做路由关键词，例如“代码 / 修复 / 测试”。
6. `delegateNote` 要一句话说明何时应该把任务交给这个 agent。
7. `delegatable` 必须明确填写；如果不希望主 agent 自动调用，就设为 false。
8. `delegatePriority` 用 0 到 100 表示候选排序，越高越靠前。
9. 修改现有 agent 前先用 `agent_list` 对照，优先更新而不是重复造新角色。
10. 如果任务只是一次性临时偏好，不要注册成 agent。

## 注册原则

- 主 agent 负责调度和总控，副 agent 负责窄职责执行。
- 可把“纯编码”、“长期记忆”、“资料整理”、“测试审计”这类稳定角色沉淀成 agent。
- agent 是长期配置，不是一次性 prompt。
";

const SYSTEM_ACTIONS: &[SystemActionSpec] = &[
    SystemActionSpec {
        id: "system.capability_registry",
        title: "原子化能力注册",
        description: "内置系统技能。指导 AI 将固定操作模式包装成新的原子化能力，并调用注册工具写入能力目录。",
        mode: "system_skill",
        capabilities: &["capability_list", "capability_register"],
        tags: &["system", "capability", "registry", "self-register"],
        prompt: CAPABILITY_REGISTRY_PROMPT,
    },
    SystemActionSpec {
        id: "system.skill_to_action",
        title: "Skill 转 Action",
        description: "内置系统技能。兼容外部 SKILL.md、prompt 型 skill 和 workflow 规范，帮助 AI 把它们转换成可运行的 action。",
        mode: "system_skill",
        capabilities: &["read", "action_list", "action_register", "capability_list"],
        tags: &["system", "skill", "action", "compatibility"],
        prompt: ACTION_SPEC_PROMPT,
    },
    SystemActionSpec {
        id: "system.agent_registry",
        title: "Agent 注册",
        description: "内置系统技能。指导 AI 根据人格、记忆范围和工具边界创建或更新 agent 配置。",
        mode: "system_skill",
        capabilities: &["agent_list", "agent_register", "capability_list"],
        tags: &["system", "agent", "registry", "multi-agent"],
        prompt: AGENT_REGISTRY_PROMPT,
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source: String,
    pub mode: String,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub editable: bool,
    pub deletable: bool,
    pub available: bool,
    pub source_hint: Option<String>,
    pub supporting_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionBlueprint {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source: String,
    pub mode: String,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
    pub source_hint: Option<String>,
    pub compiled_prompt: String,
    pub supporting_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionUpsertInput {
    pub existing_id: Option<String>,
    pub id: String,
    pub title: String,
    pub description: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub prompt: String,
    #[serde(default)]
    pub supporting_files: Vec<String>,
    pub source_hint: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct ActionCatalogFile {
    #[serde(default)]
    user_actions: Vec<UserActionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserActionRecord {
    id: String,
    title: String,
    description: String,
    capabilities: Vec<String>,
    tags: Vec<String>,
    prompt: String,
    supporting_files: Vec<String>,
    source_hint: Option<String>,
    enabled: bool,
}

pub struct ActionCatalog {
    file_path: PathBuf,
    file: ActionCatalogFile,
}

impl ActionCatalog {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let dir = data_dir.join("actions");
        std::fs::create_dir_all(&dir).map_err(|e| format!("Create actions dir: {}", e))?;
        let file_path = dir.join("catalog.json");
        let file = if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Read actions catalog: {}", e))?;
            serde_json::from_str(&content).map_err(|e| format!("Parse actions catalog: {}", e))?
        } else {
            let file = ActionCatalogFile::default();
            save_catalog_file(&file_path, &file)?;
            file
        };
        Ok(Self { file_path, file })
    }

    pub fn list_views(&self, active_capabilities: &HashSet<String>) -> Vec<ActionView> {
        let mut items = Vec::new();

        for spec in SYSTEM_ACTIONS {
            items.push(ActionView {
                id: spec.id.to_string(),
                title: spec.title.to_string(),
                description: spec.description.to_string(),
                source: "system".into(),
                mode: spec.mode.to_string(),
                capabilities: spec.capabilities.iter().map(|item| item.to_string()).collect(),
                tags: spec.tags.iter().map(|item| item.to_string()).collect(),
                enabled: true,
                editable: false,
                deletable: false,
                available: spec
                    .capabilities
                    .iter()
                    .all(|item| active_capabilities.contains(*item)),
                source_hint: Some("builtin://ohmywu/system-action".into()),
                supporting_files: Vec::new(),
            });
        }

        for item in &self.file.user_actions {
            items.push(ActionView {
                id: item.id.clone(),
                title: item.title.clone(),
                description: item.description.clone(),
                source: "user".into(),
                mode: "action_prompt".into(),
                capabilities: item.capabilities.clone(),
                tags: item.tags.clone(),
                enabled: item.enabled,
                editable: false,
                deletable: true,
                available: item
                    .capabilities
                    .iter()
                    .all(|capability| active_capabilities.contains(capability)),
                source_hint: item.source_hint.clone(),
                supporting_files: item.supporting_files.clone(),
            });
        }

        items.sort_by(|a, b| {
            source_rank(&a.source)
                .cmp(&source_rank(&b.source))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                .then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase()))
        });
        items
    }

    pub fn sync_registry(&self, registry: &ActionRegistry, active_capabilities: &HashSet<String>) {
        let mut items = Vec::new();
        for spec in SYSTEM_ACTIONS {
            items.push(Action::builtin(
                spec.id,
                spec.title,
                spec.description,
                spec.capabilities,
                spec.tags,
            ));
        }
        for item in &self.file.user_actions {
            items.push(Action::user(
                &item.id,
                &item.title,
                &item.description,
                &item.capabilities,
                &item.tags,
                item.enabled
                    && item
                        .capabilities
                        .iter()
                        .all(|capability| active_capabilities.contains(capability)),
            ));
        }
        registry.replace_all(items);
    }

    pub fn get_blueprint(&self, id: &str) -> Result<ActionBlueprint, String> {
        if let Some(spec) = SYSTEM_ACTIONS.iter().find(|item| item.id == id) {
            return Ok(ActionBlueprint {
                id: spec.id.to_string(),
                title: spec.title.to_string(),
                description: spec.description.to_string(),
                source: "system".into(),
                mode: spec.mode.to_string(),
                capabilities: spec.capabilities.iter().map(|item| item.to_string()).collect(),
                tags: spec.tags.iter().map(|item| item.to_string()).collect(),
                source_hint: Some("builtin://ohmywu/system-action".into()),
                compiled_prompt: spec.prompt.to_string(),
                supporting_files: Vec::new(),
            });
        }

        let item = self
            .file
            .user_actions
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| format!("action '{}' 未注册", id))?;

        Ok(ActionBlueprint {
            id: item.id.clone(),
            title: item.title.clone(),
            description: item.description.clone(),
            source: "user".into(),
            mode: "action_prompt".into(),
            capabilities: item.capabilities.clone(),
            tags: item.tags.clone(),
            source_hint: item.source_hint.clone(),
            compiled_prompt: item.prompt.clone(),
            supporting_files: item.supporting_files.clone(),
        })
    }

    pub fn upsert(
        &mut self,
        input: ActionUpsertInput,
        known_capabilities: &HashSet<String>,
    ) -> Result<(), String> {
        let existing_id = input.existing_id.as_deref().map(str::trim).unwrap_or("");
        let id = input.id.trim();
        let title = input.title.trim();
        let description = input.description.trim();
        let prompt = input.prompt.trim();

        validate_action_id(id)?;
        if SYSTEM_ACTIONS.iter().any(|spec| spec.id == id) {
            return Err("不能覆盖内置 system action".into());
        }
        if title.is_empty() {
            return Err("action 标题不能为空".into());
        }
        if description.is_empty() {
            return Err("action 描述不能为空".into());
        }
        if prompt.is_empty() {
            return Err("action prompt 不能为空".into());
        }
        if input.capabilities.is_empty() {
            return Err("action 至少要绑定一个 capability".into());
        }
        for capability in &input.capabilities {
            if !known_capabilities.contains(capability) {
                return Err(format!("引用了未注册 capability '{}'", capability));
            }
        }

        let duplicate = self.file.user_actions.iter().any(|item| {
            item.id == id && (existing_id.is_empty() || item.id != existing_id)
        });
        if duplicate {
            return Err(format!("action id '{}' 已存在", id));
        }

        let tags = normalize_tags(&input.tags);
        let supporting_files = normalize_files(&input.supporting_files);
        let source_hint = input
            .source_hint
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty());

        if existing_id.is_empty() {
            self.file.user_actions.push(UserActionRecord {
                id: id.to_string(),
                title: title.to_string(),
                description: description.to_string(),
                capabilities: input.capabilities,
                tags,
                prompt: prompt.to_string(),
                supporting_files,
                source_hint,
                enabled: input.enabled,
            });
        } else {
            let Some(item) = self.file.user_actions.iter_mut().find(|item| item.id == existing_id) else {
                return Err(format!("未找到可编辑 action '{}'", existing_id));
            };
            item.id = id.to_string();
            item.title = title.to_string();
            item.description = description.to_string();
            item.capabilities = input.capabilities;
            item.tags = tags;
            item.prompt = prompt.to_string();
            item.supporting_files = supporting_files;
            item.source_hint = source_hint;
            item.enabled = input.enabled;
        }

        self.file
            .user_actions
            .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        self.save()
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        if SYSTEM_ACTIONS.iter().any(|spec| spec.id == id) {
            return Err("内置 system action 不能停用".into());
        }
        let Some(item) = self.file.user_actions.iter_mut().find(|item| item.id == id) else {
            return Err(format!("action '{}' 不存在", id));
        };
        item.enabled = enabled;
        self.save()
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        if SYSTEM_ACTIONS.iter().any(|spec| spec.id == id) {
            return Err("内置 system action 不能删除".into());
        }
        let before = self.file.user_actions.len();
        self.file.user_actions.retain(|item| item.id != id);
        if self.file.user_actions.len() == before {
            return Err(format!("action '{}' 不存在", id));
        }
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        save_catalog_file(&self.file_path, &self.file)
    }
}

fn source_rank(source: &str) -> u8 {
    match source {
        "system" => 0,
        "user" => 1,
        _ => 2,
    }
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

fn normalize_files(files: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for file in files {
        let trimmed = file.trim().to_string();
        if trimmed.is_empty() || !seen.insert(trimmed.clone()) {
            continue;
        }
        out.push(trimmed);
    }
    out
}

fn validate_action_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("action id 不能为空".into());
    }
    if id.len() > 80 {
        return Err("action id 不能超过 80 个字符".into());
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err("action id 只允许字母、数字、点、下划线和短横线".into());
    }
    Ok(())
}

fn save_catalog_file(path: &Path, file: &ActionCatalogFile) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Serialize actions catalog: {}", e))?;
    std::fs::write(&tmp, json).map_err(|e| format!("Write actions catalog tmp: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Rename actions catalog: {}", e))?;
    Ok(())
}
