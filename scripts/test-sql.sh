#!/usr/bin/env bash
# Dedicated disposable database only. The test creates a table and two roles.
set -euo pipefail
[[ "${PGACTIVITY_TEST_DISPOSABLE:-}" == 1 ]] || { echo "Refusing to run: set PGACTIVITY_TEST_DISPOSABLE=1 only on a disposable database." >&2; exit 2; }
cd "$(dirname "$0")/.."
PSQL=(psql -X -v ON_ERROR_STOP=1)
"${PSQL[@]}" -f sql/identity.sql
"${PSQL[@]}" -f sql/activity.sql
"${PSQL[@]}" <<'SQL'
CREATE ROLE pgactivity_ci_monitor LOGIN;
GRANT pg_monitor TO pgactivity_ci_monitor;
CREATE ROLE pgactivity_ci_restricted LOGIN;
CREATE TABLE pgactivity_ci_locks(id integer PRIMARY KEY);
INSERT INTO pgactivity_ci_locks VALUES(1);
SET ROLE pgactivity_ci_monitor;
SELECT pg_has_role(current_user,'pg_read_all_stats','USAGE') AS can_read;
RESET ROLE;
SQL
# Test visibility and queries as both privilege levels, without external logins.
for role in pgactivity_ci_monitor pgactivity_ci_restricted; do
  { echo "SET ROLE $role;"; cat sql/identity.sql; cat sql/activity.sql; } | "${PSQL[@]}"
done
holder=''; waiter=''
cleanup(){
  [[ -z "$holder" ]] || kill "$holder" 2>/dev/null || true
  [[ -z "$waiter" ]] || kill "$waiter" 2>/dev/null || true
  "${PSQL[@]}" -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name IN ('pgactivity_ci_holder','pgactivity_ci_waiter') AND pid<>pg_backend_pid();" >/dev/null || true
  "${PSQL[@]}" -c 'DROP TABLE IF EXISTS pgactivity_ci_locks; DROP ROLE IF EXISTS pgactivity_ci_monitor; DROP ROLE IF EXISTS pgactivity_ci_restricted;' >/dev/null || true
}
trap cleanup EXIT
PGAPPNAME=pgactivity_ci_holder "${PSQL[@]}" -c 'BEGIN; UPDATE pgactivity_ci_locks SET id=1 WHERE id=1; SELECT pg_sleep(30); ROLLBACK;' & holder=$!
for i in $(seq 1 20); do
  if [[ $("${PSQL[@]}" -Atc "SELECT count(*) FROM pg_stat_activity WHERE application_name='pgactivity_ci_holder' AND wait_event='PgSleep'") == 1 ]]; then break; fi
  sleep 0.5
done
PGAPPNAME=pgactivity_ci_waiter "${PSQL[@]}" -c 'UPDATE pgactivity_ci_locks SET id=1 WHERE id=1;' & waiter=$!
found=0
for i in $(seq 1 20); do
  if [[ $("${PSQL[@]}" -Atc "SELECT count(*) FROM pg_stat_activity WHERE application_name='pgactivity_ci_waiter' AND wait_event_type='Lock' AND cardinality(pg_blocking_pids(pid))>0") == 1 ]]; then found=1;break;fi
  sleep 0.5
done
[[ "$found" == 1 ]] || { echo 'FAIL: lock not detected'; exit 1; }
# Exercise collector SQL in presence of real lock chain.
"${PSQL[@]}" -f sql/activity.sql
printf 'PASS: collector SQL and lock detection\n'

# Smoke native metrics with exact version-specific SQL on disposable CI server.
version=$("${PSQL[@]}" -Atc "SHOW server_version_num")
"${PSQL[@]}" -f sql/metrics_db.sql
if (( version >= 140000 )); then "${PSQL[@]}" -f sql/metrics_wal.sql; fi
if (( version >= 180000 )); then
  "${PSQL[@]}" -f sql/metrics_io18.sql
  "${PSQL[@]}" -f sql/metrics_pid18.sql
elif (( version >= 160000 )); then
  "${PSQL[@]}" -f sql/metrics_io16.sql
fi
printf 'PASS: version-specific metrics SQL\n'
