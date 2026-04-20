# OhMyWu

OhMyWu 是一个本地优先的个人 Agent 平台。当前仓库已完成 `M1` 骨架：

- Rust workspace
- 本地 daemon API
- Web 控制台壳
- Electron 桌面壳
- Action / Workflow / Agent Template 的最小抽象

## 目录

- `apps/daemon`: 本地 API 与运行时入口
- `apps/web`: Web 控制台
- `apps/electron`: Electron 桌面壳
- `crates/*`: 运行时核心与能力资产骨架

## 本地启动

1. 运行 daemon：`cargo run -p ohmywu-daemon`
2. 运行 web：`pnpm --dir apps/web dev`
3. 运行 electron：`pnpm --dir apps/electron dev`

Electron 默认加载 `http://127.0.0.1:5173`。如需改动，设置环境变量 `OHMYWU_WEB_URL`。

## 当前状态

当前只完成骨架与最小 API，不包含真实 Linux 管理能力。

仓库内原有的 `desktop/` 目录暂时保留，当前 M1 结构以 `apps/` 和 `crates/` 为准。
