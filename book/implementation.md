# OhMyWu 当前实现说明

> 最后更新：2026-05-20

## 文档定位

这份文档描述的是 **当前仓库已经落地的实现**，不是早期路线图。

如果你要看：

- 使用方法：`book/guide.md`
- 发布流程：`book/release-checklist.md`
- 历史参考和外部分析：`book/claude-reference.md`、`book/planapi升级.md`

## 当前阶段

项目已经不是最初的 mock 原型，当前处于：

- 桌面 Agent 工作台可运行
- 对话、工具调用、Runtime、知识库、Agent、Action、原子化能力均已有实际页面和后端实现
- 模型配置、权限规则、主题背景都具备本地持久化
- 当前重点转向优化测试、性能、稳定性和发布准备

## 整体架构

```text
Tauri Shell
  ├─ Vue 3 前端
  │   ├─ 对话 / Runtime
  │   ├─ Agent 管理
  │   ├─ 知识库
  │   ├─ 模型设置
  │   ├─ 原子化能力
  │   ├─ Action 注册
  │   ├─ 审计日志
  │   └─ 设置
  └─ Rust 后端
      ├─ Agent Loop
      ├─ Capability Catalog + Registry
      ├─ Action Catalog + Registry
      ├─ Agent Catalog
      ├─ Policy / Permission
      ├─ Runtime Store
      ├─ Session Store
      ├─ Wiki Engine
      └─ LLM Adapter
```

## 本地数据目录

运行时数据默认放在 `~/.ohmywu/`，核心目录包括：

```text
~/.ohmywu/
  agents/           # agent catalog
  actions/          # action catalog
  capabilities/     # capability catalog
  runtime/          # runtime 事件与线程记录
  sessions/         # 对话会话
  wiki/             # 知识库
  config.json       # 全局配置
```

这些数据属于本地运行数据，不应该提交进仓库。

## 后端核心实现

### 1. AppState

`src-tauri/src/lib.rs` 当前已经把核心运行对象统一挂在 `AppState`：

- `capabilities`
- `capability_catalog`
- `actions`
- `action_catalog`
- `agent_catalog`
- `session_agents`
- `policy`
- `tasks`
- `audit`
- `session`
- `config`
- `wiki`
- `runtime`

这意味着应用已经从单页 demo 转成了完整的本地状态容器。

### 2. Capability Catalog

原子化能力现在分成两层：

- 底层 `CapabilityRegistry`：真正给 Agent 暴露的可执行能力
- `CapabilityCatalog`：面向用户和 AI 的可配置目录

当前内置能力包括：

- `bash`
- `read`
- `write`
- `edit`
- `glob`
- `grep`
- `web_fetch`
- `thinking`
- `checklist_write`
- `wiki_read`
- `wiki_write`
- `wiki_search`
- `wiki_list`
- `wiki_graph`
- `capability_list`
- `capability_register`
- `action_list`
- `action_register`
- `agent_list`
- `agent_delegate`
- `agent_register`

除了内置能力，用户还可以注册自己的能力包装层。内置项不可删除，用户项可关停、可删除。

### 3. Action Catalog

Action 已经不是占位概念，而是独立目录系统。

当前有两类：

- 系统内置 Action
- 用户自定义 Action

其中内置的系统 Action 主要负责：

- 原子化能力注册
- Skill 转 Action
- Agent 注册

用户自定义 Action 以 prompt / 规范为主，由 AI 或用户注册到本地目录，再同步进运行时注册表。

### 4. Agent Catalog

Agent 目录当前支持：

- 多个 Agent 配置
- 名称、角色、人格
- 记忆范围
- 工具范围
- 是否允许委派
- 委派优先级

主 Agent 不可删除，其他 Agent 可编辑、可删除。

### 5. Session 与 Runtime

当前对话系统已经具备：

- 会话持久化
- turn 级 runtime 记录
- 工具开始 / 完成事件
- 记忆召回事件
- 委派事件

前端已经能把 Runtime 绑定到具体回复下方，而不是单独黑盒输出。

### 6. Wiki / 记忆

知识库使用本地 `WikiEngine`，已经接通：

- 读取
- 写入
- 搜索
- 列表
- 图谱

Agent Loop 会按 Agent 的 `memory_scope` 注入知识召回结果。当前仍以“手动确认写入记忆候选”为主，避免自动污染知识库。

### 7. 模型适配

模型层已经支持：

- 多 profile 配置
- 多 provider / 多协议格式
- 独立模型设置页
- 拉取模型列表
- 连接测试
- 当前激活模型切换

配置入口已经从综合设置页拆分到单独的 `模型设置` 页面。

### 8. Agent Loop

当前 `src-tauri/src/agent.rs` 已经不是简单的一问一答，而是完整循环：

1. 健康检查
2. 组装 system prompt
3. 注入最近会话历史
4. 按记忆范围召回 wiki
5. 向模型暴露当前启用工具
6. 解析 tool calls
7. 执行工具并把结果回填
8. 继续下一轮直到完成或达到上限

当前最大轮次为 `48`，不是早期的短轮次 demo。

同时已经加入：

- 兼容模型失败时的纯文本回退
- 取消执行
- 运行时事件记录
- 子 Agent 委派

## 前端当前实现

### 页面结构

当前主要页面：

- `对话`
- `Agent 管理`
- `知识库`
- `模型设置`
- `原子化能力`
- `Action 注册`
- `审计日志`
- `设置`

### 对话页

当前对话页已经支持：

- 对话管理与会话切换
- 消息 Markdown 渲染
- 一键复制
- Runtime 摘要
- 可折叠工具调用
- 工具执行明细
- 记忆候选展示
- 当前 Agent 切换

### 设置页

设置页已经收敛为两个分区：

- `外观`
- `执行与权限`

模型配置不再放在这里，避免 UI 过重，也减少复杂输入区域耦合。

### 外观系统

当前外观已支持：

- 纯色背景
- 图片背景
- 主题色独立控制
- 自动提取图片主色
- 模糊、遮罩、缩放
- 表面透明度

视频背景已经暂时下掉，后续再加。

## 权限模型

当前权限控制由两层共同完成：

- `PolicyMode`
  - `Sandbox`
  - `Danger`
- `AgentMode`
  - `plan`
  - `agent`
  - `auto`

同时支持 `allow` / `deny` 规则文本配置，`deny` 优先级始终更高。

## 当前验证基线

当前至少需要保持下面两项长期可通过：

```bash
npm run build
cargo check
```

在发布前，还应补充这些真实链路测试：

- 模型连接测试
- 工具调用展示
- Action / Capability / Agent 注册链路
- 知识库写入与图谱展示
- 权限规则拦截

## 当前仍在持续打磨的方向

- 运行效率与上下文开销
- 权限说明和审计可读性
- 知识库编辑与记忆体验
- Action 生态兼容性
- 多 Agent 协作体验
- Linux WebKit 下的输入与渲染稳定性

## 结论

当前仓库已经进入“可实际使用并持续优化”的阶段，不应再用早期 mock / Phase 1 / Phase 2 的方式理解。

如果后面实现继续变化，这份文档应该优先更新，而不是再堆新的过渡计划。
