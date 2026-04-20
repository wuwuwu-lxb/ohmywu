# OhMyWu Plan

## 1. 项目定义

### 1.1 北极星定义

OhMyWu 不是“聊天机器人加系统工具”，也不是“电脑管理产品后面再接一个 AI”。

OhMyWu 的产品本体应定义为：

> 一个本地优先、可控执行、可审计、可扩展的个人 Agent 平台。

这个平台的核心不是某一组具体功能，而是以下通用底座：

- Agent Runtime
- Action Registry
- Tool Registry
- Workflow Registry
- Skill Composition
- Policy Engine
- Task Engine
- Audit System
- Memory System
- Model Provider Layer

AI 在这个系统里不应被视为能力本体，而应被视为对这些能力资产进行规划、调用、组合、升级和协调的一层。

电脑管理能力不是产品本体，而是首发最适合落地的一组能力域。

### 1.2 v0.1 首发定义

v0.1 只聚焦一个能力域：

> `system-management`

即：

> 在 Ubuntu / Debian + systemd + apt 环境下，先把“电脑管理”这组 tools / skills 做扎实，验证通用 Agent 平台底座是否成立。

因此，v0.1 的目标不是做完整 Agent 自治，也不是做“全能桌宠”，而是先验证以下链路：

- Action 共享调用链
- Tool 执行链
- Policy 安全链
- Task 编排链
- Audit 审计链
- UI 控制链
- Daemon 与桌面壳通信链

### 1.3 产品与首发的关系

必须严格区分三层：

1. 产品本体：个人 Agent 平台
2. 首发能力域：system-management
3. 未来扩展能力域：files / browser / content / workflow / coding / personal-ops 等

电脑管理能力未来只会被部分复用为 Agent 的一组 skills / tools，不会定义 Agent 的全部边界。

## 2. 关键概念

### 2.1 Action

Action 是底层原子能力资产，用户模式和 Agent 模式共用同一套调用入口。

一个 Action 底层可以指向：

- Tool
- Workflow
- Skill
- Agent Template

**概念区分：**
- **工具（Tool Panel）**：用户直接在用户模式工具箱中操作的界面项，每个工具底层调用一个或多个 Action
- **Action**：底层原子能力，工具调用链的底层，不直接暴露给用户但 AI 可以引用

这意味着：
- 用户模式工具箱里看到的是”工具”，底层是 Action
- AI 在对话和编排时引用的是 Action
- 同一个 Action 可以被多个工具调用
- UI 层不应把“展示项”与底层 Tool 类型强绑定

对话中应允许直接引用 Action，例如：

- `/scan_storage`
- `/restart_service`
- `/md_to_pdf`

### 2.2 Tool

Tool 是最小可执行单元，具备明确输入、输出、权限级别和审计记录。

例子：

- `list_processes`
- `kill_process`
- `list_services`
- `restart_service`
- `scan_storage`
- `preview_cleanup`

### 2.3 Workflow

Workflow 是可重复执行的流程编排，可以不依赖 AI 独立运行。

Workflow 常用于承接那些已经被验证过、输入结构相对稳定、可长期复用的任务。

例子：

- `md_to_pdf`
- `collect_service_failure_bundle`
- `cleanup_cache_preview`

### 2.4 Skill

Skill 是面向任务的能力包，由 Tool、Workflow、策略和上下文组成。

例子：

- “诊断为什么电脑变卡”
- “清理磁盘空间”
- “分析启动慢的原因”

Skill 不等于 Agent。Skill 更像是能力包或任务模板。

### 2.5 Agent

Agent 是具有角色、记忆、目标、规划能力和工具调用能力的实体。

Agent 可以：

- 调用 Action
- 调用 Tool
- 调用 Workflow
- 调用 Skill
- 组合多个 Skill 完成任务
- 创建或调度子 Agent

主 Agent 对子 Agent 的创建、授权、升级和回收拥有治理权。

但在 v0.1 中，Agent 只做结构预埋，不做首发主流程。

### 2.6 Agent Instance 与 Agent Template

`Agent Instance` 是一次任务中临时存在的子 Agent。

`Agent Template` 是可复用的子 Agent 模板，可由主 Agent 或用户显式调用。

建议的生命周期：

- 创建
- 试用
- 升级
- 冻结
- 废弃

升级后的 Agent Template 可以继续被主 Agent 调用，也可以在对话中被用户显式引用，例如：

- `@system-ops`
- `@doc-worker`
- `@cleanup-reviewer`

### 2.7 Capability Domain

Capability Domain 是能力域，即一组围绕同类问题组织的 tools / skills。

首发只做：

- `system-management`

未来可以扩展：

- `file-ops`
- `browser-ops`
- `content-ops`
- `workflow-ops`
- `coding-ops`

### 2.8 Execution Trace 与 Promotion Pipeline

Execution Trace 是一次任务执行过程中完整的轨迹记录，包含：

- 谁触发了任务
- 引用了哪些 Agent
- 调用了哪些 Action
- 参数如何被填充
- 每一步的结果和失败点

Promotion Pipeline 是把稳定执行轨迹沉淀为长期资产的机制。

典型路径应为：

`Execution Trace -> Workflow Candidate -> Stable Workflow / Skill / Action`

例如：

- 一开始由 AI 调用 `/md_to_pdf`
- 多次成功后整理出稳定执行路径
- 最终沉淀为用户模式和 Agent 模式都可复用的 Action

## 3. 首发范围

### 3.1 v0.1 要做的内容

#### A. System Overview

- CPU、内存、磁盘、网络、系统版本、运行时长、基础硬件摘要
- 机器当前负载和关键资源指标

#### B. Process Management

- 进程列表
- 搜索 / 排序 / 筛选
- 进程详情
- 结束进程

#### C. Service Management

- systemd 服务列表
- 服务详情与状态
- 启动 / 停止 / 重启
- 开机自启状态展示

#### D. Storage & Cleanup

- 目录占用分析
- 大文件扫描
- 缓存候选项扫描
- 清理预览
- 明确确认后执行清理

#### E. Logs & Diagnostics

- journal 关键日志读取
- 最近异常
- 开机相关事件
- 服务失败信息聚合

#### F. Task & Audit

- 所有扫描与执行动作进入任务系统
- 每个动作都写入审计记录
- 能看到执行人、时间、目标、风险等级、结果摘要

#### G. Settings & Runtime Controls

- 运行模式切换
- 权限说明
- Action / Workflow / Agent Template 浏览入口
- Provider / Memory / Agent 入口占位

#### H. Action Center & Reference Composer

- 浏览共享 Action 列表
- 浏览可复用 Workflow
- 浏览 Agent Template
- 在对话或命令输入中直接引用 `@agent` 与 `/action`
- 支持用户手动拆分任务步骤，并指定每一步由哪个 Agent 或 Action 执行

### 3.2 v0.1 明确不做

- 无监督、高自主的 Agent 主流程
- 完整自动化的子 Agent 自创建 / 自升级闭环
- 自进化闭环
- Live2D 桌宠
- 跨发行版兼容
- 大范围“一键优化”
- 危险批量删除
- 多模型复杂调度
- 长记忆与复杂 RAG

### 3.3 平台边界

v0.1 只支持以下环境：

- Ubuntu 22.04 / 24.04
- Debian 12
- systemd
- apt

不在 v0.1 范围内：

- Arch
- Fedora
- 非 systemd 环境
- snap / flatpak / nix 特化管理

## 4. 核心产品原则

### 4.1 Daemon First

Rust 后台是系统大脑，不把核心逻辑塞进 Electron 主进程。

### 4.2 Shared Action Surface

用户和 AI 必须通过同一套 Action 表面访问能力。

Action 统一承载：

- 用户模式工具栏
- 对话中的 `/action` 引用
- Agent 编排时的能力调用

### 4.3 Asset First

真正长期有价值的资产，是 Tool、Workflow、Skill 和 Agent Template。

AI 的价值在于：

- 理解目标
- 规划步骤
- 选择能力
- 协调执行
- 促进沉淀

而不是替代这些能力资产本身。

### 4.4 Policy First

任何执行能力都必须先经过 Policy 判断。

### 4.5 Audit First

所有写操作和关键读取都必须可追溯。

### 4.6 Human Directed Orchestration

系统必须支持“用户自己拆任务，再交给 AI 或 Action 执行”。

这意味着：

- 用户可以手动定义步骤
- 用户可以显式指定 `@agent`
- 用户可以显式指定 `/action`
- AI 不是唯一的任务拆解入口

### 4.7 Web First

先做网页控制台，跑顺信息架构和交互，再嵌入 Electron。

### 4.8 Linux Native

围绕 Linux 原生能力设计，不为未来跨平台提前做抽象噪音。

### 4.9 Main-Agent Governance

多 Agent 必须由主 Agent 统一治理，不能做成自由混战。

主 Agent 负责：

- 子 Agent 创建
- 工具授权
- 生命周期管理
- 结果汇总
- 升级与回收决策

## 5. 总体架构

### 5.1 分层结构

```text
Electron Shell / Future Pet Shell
            |
   Web UI + Conversation Surface
            |
 Reference Resolver + Local API / Event Stream
            |
        Rust Daemon
            |
Core Runtime + Agent Orchestrator + Capability Assets
            |
 Linux Adapters / Controlled Executors
```

### 5.2 核心分层

#### A. Shell Layer

- Electron 主窗口
- 托盘
- 通知
- 后续悬浮模式 / 桌宠模式

#### B. Web UI Layer

- Control Center
- Conversation / Command Composer
- Action Center
- System Management Workspace
- Task / Audit / Settings 界面

#### C. Daemon Layer

- 本地 API
- 事件流
- `@agent` / `/action` 引用解析
- 任务调度
- 策略检查
- 审计记录
- Action / Workflow / Agent Template 调用入口

#### D. Core Runtime Layer

- Domain Models
- Action Registry
- Tool Registry
- Workflow Registry
- Policy Engine
- Task Engine
- Audit Engine
- Execution Trace
- Promotion Pipeline
- Store
- Agent Kernel
- Agent Lifecycle

#### E. Capability Toolkit Layer

首发只做：

- `toolkit-system-management`

以后继续加：

- `toolkit-files`
- `toolkit-browser`
- `toolkit-content`
- `toolkit-code`

#### F. Adapter Layer

- `/proc`
- `/sys`
- `systemctl`
- `journalctl`
- `du`
- `find`
- 受控命令执行器

## 6. 推荐目录结构

```text
ohmywu/
  Cargo.toml
  pnpm-workspace.yaml
  plan.md
  README.md
  docs/
    architecture.md
    api.md
    ui-map.md
    security.md
  apps/
    daemon/      # Rust 后台 — API、任务调度、事件分发、策略校验
    web/        # 网页控制台 — Vue3 + Vite，独立开发后嵌入 Electron
  desktop/     # Electron 桌面壳 — 主窗口、托盘、通知、Live2D 桌宠（未来）
  crates/
    domain/
    action-registry/
    tool-registry/
    workflow-registry/
    reference-resolver/
    policy-engine/
    task-engine/
    audit/
    store/
    agent-kernel/
    toolkit-system-management/
    linux-adapter/
    command-runner/
  scripts/
  assets/
```

### 6.1 目录职责

#### `apps/daemon`

本地后台服务。负责 API、任务调度、事件分发、策略校验和工具调用入口。

#### `apps/web`

第一阶段主交互界面。网页先独立开发，后续直接嵌入 Electron。

#### `desktop`

桌面壳。Electron + Vue3 + Vite，负责窗口、托盘、通知和后续悬浮/桌宠模式。

#### `crates/domain`

定义核心对象：

- `ActionSpec`
- `ToolSpec`
- `WorkflowSpec`
- `ActionRef`
- `AgentRef`
- `Task`
- `TaskStatus`
- `PolicyMode`
- `ApprovalRequirement`
- `AuditEvent`
- `ExecutionTrace`
- `ExecutionResult`

#### `crates/action-registry`

统一管理用户与 AI 共用的 Action 表面。

负责：

- Action 元数据
- Action 到 Tool / Workflow / Skill / Agent Template 的映射
- 对话、UI、Agent 编排层的统一引用入口

#### `crates/tool-registry`

注册和发现 Tool，提供统一元数据和调用入口。

#### `crates/workflow-registry`

管理可重复执行的 Workflow。

负责：

- Workflow 元数据
- 输入输出约束
- 参数模板
- 调用入口

#### `crates/reference-resolver`

解析显式引用协议。

负责：

- `@agent` 解析
- `/action` 解析
- 对话命令到执行计划的映射

#### `crates/policy-engine`

负责：

- 沙箱模式 / 危险模式
- 风险分级
- 动作审批
- 执行前检查

#### `crates/task-engine`

负责：

- 任务排队
- 进度更新
- 取消
- 超时
- 重试
- 任务状态机

#### `crates/audit`

记录：

- 谁触发了什么
- 使用了什么工具
- 走了什么权限策略
- 结果如何

#### `crates/store`

持久化：

- 设置
- Action 元数据
- Workflow 元数据
- Agent Template 元数据
- 任务
- 扫描结果
- 审计记录
- Execution Trace
- Promotion Candidate
- 未来 Provider 配置

#### `crates/agent-kernel`

承载 Agent 的基础运行时模型。

负责：

- 主 Agent 与子 Agent 协调模型
- Agent Template / Agent Instance
- 生命周期管理
- 主 Agent 对子 Agent 的治理接口
- 多 Agent 任务汇总入口

#### `crates/toolkit-system-management`

首发能力域实现。包含系统概览、进程、服务、存储、清理、日志相关 Tool、Workflow、Skill 和 Action 映射。

#### `crates/linux-adapter`

封装 Linux 原生读取与受控执行能力，禁止业务层直接拼 shell。

#### `crates/command-runner`

统一命令执行层，处理超时、stdout / stderr、退出码、日志和权限边界。

## 7. 运行模型

### 7.1 基本链路

```text
User Action / Manual Plan / Conversation Request / Future Agent Request
            |
   Reference Resolver (@agent, /action)
            |
        Action Registry
            |
 Planner / Orchestrator (optional)
            |
 Workflow Registry / Tool Registry / Agent Templates
            |
       Policy Engine
            |
        Task Engine
            |
   Toolkit + Linux Adapter
            |
  Audit / Execution Trace / Store
```

### 7.2 调度模式

#### User Mode

- 用户直接调用 Action / Workflow
- 不依赖 AI 也能完成稳定任务

#### Assisted Mode

- 用户手动拆任务
- 用户显式引用 `@agent` 与 `/action`
- AI 负责建议、补参与部分编排

#### Agent Mode

- 主 Agent 负责理解目标和生成任务图
- 子 Agent 按授权执行
- 仍然复用同一套 Action / Workflow / Tool 资产

### 7.3 关键约束

- UI 不直接操作系统
- 对话层不直接操作系统
- 对话层不直接调用 shell
- 所有显式引用都必须先解析成 Action / Agent Template
- Daemon 不允许绕过 Policy 直接执行危险操作
- Tool / Workflow / Action 执行结果必须统一回写任务系统和审计系统
- Execution Trace 是一等资产，后续可用于 Workflow / Action 沉淀
- 未来 Agent 只能通过 Action Registry 与 Tool Registry 访问能力
- 子 Agent 不能自行扩大权限或绕过主 Agent 治理

## 8. 安全与权限模型

### 8.1 运行模式

#### Sandbox Mode

- 默认模式
- 只读优先
- 写操作生成预览和执行计划
- 高风险动作不真实落地

#### Danger Mode

- 明确开启后才允许真实执行受控写操作
- 仍需按风险等级确认
- 审计不可关闭

### 8.2 风险等级

- `ReadOnly`
- `ControlledWrite`
- `HighRisk`

### 8.3 权限策略

v0.1 不做常驻 root daemon。

高权限相关动作采用：

- 尽可能只读
- 在当前用户权限内先实现大多数功能
- 必要时按动作触发临时提权
- 所有提权动作必须带审计记录

### 8.4 安全原则

- 不做一键高风险优化
- 不做无法解释的自动删除
- 不做隐式提权
- 不做绕开确认的危险写操作

## 9. 数据与存储

### 9.1 建议存储方案

- SQLite：设置、任务、审计、扫描结果
- 本地文件：日志、缓存结果、导出文件

### 9.2 关键数据对象

- `settings`
- `actions`
- `workflows`
- `agent_templates`
- `tasks`
- `task_events`
- `audit_events`
- `execution_traces`
- `promotion_candidates`
- `storage_scans`
- `cleanup_plans`
- `service_actions`
- `provider_configs`（预留）

## 10. API 规划

### 10.1 基础接口

- `GET /api/health`
- `GET /api/version`
- `GET /api/events`

### 10.2 Action / Workflow / Agent 接口

- `GET /api/actions`
- `GET /api/actions/:name`
- `POST /api/actions/:name/run`
- `GET /api/workflows`
- `POST /api/workflows/:name/run`
- `GET /api/agents/templates`
- `POST /api/agents/templates/:name/invoke`
- `POST /api/references/resolve`

### 10.3 System Management 接口

- `GET /api/system/overview`
- `GET /api/processes`
- `GET /api/processes/:pid`
- `POST /api/processes/:pid/kill`
- `GET /api/services`
- `GET /api/services/:name`
- `POST /api/services/:name/start`
- `POST /api/services/:name/stop`
- `POST /api/services/:name/restart`
- `POST /api/storage/scans`
- `GET /api/storage/scans/:id`
- `POST /api/cleanup/preview`
- `POST /api/cleanup/execute`
- `GET /api/logs/journal`

### 10.4 Runtime 接口

- `GET /api/tasks`
- `GET /api/tasks/:id`
- `GET /api/audits`
- `GET /api/settings`
- `PUT /api/settings`
- `GET /api/providers`
- `PUT /api/providers`

## 11. UI 规划

### 11.1 顶层信息架构

顶部模式切换器：**用户模式** / **Agent 模式**（Agent 模式暂未开放，灰掉）

#### 用户模式（类 360 工具箱）

纯工具面板，不经过对话层，用户直接操作：

- System Dashboard
- 进程管理（列表 + 搜索 + 结束进程）
- 服务管理（列表 + 启/停/重启）
- 存储扫描 + 清理
- 日志查看
- Tasks（任务记录）
- Audits（审计记录）

#### Agent 模式（暂缓）

对话入口 + 编排层，Agent 模式就绪后接入。

### 11.2 页面清单

**用户模式页面：**
- `Overview` — 系统总览 + 快速入口
- `Tools/Dashboard` — 工具箱首页
- `Tools/Processes` — 进程管理
- `Tools/Services` — 服务管理
- `Tools/Storage` — 存储扫描
- `Tools/Cleanup` — 清理预览与执行
- `Tools/Logs` — 日志查看
- `Tasks` — 任务记录
- `Audits` — 审计记录

**Agent 模式页面（暂缓）：**
- `Conversation`
- `Actions`
- `Agent Templates`
- `Settings`

### 11.3 页面重点

#### `Overview`

展示产品总览和能力域入口，不承载复杂系统操作。

#### `Conversation`

是用户模式、辅助模式和未来 Agent 模式的统一入口。

必须支持：

- 输入普通自然语言
- 直接引用 `@agent`
- 直接引用 `/action`
- 手动拆任务为多个步骤

#### `Actions`

展示共享 Action 列表，并说明每个 Action 底层映射到什么能力资产。

#### `Agent Templates`

展示可显式调用的 Agent 模板、其权限边界和可用 Action 范围。

#### `System Dashboard`

展示 Linux 当前状态和资源摘要。

#### `Processes`

重点做搜索、排序、详情和结束确认。

#### `Services`

重点做状态展示和启停控制。

#### `Storage`

回答“空间去哪了”。

#### `Cleanup`

必须先预览再执行。

#### `Logs`

回答“发生了什么异常”。

#### `Tasks`

展示扫描与执行过程。

#### `Audits`

展示行为记录与风险级别。

#### `Settings`

保留 Agent / Memory / Provider 配置入口，但默认不开放完整能力。

## 12. 实施阶段

### M0. 规划冻结

目标：

- 明确产品本体与首发能力域
- 明确目录结构
- 明确模块职责
- 明确 UI 信息架构
- 明确安全边界

验收标准：

- `plan.md` 可直接指导落地
- 任何新需求都能判断是否属于 v0.1

### M1. 项目骨架

目标：

- Rust workspace 初始化
- `apps/daemon` / `apps/web` / `desktop` 空壳
- 基础本地 API
- Action Registry / Workflow Registry 骨架
- `@agent` / `/action` 引用解析骨架
- 基础前端路由
- Electron 空窗口

验收标准：

- Daemon 能启动
- Web 能跑
- Electron 能加载 Web
- 基础引用协议可解析

#### M1.1 无返工骨架原则

M1 不是“把目录搭出来”这么简单，而是要把后续不会轻易推翻的边界先冻结。

M1 必须保证：

- Electron 不承载核心业务逻辑
- Web 不直接绑定某个具体能力域
- Action、Workflow、Tool、Agent Template 四层抽象不混用
- 用户模式与 Agent 模式从第一天起共用同一套调用资产
- 首发只实现 `system-management`，但骨架必须允许新增能力域

#### M1.2 Workspace 最小结构

M1 完成后，目录至少应达到以下状态：

```text
ohmywu/
  Cargo.toml
  pnpm-workspace.yaml
  apps/
    daemon/
    web/
    electron/
  crates/
    domain/
    action-registry/
    tool-registry/
    workflow-registry/
    reference-resolver/
    policy-engine/
    task-engine/
    audit/
    store/
    agent-kernel/
    toolkit-system-management/
    linux-adapter/
    command-runner/
```

要求：

- 所有 crate 都能被 workspace 正确发现
- 所有 app 都有最小启动入口
- 命名直接反映未来职责，避免后续重命名迁移

#### M1.3 Rust Crates 最小职责

M1 不要求把能力做完，但要求接口位置和依赖方向先正确。

`domain`

- 只放纯数据结构和枚举
- 不依赖 UI 和平台实现

`action-registry`

- 定义 Action 注册表接口
- 支持按名字查找 Action
- 支持把 Action 映射到 Tool / Workflow / Agent Template

`tool-registry`

- 定义 Tool 注册与调用接口
- 先放 mock / stub 实现也可以

`workflow-registry`

- 定义 Workflow 结构
- 支持注册稳定流程

`reference-resolver`

- 解析 `@agent` 与 `/action`
- 把显式引用转成统一的内部引用对象

`policy-engine`

- 定义 Sandbox / Danger 模式和风险等级
- M1 只需要接口与基础枚举

`task-engine`

- 定义 Task 状态机
- 支持最小的任务创建与状态流转

`audit`

- 定义 AuditEvent 结构与记录接口

`store`

- 定义持久化边界
- M1 可先用内存实现占位

`agent-kernel`

- 定义 Main Agent / Agent Template / Agent Instance 结构
- 定义主 Agent 对子 Agent 的治理接口

`toolkit-system-management`

- 先注册少量占位 Action / Tool
- 例如：`/system_overview`、`/list_processes`

`linux-adapter`

- 定义 Linux 读取与命令调用适配接口
- M1 不要求接完真实系统能力

`command-runner`

- 定义统一执行器接口
- 约束超时、退出码、输出结构

#### M1.4 Apps 最小职责

`apps/daemon`

- 启动本地 HTTP API
- 暴露健康检查接口
- 暴露最小的 Action 列表接口
- 暴露最小的引用解析接口

`apps/web`

- 跑起基础路由
- 至少包含 `Overview`、`Conversation`、`Actions` 三个页面壳
- 能调用 daemon 的健康检查和 Action 列表接口

`desktop`（Electron 壳）

- 能启动主窗口
- 能加载 web
- 先只做壳，不塞业务逻辑

#### M1.5 第一批必须冻结的接口

这些接口在 M1 就应该出现，哪怕先返回 stub 数据：

- `GET /api/health`
- `GET /api/actions`
- `POST /api/references/resolve`
- `GET /api/settings`

这些对象在 M1 就应该有明确结构：

- `ActionSpec`
- `ActionTarget`
- `WorkflowSpec`
- `AgentTemplate`
- `AgentInstance`
- `Task`
- `AuditEvent`
- `ResolvedReference`

#### M1.6 第一版引用协议

对话层和未来命令层在 M1 就应统一采用显式引用协议：

- `@agent-name`
- `/action-name`

要求：

- 可以从一段对话中识别出多个引用
- 可以区分普通文本和显式引用
- 解析结果可以直接交给 orchestrator 或手动计划器

例子：

- `@system-ops /scan_storage`
- `@doc-worker /md_to_pdf`
- `先执行 /system_overview，再把结果交给 @system-ops`

#### M1.7 第一版编排边界

M1 不做完整 AI 编排，但必须预留两条入口：

- 用户手动拆任务并指定步骤执行者
- 未来主 Agent 自动生成任务图

因此内部模型至少要支持：

- Step
- StepTarget
- PlanDraft
- ExecutionTrace

#### M1.8 M1 完成时的真实交付物

M1 结束时，应该看到这些东西：

- 一个可编译的 Rust workspace
- 一个可启动的 daemon
- 一个可运行的 web
- 一个能加载 web 的 Electron 壳
- 一个可返回 Action 列表的 API
- 一个可解析 `@agent` / `/action` 的引用解析器
- 一个最小的 `system-management` Action 注册表

#### M1.9 M1 明确不做

为了避免返工，M1 不要提前做这些：

- 复杂 AI 对话逻辑
- 真正的多 Agent 自主协作
- 完整数据库 schema
- 大量真实 Linux 管理能力
- Live2D 或桌宠交互
- 漂亮但无约束的前端页面

### M2. 只读能力落地

目标：

- 系统总览
- 进程列表
- 服务列表
- 存储扫描
- 日志读取

验收标准：

- 页面展示真实 Linux 数据
- 不再依赖 mock

**状态：✅ 完成（commit 6a681c2）**

### M3. 可控操作落地

目标：

- 结束进程
- 服务启停 / 重启
- 清理预览与执行（🔲 待做）
- 任务状态更新
- 审计记录落地

验收标准：

- 至少 3 类真实操作跑通
- 每个操作都有任务记录和审计记录

**状态：🔲 进行中（kill + 服务启停 + TaskEngine + AuditLog 已完成；清理预览待做）**

### M4. 桌面集成

目标：

- Electron 主窗口
- 托盘
- 通知
- 基础窗口管理

验收标准：

- Web 与桌面壳完成首轮整合

### M5. Agent 预埋

目标：

- Action Registry 稳定
- Tool Registry 稳定
- Workflow Registry 稳定
- Policy 稳定
- Audit 稳定
- Agent Template / Agent Instance 模型稳定
- Provider / Agent 设置入口稳定

验收标准：

- 未来 Agent 已有清晰挂载点
- 主 Agent 与子 Agent 的治理模型已冻结
- 但默认不允许高自主执行

## 13. 测试与验收

### 13.1 测试重点

- `policy-engine` 单测
- `task-engine` 单测
- `toolkit-system-management` 单测
- Linux adapter 集成测试
- API 集成测试
- 前端关键页面状态测试

### 13.2 手工验收重点

- 无权限读取日志
- 服务不存在
- 结束进程失败
- 清理预览与真实执行不一致
- 命令超时
- 任务取消
- 提权失败

## 14. v0.1 之后的路线

### v0.2

引入 AI 辅助，但先只做“读状态 + 给建议”，不做自动执行主流程。

### v0.3

允许 AI 发起 Action / Workflow 调用，但必须人工确认。

### v0.4

引入受控 Agent Workflow。

### v0.5

引入子 Agent 体系。

### v0.6

接入 Live2D 桌宠，作为轻量交互壳，不作为核心功能承载层。

## 15. 当前冻结的关键决策

- 产品本体是个人 Agent 平台，不是单纯电脑管理工具
- v0.1 首发能力域是 `system-management`
- 首发平台收窄到 Ubuntu / Debian + systemd + apt
- 先做 Web 控制台，再嵌入 Electron
- Rust 核心以独立 Daemon 形式存在
- Action 是用户与 AI 共用的统一调用表面
- 对话中允许显式引用 `@agent` 与 `/action`
- 用户可以手动拆任务，并指定步骤交给哪个 Agent 或 Action
- AI 完成过的稳定任务，应有机会沉淀为 Workflow / Action
- 主 Agent 负责子 Agent 的创建、授权、升级、冻结和回收
- 电脑管理能力未来只会被部分复用为 Agent skills / tools
- v0.1 不开放高自主 Agent 执行
- v0.1 不做桌宠
- v0.1 不做常驻 root daemon

## 16. 下一步

完成本计划后，编码阶段按以下顺序推进：

1. 初始化目录结构与 workspace
2. 搭建 daemon / web / electron 三端骨架
3. 先做只读能力链路
4. 再做可控执行链路
5. 最后补桌面集成与 Agent 预埋
