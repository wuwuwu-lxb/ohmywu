# OhMyWu 发布清单

> 最后更新：2026-05-24

## 1. 发布目标

当前建议发布版本：

- `0.25.0`
- Git tag / Release：`v0.25.0`

## 2. 发布前确认

- `cargo check`
- `npm run build`
- `npm run tauri build`
- 核对模型配置、对话、知识库等本地数据没有被误加入仓库
- 确认 README 和使用文档是最新的

## 3. 版本号同步位置

优先检查这些位置是否一致：

- `package.json`
- `Cargo.toml`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

## 4. Linux 构建基线

为了尽量避免 Ubuntu 22.04 无法运行，Linux 发布包建议直接在以下环境构建：

- `Ubuntu 22.04`

不要在过新的桌面发行版上直接构建后再拿去给旧系统跑，否则容易遇到：

- `glibc` 版本过高
- `WebKitGTK` 运行时差异
- AppImage 在旧系统上无法启动

## 5. 本地构建发布包

```bash
npm run tauri build
```

常见产物目录：

- `src-tauri/target/release/bundle/appimage/`
- `src-tauri/target/release/bundle/deb/`
- `src-tauri/target/release/bundle/rpm/`

当前 Linux 主要发布产物建议为：

- `AppImage`
- `deb`
- `rpm`

## 6. Arch / pacman 体系

Tauri 不直接产出 pacman 安装包。

Arch 侧建议做法：

- 仓库内维护 `PKGBUILD`
- 发布时同步更新 `pkgver`
- 用户可用 `makepkg -si`
- 如果后续要进 AUR，再补 `.SRCINFO`

当前建议文件位置：

- `packaging/arch/PKGBUILD`

## 7. Git 发布步骤

```bash
git status
git add <需要提交的文件>
git commit -m "release: prepare v0.25.0"
git push
git tag v0.25.0
git push origin v0.25.0
```

## 8. GitHub Release 建议

1. 新建 Release：`v0.25.0`
2. 上传以下 Linux 产物：
   - `bundle/appimage/` 下的 AppImage
   - `bundle/deb/` 下的 `.deb`
   - `bundle/rpm/` 下的 `.rpm`
3. 附带上传：
   - `packaging/arch/PKGBUILD`
4. release note 里注明：
   - AppImage / deb / rpm 为官方二进制包
   - Arch 用户使用 `PKGBUILD`
   - Linux 推荐优先在 Ubuntu 22.04 / Debian 12 系统验证

## 9. 推荐 release note 结构

- 这是什么产品
- 当前已可用的核心能力
- 本次新增亮点
- Linux 安装方式
- 已知限制
- 下一阶段方向

## 10. 当前版本定位

- 预览版
- 真实使用场景验证版

不建议定义成稳定正式版。
