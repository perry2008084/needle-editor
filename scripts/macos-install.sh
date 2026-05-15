#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_SOURCE="$SCRIPT_DIR/Needle Editor.app"
APP_TARGET="/Applications/Needle Editor.app"

if [[ ! -d "$APP_SOURCE" ]]; then
  echo "Needle Editor.app not found next to macos-install.sh" >&2
  exit 1
fi

rm -rf "$APP_TARGET"
cp -R "$APP_SOURCE" "$APP_TARGET"

echo "Needle Editor installed to: $APP_TARGET"
echo "If macOS warns about an unsigned app, right-click the app and choose Open."
