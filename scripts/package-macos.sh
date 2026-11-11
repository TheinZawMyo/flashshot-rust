#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="aarch64-apple-darwin"
APP_BUNDLE="${ROOT_DIR}/dist/Flashshot.app"

cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" --release --target "${TARGET}"

mkdir -p "${APP_BUNDLE}/Contents/MacOS"
cp "${ROOT_DIR}/target/${TARGET}/release/flashshot" "${APP_BUNDLE}/Contents/MacOS/flashshot"
cp "${ROOT_DIR}/packaging/macos/Info.plist" "${APP_BUNDLE}/Contents/Info.plist"

# Ad-hoc signing makes the local bundle self-contained for testing on this Mac.
codesign --force --deep --sign - "${APP_BUNDLE}"

echo "Created ${APP_BUNDLE}"
