# OhMyWu

一个本地优先、可控执行、可审计的桌面 Agent。不只是电脑管家，是真正的个人管家。

## 定位

比 openclaw / hermes 更好用的个人 Agent。**不擅长编码**，定位是生活和工作中的个人管家。

## 核心理念

- **本地优先** — 核心能力跑在本地，数据不离开本机
- **AI Optional, Capability Persistent** — AI 可以参与发现和编排，但稳定能力必须能脱离 AI 独立存在
- **Policy First** — 任何执行能力先经过风险判断，分为沙箱模式和危险模式
- **Audit First** — 所有关键读写操作可追踪
- **Base + Read 就够了** — 首发原子能力只需要 bash 和 read，其他能力通过 Action 组合派生

## 关键概念

### 原子能力（Atomic Capability）

底层执行单元。首发只有两个：

| 原子能力 | 说明 |
|----------|------|
| bash | 执行 shell 命令，受 policy 控制 |
| read | 读取文件内容 |

原子能力不直接在用户层暴露，只作为 Action 的执行基座。

### Action

对用户和 AI 统一暴露的稳定能力入口。Action 封装一个或多个原子能力，对外呈现"能完成什么事"，隐藏底层执行细节。

### 子 Agent

区别于 skill。skill 多了以后用户不知道如何触发，子 Agent 更像一个强大的复合能力单元，内部再做细分。Agent 可以手动或自动创建子 Agent。

### Policy

- **沙箱模式**：受限的只读/低风险操作
- **危险模式**：高风险写操作，需明确授权

### 记忆系统

待定，LLMWiki 方案在考虑中。

## 技术架构

```text
┌─────────────────────────────────────────────┐
│              Tauri Shell                      │
│  ┌─────────────────────────────────────────┐ │
│  │            Vue3 Web UI                    │ │
│  │   对话 / Action / Task / Audit            │ │
│  └─────────────────────────────────────────┘ │
│                    │                          │
│             Tauri IPC                         │
│                    │                          │
│  ┌─────────────────────────────────────────┐ │
│  │            Rust Backend                   │ │
│  │  capability-registry / action-registry   │ │
│  │  policy-engine / task-engine / audit     │ │
│  └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

### 技术栈

- **Desktop Shell**：Tauri
- **Backend**：Rust
- **Web UI**：Vue 3
- **桌宠**：Live2D
- **API 兼容**：本地 ollama + 云端模型

## Crate 设计

| Crate | 职责 |
|-------|------|
| domain | 核心类型：ActionSpec、AtomicCapability、Task、AuditEvent、RiskLevel、Policy |
| capability-registry | 原子能力注册与发现 |
| action-registry | Action 统一入口，封装原子能力或组合 |
| policy-engine | Sandbox / Danger 双模式，RiskLevel 判断 |
| task-engine | 任务状态机：创建 → 运行 → 完成 / 失败 |
| audit | 审计日志记录与反向遍历读取 |

## 交互形态

- **正常模式**：完整交互界面
- **轻量模式**：桌面悬浮 Live2D 桌宠

## v0.1 目标

把整体桌面框架跑起来——Tauri 壳 + Rust 后端 + Vue3 前端，打通能力注册系统、Action 系统、可追踪闭环和权限控制。
