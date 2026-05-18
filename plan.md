# OhMyWu Agent 下一阶段计划

> 更新时间：2026-05-19
> 目标：把当前“可用的单 Agent 对话循环”升级成“可持续运行、可分工、可恢复、可验证”的 Agent Runtime。

## 0. 说明

- 已尝试按要求把参考仓库直接 `git clone` 到本地，但当前 shell 到 GitHub 的网络被环境拦截，`git clone` 无法连通。
- 本文分析基于：
  - 当前仓库内已有的 [book/claude-reference.md](./book/claude-reference.md)
  - OpenHuman 公开仓库与文档页
  - DeepSeek-TUI 公开仓库与文档页
- 所以这份 `plan.md` 是基于源码级/文档级可验证信息整理，不是拍脑袋路线图。

## 1. 当前项目真实状态

### 已有能力

- Rust 侧已经有一个可工作的单 Agent loop：
  - [src-tauri/src/agent.rs](./src-tauri/src/agent.rs)
  - 已支持 `chat -> tool call -> 执行 -> 回填 -> 下一轮`
- 已有工具注册与 schema 下发：
  - [src-tauri/src/tools/mod.rs](./src-tauri/src/tools/mod.rs)
- 已有 session 持久化与前端聊天状态：
  - [src/stores/chat.ts](./src/stores/chat.ts)
- 已有基础权限思想：
  - 只读、写入、高风险工具分级
- 已有 wiki 能力雏形，可以视为未来 memory surface 的前置资产。

### 关键缺口

- 现在只有“单轮会话循环”，没有真正的 Runtime 概念：
  - 没有 `thread / turn / item / event` 结构
  - 没有 durable background task
  - 没有 sub-agent manager
  - 没有 checklist / verifier / restore
  - 没有统一 runtime API
- 权限还是偏“执行器级”，不是“agent orchestration 级”：
  - 缺少 mode
  - 缺少 tool visibility filtering
  - 缺少审批流事件模型
- 上下文管理还很薄：
  - 只取最近 20 条消息
  - 没有 context compaction
  - 没有 large result parking
  - 没有 prefix-stable prompt discipline

结论：现在的 OhMyWu 更像“单体 agent demo 已经可用”，但还不是“真正能演进的 agent runtime”。

## 2. 三个参考项目里真正值得抄的部分

## 2.1 Claude Code 泄漏源码 / 逆向资料

来自 [book/claude-reference.md](./book/claude-reference.md) 的有效结论：

### 应该抄的点

1. 单一执行引擎
   - 所有入口最后汇聚到同一条 `queryLoop`
   - 这意味着 CLI、GUI、SDK、自动化入口都不该各写一套 agent 行为

2. Streaming-first + 早执行只读工具
   - 流式输出期间，一旦 tool call block 已完整，就可以抢先执行只读工具
   - 这会显著降低交互延迟

3. 工具不是一坨平铺
   - 有 visibility / deferred / approval / concurrency-safe 等维度
   - 模型“能看到工具”与 runtime“允许执行工具”必须分开

4. 权限系统是 harness，不是 prompt
   - 拒绝优先
   - 模式决定基线
   - hook / deny rule / sandbox 都在模型外执行

5. 上下文压缩是 runtime 核心能力
   - 不是未来优化项，而是系统设计前提

### 对 OhMyWu 的直接启发

- 保持一个统一 agent runtime，不要为 Tauri UI、未来 headless、未来 automation 各写一套 loop
- 尽快把 tool call 流转拆成：
  - tool visible
  - tool allowed
  - tool executable
  - tool result persistence

## 2.2 DeepSeek-TUI

公开文档里最有价值的不是“终端 UI”，而是 runtime 设计。

### 应该抄的点

1. 明确 mode
   - `Plan / Agent / YOLO`
   - 参考：DeepSeek-TUI `docs/MODES.md`
   - Plan 模式只读调查，Agent 模式多步执行带审批，YOLO 才全自动

2. Durable thread / turn / event 模型
   - 参考：`docs/ARCHITECTURE.md`
   - 会话不是一坨 message，而是 runtime timeline

3. Durable task queue
   - 后台任务可跨重启继续存在
   - 任务和主聊天不是两套系统，而是同一 runtime surface

4. Sub-agent 是后台持续运行实例
   - 参考：`docs/SUBAGENTS.md`
   - 父 agent 启动子 agent 后继续工作
   - 子 agent 有 role taxonomy、并发上限、结果契约、持久状态

5. Sub-agent 输出必须结构化
   - `SUMMARY / CHANGES / EVIDENCE / RISKS / BLOCKERS`
   - 这点极其重要，避免子 agent 结果不可消费

6. Checkpoint + restore + side snapshot
   - 崩溃恢复、长任务恢复、工作区回滚都很关键

7. 技能 / hook / MCP 是 extension points，不是主循环本体
   - 主循环先稳，再扩展

### 对 OhMyWu 的直接启发

- 我们下一阶段不该先做“更多工具”，而应该先做：
  - runtime state model
  - mode
  - sub-agent manager
  - task/checklist/verifier

## 2.3 OpenHuman

OpenHuman 的价值不在 coding loop，而在“桌面 Agent 长期上下文和集成层”。

### 应该抄的点

1. Parent execution context 快照
   - 父 agent 开始 turn 时，把 provider、tools、memory、integrations、lineage 封成 context
   - 子 agent 从这个 context 派生，而不是临时拼凑

2. Typed subagent vs forked subagent
   - typed：窄上下文、窄职责、窄工具
   - fork：复用父上下文前缀，保持 prefix cache 稳定

3. 子 agent 工具过滤
   - 子 agent 不是继承全部工具
   - 需要 allowlist / disallowed / scope / skill filter

4. 子 agent 不暴露内部 transcript
   - 父 agent 只消费 compact result
   - 这是避免上下文爆炸的关键

5. 长期 memory 与 integration 自动汇入
   - OpenHuman 的 memory tree / auto-fetch 很重
   - 我们没必要现在全抄，但方向是对的

### 对 OhMyWu 的直接启发

- sub-agent 必须是 runtime 一级概念，不是普通 tool 包装
- 父子 agent 之间要有：
  - lineage
  - tool scope
  - compact result contract
  - 可选 fork_context
- wiki 后续可以升级成 lightweight memory surface，而不是另起炉灶

## 3. 我们下一阶段不该做什么

1. 不该先做超多新工具
   - 工具越多，runtime 越乱

2. 不该先做复杂多模型路由
   - 没有 durable runtime，路由只会放大复杂度

3. 不该先照抄 OpenHuman 的超重集成层
   - 118+ integrations、memory tree、auto-fetch 不是当前最短路径

4. 不该把 sub-agent 只实现成一个普通 `spawn_subagent` 工具然后完事
   - 没有 manager、状态、结果契约、并发约束，这会很快失控

## 4. 下一阶段的目标定义

下一阶段建议名称：

## Phase 3：Agent Runtime

验收标准不是“能开几个 agent”，而是：

1. 有统一 runtime 数据模型
2. 有 `Plan / Agent / Auto` 三档模式
3. 有持久化 sub-agent manager
4. 有 checklist / verifier / task surface
5. 有可恢复的 turn / task / sub-agent 状态
6. 前端能看见 agent 正在做什么，而不是只看最终一句回复

## 5. 分阶段落地计划

## P0. Runtime 基础重构

### 目标

把当前“message + exec record”升级成真正的 runtime timeline。

### 要做

1. 新增 runtime 核心模型
   - `Thread`
   - `Turn`
   - `RuntimeItem`
   - `ToolExecution`
   - `ChecklistItem`
   - `AgentRun`
   - `RuntimeEvent`

2. 持久化目录建议

```text
~/.ohmywu/
  sessions/
  runtime/
    threads/
    turns/
    events/
    tasks/
    subagents/
    checkpoints/
```

3. 把当前 `send_message` 路径改成：

```text
create/resume thread
-> start turn
-> append user item
-> run agent loop
-> append tool items / assistant items
-> finalize turn
-> persist events
```

### 代码入口

- [src-tauri/src/agent.rs](./src-tauri/src/agent.rs)
- [src-tauri/src/lib.rs](./src-tauri/src/lib.rs)
- 建议新增：
  - `src-tauri/src/runtime/mod.rs`
  - `src-tauri/src/runtime/threads.rs`
  - `src-tauri/src/runtime/events.rs`
  - `src-tauri/src/runtime/checkpoints.rs`

### 验收

- 一个聊天回合能产生完整 timeline
- 中断后可以恢复 thread
- UI 能读取 turn 内中间状态

## P1. Mode + 权限模型

### 目标

先建立行为边界，再扩张 agent 能力。

### 模式建议

1. `plan`
   - 只允许只读工具
   - 允许写 checklist
   - 不允许文件修改 / shell 执行

2. `agent`
   - 允许多步执行
   - 写入 / shell 受审批

3. `auto`
   - 对应 DeepSeek 的 Agent/YOLO 之间的中间层
   - 仅对 allowlist 中的安全写入自动放行
   - 高风险 shell 仍需明确 gate

### 要做

1. 扩展工具元数据
   - `risk_level`
   - `tool_kind`
   - `visible_in_modes`
   - `concurrency_safe`
   - `requires_approval`
   - `result_policy`

2. 把“工具可见”与“工具可执行”分离

3. 前端增加 mode 切换和待审批状态展示

### 代码入口

- [src-tauri/src/tools/mod.rs](./src-tauri/src/tools/mod.rs)
- [src-tauri/src/permission.rs](./src-tauri/src/permission.rs)
- [src/views/ChatView.vue](./src/views/ChatView.vue)
- [src/stores/chat.ts](./src/stores/chat.ts)

### 验收

- Plan 模式下模型看不到写入工具
- Agent 模式下写入工具可见但需审批
- Auto 模式下只自动放过允许的低风险动作

## P2. Sub-Agent Manager

### 目标

引入真正的父子 agent 协作能力。

### 最小设计

1. 子 agent 角色
   - `explore`
   - `plan`
   - `implement`
   - `verify`

2. 核心接口
   - `subagent_open`
   - `subagent_wait`
   - `subagent_cancel`
   - `subagent_list`

3. 每个子 agent 记录
   - `id`
   - `parent_turn_id`
   - `role`
   - `status`
   - `allowed_tools`
   - `fork_context`
   - `summary`
   - `evidence`
   - `risks`
   - `blockers`

4. 结果契约

```text
SUMMARY
CHANGES
EVIDENCE
RISKS
BLOCKERS
```

5. 并发上限
   - 默认 `4`
   - 后端可配置

### 关键原则

- 父 agent 只消费子 agent 的 compact result
- 默认 fresh context
- 只有显式 `fork_context` 才继承父上下文
- 子 agent 工具必须经过过滤

### 代码入口

- 建议新增：
  - `src-tauri/src/subagents/mod.rs`
  - `src-tauri/src/subagents/manager.rs`
  - `src-tauri/src/subagents/prompt.rs`
- [src-tauri/src/agent.rs](./src-tauri/src/agent.rs)

### 验收

- 父 agent 能并发启动多个 explorer
- explorer 结果可回填父 turn
- verifier 可独立运行测试并返回结构化结论

## P3. Checklist / Task / Verifier

### 目标

让 agent 的长期工作从“聊天”升级为“可追踪任务”。

### 要做

1. Checklist 工具
   - `checklist_write`
   - `checklist_complete`
   - `checklist_fail`

2. Task surface
   - 普通聊天 turn 内可挂 checklist
   - 也可以提升为 durable background task

3. Verifier surface
   - 统一收 tests / lint / build / smoke output
   - 把验证结果作为 first-class artifact 挂在 task 上

4. 失败回路
   - implement -> verify -> fail -> retry

### 代码入口

- `src-tauri/src/runtime/tasks.rs`
- `src-tauri/src/tools/`
- [src/components/RightPanel.vue](./src/components/RightPanel.vue)
- [src/components/ExecutionCard.vue](./src/components/ExecutionCard.vue)

### 验收

- 一个任务可以看到：
  - 当前 checklist
  - 最近工具执行
  - 最近验证结果
  - 最终结论

## P4. 上下文压缩与大结果停车

### 目标

防止 agent runtime 一扩展就被上下文成本打爆。

### 要做

1. 大结果停车
   - 大于阈值的工具结果写入磁盘
   - 上下文只放摘要 + handle

2. Context compaction
   - turn 级摘要
   - tool result 摘要
   - thread 级阶段总结

3. Prefix-stable system prompt discipline
   - system prompt 不要每轮重建
   - 把变量变化收敛到 user/context message

### 验收

- 长对话不会线性膨胀
- 子 agent 结果不会直接炸穿上下文

## P5. Memory 与 Integration 的轻量版本

### 目标

吸收 OpenHuman 的方向，但不把系统做重。

### 只做轻量版

1. 先把 wiki 升成 memory entry surface
   - 可搜索
   - 可引用
   - 可被 agent 写入 durable note

2. 只做少量本地 integration
   - filesystem
   - git
   - wiki
   - web fetch

3. 暂不做：
   - 大规模 OAuth integration
   - auto-fetch memory tree
   - 重型同步任务

### 验收

- agent 能把稳定偏好、项目约定、验证结论写入 memory
- 后续 turn 能引用这些 memory

## 6. 推荐开发顺序

不要并行开太多战线，建议顺序如下：

1. `P0 Runtime 基础重构`
2. `P1 Mode + 权限模型`
3. `P2 Sub-Agent Manager`
4. `P3 Checklist / Task / Verifier`
5. `P4 Context compaction`
6. `P5 Lightweight memory`

原因：

- 没有 runtime，sub-agent 和 task 都会变成补丁式实现
- 没有 mode/permission，agent 能力越强越危险
- 没有 checklist/verifier，multi-agent 只会产生“看起来很忙”的假象

## 7. 我建议的下一次具体实施范围

下一次开发不要贪多，只做一个窄而硬的里程碑：

## Milestone A：Runtime + Mode Skeleton

### 包含

1. 新建 runtime thread/turn/event 持久化
2. 聊天改走 runtime timeline
3. 前端增加 mode 切换
4. 工具 visibility by mode
5. `checklist_write` 最小版

### 不包含

1. 不做 sub-agent
2. 不做 memory
3. 不做复杂 integration
4. 不做自动恢复 UI

### 完成后收益

- 我们就有了一个稳定底盘
- 后面加 sub-agent 不会返工消息模型
- verifier / task / right panel 都有可挂载的数据结构

## 8. 参考结论总表

| 来源 | 真正该抄的 | 暂时不该抄的 |
|------|------------|--------------|
| Claude Code | 单一 query loop、工具可见性与执行分离、权限 harness、context compaction、streaming 早执行 | 过重的全量企业级权限矩阵 |
| DeepSeek-TUI | mode、runtime thread/turn、task queue、sub-agent manager、checkpoint、structured outputs、skills/hooks/MCP extension surface | 过早做超大终端功能面 |
| OpenHuman | parent execution context、typed/fork subagent、tool filtering、compact child result、memory/integration 方向 | 大规模 OAuth integrations、重型 memory tree、20 分钟 auto-fetch |

## 9. 最终判断

对 OhMyWu 来说，前端阶段结束后的正确下一步不是“继续加功能页面”，而是：

**先把 Agent Runtime 做对。**

如果只在现有 loop 上继续堆工具、堆页面、堆花活，很快会遇到三个问题：

1. 状态不可恢复
2. 多 Agent 不可控
3. 验证与审批不可追踪

所以下一阶段的核心不是“更聪明的模型”，而是：

**更硬的 runtime。**
