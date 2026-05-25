# OhMyWu

一个本地优先、可控执行、可审计的桌面 Agent 工作台。

当前发布目标：`v0.25.0`

当前版本重点不是“更多功能”，而是把桌面 Agent 的核心基础打稳：可见的 runtime、可控的工具执行、可沉淀的知识记忆，以及后续可扩展的 action / skill / agent 配置层。

## 当前状态

`v0.25.0` 已进入“可安装、可跑通、可持续优化”的预览阶段。当前已经具备：

- 本地对话式 Agent loop，支持工具调用、流式回复和运行时事件追踪
- 可展开的 Runtime 面板，能看到工具调用、耗时、状态和记忆召回
- 本地知识库与记忆召回链路，支持手动生成“记忆候选”并确认写入
- Agent 管理原型，支持自定义人格、结构化记忆 Scope、切换 active agent
- Action 注册表和本地 `SKILL.md` 生态兼容层
- 微信个人号直连接入原型，支持登录二维码、私聊文本收发和 `/new` 会话切分
- 前端主题、背景、消息渲染、Markdown、复制、工具过程展示等基础体验
- 多模型配置切换、模型拉取、对话分类管理与分类内新建会话

## v0.2 版本范围

这一版的重点不是继续堆功能，而是把“能不能真实使用”先打通：

- 对话、runtime、工具调用、审计信息已经形成一条完整可见链路
- 微信桥接已经能把微信私聊映射到本地会话，并支持 `/new` 隔离上下文
- 知识库、记忆候选、Agent 管理、Action 注册、Atomic Capability 页面已经进入统一主题下的可用状态
- 前端 UI 做过一轮集中清理，去掉了大部分不必要的装饰性动态效果，减少黑色蒙层和割裂感

当前仍然把 `v0.25.0` 定义为预览版，而不是稳定正式版。

当前阶段的主要目标是：

1. 优化运行效率和上下文开销
2. 补强权限控制与审计可读性
3. 验证记忆 / action / skill / agent 配置层在真实使用中的稳定性

## 技术架构

```text
┌─────────────────────────────────────────────┐
│                Tauri Shell                  │
│  ┌───────────────────────────────────────┐  │
│  │              Vue 3 UI                 │  │
│  │ Chat / Runtime / Agents / Wiki /      │  │
│  │ Actions / Audit / Settings            │  │
│  └───────────────────────────────────────┘  │
│                    │                         │
│               Tauri IPC                     │
│                    │                         │
│  ┌───────────────────────────────────────┐  │
│  │            Rust Backend               │  │
│  │ Agent Loop / Capability Registry /    │  │
│  │ Action Registry / Policy / Runtime /  │  │
│  │ Session / Wiki / Audit                │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

## 当前核心能力

### 1. Atomic Capabilities

当前后端已经注册的基础能力包括：

- `bash`
- `read`
- `write`
- `edit`
- `glob`
- `grep`
- `web_fetch`
- `thinking`
- `checklist_write`
- `wiki_read`
- `wiki_write`
- `wiki_search`
- `wiki_list`
- `wiki_graph`

这些能力是底层执行单元，会被 Agent loop、Action 和权限系统共同消费。

### 2. Action

Action 现在不再只是一个概念标签，而是结构化注册项，包含：

- `id`
- `title`
- `description`
- `source`（`builtin` / `skill`）
- `capabilities`
- `tags`
- `path`
- `entry`
- `available`

当前内建 action 主要覆盖：

- `shell.exec`
- `fs.read`
- `wiki.memory`
- `plan.track`

同时本地 `SKILL.md` 会被扫描并转化成 skill action，进入同一套注册表。

### 3. Skill → Action 兼容层

项目目前已经支持：

- 扫描项目目录和本机目录下的 `.codex/skills` / `.agents/skills`
- 解析 `SKILL.md` frontmatter
- 将 skill 注册成 action
- 生成对应的 action blueprint
- 在前端查看 skill action 的编译结果、prompt 主体和 supporting files

当前这层是“发现、注册、转化、展示兼容”。后续还会继续接入执行链路。

### 4. Runtime 可视化

每次回复都能绑定对应 turn 的 runtime 信息，当前已支持：

- runtime 摘要
- tool started / completed
- TTFT / 首工具耗时
- 记忆召回展示
- 记忆候选生成和知识库写入状态

目标是不让系统继续是黑盒。

### 5. 知识库与记忆

当前知识库目录分为：

- `concepts`
- `notes`
- `daily`
- `profile`

当前记忆链路支持：

- 结构化 memory scope
- 按 scope 召回知识
- 手动触发记忆候选
- 编辑候选后写入知识库

### 6. 微信接入

当前微信接入页已经覆盖一条最小可用链路：

- 登录二维码申请与状态轮询
- 微信私聊消息拉取
- Agent 回复直发回微信
- 微信联系人到本地对话的映射
- `/new` 新建并切换本地会话

定时任务能力已从当前版本移除，后续会在重新设计后再回归。

## Agent 管理

当前 Agent 管理页已经支持：

- 多个 Agent 配置
- 自定义名称、角色、人格
- 结构化记忆 Scope
- 召回上限配置
- 目录级记忆范围配置
- 策略说明
- 新增 / 复制 / 删除 / 切换 agent

这里还是原型层，不代表已经具备完整多 Agent 编排 runtime。当前重点仍然是知识库、记忆和执行闭环。

## 权限与执行模式

当前提供三种 Agent Mode：

- `plan`
- `agent`
- `auto`

同时保留策略模式控制：

- `Sandbox`
- `Danger`

执行目标是：

- 让工具可见性和真正可执行性分离
- 高风险工具具备清晰确认与审计
- 保持可控而不是盲目自动化

## 前端体验

当前前端已完成一轮集中清理，重点包括：

- 统一主题与背景处理
- 聊天气泡与消息层级优化
- Markdown 渲染
- 一键复制
- 工具调用折叠 / 展开
- Runtime 放到对应回复下方
- Agent 管理、知识库、Action、设置等页面整合

## 开发

### 环境

- Node.js
- Rust
- Tauri 2

### 常用命令

```bash
npm install
npm run build
npm run build:appimage
cargo check
cargo test
```

### Linux 打包

当前 AppImage 推荐走仓库脚本：

```bash
npm run build:appimage
```

产物位置：

- `target/release/bundle/appimage/OhMyWu_*.AppImage`

正式发布的统一容器化路径见：

- `book/container-packaging.md`

### Arch Linux

当前支持两种 Arch 安装方式。

1. 直接安装已经构建好的包：

```bash
sudo pacman -U packaging/arch/ohmywu-0.25.0-1-x86_64.pkg.tar.zst
```

2. 用 Docker 在 Arch 容器里重新生成包：

```bash
sudo bash scripts/build-arch-package-docker.sh
```

相关文件：

- `packaging/arch/PKGBUILD`
- `packaging/arch/Dockerfile`
- `scripts/build-arch-package-docker.sh`

### 当前验证基线

在当前阶段，至少应关注：

- `cargo check`
- `npm run build`
- 关键功能链路的最小测试

## 下一阶段

`v0.25.0` 之后，下一阶段会主要围绕：

1. 运行效率
2. 权限和审计清晰度
3. 记忆系统稳定性
4. action / skill 真正接入执行流
5. 更完整的 agent / sub-agent runtime

---

如果你想看当前设计方向，可以继续参考：

- `plan.md`
- `book/claude-reference.md`
- `book/guide.md`
- `book/release-checklist.md`
