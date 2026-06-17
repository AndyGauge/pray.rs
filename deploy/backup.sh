#!/usr/bin/env bash
# Daily SQLite backup with grandfather-father-son retention:
#   - Daily snapshots:               keep the newest 7  (~1 week)
#   - Weekly snapshots (Sundays):    keep the newest 4  (~1 month)
# => at most 11 files covering ~28 days.
#
# Uses sqlite3's online `.backup` (crash-consistent while the app is running) —
# never a plain `cp`, which can capture a mid-write database.
set -euo pipefail

DB=/opt/thanksgivings/data/thanksgivings.db
DIR=/opt/thanksgivings/backups
KEEP_DAILY=7
KEEP_WEEKLY=4

mkdir -p "$DIR"
stamp=$(date +%F)                 # YYYY-MM-DD
daily="$DIR/daily-$stamp.db"

# Consistent online snapshot.
sqlite3 "$DB" ".backup '$daily'"

# On Sundays, promote today's snapshot to a weekly restore point.
if [ "$(date +%u)" -eq 7 ]; then
  cp -f "$daily" "$DIR/weekly-$stamp.db"
fi

# Retention: keep only the newest N of each kind (newest-first, drop the rest).
prune() {  # $1 = prefix, $2 = keep count
  ls -1t "$DIR/$1"-*.db 2>/dev/null | tail -n +"$(( $2 + 1 ))" | xargs -r rm -f
}
prune daily  "$KEEP_DAILY"
prune weekly "$KEEP_WEEKLY"

echo "$(date -Is) backup ok — $(ls -1 "$DIR"/*.db 2>/dev/null | wc -l | tr -d ' ') files in $DIR"
