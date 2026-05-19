# OhMyWu Agent 下一阶段计划

> 更新时间：2026-05-19
> 目标：把当前“可用的单 Agent 对话循环”升级成“可持续运行、可分工、可恢复、可验证”的 Agent Runtime。

## 0.1 当前优先级补充：运行效率 + Runtime UI

前端基线已经可用，当前最优先的不是继续堆功能，而是先把两件事做硬：

1. 运行效率
   - 降低一次 turn 的 orchestration 开销
   - 降低工具调用等待与上下文回放成本
   - 让“同模型、同网络、同任务”下的交互体感尽量逼近 Claude Code

2. Runtime 可视化
   - Runtime 不再固定挂在聊天顶部
   - Runtime 挂到“对应的 agent 回复最下方”
   - 先看到摘要，再按需展开执行链路，再展开单个工具的输入/输出/状态/耗时

这两块应视为下一阶段的第一优先级，优先于 sub-agent、memory 扩展和更多工具。

## 0.2 本阶段成功标准

“效率达到 Claude Code 90% 以上”不能直接写成口号，必须先变成可测目标。

这里建议采用“相同 provider / 相同模型 / 相同网络 / 相近任务”的对比口径，目标不是绝对总耗时，而是把我们自己的 runtime 额外开销压到足够低。

### 性能目标定义

1. 首 token 时间
   - 在同 provider / 同模型下，`time_to_first_token` 达到 Claude Code 同类任务的 `>= 90%`
   - 等价说法：我们的首 token 额外开销尽量控制在 `+10%` 以内

2. 工具回路开销
   - 单次 `tool call -> dispatch -> 回填 -> 下一轮请求` 的 runtime 额外开销控制在 `150ms` 内
   - 只读工具如果能在流式阶段确定参数，应提前执行

3. 上下文回放成本
   - 不再简单固定“最近 20 条”
   - 要把上下文体积控制成“必要消息 + 压缩摘要 + 大结果句柄”

4. UI 可读性
   - 默认消息流只展示结果正文
   - Runtime 摘要折叠，不污染正文
   - 用户需要时再查看每个工具具体做了什么

### 明确不承诺的部分

1. 不承诺在模型本身慢、网络慢时还能绝对快过 Claude Code
2. 不承诺第一阶段就做到完整的 checkpoint / sub-agent / verifier
3. 不承诺为了快而牺牲权限审计和可追踪性

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

## P1.5. Efficiency + Runtime UX

### 目标

把当前“能运行”推进到“足够快、足够清楚”，并且不破坏现有 runtime 结构。

这一阶段只做两类事：

1. 降低 orchestration overhead
2. 重做 runtime 展示层级

---

### A. 后端效率计划

#### A1. 建立基线

先不要空谈“像 Claude Code 一样快”，先把下面这些指标打出来：

1. `time_to_provider_request`
   - 用户发消息到第一跳 provider 请求发出的时间

2. `time_to_first_token`
   - 用户发消息到收到第一段流式内容的时间

3. `time_to_first_tool_call`
   - 用户发消息到收到第一个完整 tool call 的时间

4. `tool_roundtrip_ms`
   - 单个工具从 dispatch 到结果回填的耗时

5. `post_tool_resume_ms`
   - 工具完成到下一轮模型请求发出的时间

6. `context_bytes`
   - 每轮下发给模型的消息体大小

这些指标都应写入 runtime event 或 trace，不能只打 `println`。

#### A2. 第一批性能优化

1. 把 session replay 和 runtime replay 分离
   - 当前消息历史和 runtime 事件是两套面
   - 模型上下文只拿必要的 session message，不要混入无关 runtime 数据

2. 做真正的上下文裁剪，而不是固定最近 20 条
   - 保留 system
   - 保留最近几轮完整对话
   - 旧轮次压缩成摘要
   - 大工具结果只回放摘要 + handle

3. 扩展 streaming-first 策略
   - 已有只读工具早执行雏形
   - 下一步补“工具参数完整即触发”的精确时机和 trace

4. 减少 tool loop 内重复序列化/反序列化
   - 当前每次 tool arguments 都多次 parse
   - 可以在聚合完成后缓存结构化参数，避免重复工作

5. 区分“显示文本流”和“推理/工具流”
   - 前端不需要每个 runtime 更新都触发大范围列表刷新
   - 避免 runtime event 造成消息区重复重绘

#### A3. 第二批性能优化

1. 大结果 parking
   - `read` / `grep` / `web_fetch` 这类大输出写入 artifact
   - 会话中只留摘要、路径、hash、截断预览

2. Prefix-stable prompt
   - system prompt 不应在每轮动态重建大块文本
   - 模式变化单独注入短上下文，减少重复 token

3. Tool visibility slimming
   - Plan 模式不让模型看见无关工具
   - 降低 tool schema 体积和模型决策成本

#### A4. 效率验收

1. 同模型同网络下，`time_to_first_token` 显著下降
2. 单次工具恢复链路的 runtime 额外开销稳定在 `150ms` 量级内
3. 长对话不会因为上下文线性膨胀而明显变慢

---

### B. 前端 Runtime UI 计划

#### B1. 交互目标

你要求的展示逻辑应当明确成下面这套层级：

1. Runtime 不放在聊天页面顶部做全局条
2. Runtime 绑定到“对应的 agent 回复”
3. 默认只显示一行摘要
   - 例如：`3 个工具 · 1.9s · 已完成`
4. 点击摘要展开本轮工具列表
5. 点击具体工具再展开详情
   - 工具名
   - 状态
   - 输入参数
   - 输出摘要
   - 完整输出
   - 耗时

#### B2. 数据模型调整

当前前端主要靠 `SessionMessage.executions` 渲染，这不够细。

需要改成：

1. `ChatMsg` 继续只负责正文
2. runtime 数据单独按 `turnId` 建索引
3. `RuntimeEvent` 增加更稳定的 payload
   - `capability`
   - `status`
   - `input_preview`
   - `output_preview`
   - `duration_ms`
   - `artifact_path`（如果有大结果停车）

4. 前端 store 提供：
   - `runtimeByTurnId`
   - `expandedTurns`
   - `expandedTools`

#### B3. 组件调整

建议拆成三层：

1. `ChatMessage.vue`
   - 只负责消息气泡、时间、正文
   - 在 agent 回复底部挂一个 `RuntimeSummary`

2. 新增 `RuntimeSummary.vue`
   - 展示本轮摘要
   - 控制展开/收起

3. 新增 `ToolExecutionList.vue` / `ToolExecutionItem.vue`
   - 列表层展示工具条目
   - Item 层展示输入/输出/错误/耗时详情

#### B4. 显示策略

1. 工具默认折叠
   - 正文优先

2. 大输出默认只显示预览
   - 完整内容需要二次展开

3. 正在运行中的 turn 展示渐进状态
   - `正在分析`
   - `正在调用 read`
   - `正在等待确认`
   - `已完成`

4. runtime 顺序按 turn 归属，不做跨回合混排

#### B5. 代码入口

- [src/views/ChatView.vue](./src/views/ChatView.vue)
- [src/components/ChatMessage.vue](./src/components/ChatMessage.vue)
- [src/stores/chat.ts](./src/stores/chat.ts)
- [src-tauri/src/runtime.rs](./src-tauri/src/runtime.rs)
- [src-tauri/src/agent.rs](./src-tauri/src/agent.rs)

#### B6. UI 验收

1. 顶部 runtime bar 删除或降级为极简全局状态
2. 每条 agent 回复底部都有对应 runtime 摘要
3. 能展开看到该回复触发的所有工具
4. 能继续展开看到具体输入输出
5. 正文区不会被工具卡片淹没

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

## Milestone A：Efficiency + Runtime Message UI

### 包含

1. 建立 runtime 性能埋点
   - TTFT
   - 首个 tool call 时间
   - tool roundtrip
   - post-tool resume
   - context bytes

2. 把 runtime 从顶部条迁移到“回复下方摘要”

3. 增加 turn 级工具展开列表

4. 增加单工具详情展开

5. 做第一轮上下文裁剪与大结果预览
   - 不要求一步做完整 parking
   - 先把超长输出从消息正文里剥离出来

6. 保留现有 mode / checklist / timeline 能力，不在这阶段继续扩 scope

### 不包含

1. 不做 sub-agent
2. 不做 memory
3. 不做 verifier 闭环
4. 不做复杂 integration
5. 不做自动恢复 UI
6. 不做新的高风险工具

### 完成后收益

1. 我们会第一次知道慢在哪里，而不是只靠体感
2. runtime 不再污染主聊天流
3. 后面做权限审计和 memory 时，UI 数据面不用返工
4. 为下一阶段做“权限审计 + 大结果 parking + context compaction”打底

## 7.1 Milestone A 的实际顺序

1. 后端先补 trace 和指标
2. 再做前端 runtime 迁移
3. 再做工具详情展开
4. 最后做上下文裁剪

原因：

- 没有指标先做优化，很容易优化错方向
- 没有 turn 级 runtime 归属，前端很快又会乱
- 先裁剪上下文再补 trace，不容易判断收益是否真实

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
