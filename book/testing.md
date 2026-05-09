# OhMyWu 测试方案

## 测试层次

```text
┌─────────────────────────────────┐
│        E2E (Tauri app)          │  ← 完整桌面应用测试
├─────────────────────────────────┤
│    Integration (IPC + API)      │  ← 前后端联调
├─────────────────────────────────┤
│      Unit (Rust crates)         │  ← 各 crate 单元测试
└─────────────────────────────────┘
```

## Unit Tests（Rust crates）

### domain
- 类型序列化/反序列化正确性
- RiskLevel 枚举匹配

### capability-registry
- 注册能力后能查询到
- 重复注册覆盖行为
- 并发读写安全（RwLock）

### action-registry
- 注册查询与 capability-registry 一致

### policy-engine
- 默认 Sandbox 模式
- Sandbox 下 ReadOnly 放行，HighRisk 拒绝
- Danger 下全放行
- set_mode 切换正确

### task-engine
- 创建 task 返回 Running 状态
- complete / fail 更新状态和 detail
- 并发创建 id 不重复

### audit
- record 写入后 list 可查
- limit 截断正确
- 时间戳递增

### 运行方式

```bash
cargo test
cargo test -p ohmywu-domain
cargo test -p ohmywu-policy-engine
# ...
```

## Integration Tests

### Tauri IPC 集成

- 前端 invoke `get_capabilities` 返回已注册能力列表
- 前端 invoke `get_actions` 返回已注册 Action 列表
- 前端 invoke `get_policy_mode` 返回 Sandbox
- bash 能力调用链路：invoke → policy check → task create → execute → audit record → 返回

### API 集成（若有 HTTP API）

- 保留为后续可能暴露的 HTTP API 准备

### 运行方式

```bash
cargo test --test integration  # 若有集成测试文件
# 或手动启动 Tauri dev 模式用浏览器 devtools 调试
npm run tauri dev
```

## E2E Tests

### 手动测试场景

1. **应用启动**
   - Tauri 窗口正常打开
   - 前端页面正常渲染
   - 对话界面可用

2. **对话交互**
   - 输入消息 → 收到响应
   - 换行输入（Shift+Enter）
   - 空消息不发送

3. **Policy 切换**
   - Sandbox 默认开启
   - 切换到 Danger 模式

4. **Task 追踪**
   - 执行操作后 task 列表更新
   - audit 记录可见

5. **桌宠**
   - 切换按钮可控制桌宠显隐
   - (后续) Live2D 渲染正常

### 自动化 E2E

待框架稳定后引入，可选方案：
- Playwright + Tauri
- 或手动测试清单作为发布前 checklist

## CI

待定。初步设想：
```yaml
# GitHub Actions
- cargo check
- cargo test
- cargo clippy
- cargo fmt --check
- npm run build
```

## 当前状态

- [x] cargo check 通过
- [x] vite build 通过
- [ ] crate 单元测试未编写
- [ ] 集成测试未编写
- [ ] E2E 测试未编写
