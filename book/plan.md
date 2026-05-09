# OhMyWu 开发计划

## 路线图

### M0 — 框架搭建 ✅

- [x] Tauri v2 + Vue3 + Rust workspace 骨架
- [x] 核心 crate 创建：domain / capability-registry / action-registry / policy-engine / task-engine / audit
- [x] 初始原子能力：bash + read
- [x] 初始 Action：shell.exec / fs.read / system.info
- [x] Sandbox / Danger 双模式 policy 引擎
- [x] Task 生命周期追踪 + Audit 审计日志
- [x] 前端对话界面占位
- [x] cargo check + vite build 通过

### M1 — 原子能力执行闭环

- [ ] bash 原子能力执行链路（policy gate → command execution → task/audit tracking）
- [ ] read 原子能力执行链路
- [ ] Tauri invoke 打通前后端（capability 调用 → 结果返回）
- [ ] 前端对话界面接入后端命令执行

### M2 — Agent 对话核心

- [ ] LLM 适配层（本地 ollama + 云端 API）
- [ ] Agent 对话循环（用户输入 → LLM 推理 → Tool Call → 结果返回）
- [ ] Tool/function calling 对接 Action 系统
- [ ] 对话历史管理

### M3 — 子 Agent 系统

- [ ] Agent 模板定义与实例化
- [ ] 子 Agent 创建与生命周期管理
- [ ] 子 Agent 间通信与结果汇总
- [ ] Agent 能力边界与权限继承

### M4 — 记忆系统

- [ ] 长记忆存储（LLMWiki 或其他方案）
- [ ] 对话上下文窗口管理
- [ ] 记忆检索与注入

### M5 — Live2D 桌宠

- [ ] Live2D 渲染集成
- [ ] 双窗口架构（正常模式 + 轻量悬浮模式）
- [ ] 桌宠交互（点击、语音、状态反馈）

### M6 — 完善与打磨

- [ ] 错误处理与边界情况
- [ ] UI 打磨
- [ ] 性能优化
- [ ] 打包发布

## 当前状态

**M0 完成。开始 M1。**
