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

stop_background_process() {
  local name="$1"
  local pid_file="${RUN_DIR}/${name}.pid"

  if [ ! -f "${pid_file}" ]; then
    echo "${name} not running"
    return 0
  fi

  local pid
  pid="$(cat "${pid_file}")"

  if kill -0 "${pid}" >/dev/null 2>&1; then
    kill "${pid}" >/dev/null 2>&1 || true
    echo "Stopped ${name} (pid ${pid})"
  else
    echo "${name} pid file existed but process ${pid} was not running"
  fi

  rm -f "${pid_file}"
}

stop_port_listener() {
  local name="$1"
  local port="$2"

  if ! command -v lsof >/dev/null 2>&1; then
    return 0
  fi

  local pid
  pid="$(lsof -tiTCP:${port} -sTCP:LISTEN 2>/dev/null | head -n 1 || true)"

  if [ -z "${pid}" ]; then
    return 0
  fi

  if kill -0 "${pid}" >/dev/null 2>&1; then
    kill "${pid}" >/dev/null 2>&1 || true
    echo "Stopped ${name} listener on port ${port} (pid ${pid})"
  fi
}

mkdir -p "${RUN_DIR}"

stop_background_process "api"
stop_background_process "livekit"
stop_port_listener "api" "${API_PORT}"
stop_port_listener "livekit" "7880"

if command -v brew >/dev/null 2>&1; then
  brew services stop redis >/dev/null 2>&1 || true
  brew services stop postgresql@17 >/dev/null 2>&1 || true
  echo "Stopped PostgreSQL and Redis services"
fi
