# OhMyWu 架构文档

> 本文档详细说明 OhMyWu 的架构设计、核心组件与未来路线图。

## 1. 项目概览

OhMyWu 是一个**本地优先、可控执行、可审计、可扩展**的个人 Agent 平台。

**核心原则：Daemon First** — 所有核心业务逻辑运行在 Rust Daemon，前端仅负责呈现与交互。

```
┌─────────────────────────────────────────────────────────┐
│                    Electron Shell                        │
│  ┌───────────────────────────────────────────────────┐  │
│  │                  Vue3 Web UI                       │  │
│  │  (Vite + Vue Router + Pinia + ECharts + vue-echarts)│  │
│  └───────────────────────────────────────────────────┘  │
│                           │                              │
│                    HTTP API (:3000)                      │
│                           │                              │
│  ┌───────────────────────────────────────────────────┐  │
│  │                 Rust Daemon                        │  │
│  │  Axum 0.7.9 + Tower-HTTP CORS                     │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## 2. 整体架构分层

```
┌─────────────────────────────────────────────────┐
│           Shell Layer (Electron)                │
├─────────────────────────────────────────────────┤
│           Web UI Layer (Vue3)                   │
├─────────────────────────────────────────────────┤
│           Daemon Layer (Axum HTTP)              │
│  Reference Resolver + Local API                 │
├─────────────────────────────────────────────────┤
│           Core Runtime Layer                    │
│  Agent Kernel + Registries + Engines + Store    │
├─────────────────────────────────────────────────┤
│         Capability Toolkit Layer                │
│        toolkit-system-management                │
├─────────────────────────────────────────────────┤
│            Adapter Layer                         │
│    linux-adapter + command-runner              │
└─────────────────────────────────────────────────┘
```

## 3. Crate 职责矩阵

| Crate | 职责 | 依赖 |
|-------|------|------|
| `domain` | 核心类型定义：ActionSpec、ToolSpec、WorkflowSpec、AgentTemplate、Task、AuditEvent、CleanupItem 等纯数据结构 | 无 |
| `action-registry` | 统一管理 Action 表面，BTreeMap 按名字存储 | domain |
| `tool-registry` | 注册和发现 Tool，元数据管理 | domain |
| `workflow-registry` | 管理可复用 WorkflowSpec | domain |
| `reference-resolver` | 解析 @agent 和 /action 引用协议 | domain |
| `policy-engine` | Policy 校验：Sandbox(只读) / Danger(全操作) + RiskLevel | domain |
| `task-engine` | 任务状态机：new_task / complete / fail / get_task | domain |
| `audit` | 审计日志：record 记录，list 反向遍历取最新 | domain |
| `store` | 持久化边界（当前 InMemoryStore，预留 SQLite 方案） | domain |
| `agent-kernel` | Agent 运行时，管理 Template 与 Instance | domain |
| `toolkit-system-management` | 首发能力域：system_overview / processes / services / cleanup 等 | domain |
| `linux-adapter` | Linux 原生封装：/proc / systemctl / du / journalctl / 文件删除 | domain, command-runner |
| `command-runner` | 统一命令执行：tokio::process::Command + 超时/stdout/stderr 捕获 | 无 |

### 3.1 ohmywu-daemon (apps/daemon)

Rust 后台主入口，构建 AppState 聚合所有组件，通过 Axum 暴露 REST API。

**核心路由：**

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/version` | 版本信息 |
| GET | `/api/actions` | 列出所有 Action |
| GET | `/api/workflows` | 列出所有 Workflow |
| GET | `/api/agents/templates` | 列出 Agent 模板 |
| GET | `/api/settings` | 获取设置 |
| GET | `/api/tasks` | 任务列表 |
| GET | `/api/audits` | 审计日志 |
| GET | `/api/system/overview` | 系统概览 |
| GET | `/api/processes` | 进程列表 |
| GET | `/api/services` | 服务列表 |
| GET | `/api/storage/scans` | 存储扫描 |
| GET | `/api/logs` | 日志读取 |
| POST | `/api/references/resolve` | 解析引用 |
| POST | `/api/storage/cleanup/tree` | 清理树 |
| POST | `/api/storage/cleanup/scan-path` | 按需深挖 |
| POST | `/api/storage/cleanup/preview` | 清理预览 |
| POST | `/api/storage/cleanup/execute` | 执行清理 |
| POST | `/api/processes/:pid/kill` | 杀死进程 |
| POST | `/api/services/:name/start\|stop\|restart` | 服务控制 |

## 4. 前端架构

### 4.1 技术栈

- **Vue 3** + Vite + Vue Router + Pinia
- **ECharts** + vue-echarts（存储可视化）
- **Axum** 代理转发（开发环境）

### 4.2 目录结构

```
apps/web/src/
├── views/
│   ├── OverviewView.vue     # 系统概览
│   ├── ProcessesView.vue    # 进程管理
│   ├── ServicesView.vue    # 服务管理
│   ├── StorageView.vue      # 存储管理（ECharts treemap）
│   ├── LogsView.vue         # 日志查看
│   ├── TasksView.vue        # 任务列表
│   ├── AuditsView.vue       # 审计日志
│   ├── ActionsView.vue      # Action 列表
│   └── ConversationView.vue # 对话视图
├── components/
│   ├── Toast.vue            # 轻量提示
│   ├── LoadingOverlay.vue   # 加载遮罩
│   └── ConfirmDialog.vue    # 确认对话框
├── lib/
│   └── api.ts               # API 调用封装
├── router/
│   └── index.ts             # 路由配置
└── App.vue                  # 侧边栏布局 + 模式切换
```

### 4.3 API 调用模式

所有前端请求通过 `request<T>(path, init)` 封装，返回 `Promise<T>`。

```typescript
const DAEMON_ORIGIN = import.meta.env.VITE_DAEMON_ORIGIN ?? 'http://127.0.0.1:3000'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${DAEMON_ORIGIN}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  })
  if (!res.ok) throw new Error(await res.text())
  return res.json()
}
```

## 5. Cleanup 引擎详解

### 5.1 三层清理策略

| 层级 | 路径示例 | 执行要求 |
|------|----------|----------|
| L1 | `~/.cache/*` (用户缓存) | 可直接删除 |
| L2 | `/var/tmp/*` (临时文件) | 可直接删除 |
| L3 | `/var/log/*` (系统日志) | 需 root 权限 |

### 5.2 核心流程

```
cleanup_tree() ──→ 扫描 $HOME(/tmp/var/log/var/cache/apt) 生成树
                              │
                              ▼
                   cleanup_scan_path() ──→ 按需深挖子目录
                              │
                              ▼
                   cleanup_preview() ──→ 预览可删除项
                              │
                              ▼
                   cleanup_execute() ──→ 执行删除（L3 需 pkexec）
```

### 5.3 权限检查

`can_write()` 检查路径是否可写，避免误删系统文件。

## 6. 未来路线图

### v0.2 (当前附近)
- **AI 辅助**：读状态 + 给建议
- 不做自动执行主流程

### v0.3
- 允许 AI 发起 Action/Workflow 调用
- 必须人工确认后才能执行

### v0.4
- 引入受控 Agent Workflow

### v0.5
- 引入子 Agent 体系
- 主 Agent 治理子 Agent（创建/授权/升级/冻结/回收）

### v0.6
- 接入 Live2D 桌宠作为轻量交互壳

### 能力域扩展
- files / browser / content / workflow / coding / personal-ops

### 技术演进
- 跨发行版支持：Arch、Fedora、非 systemd 环境
- SQLite 持久化落地（当前为内存存储）
- 长记忆与复杂 RAG
- 完整自动化的子 Agent 自创建/自升级闭环
- 自进化闭环

### 桌面集成
- Electron 托盘、通知
- 悬浮模式 / 桌宠模式

## 7. 设计原则

1. **Daemon First**：所有核心逻辑在 Rust，Web 仅做呈现
2. **安全闭环**：L3 文件需显式授权，Cleanup 执行有白名单
3. **可审计**：所有写操作均记录 AuditEvent
4. **分层解耦**：Adapter → Toolkit → Core Runtime → Daemon API → UI
5. **渐进增强**：能力域按需扩展，不做过度设计

---

*最后更新：2026-04-20*
