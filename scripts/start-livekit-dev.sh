#!/bin/zsh

set -euo pipefail

if ! command -v livekit-server >/dev/null 2>&1; then
  echo "livekit-server not found. Run scripts/setup-local-macos.sh first."
  exit 1
fi

exec livekit-server --dev
