# OhMyWu 当前进程

> 最后更新：2026-05-14

## 版本：v0.2.0-pre

## 当前里程碑：M2 完成，即将进入 Phase 3（Action 管道）

---

## M0 完成 ✅

- Tauri v2 + Vue3 + Rust workspace 骨架
- 7 crates：domain / capability-registry / action-registry / policy-engine / task-engine / audit / session / llm-adapter
- 全局主题系统 + 可折叠侧栏 + 三栏布局
- 初始原子能力 bash + read，初始 Action shell.exec / fs.read / system.info

## M1 完成 ✅ — 原子能力执行闭环 + 会话持久化

### 后端
- [x] `chrono_now()` 修复 — domain 统一时间戳
- [x] 数据目录 `~/.ohmywu/{sessions,actions,wiki}/` 初始化
- [x] `config.json` 读写（PolicyMode、theme、accent、llm_provider）
- [x] `crates/session` — JSONL 会话管理器
- [x] `src-tauri/src/executor.rs` — 真实执行管道
  - bash: `std::process::Command` + 30s 超时
  - read: `std::fs::read_to_string`
  - 全链路：capability lookup → policy gate → task create → spawn_blocking → task/audit record
- [x] AppState 重构 + 14 个 Tauri commands
- [x] Phase 1 fallback（`read <path>` / `run <cmd>` 指令解析）

### 前端
- [x] Pinia 状态管理
- [x] `src/stores/chat.ts` — 会话/消息/流式状态
- [x] ChatView 接入真实 Tauri 后端

## M2 完成 ✅ — Agent 对话核心

### Rust 后端
- [x] `crates/llm-adapter`
  - Ollama provider（`/api/chat`，原生 tool calling，NDJSON streaming）
  - OpenAI-compatible provider（`/v1/chat/completions`，SSE streaming）
  - 60s HTTP 超时
- [x] `src/tools.rs` — 能力→Tool 转换（bash/read → OpenAI function calling Schema）
- [x] `src/agent.rs` — Agent 对话循环
  - System prompt（中文）
  - Tool calling loop（最多 10 轮）
  - 会话历史注入（最近 20 条）
  - 流式 response via Tauri `chat-stream` event
- [x] `send_message`：配置 LLM 时走 agent loop，未配置时回退本地指令
- [x] `test_llm_connection` 命令 + 前端测试按钮
- [x] 错误信息人性化（按 HTTP status 给出中文提示）

### 前端
- [x] SettingsView LLM 配置（provider/endpoint/model/API key + 测试连接）
- [x] `chat-stream` event → 实时 token 显示 + 闪烁光标

## 优化轮次 ✅

### 性能 / 逻辑
- [x] 消除 Arc<AppState> 冗余 clone（executor 改为 `&AppState`）
- [x] bash 执行加 30s timeout
- [x] reqwest client 加 60s timeout
- [x] 流式 tool call 累积修复
- [x] audit log 封顶 10000 条
- [x] session list_sessions 只读首尾行（O(1) 替代 O(n)）
- [x] append_message 序列化移出锁

### 前端美化
- [x] 全局主题重设计 — Inter + JetBrains Mono 字体，warm charcoal 色系，噪点纹理
- [x] Chat UI — 移除 iMessage 气泡，改为专业 agent 面板（accent 图标 + 名称头）
- [x] 输入框 — focus glow + SVG 发送按钮
- [x] 空状态欢迎页
- [x] 消息渐入动画（fadeUp stagger）
- [x] Sidebar — active 高亮，SVG toggle，brand 重设计
- [x] 零编译 warning

## 已知问题

- DeepSeek API + `deepseek-v4-flash` 返回 400（可能不支持 tool calling 或 streaming+tools）
- 临时方案：设置页用「测试连接」验证基础连通性；agent loop 待加 no-tools fallback

## 下一步：Phase 3 — Action 管道

- [ ] 审计日志 → 一键生成 Action（Markdown + 可选脚本）
- [ ] Action 文件结构：`~/.ohmywu/actions/{name}/README.md + manifest.json`
- [ ] 启动时扫描 actions/ 目录注册
- [ ] Action 作为 LLM tool 暴露
- [ ] Action 验证（dry-run）

## 参考材料

- [Claude Code 架构参考](claude-reference.md) — 基于泄露源码的深度分析，涵盖 agent loop、工具系统、权限、上下文管理、记忆系统等
- [LLM 适配器升级计划](planapi升级.md) — 参考 cc-switch 升级至多协议多 provider 兼容、错误分类、能力探测

## 仓库

https://github.com/wuwuwu-lxb/ohmywu
