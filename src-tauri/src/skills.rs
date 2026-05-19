use std::fs;
use std::path::{Path, PathBuf};

use ohmywu_domain::{Action, ActionSource};

const MAX_SCAN_DEPTH: usize = 4;
const MAX_SUPPORT_FILES: usize = 24;

#[derive(Debug, Clone)]
struct SkillManifest {
    name: String,
    description: String,
    tags: Vec<String>,
    path: PathBuf,
    entry: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionBlueprint {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source: String,
    pub mode: String,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
    pub path: Option<String>,
    pub entry: Option<String>,
    pub compiled_prompt: String,
    pub supporting_files: Vec<String>,
}

pub fn discover_skill_actions() -> Vec<Action> {
    let mut skills = Vec::new();
    for root in skill_roots() {
        scan_skill_root(&root, 0, &mut skills);
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));
    skills.dedup_by(|a, b| a.name == b.name && a.path == b.path);

    skills
        .into_iter()
        .map(|skill| {
            let id = format!("skill.{}", slugify_action_id(&skill.name));
            Action::skill(
                &id,
                &skill.name,
                &skill.description,
                &skill.path.to_string_lossy(),
                &skill.entry.to_string_lossy(),
                &skill.tags,
            )
        })
        .collect()
}

pub fn build_action_blueprint(action: &Action) -> Result<ActionBlueprint, String> {
    match action.source {
        ActionSource::Builtin => Ok(ActionBlueprint {
            id: action.id.clone(),
            title: action.title.clone(),
            description: action.description.clone(),
            source: action.source.as_str().to_string(),
            mode: "builtin_flow".into(),
            capabilities: action.capabilities.clone(),
            tags: action.tags.clone(),
            path: action.path.clone(),
            entry: action.entry.clone(),
            compiled_prompt: builtin_prompt(action),
            supporting_files: Vec::new(),
        }),
        ActionSource::Skill => build_skill_blueprint(action),
    }
}

fn skill_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir.join(".codex/skills"));
        roots.push(current_dir.join(".agents/skills"));
    }

    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(&home).join(".codex/skills"));
        roots.push(PathBuf::from(&home).join(".agents/skills"));
    }

    roots
}

fn scan_skill_root(root: &Path, depth: usize, out: &mut Vec<SkillManifest>) {
    if depth > MAX_SCAN_DEPTH || !root.exists() {
        return;
    }

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                if let Some(skill) = parse_skill_manifest(&skill_md) {
                    out.push(skill);
                }
                continue;
            }
            scan_skill_root(&path, depth + 1, out);
        }
    }
}

fn parse_skill_manifest(skill_md: &Path) -> Option<SkillManifest> {
    let content = fs::read_to_string(skill_md).ok()?;
    let root = skill_md.parent()?.to_path_buf();
    let (frontmatter, body) = split_frontmatter(&content);
    let name = frontmatter
        .as_ref()
        .and_then(|meta| frontmatter_value(meta, "name"))
        .unwrap_or_else(|| {
            root.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown-skill")
                .to_string()
        });
    let description = frontmatter
        .as_ref()
        .and_then(|meta| frontmatter_value(meta, "description"))
        .unwrap_or_else(|| fallback_description(body));
    let mut tags = Vec::new();
    if root.to_string_lossy().contains("/.system/") {
        tags.push("system".into());
    } else {
        tags.push("user".into());
    }
    tags.push("codex-skill".into());

    Some(SkillManifest {
        name,
        description,
        tags,
        path: root,
        entry: skill_md.to_path_buf(),
    })
}

fn build_skill_blueprint(action: &Action) -> Result<ActionBlueprint, String> {
    let entry = action
        .entry
        .as_ref()
        .ok_or_else(|| format!("action {} 缺少 entry", action.id))?;
    let entry_path = PathBuf::from(entry);
    let content = fs::read_to_string(&entry_path)
        .map_err(|err| format!("读取 skill entry 失败: {}", err))?;
    let (_frontmatter, body) = split_frontmatter(&content);
    let root = action
        .path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| entry_path.parent().unwrap_or(Path::new("")).to_path_buf());

    Ok(ActionBlueprint {
        id: action.id.clone(),
        title: action.title.clone(),
        description: action.description.clone(),
        source: action.source.as_str().to_string(),
        mode: "skill_prompt".into(),
        capabilities: action.capabilities.clone(),
        tags: action.tags.clone(),
        path: action.path.clone(),
        entry: action.entry.clone(),
        compiled_prompt: body.trim().to_string(),
        supporting_files: collect_support_files(&root),
    })
}

fn builtin_prompt(action: &Action) -> String {
    let capability_text = if action.capabilities.is_empty() {
        "当前 action 没有声明底层 capability。".into()
    } else {
        format!("优先使用这些 capability：{}。", action.capabilities.join(", "))
    };
    format!(
        "这是一个内建 action。\n\n标题：{}\n描述：{}\n{}\n执行原则：保持步骤清晰、可审计，并优先复用稳定入口。",
        action.title, action.description, capability_text
    )
}

fn collect_support_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for dir_name in ["references", "scripts", "assets", "templates"] {
        let dir = root.join(dir_name);
        if !dir.exists() {
            continue;
        }
        collect_files_recursive(root, &dir, 0, &mut files);
        if files.len() >= MAX_SUPPORT_FILES {
            break;
        }
    }
    files.truncate(MAX_SUPPORT_FILES);
    files
}

fn collect_files_recursive(root: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
    if depth > 4 || out.len() >= MAX_SUPPORT_FILES {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= MAX_SUPPORT_FILES {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(root, &path, depth + 1, out);
            continue;
        }
        if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_string_lossy().to_string());
        }
    }
}

fn split_frontmatter(content: &str) -> (Option<String>, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---\n") {
        return (None, content);
    }
    let rest = &trimmed[4..];
    if let Some(end) = rest.find("\n---\n") {
        let meta = &rest[..end];
        let body = &rest[end + 5..];
        return (Some(meta.to_string()), body);
    }
    (None, content)
}

fn frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    frontmatter
        .lines()
        .find_map(|line| {
            let line = line.trim();
            let prefix = format!("{key}:");
            if !line.starts_with(&prefix) {
                return None;
            }
            let value = line[prefix.len()..].trim().trim_matches('"').trim_matches('\'');
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
}

fn fallback_description(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or("Skill compatibility entry")
        .to_string()
}

fn slugify_action_id(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
            continue;
        }
        if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ohmywu_domain::Action;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ohmywu-skill-test-{name}-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_skill_manifest_from_frontmatter() {
        let root = make_temp_dir("manifest");
        let skill_dir = root.join("demo-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        fs::write(
            &skill_md,
            r#"---
name: "demo-skill"
description: "Demo description"
---

# Demo Skill

Use this skill for demo tasks.
"#,
        )
        .unwrap();

        let manifest = parse_skill_manifest(&skill_md).unwrap();
        assert_eq!(manifest.name, "demo-skill");
        assert_eq!(manifest.description, "Demo description");
        assert!(manifest.tags.contains(&"codex-skill".to_string()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn builds_skill_action_blueprint_from_skill_body() {
        let root = make_temp_dir("blueprint");
        let skill_dir = root.join("research-skill");
        fs::create_dir_all(skill_dir.join("references")).unwrap();
        fs::write(skill_dir.join("references").join("latest-model.md"), "ref").unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        fs::write(
            &skill_md,
            r#"---
name: "research-skill"
description: "Research description"
---

# Research Skill

Step 1.
Step 2.
"#,
        )
        .unwrap();

        let action = Action::skill(
            "skill.research-skill",
            "research-skill",
            "Research description",
            &skill_dir.to_string_lossy(),
            &skill_md.to_string_lossy(),
            &["codex-skill".into()],
        );

        let blueprint = build_action_blueprint(&action).unwrap();
        assert_eq!(blueprint.mode, "skill_prompt");
        assert!(blueprint.compiled_prompt.contains("Step 1."));
        assert!(blueprint.supporting_files.iter().any(|file| file == "references/latest-model.md"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn builds_builtin_action_blueprint() {
        let action = Action::builtin(
            "fs.read",
            "文件读取",
            "稳定读取",
            &["read", "glob"],
            &["builtin"],
        );
        let blueprint = build_action_blueprint(&action).unwrap();
        assert_eq!(blueprint.mode, "builtin_flow");
        assert!(blueprint.compiled_prompt.contains("read, glob"));
    }
}
