# OhMyWu 发布清单

> 最后更新：2026-05-20

## 1. 发布前确认

- `npm run build`
- `cargo check`
- 核对模型配置、对话、知识库等本地数据没有被误加入仓库
- 确认 README 和使用文档是最新的

## 2. 版本号

优先检查这些位置是否一致：

- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- 根目录 `Cargo.toml`（如果需要对 workspace 版本做说明）

当前建议发布版本：

- `0.2.0`

## 3. 本地构建发布包

```bash
npm run tauri build
```

常见产物目录：

- `src-tauri/target/release/bundle/`

通常会生成：

- `AppImage`
- `deb`
- 其他平台对应安装包

## 4. Git 发布步骤

```bash
git status
git add <需要提交的文件>
git commit -m "release: prepare v0.2.0"
git push
```

## 5. GitHub Release 建议

1. 打 tag：`git tag v0.2.0`
2. 推送 tag：`git push origin v0.2.0`
3. 在 GitHub 新建 Release
4. 上传 `bundle/` 里的安装包
5. 补一段 release note

## 6. 建议的首版 release note 结构

- 这是什么产品
- 当前已经可用的核心能力
- 已知限制
- 下一阶段方向

## 7. 当前版本更适合定义为

- 内测版
- 预览版
- 面向真实使用场景的验证版

不建议现在就定义成稳定正式版。
