# Agent 工具链升级计划

> 参考 Claude Code 架构设计，把 OhMyWu agent 从 2 个工具扩展到完整工具链
> 参考：`book/claude-reference.md`

**目标：** 让 OhMyWu Agent 拥有类似 Claude Code 的工具链能力——多工具、并行执行、权限分级、子 Agent、大结果管理

**架构思路：**
- 工具注册由 `tools.rs` 集中管理，每个工具是一个独立模块
- executor 从"capability 执行器"升级为"工具调度中心"
- 权限系统从 Sandbox/Danger 两档扩展为工具级 allow/deny + 用户确认
- 参考 Claude Code 的 9 步执行管道和 streaming early execution

**当前状态：** 2 个工具（bash/read），简单 loop，两档权限

---

## 阶段 1：工具扩展 — 从 2 个到 8+ 个

### 新增工具清单

| 工具 | 类型 | 风险 | 说明 |
|------|------|------|------|
| `bash` | 已有 | HighRisk | 执行 shell 命令 |
| `read` | 已有 | ReadOnly | 读取文件 |
| `write` | 新增 | HighRisk | 写入文件内容 |
| `edit` | 新增 | HighRisk | 精确字符串替换编辑（如 Claude Code Edit） |
| `glob` | 新增 | ReadOnly | 文件模式匹配搜索 |
| `grep` | 新增 | ReadOnly | 文件内容搜索 |
| `web_fetch` | 新增 | ReadOnly | 获取 URL 内容 |
| `thinking` | 新增 | None | 模型内部推理步骤（不执行外部操作） |

### 实施步骤

#### 1a. 工具注册表重构

**改 `src-tauri/src/tools.rs`：**
- 删除对 `CapabilityRegistry` 的依赖（不再通过 capability 间接注册）
- 改为直接注册工具清单，每个工具带 risk_level 标签
- 新增 `ToolKind` 枚举区分 ReadOnly / HighRisk / None

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolKind {
    ReadOnly,
    HighRisk,
    None_,  // e.g. thinking
}

pub struct ToolMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: serde_json::Value,
    pub kind: ToolKind,
    pub requires_confirmation: bool,  // HighRisk 工具默认 true
    pub concurrency_safe: bool,       // ReadOnly 工具默认 true
}

pub fn all_tools() -> Vec<ToolMeta> {
    vec![
        ToolMeta {
            name: "bash",
            description: "执行 shell 命令",
            parameters: json!({...}),
            kind: ToolKind::HighRisk,
            requires_confirmation: true,
            concurrency_safe: false,
        },
        // ...
    ]
}

/// 转换成 LLM 可用的 ToolDef 列表（不含 deferred）
pub fn active_tool_defs(state: &AppState) -> Vec<ToolDef> { ... }
```

#### 1b. 新增工具执行模块

**新增 `src-tauri/src/tools/` 目录，每个工具一个文件：**

- `tools/mod.rs` — 重新导出 + 调度函数
- `tools/bash.rs` — 从 executor.rs 迁移
- `tools/read.rs` — 从 executor.rs 迁移
- `tools/write.rs` — 新实现
- `tools/edit.rs` — 新实现（精确字符串替换）
- `tools/glob.rs` — 新实现
- `tools/grep.rs` — 新实现
- `tools/web_fetch.rs` — 新实现
- `tools/thinking.rs` — 空操作，仅记录

**核心调度函数：**

```rust
pub async fn dispatch_tool(
    state: &AppState,
    name: &str,
    params: serde_json::Value,
    app_handle: Option<&tauri::AppHandle>,
) -> ToolResult {
    match name {
        "bash" => bash::execute(state, params).await,
        "read" => read::execute(state, params).await,
        "write" => write::execute(state, params).await,
        "edit" => edit::execute(state, params).await,
        "glob" => glob::execute(state, params).await,
        "grep" => grep::execute(state, params).await,
        "web_fetch" => web_fetch::execute(state, params).await,
        "thinking" => thinking::execute(state, params).await,
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
```

#### 1c. 工具实现细节

**`write`：**
- 参数：`path: String`, `content: String`
- 写前检查父目录是否存在，不存在则创建
- 返回写入字节数

**`edit`（参考 Claude Code Edit 工具）：**
- 参数：`file_path: String`, `old_string: String`, `new_string: String`
- 读取文件 → 精确匹配 old_string → 替换为 new_string → 写回
- old_string 必须唯一匹配，否则返回错误指示需要更多上下文
- 返回替换后的行号范围

```rust
pub fn edit_file(path: &str, old: &str, new: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取失败: {}", e))?;
    let count = content.matches(old).count();
    if count == 0 {
        return Err("未找到匹配的字符串。可能需要查看最新文件内容。".into());
    }
    if count > 1 {
        return Err(format!("找到 {} 处匹配，需要更精确的匹配范围", count));
    }
    let new_content = content.replace(old, new);
    std::fs::write(path, &new_content)
        .map_err(|e| format!("写入失败: {}", e))?;
    Ok(format!("已替换 1 处"))
}
```

**`glob`：**
- 参数：`pattern: String`, `path: Option<String>`（可选根目录）
- 使用 `glob` crate 或 `walkdir` + 手动匹配
- 返回匹配文件列表（最多 50 条）

**`grep`：**
- 参数：`pattern: String`, `path: Option<String>`, `include: Option<String>`
- 使用 `grep` crate 或直接调用 `grep -rn`
- 返回匹配行（最多 30 条）

**`web_fetch`：**
- 参数：`url: String`
- 使用 reqwest GET 请求（30s 超时）
- 返回 HTML→text 转换内容（最多 10000 字）

---

## 阶段 2：权限系统升级

### 2a. 工具级权限规则

**改 `src-tauri/src/config.rs`：**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRules {
    pub allow: Vec<String>,       // 工具名或模式，如 "read", "glob", "bash(ls *)"
    pub deny: Vec<String>,        // 同上但优先级更高
    pub require_confirm: Vec<String>,  // 需要用户确认的工具
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionConfig {
    pub mode: PermissionMode,
    pub rules: Option<PermissionRules>,
}
```

**改 `src-tauri/src/agent.rs` 权限门控：**

```rust
enum PermissionDecision {
    Allow,
    Deny(String),
    Confirm,
}

fn check_tool_permission(
    state: &AppState,
    tool_name: &str,
    params: &Value,
) -> PermissionDecision {
    // 1. 检查工具自身 risk_level
    let meta = find_tool_meta(tool_name);
    
    // 2. 检查 deny 规则（拒绝优先）
    if is_denied(&state.config, tool_name, params) {
        return PermissionDecision::Deny("被安全规则拒绝".into());
    }
    
    // 3. 检查 allow 规则
    let is_allowed = is_allowed(&state.config, tool_name, params);
    
    // 4. 根据权限模式决策
    match state.policy.current_mode() {
        PolicyMode::Sandbox => {
            match meta.kind {
                ToolKind::ReadOnly => PermissionDecision::Allow,
                ToolKind::HighRisk => PermissionDecision::Deny("Sandbox 模式下禁止高风险操作".into()),
                ToolKind::None_ => PermissionDecision::Allow,
            }
        }
        PolicyMode::Danger => {
            if meta.requires_confirmation {
                PermissionDecision::Confirm
            } else {
                PermissionDecision::Allow
            }
        }
    }
}
```

### 2b. 用户确认交互（高风险弹窗）

在 agent loop 中，当 `check_tool_permission` 返回 `Confirm` 时：
1. 暂停 agent loop
2. 向前端发送确认事件 `tool-confirm-request`
3. 前端弹窗显示：工具名 + 参数（对敏感参数部分遮盖）
4. 用户选择允许/拒绝/允许本次会话
5. 结果回传给 agent loop 继续执行

前端新增 `ConfirmDialog.vue` 组件，在 ChatView 中监听 `tool-confirm-request` 事件。

---

## 阶段 3：Agent Loop 增强

### 3a. Streaming 早期执行（参考 Claude Code）

**改 `src-tauri/src/agent.rs`：**
- streaming 过程中，检测到 tool_use 事件后立即分派只读工具（不等待整个响应完成）
- 可并发工具分组并行执行

```rust
// 流式处理中的 tool_use 分派
let concurrency_safe = HashSet::from(["read", "glob", "grep", "web_fetch"]);

while let Some(chunk) = stream.next().await {
    if let Some(tc) = chunk.tool_call_delta
        && tc.done
        && concurrency_safe.contains(tc.name)
    {
        // 提前执行：不等完整响应
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            dispatch_tool(&state_clone, tc.name, tc.params).await
        });
        early_results.insert(tc.id, handle);
    }
    // 继续处理 streaming...
}
```

### 3b. 大结果持久化

参考 Claude Code：>30KB 的工具结果自动截断。

```rust
const LARGE_RESULT_THRESHOLD: usize = 30 * 1024;
const PREVIEW_LINES: usize = 200;

fn format_tool_result(result: &str) -> String {
    if result.len() <= LARGE_RESULT_THRESHOLD {
        return result.to_string();
    }
    // 预览：显示前 200 行
    let preview: String = result.lines().take(PREVIEW_LINES).collect::<Vec<_>>().join("\n");
    format!("{}\n\n... [结果过长，已截断。共 {} 字节，仅显示前 {} 行]", 
        preview, result.len(), PREVIEW_LINES)
}
```

### 3c. 重试机制

参考 Claude Code 的指数退避重试（HTTP 429/503/529）：

```rust
async fn with_retry<F, T>(f: F) -> Result<T, LlmError>
where
    F: Fn() -> Future<Output = Result<T, LlmError>>,
{
    let mut last_err = None;
    for attempt in 0..3 {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if is_retryable(&e) {
                    let delay = Duration::from_millis(1000 * 2u64.pow(attempt));
                    tokio::time::sleep(delay).await;
                    last_err = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }
    Err(last_err.unwrap())
}
```

---

## 阶段 4：System Prompt 增强

### 动态变量注入

**改 system prompt：**

```rust
fn build_system_prompt(state: &AppState) -> String {
    format!("\
你是 OhMyWu，一个帮助用户管理电脑的桌面 AI 助手。

## 可用工具
{}

## 当前策略模式
{:?}

## 规则
- 执行可能造成破坏的命令前，先向用户确认
- 使用中文回复，保持简洁
- 直接跑在用户电脑上，拥有本地执行能力
- 对于复杂任务，先用 thinking 工具规划步骤
",
        active_tool_descriptions(state),
        state.policy.current_mode(),
    )
}
```

动态工具描述：根据当前权限模式，只列出实际可用的工具。

---

## 阶段 5：子 Agent 系统（进阶）

参考 Claude Code 的子 Agent 类型：

### Agent 配置

**新增 `crates/agent-registry/` 或直接 `src-tauri/src/agents/`：**

```rust
pub enum AgentKind {
    Explore,    // 只读搜索
    Plan,       // 任务规划
    General,    // 通用
}
```

### 子 Agent 执行

- 新建独立的 agent loop（共享 AppState 引用）
- 父 agent 等待子 agent 结果摘要
- 完整转录写入侧链 JSONL，不进入父级上下文
- 子 agent 不可生子 agent（防止任务爆炸）

### 触发方式

1. **自动分解**：主 agent 遇到复杂多步任务时，创建 Plan agent 先规划
2. **手动创建**：用户指定 "用 explore 模式搜索 X"

---

## 文件变更汇总

### 新文件

| 文件 | 阶段 | 说明 |
|------|------|------|
| `src-tauri/src/tools/mod.rs` | 1 | 工具注册表 + 调度 |
| `src-tauri/src/tools/bash.rs` | 1 | 从 executor.rs 迁移 |
| `src-tauri/src/tools/read.rs` | 1 | 从 executor.rs 迁移 |
| `src-tauri/src/tools/write.rs` | 1 | 写入文件 |
| `src-tauri/src/tools/edit.rs` | 1 | 精确字符串替换 |
| `src-tauri/src/tools/glob.rs` | 1 | 文件搜索 |
| `src-tauri/src/tools/grep.rs` | 1 | 内容搜索 |
| `src-tauri/src/tools/web_fetch.rs` | 1 | URL 获取 |
| `src-tauri/src/tools/thinking.rs` | 1 | 推理步骤 |
| `src/components/ConfirmDialog.vue` | 2 | 高风险操作确认弹窗 |

### 修改文件

| 文件 | 阶段 | 改动 |
|------|------|------|
| `src-tauri/src/tools.rs` | 1 | 重写为工具注册表 |
| `src-tauri/src/executor.rs` | 1 | 简化为调度到各工具模块 |
| `src-tauri/src/agent.rs` | 1-3 | 新增 early execution、重试、权限门控 |
| `src-tauri/src/config.rs` | 2 | PermissionRules + PermissionConfig |
| `src-tauri/src/lib.rs` | 1-2 | 注册新 commands（若有） |
| `src/views/SettingsView.vue` | 2 | 权限规则配置 UI |

## 验证

1. `cargo build && cargo clippy`
2. Agent 能调用 `write` 创建新文件
3. Agent 能调用 `edit` 精确修改文件指定内容
4. Agent 能调用 `glob`/`grep` 搜索文件和内容
5. 只读工具（read/glob/grep）在流式响应中提前执行
6. 高风险工具在 Sandbox 模式下被拒绝，在 Danger 模式下弹出确认
7. >30KB 工具结果自动截断
8. 重试机制：断网恢复后自动重试（最多 3 次）
