# OhMyWu 当前进程

> 最后更新：2026-05-20

## 版本：v0.4.0-pre

## 当前里程碑：优化测试阶段 + 模型配置独立页 + 发布准备

---

## M0 完成 ✅

- Tauri v2 + Vue3 + Rust workspace 骨架
- 全局主题系统 + 可折叠侧栏 + 三栏布局

## M1 完成 ✅ — 原子能力执行闭环 + 会话持久化

- [x] 数据目录 `~/.ohmywu/{sessions,actions,wiki}/` 初始化
- [x] `config.json` 读写（主题、背景、模型 profile、权限规则）
- [x] `crates/session` — JSONL 会话管理器
- [x] `src-tauri/src/executor.rs` — 真实执行管道
- [x] AppState 重构 + 14 个 Tauri commands
- [x] Pinia 状态管理 + ChatView 接入真实后端

## M2 完成 ✅ — Agent 对话核心

- [x] `crates/llm-adapter` — Ollama + OpenAI-compatible provider
- [x] Agent 对话循环（tool calling loop，当前上限 48 轮）
- [x] 流式 response via Tauri `chat-stream` event
- [x] 前端模型配置与测试连接

## M3 完成 ✅ — 前端体验重做 + 目录系统接入

### 界面与交互

- [x] 半透明表面系统：`--surface-1/2/3` 三档可调透明度，`--border-color/hover` 两档边框
- [x] RGB 色系 token：`--accent-rgb` 分量供 `rgba()` 构造，accent 推导全部表面/边框/hover 颜色
- [x] SPlayer AppLayout 三层结构：background-container → mask → transparent app shell
- [x] 纯色模式收口：取消噪点层与发糊蒙版感，统一为更实的纯色背景
- [x] 图片背景同步主色：图片上传后自动提取主色并驱动主题色
- [x] 对话消息 Markdown 渲染与一键复制
- [x] Runtime 下挂到具体回复，并支持工具调用折叠查看
- [x] 会话管理视图、左右切换与基础持久化

### 背景与主题

- [x] 删除 4 个内置 CSS 渐变"壁纸"，默认纯色背景
- [x] 背景模式收敛为 solid / image，视频上传暂缓
- [x] 自定义图片上传（HTML input → ArrayBuffer → Rust `save_background_file` → `asset://` URL）
- [x] 背景控制：scale (50-200%) / blur (0-40px) / maskOpacity (0-80%)

### 目录系统

- [x] 原子化能力目录：内置能力 + 用户自定义能力
- [x] Action 目录：系统 Action + 用户 Action
- [x] Agent 目录：主 Agent + 可编辑子 Agent

### 模型与设置

- [x] 多 Provider 支持：Anthropic / Gemini / OpenAI / Ollama adapter
- [x] `LlmError` 结构化错误分类（认证/网络/限流/模型/格式错误）
- [x] `ProviderMetadata` + `builtin_providers()` 作为 provider 单一数据源
- [x] `ApiFormat` 枚举 + `infer_api_format()` 自动推断
- [x] `health_check()` / `probe_capabilities()` 能力探测
- [x] `test_llm_connection_with_config` — 按表单值真测试，返回 `LlmTestResult` 结构化结果
- [x] `get_llm_providers` — 后端 provider 列表下发前端
- [x] 模型设置独立页，多 profile 管理、拉取模型、连接测试、激活切换
- [x] 设置页收敛为 `外观` 与 `执行与权限`

## 当前观察点

- Linux WebKit 环境下仍需继续验证复杂输入区稳定性
- 多 provider 的工具调用兼容性仍需要继续做真实回归测试
- 知识库图谱与编辑体验还有继续优化空间

## 2026-05-20 增量

- [x] 模型配置从综合设置页拆出，独立为 `模型设置`
- [x] 设置页收敛为 `外观` 与 `执行与权限`
- [x] 补充使用引导：`book/guide.md`
- [x] 补充发布清单：`book/release-checklist.md`
- [x] 重写 `book/implementation.md`，同步当前实现状态

## 下一步重点

- [ ] 性能与长任务效率优化
- [ ] 权限说明、确认流和审计可读性优化
- [ ] 知识库编辑、搜索与图谱体验优化
- [ ] Action / Skill 转换链路继续补强
- [ ] 多 Agent 协作和委派体验继续完善

## 参考材料

- [Claude Code 架构参考](claude-reference.md) — 基于泄露源码的深度分析，涵盖 agent loop、工具系统、权限、上下文管理、记忆系统等
- [LLM 适配器升级计划](planapi升级.md) — 历史升级方案参考，主要内容已大部分落地

## 仓库

https://github.com/wuwuwu-lxb/ohmywu
