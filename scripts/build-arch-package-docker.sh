#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

IMAGE_TAG="ohmywu-arch-packager:0.25.0"

chmod -R u+rwx packaging/arch/pkg packaging/arch/src 2>/dev/null || true

cp -f target/release/ohmywu packaging/arch/ohmywu-bin
cp -f src-tauri/icons/128x128.png packaging/arch/ohmywu-128x128.png

docker build -t "$IMAGE_TAG" -f packaging/arch/Dockerfile .
docker run --rm \
  -v "$ROOT_DIR:/workspace" \
  -w /workspace/packaging/arch \
  "$IMAGE_TAG"

ls -lh packaging/arch/*.pkg.tar.zst
