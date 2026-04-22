# OhMyWu

OhMyWu 是一个**本地优先、可控执行、可审计**的电脑管家，同时也是一个会持续沉淀能力资产的个人 Agent 平台。

当前 v0.1 先以 **Linux 电脑管家** 的形态落地：先把系统管理做扎实，让用户不用依赖 AI 也能直接完成稳定任务；同时保留统一能力资产和未来 Agent 编排的底座。

## 当前产品定位

- **对外**：真正可用的 Linux 电脑管家
- **对内**：个人 Agent 平台的首发能力域验证
- **首发能力域**：`system-management`
- **支持环境**：Ubuntu 22.04 / 24.04、Debian 12、systemd、apt

## v0.1 目标

v0.1 不追求完整 Agent 自治，也不把桌宠或多 Agent 作为主路径。

当前只验证这条闭环：

```text
系统问题观察
  → 用户或 AI 触发稳定能力
  → 受控执行
  → Task + Audit 可追踪
  → 能力继续沉淀为可复用 Action
```

也就是说：
- AI 可以参与发现、规划和沉淀能力
- 稳定能力最终必须能脱离 AI 独立存在
- 用户模式和未来 Agent 模式共享同一套能力资产

## 当前用户模式结构

用户模式当前收敛为四个主入口：

1. **总览**
   - 展示当前系统状态、Daemon 状态和关键入口
2. **系统能力中心**
   - 承接进程、服务、存储、日志四个系统工作台
   - 页面顶部使用固定二级导航：进程 / 服务 / 存储 / 日志
3. **Action**
   - 稳定能力资产中心
   - 当前先保留占位，后续承接动态增长的可直接触发能力
4. **任务总览**
   - 合并 Task 与 Audit
   - 强调命令、风险等级、执行结果、审计可追踪

## 核心原则

### 1. Computer Manager First
首发先做电脑管家，不是因为目标只有电脑管家，而是因为 system-management 最适合沉淀第一批稳定能力。

### 2. AI Optional, Capability Persistent
AI 可以参与能力发现、规划、复现和编排；但一旦能力稳定，就应当能脱离 AI 独立存在，并被用户直接触发。

### 3. Shared Capability Surface
用户模式、命令入口和未来 Agent 模式应共享同一套能力资产，而不是维护两套系统。

### 4. Policy First
任何执行能力都必须经过风险判断。系统分为沙箱模式与危险模式，能力按 ReadOnly / ControlledWrite / HighRisk 分级。

### 5. Audit First
所有关键读取和写操作都必须可追踪。Task 与 Audit 不是配套页，而是用户信任闭环的一部分。

## 关键概念

### 原子能力（Atomic Capability）
底层执行单元，例如 read、terminal command、curl、script runner。原子能力不直接在用户模式暴露，只作为 Action / Workflow / Agent 的执行基座。

### Action
对用户和 AI 统一暴露的稳定能力入口。Action 可以封装 Tool、Workflow 或脚本执行链路。用户看到的是“能完成什么事”，不是底层执行细节。

### Workflow
经过验证、可重复执行的流程编排，是从一次次执行轨迹中沉淀出来的中间资产。

### Agent
具有角色、记忆、规划和编排能力的实体。未来 Agent 模式会调用同一套能力资产，但 v0.1 不以 Agent 自治为首发主路径。

## 当前已落地能力

### 系统能力中心
- 系统总览
- 进程查看与结束进程
- systemd 服务查看与启动/停止/重启
- 存储扫描与清理预览/执行
- journal 日志读取与基础诊断

### 统一追踪
- Task 记录所有扫描与执行动作
- Audit 记录执行主体、目标、风险等级和结果摘要

## 为什么不是普通电脑管家

OhMyWu 和传统系统工具的区别不在于“会不会显示 CPU 和内存”，而在于：

- 它把稳定能力沉淀成可复用资产
- 它允许这些能力被用户直接触发，也能被未来 Agent 编排复用
- 它强调能力治理、风险控制和执行追踪
- 它为后续的个人 Agent 平台打下统一底座

## 当前开发重点

- 收敛用户模式信息架构，减少导航噪音
- 把系统管理能力统一收口到系统能力中心
- 保持 Action 的独立地位，突出其长期资产意义
- 完成任务总览的统一追踪闭环
- 保证 Rust daemon、Web 前端和 Electron 壳持续可构建

## 技术架构

```text
Electron Shell / Future Floating Shell / Future Pet Shell
                    │
         Web UI (Overview / System / Action / Tasks)
                    │
      Local API + Task/Audit + Reference Surface
                    │
                Rust Daemon
                    │
      Action / Workflow / Policy / Task / Audit
                    │
         Atomic Capabilities + Linux Adapters
```

### 当前技术栈
- **Daemon**：Rust + Axum
- **Web UI**：Vue 3 + Vue Router + Pinia + ECharts
- **Desktop Shell**：Electron
- **首发环境**：Ubuntu / Debian + systemd + apt

## 后续方向

当前不在 v0.1 主路径的内容：
- 高自主 Agent 主流程
- 子 Agent 自创建 / 自升级闭环
- Live2D 桌宠完整交互
- 长记忆 / RAG / 自进化闭环
- 跨发行版支持

这些方向都保留，但会建立在当前电脑管家与统一能力资产底座之上。
