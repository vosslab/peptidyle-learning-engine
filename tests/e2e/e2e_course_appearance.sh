#!/usr/bin/env bash
# e2e_course_appearance.sh - leased PostgreSQL and MinIO cross-store oracle.
#
# The public entry delegates lifecycle ownership to the fixed live-demo lease.
# Its private child receives one owner-created, non-secret runtime locator and
# cannot select a Compose project or credential source.

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$script_directory/../.." && pwd -P)"
readonly REPO_ROOT

if [ "${1:-}" != "--owned-child" ]; then
	cd "$REPO_ROOT"
	exec python3 -m local_stack_control.course_appearance_cross_store_owner
fi
shift

[ "$#" -eq 2 ] && [ "$1" = "--runtime-manifest" ] && [ "$2" = "runtime.yaml" ] || {
	echo "course appearance E2E: private child requires the owner-created runtime manifest" >&2
	exit 2
}
runtime_manifest="$2"
workspace="$(pwd -P)"
runtime_manifest_path="$workspace/$runtime_manifest"
postgres_migration_acceptance_runtime_manifest_path="$workspace/postgres_migration_acceptance/runtime.yaml"
compose_started=0

readonly database_name="ple_e2e_baseline"
readonly bootstrap_user="ple_e2e_migrator"
readonly postgres_database="postgres"

fail() {
	echo "course appearance E2E: $*" >&2
	exit 1
}

require_command() {
	command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

compose() {
	(
		cd "$REPO_ROOT"
		python3 -m local_stack_control.disposable_stack_command compose \
			--manifest "$runtime_manifest_path" "$@"
	)
}

cleanup() {
	local status="$?"
	local cleanup_failed=0
	if [ "$compose_started" = "1" ]; then
		(
			cd "$REPO_ROOT"
			python3 -m local_stack_control.disposable_stack_command cleanup \
				--manifest "$runtime_manifest_path"
		) || cleanup_failed=1
	fi
	if [ "$cleanup_failed" = "1" ]; then
		echo "course appearance E2E: cleanup failed for the leased cross-store profile" >&2
		[ "$status" -ne 0 ] || status=1
	fi
	exit "$status"
}
trap cleanup EXIT

wait_for_postgres() {
	for _ in {1..30}; do
		if compose exec -T postgres pg_isready -U "$bootstrap_user" -d "$postgres_database" \
			>/dev/null 2>&1; then
			return 0
		fi
		sleep 1
	done
	fail "disposable PostgreSQL did not become ready"
}

wait_for_minio() {
	for _ in {1..30}; do
		if compose exec -T minio mc ready local >/dev/null 2>&1; then
			return 0
		fi
		sleep 1
	done
	fail "disposable MinIO did not become ready"
}

run_project_tools() {
	(
		cd "$workspace"
		cargo run --manifest-path "$REPO_ROOT/Cargo.toml" --quiet -p project-tools -- \
			database "$@" --acceptance-runtime
	)
}

run_postgres_migration_acceptance_project_tools() {
	(
		cd "$workspace"
		PLE_ACCEPTANCE_RUNTIME_MANIFEST="$postgres_migration_acceptance_runtime_manifest_path" \
			cargo run --manifest-path "$REPO_ROOT/Cargo.toml" --quiet -p project-tools -- \
			database "$@" --acceptance-runtime
	)
}

run_live_cargo_test() {
	local label="$1"
	shift
	[ "${1:-}" = "cargo" ] && [ "${2:-}" = "test" ] ||
		fail "$label must use the repository-owned Cargo test boundary"
	shift 2
	local output
	if ! output="$(cd "$workspace" && cargo test --manifest-path "$REPO_ROOT/Cargo.toml" "$@" 2>&1)"; then
		printf '%s\n' "$output" >&2
		fail "$label cargo test command failed"
	fi
	printf '%s\n' "$output"
	if ! grep -Eq 'test result: ok\. [1-9][0-9]* passed;' <<<"$output"; then
		fail "$label selected no live tests; update its exact test target"
	fi
}

cd "$REPO_ROOT"
require_command cargo
require_command podman
require_command python3
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
export PLE_ACCEPTANCE_RUNTIME_MANIFEST="$runtime_manifest_path"

echo "course appearance E2E: starting leased PostgreSQL and MinIO"
compose_started=1
compose up -d postgres minio
wait_for_postgres
wait_for_minio
compose run --rm createbuckets
compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U "$bootstrap_user" -d "$postgres_database" <<'SQL'
CREATE ROLE ple_database_owner NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_migrator LOGIN NOINHERIT NOSUPERUSER NOCREATEDB CREATEROLE
    NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 2;
GRANT ple_database_owner TO ple_migrator WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
CREATE DATABASE ple_e2e_baseline OWNER ple_database_owner;
SQL
(
	cd "$REPO_ROOT"
	python3 -m local_stack_control.runtime_manifest --emit-migration-acceptance-bootstrap "$workspace"
) | compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U "$bootstrap_user" -d "$postgres_database"
compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U "$bootstrap_user" -d "$postgres_database" -c \
	"REVOKE CONNECT, CREATE, TEMPORARY ON DATABASE $database_name FROM PUBLIC; GRANT CONNECT ON DATABASE $database_name TO ple_migrator"
compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U "$bootstrap_user" -d "$database_name" -c \
	'REVOKE ALL ON SCHEMA public FROM PUBLIC; GRANT CREATE, USAGE ON SCHEMA public TO ple_migrator; GRANT USAGE ON SCHEMA pg_catalog TO ple_migrator'

echo "course appearance E2E: applying and verifying the accepted migration set"
run_postgres_migration_acceptance_project_tools migration-acceptance-migrate
run_postgres_migration_acceptance_project_tools migration-acceptance-verify

echo "course appearance E2E: MinIO object-store conformance"
run_live_cargo_test "MinIO object-store conformance" cargo test -p objects --features s3 \
	--test conformance minio_object_store_conforms -- --ignored --exact --test-threads=1

echo "course appearance E2E: typed banner object contract"
# The fresh PostgreSQL schema intentionally has no course-appearance current-pointer
# capability yet.  `minio_object_store_conforms` exercises the candidate and
# current Course Banner object addresses against real MinIO; the future
# database-backed promotion and cleanup oracle belongs with that capability.

echo "course appearance E2E: PASS"
