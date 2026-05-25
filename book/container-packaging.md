# 容器化打包说明

> 最后更新：2026-05-25

## 1. 为什么要容器化

桌面包，尤其是 `AppImage`，在宿主机上直接构建很容易遇到这些问题：

- `linuxdeploy` / `appimagetool` 对宿主机依赖布局敏感
- `gdk-pixbuf`、`webkit2gtk`、`gstreamer` 在新系统上的路径和旧系统不一致
- `fuse`、`mksquashfs`、runtime 下载等步骤在不同系统表现不一致
- 在 Arch、NixOS、较新的 Ubuntu 上构建，产物不一定适合 Ubuntu 22.04

因此后续统一建议：

- 日常快速本地构建可继续使用宿主机脚本
- 正式发布统一走容器环境

## 2. 当前统一目标环境

建议的容器构建基线：

- `ubuntu:22.04`

目标：

- 尽量降低 `glibc` 和桌面运行时版本漂移
- 让 AppImage、deb、rpm 的构建环境可复现

## 3. 宿主机前置条件

宿主机需要具备：

- `docker`
- 可访问外网的容器镜像拉取能力

如果当前用户不能直接连接 Docker daemon，可以用：

```bash
sudo docker ps
```

或者把当前用户加入 `docker` 组后重新登录。

## 4. 推荐目录约定

统一约定：

- 仓库根目录执行容器打包命令
- 容器内工作目录固定为 `/workspace`
- 产物仍然输出到仓库内的 `target/release/bundle/`

## 5. 容器镜像内容建议

容器内至少需要：

- `curl`
- `file`
- `fuse3`
- `libwebkit2gtk-4.1-dev`
- `libgtk-3-dev`
- `libsoup-3.0-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`
- `patchelf`
- `squashfs-tools`
- `build-essential`
- `pkg-config`
- `git`
- `ca-certificates`
- Node.js 20
- Rust stable

## 6. 容器内推荐打包步骤

```bash
npm ci
cargo check
npm run build:appimage
npm run tauri build --bundles deb,rpm --ci
```

说明：

- `build:appimage` 负责补齐 runtime、抽取 `appimagetool`、完成 AppImage 封包
- `deb` / `rpm` 仍可继续交给 Tauri 默认 bundler

## 7. 一次性创建并进入容器

下面是推荐做法：

```bash
docker run --rm -it \
  -v "$(pwd):/workspace" \
  -w /workspace \
  ubuntu:22.04 \
  bash
```

进入后按“容器内推荐打包步骤”执行。

## 8. 推荐做成固定镜像

正式发布建议先做一层固定镜像，再重复复用：

1. 在仓库中维护 Dockerfile
2. 先 `docker build`
3. 再 `docker run` 执行构建

这样可以避免每次都手工装依赖。

## 9. 本项目当前实际可用路径

当前仓库已经有一个可用的宿主机 AppImage 构建入口：

```bash
npm run build:appimage
```

它解决了这些问题：

- Tauri 默认 `linuxdeploy` 失败后继续接管
- `appimagetool` 缺 `mksquashfs`
- AppImage runtime 在线下载失败
- 最终 `.AppImage` 封包

当前已验证产物：

- `target/release/bundle/appimage/OhMyWu_0.25.0_amd64.AppImage`

## 10. 发布时的推荐顺序

建议以后统一按下面顺序：

1. 先在容器内构建 `AppImage`
2. 再构建 Arch `pkg.tar.zst`
2. 再构建 `deb`
3. 再构建 `rpm`
4. 最后校验文件大小、`file` 输出和可执行权限

## 11. Arch 容器打包

当前仓库已经补了一条 Arch 二进制包容器化路径：

- `packaging/arch/Dockerfile`
- `scripts/build-arch-package-docker.sh`

用法：

```bash
sudo bash scripts/build-arch-package-docker.sh
```

这条链路的目标不是“在容器里重新完整编译整个项目”，而是：

1. 复用当前已经生成的 `target/release/ohmywu`
2. 复制图标到 `packaging/arch/`
3. 在 Arch 容器内运行 `makepkg`
4. 生成 `packaging/arch/*.pkg.tar.zst`

当前生成的包可直接安装：

```bash
sudo pacman -U packaging/arch/ohmywu-0.25.0-1-x86_64.pkg.tar.zst
```

## 12. 产物检查

```bash
ls -lh target/release/bundle/appimage/
file target/release/bundle/appimage/*.AppImage
```

```bash
ls -lh packaging/arch/*.pkg.tar.zst
file packaging/arch/*.pkg.tar.zst
```

## 13. 后续建议

下一步最好补两样东西：

1. 仓库级 `Dockerfile.release`
2. `scripts/build-release-container.sh`

这样以后就能固定成一条统一命令，而不是靠手动进入容器。
