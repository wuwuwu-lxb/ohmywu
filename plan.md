# OhMyWu Plan

## 1. 项目定义

### 1.1 北极星定义

OhMyWu 的长期目标是：

> 一个本地优先、可控执行、可审计、可扩展的个人 Agent 平台。

但 v0.1 的首发形态必须更聚焦：

> 一个真正可用的 Linux 电脑管家。

这两者不是冲突，而是分层表达：
- **对外产品形态**：电脑管家
- **对内平台定义**：个人 Agent 平台

电脑管家不是最终边界，而是最适合验证首发能力闭环的第一个能力域。

### 1.2 v0.1 首发定义

v0.1 只聚焦一个能力域：

> `system-management`

即：

> 在 Ubuntu / Debian + systemd + apt 环境下，先把系统管理能力做扎实，验证“能力资产沉淀 + 受控执行 + 可追踪闭环”是否成立。

v0.1 不追求完整 Agent 自治，也不把桌宠和多 Agent 作为主路径。

v0.1 当前要验证的核心链路是：

- 系统状态观察
- 稳定能力触发
- Policy 风险判断
- Task / Audit 追踪
- 能力沉淀为可复用 Action

### 1.3 产品与首发的关系

必须区分三层：

1. **产品本体**：个人 Agent 平台
2. **首发产品形态**：电脑管家
3. **首发能力域**：system-management

未来还会扩展其他能力域：
- files
- browser
- content
- workflow
- coding
- personal-ops

system-management 只是第一个能力域，不定义产品的全部边界。

## 2. 核心原则

### 2.1 Computer Manager First
先做电脑管家，不是因为目标只有电脑管家，而是因为系统管理最容易沉淀第一批稳定能力，并验证执行闭环。

### 2.2 AI Optional, Capability Persistent
AI 可以参与能力发现、规划、复现和编排；但一旦能力稳定，就必须能够脱离 AI 独立存在，并被用户直接触发。

### 2.3 Shared Capability Surface
用户模式、命令入口和未来 Agent 模式共享同一套能力资产，而不是维护两套系统。

### 2.4 Policy First
任何执行能力都必须先经过风险判断。

### 2.5 Audit First
所有关键读取和写操作都必须可追踪。

### 2.6 Human Directed Orchestration
用户必须可以自己理解和触发能力，不应被迫依赖 AI 才能完成稳定任务。

## 3. 关键概念

### 3.1 Atomic Capability
原子能力是底层执行单元，例如：
- read
- terminal command
- curl
- script runner

原子能力不直接在用户模式暴露，只作为 Action / Workflow / Agent 的执行基座。

### 3.2 Action
Action 是统一暴露给用户和 AI 的稳定能力入口。

Action 可以封装：
- Tool
- Workflow
- Script-based execution chain
- Future Agent template entry

Action 的关键不是底层怎么实现，而是：
- 能做什么事
- 需要什么输入
- 风险等级是什么
- 是否可直接触发
- 是否可追踪

### 3.3 Workflow
Workflow 是可重复执行的流程编排，是从一次次执行轨迹中沉淀出来的中间资产。

典型演化路径：

```text
Execution Trace
  → Reusable Workflow
  → Stable Action
  → Future Agent / capability asset
```

### 3.4 Agent
Agent 是具备角色、记忆、规划与能力调用能力的实体。

在 OhMyWu 中：
- Agent 未来调用同一套能力资产
- Agent 不拥有独立的一套底层能力体系
- v0.1 暂不以 Agent 自治为首发主流程

### 3.5 Capability Domain
Capability Domain 是围绕同类问题组织的能力域。

首发：
- `system-management`

未来：
- `file-ops`
- `browser-ops`
- `content-ops`
- `workflow-ops`
- `coding-ops`

## 4. v0.1 用户模式结构

### 4.1 全局主导航
当前用户模式收敛为四个主入口：

1. **总览**
2. **系统能力中心**
3. **Action**
4. **任务总览**

用户模式不直接暴露原子能力。

### 4.2 系统能力中心
系统能力中心是当前 system-management 的统一工作台。

系统能力中心内部采用固定二级导航：
- 进程
- 服务
- 存储
- 日志

这四项当前已经明确，因此直接写死，不做动态收纳。

### 4.3 Action 页面
Action 页面在用户模式中独立存在，用于承接已经成熟、稳定、可直接触发的能力资产。

本轮先保留独立页面与结构位置，后续再承接动态增长的能力池。

### 4.4 任务总览
任务总览在用户模式中合并：
- Task
- Audit

目标是让用户在一页中看到：
- 做了什么
- 谁触发的
- 风险等级
- 结果如何
- 是否可追溯

## 5. v0.1 范围

### 5.1 当前要做

#### A. Overview
- Daemon 运行状态
- 系统健康与基础摘要
- 关键入口与主能力概览

#### B. System Capability Center
- 进程查看与筛选
- 结束进程
- 服务查看与控制
- 存储扫描与清理
- 日志查看与基础诊断

#### C. Action Surface
- 保留独立页面与独立导航位
- 为后续稳定能力沉淀预留位置

#### D. Task Overview
- 统一展示 Task 与 Audit
- 强调执行追踪闭环

### 5.2 当前明确不做

- 高自主 Agent 主流程
- 子 Agent 自创建 / 自升级闭环
- 原子能力在用户模式直接暴露
- Live2D 桌宠完整交互主流程
- 长记忆 / 复杂 RAG
- 跨发行版兼容

## 6. 架构原则

### 6.1 Daemon First
Rust 后台是系统大脑，不把核心逻辑塞进 Electron 主进程。

### 6.2 Atomic Capability Hidden by Default
原子能力默认不在用户模式暴露，只通过更高层的 Action / Workflow / Agent 调用。

### 6.3 Action as Stable Asset Surface
Action 代表稳定能力资产的统一表面，不与具体 UI 或调用方式强绑定。

### 6.4 UI Around Problem Solving
用户界面应优先围绕“能完成什么问题”组织，而不是围绕底层实现颗粒度组织。

## 7. 当前信息架构

```text
User Mode
 ├─ Overview
 ├─ System Capability Center
 │   ├─ Processes
 │   ├─ Services
 │   ├─ Storage
 │   └─ Logs
 ├─ Action
 └─ Task Overview

Future Agent Mode
 ├─ Conversation
 ├─ Atomic Capabilities
 ├─ Agent Templates / Sub-agents
 └─ Orchestration / References
```

## 8. 当前技术实现策略

### 8.1 前端
- 使用左侧主导航承载四个主入口：总览 / 系统能力中心 / Action / 任务总览
- 在系统能力中心内部提供固定二级导航：进程 / 服务 / 存储 / 日志
- 复用现有进程 / 服务 / 存储 / 日志视图，避免无谓重写
- 合并任务与审计视图为统一的任务总览
- 存储页按 `safe / caution / manual / system` 四层语义组织，降低误判与误删心智负担

### 8.2 后端
- 保持 Rust daemon 为统一能力入口
- Action / Task / Audit 继续作为共享基础设施
- 原子能力继续作为底层执行实现，不提升到用户模式
- Cleanup classify 只把明确可再生内容归入 `L1`
- 大体积但未可靠分类的目录统一归入 `unknown`，默认不纳入快捷清理

### 8.3 Desktop
- 继续采用 Web first，再嵌入 Electron
- Desktop 当前以承接统一信息架构为主，不额外扩张新的用户模式导航
- 桌宠 / 悬浮模式放在后续阶段

## 9. 近期里程碑调整

### M0 — 定义收敛
- README、book 与 plan 对齐“电脑管家 + 稳定能力资产”叙事
- 用户模式 IA 收敛为左侧四个主入口

### M1 — 系统工作台闭环
- 总览
- 系统能力中心
- 固定二级导航
- Task / Audit 基础闭环

### M1.1 — 存储清理语义修正
- 修复 `folder/root` 被误算入“可放心清理”的问题
- 后端 classify 收紧，避免大目录仅因体积进入 `L2`
- 为 `unknown` 增加“默认不纳入快捷清理”分组

### M2 — 原子能力夯实
- command-runner 补齐超时精确控制、stderr 分离、exit_code 语义标准化
- linux-adapter 所有命令调用统一经由 command-runner，避免各 handler 自行调用 CommandRunner
- 确立原子能力的执行接口：timeout / stdout / stderr / exit_code 统一抽象
- 为后续 Action / Workflow / Agent 层提供确定性执行基座

### M3 — Action 执行闭环
- POST /api/actions/execute 统一执行入口
- ActionSpec 动态注册与查询能力
- Action 页面从展示清单升级为可触发操作的面板
- Action 与系统专项路由（/api/processes/:pid/kill 等）并存，共同暴露给 AI/Agent 调用层

### M4 — Agent 模式预埋
- 在不破坏用户模式的前提下，引入对话、原子能力展示和未来编排能力

## 10. 当前判断标准

如果以下条件成立，说明 v0.1 路线正确：

- 用户不依赖 AI 也能完成核心系统管理任务
- Action 与系统能力在 UI/路由层各自独立，在 AI/Agent 调用层统一，不相互阻断
- 任务和审计能够完整追踪执行链路
- 原子能力没有污染用户模式心智
- 存储快捷清理不会把未分类的大目录误导进安全路径
- 现有架构不会阻断未来 Agent 模式接入
