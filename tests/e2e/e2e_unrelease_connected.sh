#!/usr/bin/env bash
# Connected Unrelease acceptance. The baseline owner passes the exact running
# PostgreSQL container selected from its lease-owned Compose snapshot.
set -euo pipefail

postgres="${1:?baseline PostgreSQL container is required}"
script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repository_root="$(cd "$script_directory/../.." && pwd -P)"

case "$postgres" in
	????????????????????????????????????????????????????????????????)
		case "$postgres" in
			*[!0123456789abcdef]*)
				echo "Unrelease connected acceptance: PostgreSQL container identity is invalid" >&2
				exit 2
				;;
		esac
		;;
	*)
		echo "Unrelease connected acceptance: PostgreSQL container identity is invalid" >&2
		exit 2
		;;
esac

# Keep the database client in the owned PostgreSQL container. The host neither
# needs psql nor receives the private administrator password.
psql_admin() {
	podman exec -i "$postgres" sh -lc '
		PGPASSWORD="$(cat "$POSTGRES_PASSWORD_FILE")"
		export PGPASSWORD
		exec psql -X -v ON_ERROR_STOP=1 -U ple_e2e_migrator -d ple_e2e_baseline "$@"
	' sh "$@"
}

psql_admin < "$repository_root/tests/e2e/unrelease_connected_oracle.sql"

# Exercise the Assignment-first lock ordering with two real database sessions.
# Session A holds the same Assignment row a Student Work mutator takes; session
# B waits inside Unrelease, then changes lifecycle state.  A fresh Student
# start after B wins must fail and leave no new Attempt behind.
race_workspace="$(mktemp -d "${TMPDIR:-/tmp}/ple-unrelease-race.XXXXXX")"
trap 'rm -rf "$race_workspace"' EXIT
mkfifo "$race_workspace/release_locker"

{
	printf '%s\n' \
		'BEGIN;' \
		'SET LOCAL ROLE ple_data_owner;' \
		'SELECT 1 FROM ple_data.assignment WHERE reference_number = 3 FOR UPDATE;' \
		"SELECT 'race_lock_held';"
	# The parent writes this FIFO only after it has observed Unrelease waiting on
	# the Assignment row.  That makes release a causal step, not a timed guess.
	read -r _ < "$race_workspace/release_locker"
	printf '%s\n' 'COMMIT;'
} | psql_admin -qAt >"$race_workspace/locker.out" 2>"$race_workspace/locker.err" &
locker_pid=$!

for _ in $(seq 1 40); do
	if grep -q '^race_lock_held$' "$race_workspace/locker.out"; then
		break
	fi
	sleep 0.1
done
grep -q '^race_lock_held$' "$race_workspace/locker.out" || {
	echo "Unrelease connected acceptance: row-lock session did not become ready" >&2
	exit 1
}

{
	psql_admin -qAt <<'SQL'
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000001', true) \g /dev/null
SELECT * FROM ple_api.unrelease_assignment(1, 3, 1, 'Unrelease lock race');
COMMIT;
SQL
} >"$race_workspace/unrelease.out" 2>"$race_workspace/unrelease.err" &
unrelease_pid=$!

waited=0
for _ in $(seq 1 30); do
	if psql_admin -qAt \
		-c "SELECT count(*) FROM pg_catalog.pg_stat_activity WHERE query LIKE '%unrelease_assignment(1, 3%' AND wait_event_type = 'Lock'" \
		| grep -qx '1'; then
		waited=1
		break
	fi
	sleep 0.1
done
if [ "$waited" -ne 1 ]; then
	echo "Unrelease connected acceptance: Unrelease did not wait on the Assignment lock" >&2
	exit 1
fi

printf 'release\n' > "$race_workspace/release_locker"
wait "$locker_pid"
wait "$unrelease_pid"
grep -qx '3|Unrelease lock race|unreleased|2|0|0|0|0' "$race_workspace/unrelease.out" || {
	echo "Unrelease connected acceptance: lock-race Unrelease receipt is invalid" >&2
	cat "$race_workspace/unrelease.err" >&2
	exit 1
}

psql_admin -qAt <<'SQL'
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000002', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.start_assignment_attempt(
        '50000000-0000-0000-0000-000000000003',
        '30000000-0000-0000-0000-000000000002',
        '40000000-0000-0000-0000-000000000003',
        '[]'::jsonb,
        '[{"issued_question_id":"50000000-0000-0000-0000-000000000013","assignment_entry_id":"40000000-0000-0000-0000-000000000013","issued_position":0,"question_id":"ABCDEF0","revision_number":1,"question_seed":"9"}]'::jsonb
    );
    RAISE EXCEPTION 'Student Work started after Unrelease changed the Assignment state';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_private.assignment_attempt
        WHERE assignment_id = '40000000-0000-0000-0000-000000000003'
    ) THEN
        RAISE EXCEPTION 'Student Work survived the Unrelease lock race';
    END IF;
END $$;
COMMIT;
SQL

echo "Unrelease connected acceptance: PASS"
