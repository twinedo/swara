#!/usr/bin/env bash
# =============================================================================
# run_migrations.sh — Apply Swara SQL migrations in order
#
# Usage:
#   ./run_migrations.sh                        # uses DATABASE_URL env var
#   ./run_migrations.sh postgres://user:pw@host/db
#   ./run_migrations.sh --seed                 # also apply 005_seed_dev.sql
#
# Requires: psql on $PATH
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATIONS_DIR="$SCRIPT_DIR/migrations"

# ── Resolve connection string ────────────────────────────────────────────────
if [[ "${1:-}" == postgres://* || "${1:-}" == postgresql://* ]]; then
  DB_URL="$1"
  shift
elif [[ -n "${DATABASE_URL:-}" ]]; then
  DB_URL="$DATABASE_URL"
else
  echo "ERROR: No DATABASE_URL set and no connection string provided." >&2
  echo "  Usage: ./run_migrations.sh [postgres://...] [--seed]" >&2
  exit 1
fi

SEED="${1:-}"

# ── Helper ───────────────────────────────────────────────────────────────────
run_migration() {
  local file="$1"
  local name
  name="$(basename "$file")"
  echo "  → Applying $name ..."
  psql "$DB_URL" \
    --single-transaction \
    --set ON_ERROR_STOP=1 \
    --file "$file" \
    --quiet
  echo "    ✓ $name"
}

# ── Create migrations tracking table ────────────────────────────────────────
psql "$DB_URL" --quiet <<'SQL'
CREATE TABLE IF NOT EXISTS _migrations (
  filename    TEXT        PRIMARY KEY,
  applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
SQL

echo ""
echo "Swara — running migrations against: $DB_URL"
echo "────────────────────────────────────────────"

# ── Apply each migration if not already recorded ────────────────────────────
for file in "$MIGRATIONS_DIR"/0*.sql; do
  name="$(basename "$file")"

  # Skip seed file unless --seed flag is passed
  if [[ "$name" == "005_seed_dev.sql" && "$SEED" != "--seed" ]]; then
    echo "  ⊘ Skipping $name (pass --seed to apply)"
    continue
  fi

  already_applied=$(psql "$DB_URL" --tuples-only --no-align \
    --command "SELECT COUNT(*) FROM _migrations WHERE filename = '$name'")

  if [[ "$already_applied" -gt 0 ]]; then
    echo "  ✓ $name (already applied)"
    continue
  fi

  run_migration "$file"

  psql "$DB_URL" --quiet \
    --command "INSERT INTO _migrations (filename) VALUES ('$name')"
done

echo "────────────────────────────────────────────"
echo "All migrations complete."
echo ""

# ── Verify PostGIS ───────────────────────────────────────────────────────────
echo "PostGIS version: $(psql "$DB_URL" --tuples-only --no-align \
  --command "SELECT PostGIS_Version();")"

# ── Verify spatial index exists ─────────────────────────────────────────────
IDX=$(psql "$DB_URL" --tuples-only --no-align --command \
  "SELECT COUNT(*) FROM pg_indexes
   WHERE tablename = 'channels'
   AND indexname = 'channels_location_gist_idx';")

if [[ "$IDX" -eq 1 ]]; then
  echo "Spatial index: ✓ channels_location_gist_idx present"
else
  echo "Spatial index: ✗ MISSING — /nearby queries will be slow!" >&2
  exit 1
fi

echo ""
