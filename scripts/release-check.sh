#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

APP_BUNDLE="target/release/bundle/macos/vaexcore studio.app"
APP_BINARY="$APP_BUNDLE/Contents/MacOS/vaexcore-studio"
MEDIA_RUNNER="$APP_BUNDLE/Contents/MacOS/media-runner"

cargo fmt --all -- --check
cargo test --workspace
npm run typecheck
npm run build
npm run tauri -w apps/desktop -- build

test -d "$APP_BUNDLE"
test -x "$APP_BINARY"
test -x "$MEDIA_RUNNER"

if command -v codesign >/dev/null 2>&1; then
  codesign -dvvv --entitlements :- "$APP_BUNDLE" || true
  codesign -dvvv "$MEDIA_RUNNER" || true
fi

if command -v spctl >/dev/null 2>&1; then
  spctl -a -vv "$APP_BUNDLE" || true
fi

if command -v plutil >/dev/null 2>&1; then
  plutil -p "$APP_BUNDLE/Contents/Info.plist" >/dev/null
fi

echo "release check complete"
