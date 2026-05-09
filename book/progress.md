# OhMyWu 当前进程

> 最后更新：2026-05-10

## 版本：v0.1.0-pre

## 当前里程碑：M0 → M1

M0（框架搭建）已全部完成，正在进入 M1（原子能力执行闭环）。

## M0 完成清单

### 项目骨架
- [x] Tauri v2 + Vue3 + TypeScript 前端
- [x] Rust workspace（7 crates）
- [x] Cargo.toml workspace 配置
- [x] package.json + vite.config.ts + tsconfig.json
- [x] `.gitignore`

### Rust Backend
- [x] `domain` — 核心类型定义
- [x] `capability-registry` — 原子能力注册/查询
- [x] `action-registry` — Action 注册/查询
- [x] `policy-engine` — Sandbox/Danger 双模式
- [x] `task-engine` — Task 生命周期管理
- [x] `audit` — 审计日志记录

### Tauri Shell
- [x] `tauri.conf.json` 配置
- [x] `capabilities/default.json` 权限
- [x] `lib.rs` — AppState + Tauri commands + run()
- [x] `main.rs` — 入口

### 前端
- [x] `App.vue` — Shell + 桌宠切换按钮
- [x] `ChatView.vue` — 对话界面（echo placeholder）
- [x] `main.ts` — Vue + Router 入口
- [x] `style.css` — 全局样式

### 构建验证
- [x] `cargo check` ✅
- [x] `vue-tsc --noEmit` ✅
- [x] `vite build` ✅

## 下一步：M1 — 原子能力执行闭环

### 后端
- [ ] bash 原子能力：policy gate → std::process::Command → task/audit
- [ ] read 原子能力：policy gate → std::fs::read_to_string → task/audit
- [ ] Tauri command：`execute_capability(name, params)` 统一执行入口

### 前端
- [ ] 对话界面 invoke Tauri commands
- [ ] 执行结果回显到对话

## 已注册资产

### 原子能力
| 名称 | 风险等级 |
|------|---------|
| bash | HighRisk |
| read | ReadOnly |

### Action
| ID | 说明 |
|----|------|
| shell.exec | 执行 shell 命令 |
| fs.read | 读取文件 |
| system.info | 获取系统信息 |

## 仓库

https://github.com/wuwuwu-lxb/ohmywu
