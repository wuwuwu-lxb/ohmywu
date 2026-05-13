# OhMyWu 实施计划

> 最后更新：2026-05-14

## 当前状态

M0 框架搭建已完成，但存在关键缺口：

- **ChatView 完全是 mock** — 没有真正的 Tauri IPC，800ms 假延迟 + 硬编码回复
- **没有执行管道** — bash/read 只是注册了元数据，从未真正调用
- **没有持久化** — 所有状态在内存中，重启即丢失
- **没有 LLM 接入** — Agent 实际上不会思考和回复
- **Policy 引擎和 Audit 日志从未被调用** — 存在但闲置

## 数据目录

```
~/.ohmywu/
  sessions/                   # JSONL 会话文件
    session-20260514-001.jsonl
    session-20260514-002.jsonl
  actions/                    # 用户 Action
    restart-nginx/
      README.md               # 口语化描述
      script.sh               # 可选脚本
      manifest.json            # 自动生成元数据
  wiki/                       # LLM-Wiki
    部署流程.md
    nginx配置模板.md
  config.json                 # 应用配置
```

---

## Phase 1：基础 — 会话持久化 + 真实执行

### 目标

bash/read 真正可执行，走通 policy→task→audit 全链路，会话存为 JSONL 文件，前端接入真实后端。

### 1.1 修复 chrono_now() — domain 共享工具

**改 `crates/domain/Cargo.toml`** — 添加 `chrono` 依赖：
```toml
chrono = { version = "0.4", features = ["serde"] }
```

**改 `crates/domain/src/lib.rs`** — 添加：
```rust
pub fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}
```

**改 `crates/task-engine/src/lib.rs`** 和 **`crates/audit/src/lib.rs`** — 删除各自私有的 `chrono_now()`，改用 `domain::chrono_now()`。

### 1.2 数据目录初始化

**新增 `src-tauri/src/data_dir.rs`：**
- 用 `dirs::home_dir()` 获取 `~/.ohmywu/`
- 确保 `sessions/`、`actions/`、`wiki/` 子目录存在
- 若无则递归创建

**新增 `src-tauri/src/config.rs`：**
```rust
pub struct AppConfig {
    pub policy_mode: PolicyMode,   // Sandbox / Danger
    pub theme: String,             // midnight / slate / amber
    pub accent: String,            // #3b82f6
    pub llm_provider: Option<LlmConfig>,
}

pub struct LlmConfig {
    pub provider_type: String,     // "ollama" | "openai_compatible"
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
}
```
- `load_config(data_dir)` — 读 config.json，不存在则返回默认值
- `save_config(data_dir, config)` — 先写临时文件再 rename（原子写入）

**src-tauri/Cargo.toml 新增依赖：** `dirs`

### 1.3 新 crate：session（JSONL 会话持久化）

**新增 `crates/session/Cargo.toml`** + **`crates/session/src/lib.rs`**

```rust
pub struct SessionManager {
    sessions_dir: PathBuf,
    write_lock: Mutex<()>,  // 防止并发写入交错
}

pub struct SessionMessage {
    pub role: String,           // "user" | "agent"
    pub content: String,
    pub executions: Option<Vec<ExecutionRecord>>,
    pub task_id: Option<String>,
    pub timestamp: String,
}

pub struct ExecutionRecord {
    pub capability: String,
    pub input: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub status: String,         // "success" | "failed"
    pub duration_ms: u64,
}

pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
}
```

**API：**
- `create_session(name) → SessionSummary` — 创建新 JSONL 文件，ID 格式 `session-{YYYYMMDD}-{counter}`
- `append_message(session_id, msg)` — 追加一行 JSON 到文件
- `load_session(session_id) → Vec<SessionMessage>` — 读取所有行，逐行解析 JSON
- `list_sessions() → Vec<SessionSummary>` — 扫描目录，读每个文件的头尾
- `delete_session(session_id)` — 删除 JSONL 文件

**依赖：** serde, serde_json, ohmywu-domain

**Workspace 更新：** 根 `Cargo.toml` members 添加 `crates/session`，`src-tauri/Cargo.toml` 添加 `ohmywu-session` 依赖。

### 1.4 真实能力执行器

**新增 `src-tauri/src/executor.rs`**

```rust
pub struct ExecuteRequest {
    pub capability: String,  // "bash" | "read"
    pub params: Value,       // { "command": "..." } 或 { "path": "..." }
}

pub struct ExecuteResult {
    pub capability: String,
    pub status: String,          // "success" | "failed" | "denied"
    pub output: Option<String>,
    pub error: Option<String>,
    pub task_id: String,
    pub duration_ms: u64,
    pub policy_decision: String, // "allowed" | "denied"
}
```

**`execute_capability(state, request) → ExecuteResult`** 执行管道：

1. **能力查找** — `state.capabilities.get(name)`，找不到则返回错误
2. **策略门控** — `state.policy.check(cap.risk_level)`，Sandbox 模式下拒绝 HighRisk
3. **创建 Task** — `state.tasks.create(name, &target)`，状态为 Running
4. **执行**（在 `spawn_blocking` 中）：
   - `bash`：`std::process::Command::new("sh").arg("-c").arg(command).output()`
   - `read`：`std::fs::read_to_string(path)`
5. **Task 更新** — 成功则 `complete(id, &output)`，失败则 `fail(id, &error)`
6. **审计记录** — `state.audit.record(actor, action, target, risk, status, detail)`
7. **返回结果**

**安全措施：**
- 工作目录设为用户 home
- 30 秒超时（`tokio::time::timeout`）
- 输出截断到 10000 字符
- 不依赖 tauri-plugin-shell 的 execute（直接用 std::process::Command）

### 1.5 AppState 重构 + 新 Tauri commands

**改 `src-tauri/src/lib.rs`**

AppState 变为：
```rust
pub struct AppState {
    pub capabilities: Arc<CapabilityRegistry>,
    pub actions: Arc<ActionRegistry>,
    pub policy: Arc<PolicyEngine>,
    pub tasks: Arc<TaskEngine>,
    pub audit: Arc<AuditLog>,
    pub session: Arc<SessionManager>,       // 新增
    pub config: Arc<RwLock<AppConfig>>,     // 新增
    pub data_dir: PathBuf,                  // 新增
}
```

**新增 Tauri commands（全部 async）：**

| Command | 功能 |
|---------|------|
| `execute_capability(request)` | 执行 bash/read 能力 |
| `set_policy_mode(mode)` | 切换 Sandbox/Danger + 持久化 |
| `create_session(name)` | 创建新会话 |
| `list_sessions()` | 列出所有会话 |
| `load_session(session_id)` | 加载会话消息 |
| `send_message(session_id, content)` | 发送消息（Phase 1 mock，Phase 2 接入 LLM） |
| `get_config()` | 读取配置 |
| `save_config(config)` | 保存配置 |

**Phase 1 的 `send_message` mock 实现：**
- 保存用户消息到 session JSONL
- 解析输入：`read <path>` → 调用 execute_capability("read")，`run <cmd>` → 调用 execute_capability("bash")
- 其他输入 → 返回帮助信息
- 保存 agent 消息到 session JSONL
- 目的是在 LLM 接入前就能端到端测试全链路

### 1.6 前端：Pinia + 真实 ChatView

**安装 Pinia：** `npm install pinia`

**新增 `src/stores/chat.ts`：**
- `messages: ChatMsg[]` — 当前会话消息
- `sessions: SessionSummary[]` — 会话列表
- `currentSessionId: string | null`
- `pending: boolean` — 等待回复
- `streamingContent: string` — 流式 token（Phase 2 用）
- Actions：`sendMessage()`, `loadSession()`, `createSession()`, `listSessions()`

**重写 `src/views/ChatView.vue`：**
- 用 `useChatStore()` 替代 mock 逻辑
- `send()` 调用 `invoke("send_message", { sessionId, content })`
- 顶部会话选择器（下拉切换/新建）
- 启动时自动加载最后一个会话或创建新会话

### 1.7 修 bug

**Sidebar 折叠：**
- `--sidebar-collapsed-w` 从 `0px` 改为 `36px`
- 移除 `pointer-events: none` 和 `opacity: 0`
- 折叠时只显示 toggle 按钮

**RightPanel 接入：**
- `App.vue` 添加 `handleShowTask(taskId)` → 设 `rightPanelOpen = true`
- `ChatView.vue` 监听 `@show-task` 事件并向上 emit
- `RightPanel.vue` 根据 taskId 显示任务详情

### Phase 1 完成标准

- `cargo check` ✅
- `vue-tsc --noEmit` ✅
- `vite build` ✅
- 对话中输入 `run ls -la` 能看到真实命令执行结果
- 对话中输入 `read /etc/hostname` 能看到文件内容
- 关掉重开 → 之前的对话还在
- Sandbox 模式下 bash 命令被拒绝
- 侧栏折叠/展开正常
- 点"查看执行链路"能打开右侧面板

---

## Phase 2：LLM 接入

### 目标

接入 Ollama（本地）和 OpenAI 兼容 API（云端），实现真正的 Agent 对话循环，tool calling 对接 bash/read 能力。

### 2.1 新 crate：llm-adapter

**新增 `crates/llm-adapter/Cargo.toml`** + **源码文件：**

| 文件 | 内容 |
|------|------|
| `types.rs` | ChatMessage, ToolCall, ToolDef, ChatResponse, ChatStreamChunk |
| `lib.rs` | `LlmProvider` trait 定义 + `create_provider()` 工厂 |
| `ollama.rs` | Ollama `/api/chat` 实现（原生 tool calling） |
| `openai_compat.rs` | OpenAI `/v1/chat/completions` 实现（SSE 流式） |

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage], tools: &[ToolDef])
        -> Result<ChatResponse, String>;

    async fn chat_stream(&self, messages: &[ChatMessage], tools: &[ToolDef])
        -> Result<Box<dyn Stream<Item = Result<ChatStreamChunk, String>> + Unpin + Send>, String>;
}
```

**依赖：** serde, serde_json, reqwest (json feature), tokio, futures, async-trait

### 2.2 能力 → Tool 转换

**新增 `src-tauri/src/tools.rs`：**
```rust
pub fn capabilities_as_tools(state: &AppState) -> Vec<ToolDef>
```

把注册的 bash/read 能力转成 OpenAI function calling JSON Schema：
```json
{
  "name": "bash",
  "description": "执行 shell 命令。受策略引擎控制。",
  "parameters": {
    "type": "object",
    "properties": {
      "command": { "type": "string", "description": "要执行的 shell 命令" }
    },
    "required": ["command"]
  }
}
```

### 2.3 Agent 对话循环

**新增 `src-tauri/src/agent.rs`**

```
用户输入
  → 构建消息列表（system prompt + session 历史 + 用户消息）
  → LLM.chat(messages, tools)
  → 如果返回 content：直接作为最终回复
  → 如果返回 tool_calls：
      → 对每个 tool call 调用 executor::execute_capability
      → 把 tool 结果追加到消息列表
      → 继续调 LLM（最多 10 轮）
```

**System prompt：**
```
你是 OhMyWu，一个帮助用户管理电脑的桌面 AI 助手。
你可以执行 shell 命令和读取文件来帮助用户解决问题。
在执行可能有破坏性的操作前，先向用户确认。
默认使用中文回复，保持简洁。
```

**流式支持：** 当有 stream_emitter 时，通过 Tauri event `chat-stream` 推送 token delta。

### 2.4 替换 mock send_message

把 Phase 1 的 mock 实现替换为真实 agent_loop。

### 2.5 前端流式 + LLM 设置 UI

**ChatView 流式显示：**
- 用 `listen("chat-stream", ...)` 监听 token 事件
- 实时追加到当前 agent 消息
- 收到 `done: true` 后定稿消息

**SettingsView LLM 配置区：**
- Provider 下拉：Ollama / OpenAI Compatible / 无
- Endpoint 输入框（默认 `http://localhost:11434`）
- Model 输入框（默认 `qwen2.5`）
- API Key 密码框（仅 OpenAI Compatible 时显示）
- 保存按钮 → `invoke("save_config", ...)`

### Phase 2 完成标准

- 配置 Ollama 端点，发送消息，Agent 真实回复
- Agent 能主动调用 bash/read 工具（如用户问"我桌面有什么文件"，Agent 调 `ls ~/Desktop`）
- 流式 token 实时显示
- 切换 OpenAI Compatible 端点也能正常工作

---

## Phase 3：Action 管道

### 目标

审计日志 → 用户判断可复用 → 生成 Action（Markdown + 可选脚本）→ 注册供后续调用。

### 3.1 Action 文件结构

```
~/.ohmywu/actions/
  restart-nginx/
    README.md       # 口语化描述 + 使用说明
    script.sh        # 可选：可执行脚本
    manifest.json    # 自动生成：name, description, capabilities, created_at
```

**manifest.json 示例：**
```json
{
  "name": "restart-nginx",
  "description": "重启 nginx 并验证状态",
  "capabilities": ["bash"],
  "created_at": "2026-05-14T10:30:00Z",
  "last_verified_at": null,
  "source_audit_id": 3
}
```

### 3.2 从审计日志创建 Action

**新增 Tauri command：** `create_action_from_audit(audit_index, action_name, description)`

1. 取审计日志第 index 条
2. 创建 `~/.ohmywu/actions/{name}/` 目录
3. 生成 `README.md`（标题、描述、用了什么能力、原始命令）
4. 生成 `manifest.json`
5. 注册到 ActionRegistry

### 3.3 启动时扫描 Action

AppState::new() 中扫描 `actions/` 目录，每个有 `manifest.json` 的子目录注册为一个 Action。

Action 同时作为 LLM tool 暴露（`capabilities_as_tools` 中合并）。

### 3.4 Action 验证

**新增命令：** `verify_action(name)` — 如果 action 有 script.sh，在 dry-run 模式下执行，返回验证结果。

### 3.5 前端 Action UI

**AuditView.vue：** 每行加"创建 Action"按钮 → 弹窗输入名称和描述 → 调用 `create_action_from_audit`

**ActionsView.vue：** 显示 Action 来源（from audit / user-created），加"执行"和"验证"按钮。

### Phase 3 完成标准

- 在审计日志中点"创建 Action"，文件系统生成对应目录和 README.md
- ActionsView 能看到新创建的 Action
- LLM 对话中能调用用户创建的 Action

---

## Phase 4：记忆搜索

### 4.1 Grep 级跨会话搜索

**新增命令：** `search_sessions(query, limit) → Vec<SearchResult>`

- 遍历 `sessions/` 下所有 JSONL 文件
- 逐行匹配 query（大小写不敏感）
- 返回匹配的消息 + session 上下文

### 4.2 LLM-Wiki

**新增命令：**
- `save_to_wiki(filename, content)` — 写入 `~/.ohmywu/wiki/{filename}.md`
- `list_wiki_pages()` — 列出所有 wiki 页面
- `read_wiki_page(filename)` — 读取 wiki 页面内容

**前端：**
- ChatMessage 上加"保存到 Wiki"按钮 → 弹窗输入文件名 → 调用 `save_to_wiki`
- 新增 WikiView（侧栏注册），浏览 wiki 页面

### 4.3 FTS5 升级（按需）

当会话数量导致 grep 变慢时，引入 SQLite FTS5 索引。JSONL 永远是真相，索引只是缓存。

---

## 文件变更汇总

### 新 crate

| Crate | Phase | 用途 |
|-------|-------|------|
| `crates/session` | 1 | JSONL 会话持久化 |
| `crates/llm-adapter` | 2 | LLM provider 抽象层 |

### 新文件（src-tauri）

| 文件 | Phase | 用途 |
|------|-------|------|
| `src/data_dir.rs` | 1 | 数据目录初始化 |
| `src/config.rs` | 1 | 配置读写 |
| `src/executor.rs` | 1 | 能力执行管道 |
| `src/tools.rs` | 2 | 能力→Tool 转换 |
| `src/agent.rs` | 2 | Agent 对话循环 |

### 新文件（前端）

| 文件 | Phase | 用途 |
|------|-------|------|
| `src/stores/chat.ts` | 1 | Pinia 聊天状态管理 |

### 修改的关键文件

| 文件 | Phase | 改动 |
|------|-------|------|
| `crates/domain/Cargo.toml` | 1 | +chrono |
| `crates/domain/src/lib.rs` | 1 | +chrono_now(), 新类型 |
| `crates/task-engine/src/lib.rs` | 1 | 用 domain::chrono_now() |
| `crates/audit/src/lib.rs` | 1 | 同上 |
| `Cargo.toml` | 1 | workspace members +session +llm-adapter |
| `src-tauri/Cargo.toml` | 1-2 | +dirs, +chrono, +session, +llm-adapter |
| `src-tauri/src/lib.rs` | 1-4 | AppState 重构，全部新命令 |
| `src-tauri/capabilities/default.json` | 1 | +path 权限 |
| `package.json` | 1 | +pinia |
| `src/main.ts` | 1 | +pinia plugin |
| `src/App.vue` | 1 | 接入 show-task, RightPanel |
| `src/views/ChatView.vue` | 1-2 | 完全重写，接入真实后端 |
| `src/views/ActionsView.vue` | 3 | 执行/验证按钮 |
| `src/views/AuditView.vue` | 3 | 创建 Action 按钮 |
| `src/views/SettingsView.vue` | 2 | LLM 配置区 |
| `src/components/Sidebar.vue` | 1 | 修复折叠行为 |
| `src/components/RightPanel.vue` | 1 | 动态任务详情 |
| `src/components/ChatMessage.vue` | 2 | 流式显示 + 保存到 Wiki |

---

## 新依赖汇总

### Rust（Cargo）

| 依赖 | 用在哪 | 用途 |
|------|--------|------|
| `chrono` (serde) | domain | 时间戳 |
| `dirs` | src-tauri | 获取 home 目录 |
| `serde` + `serde_json` | session, llm-adapter | 序列化 |
| `reqwest` (json) | llm-adapter | HTTP 请求 |
| `tokio` | llm-adapter, src-tauri | 异步运行时（Tauri 自带） |
| `futures` | llm-adapter | Stream trait |
| `async-trait` | llm-adapter | async trait |

### npm

| 包 | 用途 |
|----|------|
| `pinia` | 前端状态管理 |

---

## 不做的事（明确边界）

- **不做子 Agent 系统**（M3 以后的事）
- **不做 Live2D**（M5 以后的事）
- **不做向量数据库**（FTS 够用了再加）
- **不做 Schema 迁移系统**（文件格式简单，手动迁移即可）
- **不做多用户**（本地单用户桌面应用）
- **不做远程访问**（local-first，不是 cloud-first）
- **不做插件市场**（Action 是个人化的，分享靠文件复制即可）
