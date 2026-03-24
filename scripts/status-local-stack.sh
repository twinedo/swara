#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUN_DIR="${ROOT_DIR}/.local/run"
ENV_FILE="${ROOT_DIR}/.env.local"

if [ -f "${ENV_FILE}" ]; then
  set -a
  source "${ENV_FILE}"
  set +a
fi

API_PORT="${PORT:-3100}"

print_service_lines() {
  if command -v rg >/dev/null 2>&1; then
    rg 'postgresql@17|redis' || true
  else
    grep -E 'postgresql@17|redis' || true
  fi
}

print_process_status() {
  local name="$1"
  local pid_file="${RUN_DIR}/${name}.pid"
  local port="${2:-}"

  if [ ! -f "${pid_file}" ]; then
    if [ -n "${port}" ] && command -v lsof >/dev/null 2>&1; then
      local port_pid
      port_pid="$(lsof -tiTCP:"${port}" -sTCP:LISTEN 2>/dev/null | head -n 1 || true)"
      if [ -n "${port_pid}" ]; then
        echo "${name}: running outside managed scripts (pid ${port_pid}, port ${port})"
        return 0
      fi
    fi
    echo "${name}: not running"
    return 0
  fi

  local pid
  pid="$(cat "${pid_file}")"

  if kill -0 "${pid}" >/dev/null 2>&1; then
    echo "${name}: running (pid ${pid})"
  else
    echo "${name}: stale pid file (${pid})"
  fi
}

mkdir -p "${RUN_DIR}"

echo "Local stack status"
echo

if command -v brew >/dev/null 2>&1; then
  echo "Homebrew services:"
  brew services list | print_service_lines
  echo
fi

echo "Background processes:"
print_process_status "livekit" "7880"
print_process_status "api" "${API_PORT}"
echo

echo "Endpoints:"
echo "swagger-ui: http://127.0.0.1:${API_PORT}/swagger-ui/"
echo "openapi:    http://127.0.0.1:${API_PORT}/api-docs/openapi.json"
