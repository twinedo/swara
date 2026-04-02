#!/bin/zsh

set -euo pipefail

if ! command -v livekit-server >/dev/null 2>&1; then
  echo "livekit-server not found. Run scripts/setup-local-macos.sh first."
  exit 1
fi

LIVEKIT_BIND="${LIVEKIT_BIND:-0.0.0.0}"

if [[ -n "${LIVEKIT_NODE_IP:-}" ]]; then
  exec livekit-server --dev --bind "${LIVEKIT_BIND}" --node-ip "${LIVEKIT_NODE_IP}"
fi

exec livekit-server --dev --bind "${LIVEKIT_BIND}"
