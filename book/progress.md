# OhMyWu 当前进程

> 最后更新：2026-05-14

## 版本：v0.3.0-pre

## 当前里程碑：UI 全面升级 + LLM 适配器重构完成

---

## M0 完成 ✅

- Tauri v2 + Vue3 + Rust workspace 骨架
- 全局主题系统 + 可折叠侧栏 + 三栏布局

## M1 完成 ✅ — 原子能力执行闭环 + 会话持久化

- [x] 数据目录 `~/.ohmywu/{sessions,actions,wiki}/` 初始化
- [x] `config.json` 读写（PolicyMode、theme、accent、llm_provider）
- [x] `crates/session` — JSONL 会话管理器
- [x] `src-tauri/src/executor.rs` — 真实执行管道
- [x] AppState 重构 + 14 个 Tauri commands
- [x] Pinia 状态管理 + ChatView 接入真实后端

## M2 完成 ✅ — Agent 对话核心

- [x] `crates/llm-adapter` — Ollama + OpenAI-compatible provider
- [x] Agent 对话循环（tool calling loop，最多 10 轮）
- [x] 流式 response via Tauri `chat-stream` event
- [x] 前端 SettingsView LLM 配置 + 测试连接

## v0.3.0 — UI 全面升级 + LLM 适配器重构 ✅

### SPlayer 风视觉重做

- [x] 半透明表面系统：`--surface-1/2/3` 三档可调透明度，`--border-color/hover` 两档边框
- [x] RGB 色系 token：`--accent-rgb` 分量供 `rgba()` 构造，accent 推导全部表面/边框/hover 颜色
- [x] SPlayer AppLayout 三层结构：background-container → mask → transparent app shell
- [x] 字体更换：DM Sans + Space Mono（取代 Inter）
- [x] 侧栏图标 emoji 化，消息渐入动画（fadeUp stagger）

### 背景系统重写

- [x] 删除 4 个内置 CSS 渐变"壁纸"，默认纯色背景（accent 推导微妙渐变）
- [x] 三档背景：solid / image / video
- [x] 自定义图片/视频上传（HTML input → ArrayBuffer → Rust `save_background_file` → `asset://` URL）
- [x] 背景控制：scale (100-200%) / blur (0-40px) / maskOpacity (0-80%)

### 主题持久化

- [x] `useTheme.ts` 内存状态 → `config.json` 双向同步
- [x] `initFromConfig()` 启动时恢复全部外观状态

### LLM 适配器重构（book/planapi升级.md）

- [x] 多 Provider 支持：Anthropic / Gemini / OpenAI / Ollama adapter
- [x] `LlmError` 结构化错误分类（认证/网络/限流/模型/格式错误）
- [x] `ProviderMetadata` + `builtin_providers()` 作为 provider 单一数据源
- [x] `ApiFormat` 枚举 + `infer_api_format()` 自动推断
- [x] `health_check()` / `probe_capabilities()` 能力探测
- [x] `test_llm_connection_with_config` — 按表单值真测试，返回 `LlmTestResult` 结构化结果
- [x] `get_llm_providers` — 后端 provider 列表下发前端

## 已知问题

- DeepSeek API + `deepseek-v4-flash` 返回 400（可能不支持 tool calling 或 streaming+tools）
- 临时方案：不支持 tools 的模型自动降级为纯文本模式

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
