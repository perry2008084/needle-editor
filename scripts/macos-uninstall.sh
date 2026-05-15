#!/usr/bin/env bash
set -euo pipefail

APP_TARGET="/Applications/Needle Editor.app"

if [[ -d "$APP_TARGET" ]]; then
  rm -rf "$APP_TARGET"
  echo "Needle Editor removed from: $APP_TARGET"
else
  echo "Needle Editor.app is not installed in /Applications"
fi
