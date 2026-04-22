# OhMyWu 架构文档

> 本文档说明 OhMyWu 当前以“Linux 电脑管家”形态落地时的产品结构、系统分层、能力资产关系与后续演进方向。

## 1. 项目概览

OhMyWu 是一个**本地优先、可控执行、可审计**的电脑管家，同时也是一个持续沉淀能力资产的个人 Agent 平台。

当前 v0.1 的对齐方式是：

- **对外产品形态**：真正可用的 Linux 电脑管家
- **对内平台定义**：个人 Agent 平台的首发能力域验证
- **首发能力域**：`system-management`

**核心原则：Daemon First** —— 核心业务逻辑运行在 Rust Daemon，前端负责界面编排、状态呈现与受控交互。

```text
┌─────────────────────────────────────────────────────────┐
│                    Electron Shell                        │
│  ┌───────────────────────────────────────────────────┐  │
│  │                  Vue3 Web UI                       │  │
│  │         用户模式 / 系统能力中心 / Action           │  │
│  └───────────────────────────────────────────────────┘  │
│                           │                              │
│                    HTTP API (:3000)                      │
│                           │                              │
│  ┌───────────────────────────────────────────────────┐  │
│  │                 Rust Daemon                        │  │
│  │        Axum + Policy + Task + Audit + Registry     │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## 2. 产品结构与能力关系

v0.1 不追求完整 Agent 自治，也不把桌宠或多 Agent 作为首发主路径。

当前只验证这条闭环：

```text
系统状态观察
  → 用户或 AI 触发稳定能力
  → 受控执行
  → Task + Audit 可追踪
  → 能力沉淀为可复用 Action
```

这意味着：

1. **AI 可以参与发现、规划和编排**
2. **稳定能力必须能脱离 AI 独立存在**
3. **用户模式与未来 Agent 模式共享同一套能力资产**
4. **原子能力默认不在用户模式直接暴露**

## 3. 整体架构分层

```text
┌─────────────────────────────────────────────────┐
│           Shell Layer (Electron)                │
├─────────────────────────────────────────────────┤
│           Web UI Layer (Vue3)                   │
│   Overview / System Center / Action / Tasks     │
├─────────────────────────────────────────────────┤
│           Daemon API Layer (Axum)               │
│   Reference Resolver + Policy + Local REST API  │
├─────────────────────────────────────────────────┤
│             Core Runtime Layer                  │
│ Agent Kernel + Registries + Task + Audit + Store│
├─────────────────────────────────────────────────┤
│          Capability Toolkit Layer               │
│            toolkit-system-management            │
├─────────────────────────────────────────────────┤
│            Adapter Layer                        │
│         linux-adapter + command-runner          │
└─────────────────────────────────────────────────┘
```

## 4. Crate 职责矩阵

| Crate | 职责 | 依赖 |
|-------|------|------|
| `domain` | 核心类型定义：ActionSpec、ToolSpec、WorkflowSpec、AgentTemplate、Task、AuditEvent、CleanupNode 等纯数据结构 | 无 |
| `action-registry` | 统一管理 Action 表面，按名字存储稳定能力资产 | domain |
| `tool-registry` | 注册和发现 Tool，维护底层能力元数据 | domain |
| `workflow-registry` | 管理可复用 WorkflowSpec | domain |
| `reference-resolver` | 解析引用协议，如 Action / Agent 入口 | domain |
| `policy-engine` | Policy 校验：Sandbox / Danger + RiskLevel | domain |
| `task-engine` | 任务状态机：创建、运行、完成、失败 | domain |
| `audit` | 审计日志记录与反向遍历读取 | domain |
| `store` | 持久化边界（当前 InMemoryStore，预留 SQLite） | domain |
| `agent-kernel` | Agent 运行时，管理 Template 与 Instance | domain |
| `toolkit-system-management` | 首发能力域：overview / processes / services / cleanup / logs | domain |
| `linux-adapter` | Linux 原生封装：/proc / systemctl / du / journalctl / 文件删除 | domain, command-runner |
| `command-runner` | 统一命令执行：超时 / stdout / stderr / exit_code | 无 |

### 4.1 ohmywu-daemon (apps/daemon)

Rust 后台主入口，构建 AppState 聚合所有组件，通过 Axum 暴露本地 REST API。

**当前核心路由：**

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/actions` | 列出稳定 Action |
| GET | `/api/system/overview` | 系统概览 |
| GET | `/api/processes` | 进程列表 |
| GET | `/api/services` | 服务列表 |
| GET | `/api/tasks` | 任务列表 |
| GET | `/api/audits` | 审计日志 |
| POST | `/api/references/resolve` | 解析引用 |
| POST | `/api/storage/scans` | 通用存储扫描 |
| POST | `/api/logs/journal` | 日志读取 |
| POST | `/api/cleanup/tree` | 清理树 |
| POST | `/api/cleanup/scan-path` | 按需深挖 |
| POST | `/api/cleanup/preview` | 清理预览 |
| POST | `/api/cleanup/execute` | 执行清理 |
| POST | `/api/processes/:pid/kill` | 结束进程 |
| POST | `/api/services/:name/start\|stop\|restart` | 服务控制 |

## 5. 前端架构

### 5.1 当前用户模式结构

当前用户模式收敛为四个主入口：

1. **总览**
2. **系统能力中心**
3. **Action**
4. **任务总览**

其中：

- 左侧主导航只保留上述四个入口，降低侧边栏噪音
- 系统能力中心内部使用固定二级导航：**进程 / 服务 / 存储 / 日志**
- Action 保持独立页面，承接后续沉淀后的稳定能力资产
- 任务总览统一展示 Task 与 Audit，强调执行闭环与审计追踪

### 5.2 当前目录结构

```text
apps/web/src/
├── views/
│   ├── OverviewView.vue         # 总览
│   ├── SystemCenterView.vue     # 系统能力中心容器
│   ├── ProcessesView.vue        # 进程工作台
│   ├── ServicesView.vue         # 服务工作台
│   ├── StorageView.vue          # 存储清理工作台
│   ├── LogsView.vue             # 日志工作台
│   ├── ActionsView.vue          # Action 页面
│   └── TasksOverviewView.vue    # Task + Audit 总览
├── components/
│   ├── Toast.vue
│   └── ConfirmDialog.vue
├── lib/
│   └── api.ts
├── router/
│   └── index.ts
└── App.vue
```

### 5.3 路由组织

```text
/
├─ /
├─ /system
│  ├─ /system/processes
│  ├─ /system/services
│  ├─ /system/storage
│  └─ /system/logs
├─ /actions
└─ /tasks
```

### 5.4 API 调用模式

所有前端请求统一通过 `request<T>(path, init)` 封装，前端仅依赖 daemon API 契约，不直接耦合到底层实现。

## 6. 存储清理模型

### 6.1 当前分层语义

当前清理模型不再把“大目录”直接等同于“建议清理”。

| 分组 | 来源 level | 含义 |
|------|------------|------|
| 可放心清理 | `L1` | 明确可再生缓存或残留文件 |
| 建议确认后清理 | `L2` | 工具链、运行时或应用缓存，删除有重建成本 |
| 默认不纳入快捷清理 | `unknown` | 个人资料、项目目录或未可靠分类目录，只提示不默认操作 |
| 系统级项目 | `L3` | 系统级目录，通常需要更高权限 |

### 6.2 当前分类原则

- `L1`：只给明确可再生内容
  - 如 pip / uv / go-build / thumbnails / npm cache / pnpm store / cargo registry cache
- `L2`：只给需确认但仍具备一定可重建性的目录
  - 如浏览器缓存、toolchain versions、runtime files
- `unknown`：所有“只是大，但无法可靠证明可删”的目录
  - 如 workspace、文档、下载、个人配置、项目目录等
- `L3`：系统级目录
  - 如 `/var/log`、`/var/cache/apt`

### 6.3 设计目标

存储清理的目标不是“尽量多删”，而是：

1. 先保障用户数据安全
2. 再提供快捷清理路径
3. 未分类内容只做提示，不放进一键清理主路径
4. 所有执行继续进入 Task + Audit 链路

## 7. 当前运行闭环

```text
用户进入工作台
  → 前端请求 daemon API
  → daemon 通过 toolkit 调用 linux-adapter
  → policy / task / audit 参与执行闭环
  → UI 展示结果、风险等级与追踪记录
```

写操作链路保持一致：

```text
用户触发操作
  → 风险判断
  → 执行命令或清理动作
  → 写入 Task
  → 写入 Audit
  → 前端展示结果
```

## 8. 未来路线

### 8.1 短期
- 完成桌面集成（Electron 主窗口、托盘、通知）
- 继续完善 Action 页面，承接稳定能力资产
- 继续细化存储清理分类，让更多目录从 unknown 精准沉淀为 L1 / L2

### 8.2 中期
- 允许 AI 发起 Action / Workflow 调用
- 保持人工确认与 Policy 控制
- 让执行轨迹持续沉淀为 Workflow 与 Action

### 8.3 长期
- 引入 Agent 模式与子 Agent 体系
- 接入 Live2D 桌宠作为轻量交互壳
- 扩展 files / browser / content / coding / personal-ops 等能力域

## 9. 设计原则

1. **Daemon First**：核心逻辑在 Rust，UI 不承载业务根逻辑
2. **Action as Stable Asset Surface**：Action 是稳定能力资产的统一表面
3. **Atomic Capability Hidden by Default**：原子能力默认不在用户模式直接暴露
4. **Audit First**：所有关键动作必须可追踪
5. **UI Around Problem Solving**：围绕用户问题组织，不围绕底层实现颗粒度组织
6. **Safety Before Convenience**：快捷清理必须建立在清晰分类上

---

*最后更新：2026-04-22*
