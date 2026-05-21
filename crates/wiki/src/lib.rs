use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    pub slug: String,
    pub title: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub created: String,
    pub updated: String,
    pub links_to: Vec<String>,
    pub linked_from: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiNote {
    pub slug: String,
    pub title: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub created: String,
    pub updated: String,
    pub body: String,
    #[serde(skip)]
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub link_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub slug: String,
    pub title: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallHit {
    pub slug: String,
    pub title: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub updated: String,
    pub score: usize,
    pub snippet: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Frontmatter {
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    created: Option<String>,
    #[serde(default)]
    updated: Option<String>,
}

// ── Engine ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct WikiEngine {
    root: PathBuf,
}

impl WikiEngine {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn init(&self) -> Result<(), String> {
        let dirs = ["concepts", "notes", "daily", "profile"];
        for d in &dirs {
            let path = self.root.join(d);
            fs::create_dir_all(&path)
                .map_err(|e| format!("create dir {:?}: {}", path, e))?;
        }

        let readme = self.root.join("README.md");
        if !readme.exists() {
            let content = r#"# OhMyWu Wiki

你的个人知识库，由 AI 助手维护。

## 目录

- **[概念](./concepts/)** — 技术概念、框架、方法论
- **[笔记](./notes/)** — 自由笔记
- **[每日](./daily/)** — 每日学习记录
- **[画像](./profile/)** — 用户画像与学习轨迹
"#;
            fs::write(&readme, content)
                .map_err(|e| format!("write README.md: {}", e))?;
        }

        let guide_note = self.root.join("notes").join("ohmywu-guide.md");
        if !guide_note.exists() {
            let content = r#"---
title: OhMyWu 使用引导
tags:
  - guide
  - onboarding
  - product
---

## 项目定位

OhMyWu 是一个本地优先的桌面 Agent 工作台，重点不是堆更多入口，而是把下面几件事做扎实：

- 对话可持续
- 工具调用可见
- 执行权限可控
- 知识可沉淀
- Action / Skill / Agent 能继续扩展

## 推荐起步顺序

1. 先去模型设置配置至少一套可用模型。
2. 再到设置里确认权限策略和 Agent Mode。
3. 按任务类型切换或编辑 Agent。
4. 需要长期记忆的内容，先写进知识库。
5. 最后回到对话页实际运行任务，并检查 Runtime。

## 页面说明

### 对话

- 发起任务、查看回复和工具执行过程
- 每条回复下方都能展开 Runtime 和工具调用
- 适合持续跟踪任务链路，而不是只看最终结果

### Agent 管理

- 管理多个 Agent 档案
- 可编辑名称、角色、人格、记忆范围
- 当前阶段以主 Agent 配置管理为主

### 知识库

- 管理长期有效的信息、偏好、规范和项目事实
- 支持手动确认“记忆候选”后再写入
- 图谱页可以查看笔记间链接关系

### 模型设置

- 管理多套模型配置
- 支持手动填写 Provider / Endpoint / Model
- 支持获取模型列表和测试连接
- 支持切换当前启用模型

### 原子化能力

- 查看底层 capability 注册情况
- 这里是 Action 和 Agent 真正调用的执行单元

### Action 注册

- 查看当前系统 Action
- 兼容 `SKILL.md` 生态，支持 Skill 转 Action

### 审计日志

- 查看关键执行记录
- 用于回溯权限决策和高风险动作

## 权限模式

### Policy Mode

- `Sandbox`：更保守，非只读能力会更容易被挡下
- `Danger`：允许进入执行阶段，再由规则和确认流程控制

### Agent Mode

- `plan`：偏分析，只暴露只读工具和 checklist
- `agent`：完整工具集，高风险默认需要确认
- `auto`：完整工具集，高风险可直接执行

## 知识库建议

- 把稳定事实写进知识库，不要把一次性上下文都塞进去
- 优先记录项目规则、接口约束、运行习惯和常见故障
- 记忆候选建议人工确认，避免错误内容沉淀

## 常见排查

### 拉不到模型列表

- 检查 Endpoint 是否正确
- 检查 API Key 是否有效
- 检查 API Format 是否匹配

### 工具没有执行

- 检查设置里的权限策略
- 看是否被 `Sandbox` 或 deny 规则挡住
- 到 Runtime 和审计日志里看具体原因

### 背景或主题不符合预期

- 纯色模式下分别调整背景色和主题色
- 图片模式下可重新提取背景主色
- 模糊、遮罩、缩放建议配合一起调

## 当前版本说明

当前版本是 `v0.2.0` 预览版，已经能跑通核心链路，但仍以优化测试为主。
"#;
            fs::write(&guide_note, content)
                .map_err(|e| format!("write guide note: {}", e))?;
        }

        // ensure index.md
        self.rebuild_index()?;

        Ok(())
    }

    // ── CRUD ──────────────────────────────────────────────────────

    pub fn list_notes(&self) -> Result<Vec<NoteMeta>, String> {
        let mut notes = Vec::new();
        self.walk_md_files(&self.root.clone(), &mut |path| {
            if let Ok(note) = self.read_note_file(path) {
                let backlinks = self.find_backlinks(&note.slug);
                notes.push(NoteMeta {
                    slug: note.slug,
                    title: note.title,
                    folder: note.folder,
                    tags: note.tags,
                    created: note.created,
                    updated: note.updated,
                    links_to: extract_links(&note.body),
                    linked_from: backlinks,
                    snippet: None,
                });
            }
        })?;
        // sort by updated desc
        notes.sort_by(|a, b| b.updated.cmp(&a.updated));
        Ok(notes)
    }

    pub fn read_note(&self, slug: &str) -> Result<WikiNote, String> {
        let path = self.find_by_slug(slug)?;
        let mut note = self.read_note_file(&path)?;
        note.file_path = path;
        Ok(note)
    }

    pub fn write_note(
        &self,
        slug: &str,
        title: &str,
        body: &str,
        tags: &[String],
        folder: &str,
    ) -> Result<WikiNote, String> {
        let slug = slugify(slug);
        let dir = if folder.is_empty() { "notes" } else { folder };
        let path = self.root.join(dir).join(format!("{}.md", &slug));

        // determine created timestamp
        let created = if path.exists() {
            // preserve existing
            match self.read_note_file(&path) {
                Ok(existing) => existing.created,
                Err(_) => ohmywu_domain::chrono_now(),
            }
        } else {
            ohmywu_domain::chrono_now()
        };
        let updated = ohmywu_domain::chrono_now();

        let mut file = fs::File::create(&path)
            .map_err(|e| format!("create {:?}: {}", path, e))?;
        write_frontmatter(
            &mut file,
            &Frontmatter {
                title: title.to_string(),
                tags: tags.to_vec(),
                created: Some(created.clone()),
                updated: Some(updated.clone()),
            },
        )?;
        write!(file, "\n{}", body).map_err(|e| format!("write body: {}", e))?;

        self.rebuild_index()?;

        Ok(WikiNote {
            slug,
            title: title.to_string(),
            folder: dir.to_string(),
            tags: tags.to_vec(),
            created,
            updated,
            body: body.to_string(),
            file_path: path,
        })
    }

    pub fn upsert_note(
        &self,
        current_slug: Option<&str>,
        slug: Option<&str>,
        title: &str,
        body: &str,
        tags: &[String],
        folder: &str,
    ) -> Result<WikiNote, String> {
        let base_slug = slug
            .filter(|value| !value.trim().is_empty())
            .or(current_slug)
            .unwrap_or(title);
        let next_slug = slugify(base_slug);
        if next_slug.is_empty() {
            return Err("slug/title 不能为空".into());
        }

        let target_folder = if folder.trim().is_empty() {
            "notes"
        } else {
            folder.trim()
        };
        let target_path = self.root.join(target_folder).join(format!("{}.md", next_slug));

        let existing = current_slug
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| self.read_note(value).ok());

        let note = self.write_note(&next_slug, title, body, tags, target_folder)?;

        if let Some(previous) = existing {
            if previous.file_path != target_path && previous.file_path.exists() {
                fs::remove_file(&previous.file_path)
                    .map_err(|e| format!("remove old note {:?}: {}", previous.file_path, e))?;
                self.rebuild_index()?;
            }
        }

        Ok(note)
    }

    pub fn delete_note(&self, slug: &str) -> Result<(), String> {
        let path = self.find_by_slug(slug)?;
        // safety: only delete .md files inside the wiki root
        if !path.starts_with(&self.root) || path.extension().is_none_or(|e| e != "md") {
            return Err(format!("refusing to delete outside wiki: {:?}", path));
        }
        fs::remove_file(&path).map_err(|e| format!("delete {:?}: {}", path, e))?;
        self.rebuild_index()?;
        Ok(())
    }

    // ── Search ────────────────────────────────────────────────────

    pub fn search(&self, query: &str) -> Result<Vec<NoteMeta>, String> {
        let q_lower = query.to_lowercase();
        let mut results: Vec<(NoteMeta, usize)> = Vec::new();

        self.walk_md_files(&self.root.clone(), &mut |path| {
            if let Ok(note) = self.read_note_file(path) {
                let mut score = 0usize;
                let title_lower = note.title.to_lowercase();
                let body_lower = note.body.to_lowercase();

                // title match = high score
                if title_lower.contains(&q_lower) {
                    score += 100;
                    // exact title match bonus
                    if title_lower == q_lower {
                        score += 50;
                    }
                }
                // tag match
                for tag in &note.tags {
                    if tag.to_lowercase().contains(&q_lower) {
                        score += 30;
                    }
                }
                // body match
                score += body_lower.matches(&q_lower).count() * 2;

                if score > 0 {
                    let backlinks = self.find_backlinks(&note.slug);
                    results.push((
                        NoteMeta {
                            slug: note.slug,
                            title: note.title,
                            folder: note.folder,
                            tags: note.tags,
                            created: note.created,
                            updated: note.updated,
                            links_to: extract_links(&note.body),
                            linked_from: backlinks,
                            snippet: Some(build_snippet(&note.body, &q_lower, 220)),
                        },
                        score,
                    ));
                }
            }
        })?;

        results.sort_by_key(|b| std::cmp::Reverse(b.1));
        Ok(results.into_iter().map(|(m, _)| m).collect())
    }

    // ── Graph ─────────────────────────────────────────────────────

    pub fn build_graph(&self) -> Result<GraphData, String> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut seen_nodes: HashSet<String> = HashSet::new();
        let mut seen_edges: HashSet<(String, String)> = HashSet::new();

        // collect all notes
        let notes = self.list_notes()?;
        let note_map: HashMap<String, NoteMeta> = notes
            .into_iter()
            .map(|n| (n.slug.clone(), n))
            .collect();

        for (slug, meta) in &note_map {
            if seen_nodes.insert(slug.clone()) {
                nodes.push(GraphNode {
                    id: slug.clone(),
                    label: meta.title.clone(),
                    folder: meta.folder.clone(),
                    tags: meta.tags.clone(),
                    link_count: meta.links_to.len() + meta.linked_from.len(),
                });
            }

            for target in &meta.links_to {
                let key = (slug.clone(), target.clone());
                if seen_edges.insert(key) {
                    edges.push(GraphEdge {
                        source: slug.clone(),
                        target: target.clone(),
                    });
                }
                // ensure target node exists even if not a real page yet
                if !note_map.contains_key(target) && seen_nodes.insert(target.clone()) {
                    nodes.push(GraphNode {
                        id: target.clone(),
                        label: target.clone(),
                        folder: "ghost".to_string(),
                        tags: vec![],
                        link_count: 0,
                    });
                }
            }
        }

        Ok(GraphData { nodes, edges })
    }

    pub fn recall(
        &self,
        query: &str,
        folders: &[String],
        limit: usize,
    ) -> Result<Vec<RecallHit>, String> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let folder_set: HashSet<String> = folders.iter().map(|folder| folder.to_lowercase()).collect();
        let mut hits = Vec::new();

        self.walk_md_files(&self.root.clone(), &mut |path| {
            if let Ok(note) = self.read_note_file(path) {
                if !folder_set.is_empty() && !folder_set.contains(&note.folder.to_lowercase()) {
                    return;
                }

                let title_lower = note.title.to_lowercase();
                let body_lower = note.body.to_lowercase();
                let mut score = 0usize;

                if title_lower.contains(&q) {
                    score += 100;
                    if title_lower == q {
                        score += 50;
                    }
                }
                for tag in &note.tags {
                    if tag.to_lowercase().contains(&q) {
                        score += 30;
                    }
                }
                score += body_lower.matches(&q).count() * 2;

                if score == 0 {
                    return;
                }

                hits.push(RecallHit {
                    slug: note.slug,
                    title: note.title,
                    folder: note.folder,
                    tags: note.tags,
                    updated: note.updated,
                    score,
                    snippet: build_snippet(&note.body, &q, 220),
                });
            }
        })?;

        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| b.updated.cmp(&a.updated)));
        hits.truncate(limit);
        Ok(hits)
    }

    // ── Helpers ───────────────────────────────────────────────────

    fn read_note_file(&self, path: &Path) -> Result<WikiNote, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("read {:?}: {}", path, e))?;

        let (fm, body) = parse_frontmatter(&content)?;

        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(WikiNote {
            slug,
            title: fm.title,
            folder: self.folder_for_path(path),
            tags: fm.tags,
            created: fm.created.unwrap_or_else(ohmywu_domain::chrono_now),
            updated: fm.updated.unwrap_or_else(ohmywu_domain::chrono_now),
            body: body.to_string(),
            file_path: path.to_path_buf(),
        })
    }

    fn find_by_slug(&self, slug: &str) -> Result<PathBuf, String> {
        let mut found: Option<PathBuf> = None;
        self.walk_md_files(&self.root.clone(), &mut |path| {
            if found.is_some() {
                return;
            }
            if path.file_stem().and_then(|s| s.to_str()) == Some(slug) {
                found = Some(path.to_path_buf());
            }
        })?;
        found.ok_or_else(|| format!("note '{}' not found", slug))
    }

    fn find_backlinks(&self, target_slug: &str) -> Vec<String> {
        let mut backlinks = Vec::new();
        let _ = self.walk_md_files(&self.root.clone(), &mut |path| {
            if let Ok(content) = fs::read_to_string(path) {
                let (_, body) = parse_frontmatter(&content).unwrap_or_default();
                let links = extract_links(&body);
                if links.contains(&target_slug.to_string())
                    && let Some(slug) = path.file_stem().and_then(|s| s.to_str())
                    && slug != target_slug
                {
                    backlinks.push(slug.to_string());
                }
            }
        });
        backlinks.sort();
        backlinks.dedup();
        backlinks
    }

    fn walk_md_files(
        &self,
        dir: &PathBuf,
        cb: &mut dyn FnMut(&Path),
    ) -> Result<(), String> {
        if !dir.exists() {
            return Ok(());
        }
        let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {:?}: {}", dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("entry: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                // skip .obsidian config dir
                if path.file_name().is_some_and(|n| n == ".obsidian") {
                    continue;
                }
                self.walk_md_files(&path, cb)?;
            } else if path.extension().is_some_and(|e| e == "md") {
                // skip index / readme for listing purposes
                let is_meta = path
                    .file_name()
                    .is_some_and(|n| n == "index.md" || n == "README.md");
                if !is_meta {
                    cb(&path);
                }
            }
        }
        Ok(())
    }

    fn rebuild_index(&self) -> Result<(), String> {
        let notes = {
            let mut notes = Vec::new();
            self.walk_md_files(&self.root.clone(), &mut |path| {
                if let Ok(note) = self.read_note_file(path) {
                    notes.push(IndexEntry {
                        slug: note.slug,
                        title: note.title,
                        folder: note.folder,
                        tags: note.tags,
                        updated: note.updated,
                    });
                }
            })?;
            notes.sort_by(|a, b| b.updated.cmp(&a.updated));
            notes
        };

        let mut index = String::from("# 知识库索引\n\n");
        for entry in &notes {
            index.push_str(&format!(
                "- [{}]({})\n  - 范围: {}\n  - 标签: {}\n  - 更新: {}\n\n",
                entry.title,
                entry.slug,
                entry.folder,
                entry.tags.join(", "),
                entry.updated,
            ));
        }

        let path = self.root.join("index.md");
        fs::write(&path, index).map_err(|e| format!("write index.md: {}", e))?;
        Ok(())
    }

    fn folder_for_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .ok()
            .and_then(|relative| relative.components().next())
            .and_then(|component| component.as_os_str().to_str())
            .unwrap_or("notes")
            .to_string()
    }
}

// ── Frontmatter parsing ────────────────────────────────────────────

fn parse_frontmatter(content: &str) -> Result<(Frontmatter, String), String> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Ok((
            Frontmatter {
                title: "Untitled".into(),
                tags: vec![],
                created: None,
                updated: None,
            },
            content.to_string(),
        ));
    }

    let rest = &content[3..]; // skip first ---
    let end = rest.find("---").unwrap_or(0);
    let fm_str = &rest[..end].trim();
    let body = rest[end + 3..].trim().to_string();

    let fm: Frontmatter = serde_yaml::from_str(fm_str)
        .map_err(|e| format!("parse frontmatter: {}", e))?;

    Ok((fm, body))
}

fn write_frontmatter(file: &mut fs::File, fm: &Frontmatter) -> Result<(), String> {
    let yaml = serde_yaml::to_string(fm)
        .map_err(|e| format!("serialize frontmatter: {}", e))?;
    write!(file, "---\n{}---\n", yaml).map_err(|e| format!("write fm: {}", e))?;
    Ok(())
}

// ── Wiki link parsing ──────────────────────────────────────────────

/// Extract linked slugs from markdown body.
/// Supports `[[slug]]` and `[[slug|text]]`.
pub fn extract_links(body: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = body.chars().collect();
    while i < chars.len().saturating_sub(1) {
        if chars[i] == '[' && chars[i + 1] == '[' {
            let start = i + 2;
            if let Some(end) = chars[start..].iter().position(|&c| c == ']')
                && chars.get(start + end + 1) == Some(&']')
            {
                let inner: String = chars[start..start + end].iter().collect();
                let slug = inner.split('|').next().unwrap_or(&inner);
                links.push(slug.trim().to_string());
                i = start + end + 2;
                continue;
            }
        }
        i += 1;
    }
    links.sort();
    links.dedup();
    links
}

// ── Utilities ──────────────────────────────────────────────────────

fn build_snippet(body: &str, query_lower: &str, max_chars: usize) -> String {
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let collapsed_lower = collapsed.to_lowercase();
    if let Some(index) = collapsed_lower.find(query_lower) {
        let char_positions = collapsed
            .char_indices()
            .map(|(byte_index, _)| byte_index)
            .chain(std::iter::once(collapsed.len()))
            .collect::<Vec<_>>();
        let match_char_index = char_positions
            .partition_point(|byte_index| *byte_index < index);
        let query_char_len = query_lower.chars().count().max(1);
        let total_chars = char_positions.len().saturating_sub(1);
        let start_char = match_char_index.saturating_sub(max_chars / 3);
        let end_char = (match_char_index + query_char_len + (max_chars * 2 / 3)).min(total_chars);
        let start = char_positions[start_char];
        let end = char_positions[end_char];
        let prefix = if start_char > 0 { "…" } else { "" };
        let suffix = if end_char < total_chars { "…" } else { "" };
        return format!("{}{}{}", prefix, &collapsed[start..end], suffix);
    }

    let snippet: String = collapsed.chars().take(max_chars).collect();
    if collapsed.chars().count() > max_chars {
        format!("{}…", snippet)
    } else {
        snippet
    }
}

/// Convert a title/name into a file-system-safe slug.
fn slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(80)
        .collect()
}

impl Default for WikiEngine {
    fn default() -> Self {
        Self::new(PathBuf::from("wiki"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Rust 生命週期"), "rust-生命週期");
        assert_eq!(slugify("async/await"), "async-await");
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
title: "Test Note"
tags: [rust, memory]
created: "2026-05-14T10:00:00Z"
---

This is the body.

With multiple lines.
"#;
        let (fm, body) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.title, "Test Note");
        assert_eq!(fm.tags, vec!["rust", "memory"]);
        assert_eq!(fm.created.unwrap(), "2026-05-14T10:00:00Z");
        assert!(body.starts_with("This is the body"));
    }

    #[test]
    fn test_parse_frontmatter_no_fm() {
        let content = "Just a plain note.";
        let (fm, body) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.title, "Untitled");
        assert_eq!(body, "Just a plain note.");
    }

    #[test]
    fn test_extract_links() {
        let body = "See also [[rust-lifetimes]] and [[ownership]].\n\nRelated: [[rust-lifetimes|Rust lifetimes]].";
        let links = extract_links(body);
        assert!(links.contains(&"rust-lifetimes".to_string()));
        assert!(links.contains(&"ownership".to_string()));
    }

    #[test]
    fn test_extract_links_no_links() {
        let body = "No wikilinks here.";
        let links = extract_links(body);
        assert!(links.is_empty());
    }

    #[test]
    fn test_write_and_read_and_search() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = WikiEngine::new(tmp.path().to_path_buf());
        engine.init().unwrap();

        // write a note
        let note = engine
            .write_note(
                "rust-ownership",
                "Rust 所有权",
                "所有权是 Rust 的核心概念。参见 [[borrowing]]。",
                &["rust".into(), "concept".into()],
                "concepts",
            )
            .unwrap();
        assert_eq!(note.slug, "rust-ownership");
        assert_eq!(note.title, "Rust 所有权");

        // write another
        engine
            .write_note(
                "borrowing",
                "借用",
                "借用允许在不转移所有权的情况下使用值。参见 [[rust-ownership]]。",
                &["rust".into(), "concept".into()],
                "concepts",
            )
            .unwrap();

        // read
        let note = engine.read_note("rust-ownership").unwrap();
        assert!(note.body.contains("所有权"));

        // backlinks
        let metas = engine.list_notes().unwrap();
        let owner_meta = metas.iter().find(|m| m.slug == "rust-ownership").unwrap();
        assert!(owner_meta.linked_from.contains(&"borrowing".to_string()));

        // search
        let results = engine.search("所有权").unwrap();
        assert_eq!(results.len(), 2); // both notes mention 所有权

        let results = engine.search("核心概念").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slug, "rust-ownership");

        let results = engine.search("rust").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_build_graph() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = WikiEngine::new(tmp.path().to_path_buf());
        engine.init().unwrap();

        engine
            .write_note(
                "rust-ownership",
                "Ownership",
                "See [[borrowing]] and [[lifetimes]].",
                &["rust".into()],
                "concepts",
            )
            .unwrap();
        engine
            .write_note(
                "borrowing",
                "Borrowing",
                "Related to [[rust-ownership]] and [[lifetimes]].",
                &["rust".into()],
                "concepts",
            )
            .unwrap();

        let graph = engine.build_graph().unwrap();
        assert_eq!(graph.nodes.len(), 3); // ownership, borrowing, lifetimes (ghost)
        assert!(graph.edges.len() >= 2);
    }

    #[test]
    fn test_delete_note() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = WikiEngine::new(tmp.path().to_path_buf());
        engine.init().unwrap();

        engine
            .write_note("test-note", "Test", "Body", &[], "notes")
            .unwrap();
        assert!(engine.read_note("test-note").is_ok());

        engine.delete_note("test-note").unwrap();
        assert!(engine.read_note("test-note").is_err());
    }

    #[test]
    fn test_recall_scoped() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = WikiEngine::new(tmp.path().to_path_buf());
        engine.init().unwrap();

        engine
            .write_note(
                "rust-memory",
                "Rust Memory",
                "Rust 的所有权和内存模型非常关键。",
                &["rust".into()],
                "concepts",
            )
            .unwrap();
        engine
            .write_note(
                "personal-memory",
                "Personal Memory",
                "我喜欢把知识沉淀进知识库。",
                &["profile".into()],
                "profile",
            )
            .unwrap();

        let concept_hits = engine.recall("内存", &["concepts".into()], 5).unwrap();
        assert_eq!(concept_hits.len(), 1);
        assert_eq!(concept_hits[0].folder, "concepts");

        let profile_hits = engine.recall("知识库", &["profile".into()], 5).unwrap();
        assert_eq!(profile_hits.len(), 1);
        assert_eq!(profile_hits[0].folder, "profile");
    }

    #[test]
    fn test_build_snippet_is_utf8_safe() {
        let body = "这是一个中文段落，里面有数字 12345 和更多内容，用来验证搜索截断不会崩溃。";
        let snippet = build_snippet(body, "1", 18);
        assert!(snippet.contains('1'));
        assert!(!snippet.is_empty());
    }
}
