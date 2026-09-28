#!/usr/bin/env bash
# Regenerate .sqlx/ — the committed query metadata that lets sqlx's query!/
# query_as! macros type-check every SQL statement at compile time without a
# database (builds run with SQLX_OFFLINE=true via .cargo/config.toml).
#
# Builds a throwaway SQLite schema from crates/db/migrations, then recompiles
# the db crate online against it with SQLX_OFFLINE_DIR set, which makes the
# macros write one query-<hash>.json per statement.
#
#   scripts/sqlx-prepare.sh          regenerate .sqlx/
#   scripts/sqlx-prepare.sh --check  fail if .sqlx/ is stale (for CI / pre-deploy)
#
# Run it after changing any query!/query_as! SQL or adding a migration.
set -euo pipefail
cd "$(dirname "$0")/.."

work=target/sqlx-prepare
rm -rf "$work"; mkdir -p "$work/out"
db="$work/schema.db"

for m in crates/db/migrations/*.sql; do
  sqlite3 "$db" < "$m"
done

# Force the macros to re-expand (they don't track DATABASE_URL changes).
touch crates/db/src/lib.rs
SQLX_OFFLINE=false \
SQLX_OFFLINE_DIR="$PWD/$work/out" \
DATABASE_URL="sqlite://$PWD/$db" \
  cargo check -q -p thanksgivings-db --tests

if [[ "${1:-}" == "--check" ]]; then
  if ! diff -r -q .sqlx "$work/out" >/dev/null 2>&1; then
    echo "error: .sqlx/ is stale — run scripts/sqlx-prepare.sh and commit the result" >&2
    diff -r -q .sqlx "$work/out" >&2 || true
    exit 1
  fi
  echo ".sqlx/ is up to date"
else
  rm -rf .sqlx && mv "$work/out" .sqlx
  echo "wrote $(ls .sqlx | wc -l | tr -d ' ') query files to .sqlx/"
fi
