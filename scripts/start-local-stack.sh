#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${ROOT_DIR}/.env.local"
RUN_DIR="${ROOT_DIR}/.local/run"

mkdir -p "${RUN_DIR}"

if [ ! -f "${ENV_FILE}" ]; then
  echo "Missing ${ENV_FILE}. Run ./scripts/setup-local-macos.sh first."
  exit 1
fi

set -a
source "${ENV_FILE}"
set +a

API_PORT="${PORT:-3100}"

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required but was not found."
  exit 1
fi

if ! command -v livekit-server >/dev/null 2>&1; then
  echo "livekit-server not found. Run ./scripts/setup-local-macos.sh first."
  exit 1
fi

start_background_process() {
  local name="$1"
  local command="$2"
  local pid_file="${RUN_DIR}/${name}.pid"
  local log_file="${RUN_DIR}/${name}.log"

  if [ -f "${pid_file}" ]; then
    local existing_pid
    existing_pid="$(cat "${pid_file}")"
    if kill -0 "${existing_pid}" >/dev/null 2>&1; then
      echo "${name} already running with pid ${existing_pid}"
      return 0
    fi
    rm -f "${pid_file}"
  fi

  /bin/zsh -lc "cd '${ROOT_DIR}' && source '${ENV_FILE}' && ${command}" >>"${log_file}" 2>&1 &
  local pid=$!
  echo "${pid}" > "${pid_file}"
  echo "Started ${name} with pid ${pid}"
}

echo "Starting PostgreSQL and Redis services..."
brew services start postgresql@17 >/dev/null 2>&1 || true
brew services start redis >/dev/null 2>&1 || true

echo "Starting LiveKit and API..."
start_background_process "livekit" "./scripts/start-livekit-dev.sh"
start_background_process "api" "./scripts/run-api-local.sh"

cat <<EOF

Local stack started.

Swagger UI:
  http://127.0.0.1:${API_PORT}/swagger-ui/

OpenAPI JSON:
  http://127.0.0.1:${API_PORT}/api-docs/openapi.json

Logs:
  ${RUN_DIR}/livekit.log
  ${RUN_DIR}/api.log
EOF
