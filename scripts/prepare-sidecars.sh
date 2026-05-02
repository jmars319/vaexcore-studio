#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAURI_DIR="$ROOT_DIR/apps/desktop/src-tauri"
BIN_DIR="$TAURI_DIR/binaries"

TARGET_TRIPLE="${TAURI_TARGET_TRIPLE:-}"
if [ -z "$TARGET_TRIPLE" ]; then
  if rustc --print host-tuple >/dev/null 2>&1; then
    TARGET_TRIPLE="$(rustc --print host-tuple)"
  else
    TARGET_TRIPLE="$(rustc -Vv | awk '/host:/ { print $2 }')"
  fi
fi

if [ -z "$TARGET_TRIPLE" ]; then
  echo "failed to determine Rust target triple" >&2
  exit 1
fi

EXT=""
case "$TARGET_TRIPLE" in
  *windows* | *msvc* | *pc-windows*) EXT=".exe" ;;
esac

cargo build -p vaexcore-media-runner --release

SOURCE="$ROOT_DIR/target/release/media-runner$EXT"
DEST="$BIN_DIR/media-runner-$TARGET_TRIPLE$EXT"

if [ ! -f "$SOURCE" ]; then
  echo "expected sidecar binary not found: $SOURCE" >&2
  exit 1
fi

mkdir -p "$BIN_DIR"
cp "$SOURCE" "$DEST"
chmod 755 "$DEST" 2>/dev/null || true

echo "prepared sidecar: $DEST"
