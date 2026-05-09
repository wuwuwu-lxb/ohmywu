# OhMyWu 设计思路

## 定位

比 openclaw / hermes 更好用的个人桌面 Agent。定位是生活和工作中的个人管家。

（编码能力定位不同于 Codex、Claude Code 等专用编码 Agent，但有基础编码支持。）

## 核心设计原则

### 1. 本地优先（Local First）

核心能力跑在本地，数据不离开本机。云端模型作为可选增强，而非必需依赖。

### 2. AI Optional, Capability Persistent

AI 可以参与能力发现、规划和编排，但一旦能力稳定，就必须能脱离 AI 独立存在，被用户直接触发。

这条原则区分了两种使用模式：
- **用户自己触发**：通过 UI 直接调用稳定 Action
- **AI 代理触发**：Agent 根据任务自动选择和编排 Action

### 3. Shared Capability Surface

用户模式和 Agent 模式共享同一套能力资产。不维护两套系统。

### 4. Policy First

任何执行能力必须先经过风险判断。

两级模式：
- **Sandbox（沙箱）**：仅允许 ReadOnly 操作
- **Danger（危险）**：允许所有操作，但 HighRisk 操作需审计追踪

### 5. Audit First

所有关键读写操作必须可追踪。审计不是附属功能，是信任闭环的基础。

## 核心概念

### 原子能力（Atomic Capability）

底层执行单元。首发只有两个：

| 原子能力 | 风险等级 | 说明 |
|----------|---------|------|
| bash | HighRisk | 执行 shell 命令，受 policy 控制 |
| read | ReadOnly | 读取文件内容 |

原子能力不直接暴露给用户。所有上层功能通过 Action 组合这两个原子能力派生。

为什么只需要两个？
- bash 可以完成系统操作（进程管理、服务控制、文件写入……）
- read 负责信息获取
- 其他能力都是二者的组合或封装

### Action

对用户和 AI 统一暴露的稳定能力入口。

Action 封装一个或多个原子能力，对外呈现"能完成什么事"，隐藏底层执行细节。

例如：
- `shell.exec` — 基于 bash 能力
- `fs.read` — 基于 read 能力
- `system.info` — 基于 bash + read 的组合

### 子 Agent

区别于 skill 泛滥。

skill 多了以后用户不知道如何触发；子 Agent 更像一个强大的复合能力单元，内部再做细分。

Agent（主 Agent）可以手动或自动创建子 Agent，子 Agent 有明确的任务边界和生命周期。

### 记忆系统

待定。LLMWiki 方案在考虑中。

核心思路：
- 长记忆文档（固定的知识库）
- 最近活跃对话（短期记忆）
- 搜索和检索增强

不同于 hermes 那种把反复工作写成 skill 的思路。

### Task

一次可追踪的执行单元。状态机：Pending → Running → Completed / Failed。

每次能力调用都创建一个 Task，记录：谁触发的、做了什么、结果如何。

### Audit

不可变的审计记录。记录：执行主体、动作、目标、风险等级、状态、详细信息。

与 Task 的区别：Task 是执行者视角（任务是否成功），Audit 是安全视角（谁在何时做了什么、风险多大）。

## 架构分层

```text
┌─────────────────────────────────────────────┐
│              Tauri Shell                      │
│  ┌─────────────────────────────────────────┐ │
│  │            Vue3 Web UI                    │ │
│  │   对话 / Action / Task / Audit            │ │
│  └─────────────────────────────────────────┘ │
│                    │                          │
│             Tauri IPC (invoke)                │
│                    │                          │
│  ┌─────────────────────────────────────────┐ │
│  │            Rust Backend                   │ │
│  │  ┌───────────┐  ┌────────────────────┐   │ │
│  │  │ Policy    │  │ Action Registry     │   │ │
│  │  │ Engine    │  │ capability compose  │   │ │
│  │  └───────────┘  └────────────────────┘   │ │
│  │  ┌───────────┐  ┌────────────────────┐   │ │
│  │  │ Task      │  │ Audit               │   │ │
│  │  │ Engine    │  │ Log                 │   │ │
│  │  └───────────┘  └────────────────────┘   │ │
│  │  ┌──────────────────────────────────┐    │ │
│  │  │ Capability Registry               │    │ │
│  │  │ bash · read                       │    │ │
│  │  └──────────────────────────────────┘    │ │
│  └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

## 调用链路

用户 / Agent 发起调用：

```text
1. 请求 Action (e.g., "shell.exec")
2. Policy Engine 检查风险等级
   ├─ Sandbox + HighRisk → 拒绝
   └─ 放行
3. Task Engine 创建 Task (status: Running)
4. Capability Registry 获取原子能力
5. 执行能力（bash / read）
6. Task Engine 更新 Task (Completed / Failed)
7. Audit Log 记录审计事件
8. 返回结果
```

## 关键设计决策

### 为什么不是 Electron？

- 包体小、内存低、Rust 生态原生
- 和 Rust backend 天然搭配
- 不用维护 node_modules 地狱

### 为什么不用 Web 前端？

- 桌面应用需要系统级交互（托盘、悬浮窗、快捷键）
- Tauri 在轻量和功能间取平衡
- Vue3 提供良好的 UI 开发体验

### 为什么首发只给 bash + read？

- 两个原子能力已覆盖所有操作场景
- 避免过早抽象的复杂度
- 把资源集中在 Action 系统和执行闭环上
