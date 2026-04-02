#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${ROOT_DIR}/.env.local"

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required but was not found."
  exit 1
fi

echo "Installing local dependencies with Homebrew..."
brew install postgresql@17 postgis redis livekit

echo "Starting PostgreSQL and Redis..."
brew services stop postgresql@16 >/dev/null 2>&1 || true
brew services start postgresql@17
brew services start redis

PG_BIN="/opt/homebrew/opt/postgresql@17/bin"
export PATH="${PG_BIN}:$(brew --prefix)/bin:$PATH"

echo "Waiting for PostgreSQL to accept connections..."
for _ in {1..20}; do
  if pg_isready -h 127.0.0.1 -p 5432 >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! pg_isready -h 127.0.0.1 -p 5432 >/dev/null 2>&1; then
  echo "PostgreSQL did not become ready in time."
  exit 1
fi

if ! psql -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = 'swara_dev'" | grep -q 1; then
  echo "Creating swara_dev database..."
  createdb swara_dev
fi

echo "Enabling required PostgreSQL extensions..."
psql -d swara_dev -c 'CREATE EXTENSION IF NOT EXISTS postgis;'
psql -d swara_dev -c 'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";'

if [ ! -f "${ENV_FILE}" ]; then
  cat > "${ENV_FILE}" <<'EOF'
DATABASE_URL=postgresql:///swara_dev
REDIS_URL=redis://127.0.0.1:6379
JWT_SECRET=dev-jwt-secret-change-me
LIVEKIT_URL=http://127.0.0.1:7880
LIVEKIT_API_KEY=devkey
LIVEKIT_API_SECRET=secret
LIVEKIT_TOKEN_TTL_SECS=21600
PORT=3100
EOF
  echo "Created ${ENV_FILE}"
else
  echo "${ENV_FILE} already exists; left unchanged."
fi

cat <<'EOF'

Local dependencies are ready.

Next terminal:
  source .env.local
  ./scripts/start-livekit-dev.sh

API terminal:
  source .env.local
  cargo run -p swara-api
EOF
