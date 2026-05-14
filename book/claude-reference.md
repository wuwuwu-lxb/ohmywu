# Claude Code 架构参考

> 基于 Claude Code 泄露源码（npm source map，2026-03-31）及社区逆向分析的综合整理。
> 来源：`claude-code-from-scratch`、`Dive-into-Claude-Code`、`Inside-Claude-Code-Architecture-and-Design-Philosophy`

---

## 核心哲学

### 单一执行引擎

Claude Code 只有一个 `queryLoop`（`query.ts`），所有入口——交互式 CLI、headless CLI（`claude -p`）、Agent SDK、IDE 插件——最终都汇聚到同一套代码路径。`QueryEngine` 只是会话包装层，不是独立的引擎。

### 四个设计问题

| 问题 | Claude Code 的答案 |
|------|-------------------|
| 推理放在哪里？ | 模型推理；harness 强制执行。约 1.6% AI 决策逻辑，98.4% 基础设施 |
| 多少个执行引擎？ | 一个，供所有入口共用 |
| 默认安全姿态？ | 拒绝优先：拒绝 > 询问 > 允许 |
| 最根本的资源约束？ | ~200K token 上下文窗口 |

---

## Agent Loop

### 核心模式

```
while not done:
  1. 构建 messages（system prompt + 历史 + tool 结果）
  2. 调 LLM API，传入 messages + tool schemas
  3. 解析响应：
     - 纯文本 → done，返回用户
     - tool_use → 执行每个工具调用 → 添加结果 → 继续循环
  4. 检查终止条件（最大轮次、预算、超时、完成）
```

### 9 步执行管道（每轮）

```
设置解析 → 状态初始化 → 上下文组装 → 5 阶段预模型整形 → 模型调用 → 工具分派 → 权限门控 → 工具执行 → 停止条件检查
```

### 流式早期执行

Streaming 过程中，tool_use 的 `content_block_stop` 事件一触发就立即开始执行工具——不等整个响应完成。只读工具（read_file/grep_search/list_files）可以同时并发跑，模型还在生成后续 block 时工具已经在执行。

参考代码（`agent.ts`）：

```typescript
const earlyExecutions = new Map<string, Promise<string>>();
const response = await this.callAnthropicStream((block) => {
  if (CONCURRENCY_SAFE_TOOLS.has(block.name)) {
    const perm = checkPermission(block.name, input, this.permissionMode);
    if (perm.action === "allow") {
      earlyExecutions.set(block.id, this.executeToolCall(block.name, input));
    }
  }
});
```

### 重试机制

指数退避重试（最多 3 次），针对 HTTP 429/503/529、连接重置、超时等可恢复错误：

```typescript
function isRetryable(error: any): boolean {
  const status = error?.status || error?.statusCode;
  if ([429, 503, 529].includes(status)) return true;
  if (error?.code === "ECONNRESET" || error?.code === "ETIMEDOUT") return true;
  if (error?.message?.includes("overloaded")) return true;
  return false;
}
```

---

## 工具系统

### 工具定义

使用标准的 OpenAI function calling 格式，额外支持 `deferred` 标记：

```typescript
type ToolDef = Anthropic.Tool & { deferred?: boolean };
```

完整示例：

```typescript
{
  name: "read_file",
  description: "Read the contents of a file. Returns the file content with line numbers.",
  input_schema: {
    type: "object",
    properties: {
      file_path: { type: "string", description: "The path to the file to read" }
    },
    required: ["file_path"],
  },
}
```

### Deferred 工具（延迟加载）

- 只传工具名（不含 schema）给模型，节省 tokens
- 模型调用 `tool_search` 查询后，匹配的工具才激活，后续调用带上完整 schema
- 适用：低频使用的工具

```typescript
export function getActiveToolDefinitions(allTools?: ToolDef[]): Anthropic.Tool[] {
  return allTools.filter(t => !t.deferred || activatedTools.has(t.name))
    .map(({ deferred, ...rest }) => rest);
}
```

### 并发安全工具

只读工具自动并行执行，返回结果再统一处理：

```typescript
const CONCURRENCY_SAFE_TOOLS = new Set([
  "read_file", "list_files", "grep_search", "web_fetch"
]);
```

执行时分组：连续的安全工具合并成一个并行批次，非安全工具串行执行。

### 大结果持久化

>30KB 的工具结果自动写道 `~/.mini-claude/tool-results/{timestamp}-{toolName}.txt`，上下文只保留 200 行预览：

```typescript
private persistLargeResult(toolName: string, result: string): string {
  const THRESHOLD = 30 * 1024;
  if (Buffer.byteLength(result) <= THRESHOLD) return result;
  // 写磁盘 + 返回预览
}
```

### 工具池组装（5 步管道）

```
基础枚举（最多 54 个工具）→ 模式过滤 → 拒绝规则预过滤 → MCP 集成 → 去重
```

---

## System Prompt

### 模板 + 变量替换

```typescript
const SYSTEM_PROMPT_TEMPLATE = `...{{cwd}}...{{date}}...{{platform}}...{{git_context}}...
{{claude_md}}{{memory}}{{skills}}{{agents}}{{deferred_tools}}`;

export function buildSystemPrompt(): string {
  return SYSTEM_PROMPT_TEMPLATE
    .split("{{cwd}}").join(process.cwd())
    // ...
    .split("{{deferred_tools}}").join(deferredSection);
}
```

### @include 语法

递归解析 `@./path`、`@~/path`、`@/path` 引用，防止循环引用（最大深度 5）：

```typescript
const INCLUDE_REGEX = /^@(\.\/[^\s]+|~\/[^\s]+|\/[^\s]+)$/gm;
```

### .claude/rules/ 自动加载

按文件名排序自动加载 `.claude/rules/*.md`，作为规则注入 system prompt。

### CLAUDE.md 层级（4 级）

| 级别 | 路径 | 范围 |
|------|------|------|
| 托管 | `/etc/claude-code/CLAUDE.md` | 系统范围（企业） |
| 用户 | `~/.claude/CLAUDE.md` | 用户级 |
| 项目 | `CLAUDE.md`、`.claude/CLAUDE.md`、`.claude/rules/*.md` | 项目级 |
| 本地 | `CLAUDE.local.md` | 个人（被 gitignore 忽略） |

关键设计：CLAUDE.md 作为**用户上下文**传递（模型概率性遵从），而非系统提示（确定性遵从）。

---

## 权限系统

### 拒绝优先原则

```
拒绝 > 询问 > 允许
```

所有权限规则评估中，deny 始终覆盖 allow——即使 allow 规则更具体。

### 7 个权限模式

| 模式 | 行为 | 信任级别 |
|------|------|---------|
| `plan` | 用户在执行前批准所有计划 | 最低 |
| `default` | 标准交互批准 | 低 |
| `acceptEdits` | 文件编辑 + 文件系统 shell 自动批准 | 中 |
| `auto` | ML 分类器评估工具安全性 | 高 |
| `dontAsk` | 无提示，仍强制执行拒绝规则 | 较高 |
| `bypassPermissions` | 跳过大多数提示，仍保留安全相关的关键检查 | 最高 |
| `bubble` | 内部：子智能体向父级上报 | 特殊 |

### 7 层安全

请求必须通过**所有**适用层：

1. 工具预过滤——把被全局拒绝的工具从模型可见的工具清单中彻底剔除
2. 拒绝优先规则评估——拒绝始终覆盖允许
3. 权限模式约束——当前活跃模式决定基线处理
4. Auto 模式 ML 分类器——独立评估安全性的单独 LLM 调用
5. Shell 沙箱——对 shell 命令实施文件系统 + 网络隔离
6. 恢复会话时权限永不自动恢复
7. 基于钩子的拦截（PreToolUse 钩子可以修改或阻止操作）

### 声明式规则

`.claude/settings.json` 中配置，支持 tool 级别和模式匹配：

```json
{
  "permissions": {
    "allow": ["read_file", "run_shell(ls *)", "write_file(src/*)"],
    "deny": ["run_shell(rm *)", "run_shell(sudo *)"]
  }
}
```

---

## 上下文管理

### 5 阶段压缩管道

每次模型调用前按顺序执行，开销从低到高：

| 阶段 | 策略 | 触发条件 |
|------|------|---------|
| 预算削减 | 每条消息大小上限 | 利用率 >50% |
| 裁剪 | 裁剪较旧的历史 | 特性开关（HISTORY_SNIP） |
| 微压缩 | 缓存感知的细粒度压缩, 清 5min+ 旧结果 | 始终启用 |
| 上下文折叠 | 读取时虚拟投影（非破坏性） | 特性开关（CONTEXT_COLLAPSE） |
| 自动压缩 | 模型生成的摘要（最后手段） | 其他阶段都失败 |

### 预算削减

利用率 >50% 时激活，>70% 时更激进：

```typescript
private budgetToolResultsAnthropic(): void {
  const utilization = this.lastInputTokenCount / this.effectiveWindow;
  if (utilization < 0.5) return;
  const budget = utilization > 0.7 ? 15000 : 30000;
  // 对 tool_result 内容做 head/tail 截断
}
```

### 裁剪

删除重复的读文件结果（同一文件被多次读取只保留最新一次），以及超出最近 N 条的旧结果。

### 自动压缩

最后手段。调 LLM 生成对话摘要，替换旧历史：

```typescript
const summaryReq = [{
  role: "user",
  content: "Summarize the conversation so far, preserving key decisions, file paths, and context needed to continue."
}];
const summaryResp = await client.messages.create({
  system: "You are a conversation summarizer.",
  messages: [...oldMessages, ...summaryReq],
});
// 替换历史为摘要
this.messages = [
  { role: "user", content: `[Summary]\n${summaryText}` },
  { role: "assistant", content: "Understood. I have the context." },
];
```

---

## 记忆系统

### 文件优先，No Vector DB

- 不用向量数据库，不用嵌入向量
- 文件头（YAML frontmatter）携带元数据
- LLM（便宜模型）做侧查询（sideQuery）判断相关性

### 记忆文件格式

```markdown
---
name: user_preferences
type: user
description: Frontend developer, prefers TypeScript
---
内容...
```

### 4 种记忆类型

| 类型 | 用途 |
|------|------|
| user | 用户角色、偏好、知识背景 |
| feedback | 行为指导、纠正、已验证的方法 |
| project | 项目上下文、目标、当前工作 |
| reference | 外部系统指针 |

### 检索流程

1. **侧查询（sideQuery）**：调 LLM（256 token）判断用户消息与哪些记忆相关
2. **异步预取**：模型生成响应时并行加载记忆
3. **最多 5 条**：只注入最相关的，防止上下文污染
4. **去重**：已展示过的记忆不再重复

```typescript
// sideQuery 签名
type SideQueryFn = (system: string, userMessage: string, signal?: AbortSignal) => Promise<string>;
```

### 记忆更新

写入记忆文件时自动重建 `MEMORY.md` 索引（含 name/type/description 摘要）。

---

## 子 Agent 系统

### SkillTool vs AgentTool

| 特性 | SkillTool | AgentTool |
|------|-----------|-----------|
| 上下文成本 | 便宜（注入当前上下文） | 贵（新窗口，~7x token） |
| 隔离性 | 共享上下文 | 完全隔离 |
| 适用场景 | 简单指令、模板注入 | 复杂任务、并行探索 |

### Fork 模式（SkillTool）

技能支持 inline（注入当前上下文）和 fork（新子 Agent 执行）两种模式：

```typescript
if (result.context === "fork") {
  const subAgent = new Agent({
    customSystemPrompt: result.prompt,
    customTools: tools,
    isSubAgent: true,
    permissionMode: "bypassPermissions",
  });
  const subResult = await subAgent.runOnce(input.args || "Execute.");
  return subResult.text;
}
```

### 6 个子 Agent 类型

Explore（只读搜索）、Plan（结构化规划）、General-purpose、Claude Code Guide、Verification、Statusline-setup

自定义 Agent 通过 `.claude/agents/*.md` 定义（YAML frontmatter 支持 tools/model/permissions/hooks）。

### 侧链转录稿

每个子 Agent 写入自己的 `.jsonl` 文件，只把摘要回传给父级。完整历史永远不会进入父级上下文。多实例之间通过 POSIX `flock()` 协调，零外部依赖。

### 关键约束

- 子 Agent 不能生子 Agent（防止不受控的任务爆炸）
- Lead Agent 分解查询 → 多个子 Agent 并行搜索/分析 → Coordinator 合成
- 多 Agent 模式比单 Agent 在内部研究评测上高 90.2%，但 token 消耗 ~15x

---

## 会话持久化

### 三个通道

| 通道 | 格式 | 目的 |
|------|------|------|
| 会话转录稿 | 仅追加 JSONL | 完整对话，压缩边界采用链式修补 |
| 会话文件 | NDJSON | 提示历史 + 压缩链信息，用于恢复 |
| 侧链文件 | JSONL | 子 Agent 转录稿，父级不感知 |

### 关键设计

- **仅追加**：从不修改已有记录
- **压缩链式修补**：压缩后的摘要通过链式引用关联原始记录
- **权限不跨会话**：恢复会话时权限模式重置为 default

---

## Hook 系统

27 个事件钩子，覆盖工具调用生命周期：

| 钩子 | 触发时机 | 可阻断？ |
|------|---------|---------|
| PreToolUse | 工具执行前 | 是（返回 permissionDecision） |
| PostToolUse | 工具执行后 | 否 |
| PreQuery | 模型调用前 | 是 |
| PostQuery | 模型调用后 | 否 |

4 种执行类型：shell、LLM、webhook、subagent 验证器。

---

## 对 OhMyWu 的参考价值总结

| 特性 | Claude Code 方案 | OhMyWu 当前状态 | 建议采纳时机 |
|------|-----------------|----------------|-------------|
| Agent loop | while 循环 + streaming early execution | 基础 loop 已实现 | 优化 agent.rs 时加 streaming 早期执行 |
| 工具格式 | OpenAI function calling + deferred | 已用相同格式 | 工具 >5 个时引入 deferred |
| 权限系统 | 7 模式 + 声明式规则 | Sandbox/Danger 两模式 | Phase 3 扩展 |
| 上下文管理 | 5 阶段管道 | 无 | Phase 4 |
| 记忆系统 | 文件系统 + sideQuery | 未实现 | Phase 4 直接抄 |
| 大结果持久化 | >30KB 写磁盘 | 无 | 马上可加 |
| 子 Agent | AgentTool/SkillTool | 未实现 | Phase 5 参考 |
| Hook 系统 | 27 事件 | 无 | 长期规划 |
