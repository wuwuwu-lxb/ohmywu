#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

APP_NAME="OhMyWu"
APP_VERSION="$(node -p "require('./package.json').version")"
APPDIR="target/release/bundle/appimage/${APP_NAME}.AppDir"
APPIMAGE_OUTPUT="target/release/bundle/appimage/${APP_NAME}_${APP_VERSION}_amd64.AppImage"
TAURI_PLUGIN_APPIMAGE="${HOME}/.cache/tauri/linuxdeploy-plugin-appimage.AppImage"
APPIMAGE_TOOL_ROOT="${ROOT_DIR}/target/appimage-tools"
APPIMAGE_RUNTIME_CACHE="${ROOT_DIR}/target/appimage-runtime"
APPIMAGE_RUNTIME_FILE="${APPIMAGE_RUNTIME_CACHE}/runtime-x86_64"
APPIMAGE_RUNTIME_URL="https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64"

log() {
  printf '[appimage] %s\n' "$*"
}

require_file() {
  local path="$1"
  if [[ ! -e "$path" ]]; then
    printf 'missing required file: %s\n' "$path" >&2
    exit 1
  fi
}

prepare_runtime() {
  mkdir -p "$APPIMAGE_RUNTIME_CACHE"

  if [[ ! -s "$APPIMAGE_RUNTIME_FILE" ]]; then
    log "downloading AppImage runtime"
    curl -L --fail --progress-bar -o "$APPIMAGE_RUNTIME_FILE" "$APPIMAGE_RUNTIME_URL"
    chmod +x "$APPIMAGE_RUNTIME_FILE"
  fi
}

extract_appimagetool() {
  require_file "$TAURI_PLUGIN_APPIMAGE"
  mkdir -p "$APPIMAGE_TOOL_ROOT"

  if [[ ! -x "$APPIMAGE_TOOL_ROOT/squashfs-root/appimagetool-prefix/usr/bin/appimagetool" ]]; then
    log "extracting appimagetool helper"
    (
      cd "$APPIMAGE_TOOL_ROOT"
      rm -rf squashfs-root
      APPIMAGE_EXTRACT_AND_RUN=1 "$TAURI_PLUGIN_APPIMAGE" --appimage-extract >/dev/null
    )
  fi
}

stage_appdir() {
  log "building frontend"
  npm run build

  log "asking tauri to stage AppDir"
  rm -rf "$APPDIR"
  rm -f "$APPIMAGE_OUTPUT"
  set +e
  APPIMAGE_EXTRACT_AND_RUN=1 node_modules/.bin/tauri build --bundles appimage --ci
  local rc=$?
  set -e

  if [[ ! -d "$APPDIR" ]]; then
    printf 'tauri did not produce %s\n' "$APPDIR" >&2
    exit "${rc:-1}"
  fi

  if [[ $rc -ne 0 ]]; then
    log "tauri appimage stage returned ${rc}; continuing with manual finalize"
  fi
}

ensure_root_files() {
  local desktop_file="${APPDIR}/usr/share/applications/${APP_NAME}.desktop"
  local icon_file="${APPDIR}/usr/share/icons/hicolor/128x128/apps/ohmywu.png"
  local root_desktop_file="${APPDIR}/${APP_NAME}.desktop"
  local root_icon_file="${APPDIR}/${APP_NAME}.png"
  local root_icon_alias_file="${APPDIR}/ohmywu.png"

  require_file "$desktop_file"
  require_file "$icon_file"

  if [[ ! -e "$root_desktop_file" || "$(readlink -f "$desktop_file")" != "$(readlink -f "$root_desktop_file")" ]]; then
    cp -f "$desktop_file" "$root_desktop_file"
  fi

  if [[ ! -e "$root_icon_file" || "$(readlink -f "$icon_file")" != "$(readlink -f "$root_icon_file")" ]]; then
    cp -f "$icon_file" "$root_icon_file"
  fi

  if [[ ! -e "$root_icon_alias_file" || "$(readlink -f "$icon_file")" != "$(readlink -f "$root_icon_alias_file")" ]]; then
    cp -f "$icon_file" "$root_icon_alias_file"
  fi

  if [[ ! -x "${APPDIR}/AppRun" ]]; then
    cat > "${APPDIR}/AppRun" <<'EOF'
#!/usr/bin/env bash
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
exec "$HERE/usr/bin/ohmywu" "$@"
EOF
    chmod +x "${APPDIR}/AppRun"
  fi
}

build_appimage() {
  local appimagetool="${APPIMAGE_TOOL_ROOT}/squashfs-root/appimagetool-prefix/usr/bin/appimagetool"
  require_file "$appimagetool"

  log "packing AppImage"
  PATH="${APPIMAGE_TOOL_ROOT}/squashfs-root/appimagetool-prefix/usr/bin:${PATH}" \
    "$appimagetool" \
    --runtime-file "$APPIMAGE_RUNTIME_FILE" \
    --no-appstream \
    "$APPDIR" \
    "$APPIMAGE_OUTPUT"
}

stage_appdir
prepare_runtime
extract_appimagetool
ensure_root_files
build_appimage

log "done: ${APPIMAGE_OUTPUT}"
