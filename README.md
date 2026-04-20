# OhMyWu

一个本地优先、可控执行、可审计、可扩展的**个人 Agent 平台**。

> 电脑管理能力不是产品本体，而是首发最适合落地的一组能力域。

v0.1 首发能力域：`system-management`，在 Ubuntu/Debian + systemd + apt 环境下，验证 Agent 平台底座是否成立。

## 当前里程碑

```
✅ M1 — 项目骨架（Rust workspace + Vue3 frontend + Electron 壳）
✅ M2 — 只读能力落地（进程/服务/存储/日志真实 Linux 数据）
✅ Tool Enhancement B — ECharts 图表化（CPU/Mem/状态分布图表）
✅ M3 — 可控操作落地（kill 进程、服务启停、清理任务、审计记录）
✅ M3 Write Ops — cleanup tree API + split-pane treemap UI + sudo 降级
🔲 M4 — 桌面集成（Electron 主窗口 + 托盘 + 通知）
🔲 M5 — Agent 预埋（Action/Tool/Workflow Registry 稳定化）
```

## 功能预览

**用户模式**（类 360 工具箱，直接操作，无需 AI）：

| 页面 | 功能 | 数据来源 |
|------|------|----------|
| 概览 | 系统总览 + CPU/Mem TOP10 图表（5s 轮询） | /proc |
| 进程管理 | 进程列表 + 搜索 + CPU/Mem/状态图表 | /proc |
| 服务管理 | systemd 服务列表 + 状态分布饼图 | systemctl |
| 存储扫描 | 目录占用分析 + TOP10 图表 | du |
| 日志查看 | journal 日志 + 优先级/单位分布图表 | journalctl |
| 任务记录 | 扫描与执行历史（stub） | Task Engine |
| 审计记录 | 行为日志（stub） | Audit Engine |

**Agent 模式**（暂未开放，入口已预留）

## 技术架构

```
Electron Shell / 未来桌宠壳
            │
       Web UI（Vue3 + ECharts）
            │  Vite Proxy /api → :3000
        Rust Daemon（Axum）
            │
  Core Runtime + Agent Orchestrator
            │
   toolkit-system-management（首发能力域）
            │
    linux-adapter（/proc + systemctl + journalctl + du）
            │
     command-runner（tokio async 受控命令执行）
```

**Rust Crates（14 个）**

| Crate | 职责 |
|-------|------|
| `domain` | 核心数据结构：ProcessInfo, ServiceInfo, JournalEntry... |
| `action-registry` | Action 注册与发现，用户/AI 共用同一套调用表面 |
| `tool-registry` | Tool 元数据与调用入口 |
| `workflow-registry` | 可复用流程编排注册 |
| `policy-engine` | Sandbox/Danger 模式 + 风险分级 |
| `task-engine` | 任务队列、状态机、取消、超时 |
| `audit` | 审计事件记录 |
| `agent-kernel` | 主/子 Agent 治理模型 |
| `toolkit-system-management` | 首发能力域：进程/服务/存储/日志 Action+Tool |
| `linux-adapter` | /proc 读取 + systemctl + journalctl + du 封装 |
| `command-runner` | tokio async 命令执行器（超时、stdout/stderr） |

**前端（apps/web）**
- Vue 3 + Vue Router + Pinia
- ECharts 5 + vue-echarts（存储页面 treemap 可视化）
- Vite 开发服务器，proxy `/api/*` → `http://127.0.0.1:3000`

## 本地启动

```bash
# Rust daemon
cargo run -p ohmywu-daemon
# 监听 http://127.0.0.1:3000

# Web 前端
pnpm --dir apps/web dev
# 访问 http://127.0.0.1:5173
```

## API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/processes` | GET | 真实进程列表（602 条） |
| `/api/services` | GET | 真实 systemd 服务（214 条） |
| `/api/storage/scans` | POST | 目录占用扫描 |
| `/api/logs/journal` | POST | journal 日志读取 |
| `/api/system/overview` | GET | 系统总览 |
| `/api/health` | GET | 健康检查 |

完整 API 规范见 `plan.md`（Section 10）。

## 关键概念

**Action** — 底层原子能力，用户模式和 Agent 模式共用同一套调用入口。对话中可直接引用：`/scan_storage`、`/restart_service`。

**Tool** — 最小可执行单元，具备输入、输出、权限级别和审计记录。

**Workflow** — 可重复执行的流程编排，不依赖 AI 独立运行。

**Agent** — 具有角色、记忆、目标和工具调用能力的实体。v0.1 只做结构预埋。

**运行模式** — 默认沙箱模式，只读优先；写操作需确认，审计不可关闭。

## 目录结构

```
ohmywu/
├── apps/
│   ├── daemon/       Rust Axum API server
│   ├── web/           Vue3 + Vite + ECharts 前端
│   └── electron/      Electron 桌面壳
└── crates/            14 个 Rust crate
```

## 设计原则

- **Daemon First** — Rust 后台是系统大脑，Electron 不承载核心逻辑
- **Shared Action Surface** — 用户和 AI 通过同一套 Action 访问能力
- **Policy First** — 任何执行能力都必须经过 Policy 判断
- **Audit First** — 所有写操作和关键读取都必须可追溯
- **Human Directed** — 支持用户手动拆任务，不依赖 AI 也能完成稳定任务

完整设计文档：`plan.md`

详细架构文档：`book/architecture.md`

## 平台边界

**v0.1 支持**：Ubuntu 22.04/24.04、Debian 12、systemd、apt

**v0.1 不做**：高自主 Agent 执行、Live2D 桌宠、跨发行版兼容、常驻 root daemon
