#!/usr/bin/env bash
# e2e_postgres_migration_acceptance.sh - disposable PostgreSQL Migration Acceptance Runtime.
# The public entry point delegates the lease, private manifest, and fixed Compose ownership to local_stack_control.postgres_migration_acceptance_owner; the private child owns only this PostgreSQL 17 oracle.
set -euo pipefail
script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$script_directory/../.." && pwd -P)"
readonly REPO_ROOT
if [ "${1:-}" != "--owned-child" ]; then
	cd "$REPO_ROOT"
	exec python3 -m local_stack_control.postgres_migration_acceptance_owner
fi
shift
[ "$#" -eq 2 ] && [ "$1" = "--runtime-manifest" ] && [ "$2" = "runtime.yaml" ] || {
	echo "PostgreSQL Migration Acceptance Runtime E2E: private child requires the owner-created runtime manifest" >&2
	exit 2
}
RUNTIME_MANIFEST="$2"
WORKSPACE="$(pwd -P)"
RUNTIME_MANIFEST_PATH="$WORKSPACE/$RUNTIME_MANIFEST"
POSTGRES_MIGRATION_ACCEPTANCE_RUNTIME_MANIFEST_PATH="$WORKSPACE/postgres_migration_acceptance/runtime.yaml"
readonly RUNTIME_MANIFEST WORKSPACE RUNTIME_MANIFEST_PATH POSTGRES_MIGRATION_ACCEPTANCE_RUNTIME_MANIFEST_PATH
readonly DATABASE_NAME="ple_e2e_baseline"
readonly BOOTSTRAP_USER="ple_e2e_migrator"
readonly POSTGRES_DB="postgres"
readonly PROJECT_NAME="ple-live-demo-browser"
readonly POSTGRES_MIGRATION_ACCEPTANCE_MIGRATION="2026082901"
compose_started=0
postgres_volume_name=""
fail() {
	echo "PostgreSQL Migration Acceptance Runtime E2E: $*" >&2
	exit 1
}
require_command() {
	command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}
compose() { (cd "$REPO_ROOT"; python3 -m local_stack_control.disposable_stack_command compose --manifest "$RUNTIME_MANIFEST_PATH" "$@"); }
capture_postgres_volume() {
	local container_ids container_id volume_projects
	container_ids="$(podman ps -aq --filter "label=io.podman.compose.project=$PROJECT_NAME" --filter 'label=io.podman.compose.service=postgres')"
	if [ "$(printf '%s\n' "$container_ids" | sed '/^$/d' | wc -l | tr -d ' ')" -ne 1 ]; then
		fail "could not resolve exactly one labelled PostgreSQL container"
	fi
	container_id="$container_ids"
	postgres_volume_name="$(podman inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' "$container_id")"
	[ -n "$postgres_volume_name" ] || fail "PostgreSQL container has no data volume"
	podman volume inspect "$postgres_volume_name" >/dev/null 2>&1 || fail "captured PostgreSQL volume is unavailable"
	volume_projects="$(podman volume inspect "$postgres_volume_name" --format '{{index .Labels "io.podman.compose.project"}}|{{index .Labels "com.docker.compose.project"}}')"
	case "$volume_projects" in
		*'|containers|'* | 'containers|'* | *'|containers')
			fail "refused to claim the ordinary containers volume"
			;;
	esac
	echo "PostgreSQL Migration Acceptance Runtime E2E: captured disposable PostgreSQL volume $postgres_volume_name"
}
cleanup() {
	local status="$?"
	local cleanup_failed=0
	if [ "$compose_started" = "1" ]; then
		(cd "$REPO_ROOT"; python3 -m local_stack_control.disposable_stack_command cleanup --manifest "$RUNTIME_MANIFEST_PATH") || cleanup_failed=1
	fi
	if [ "$cleanup_failed" = "0" ]; then
		if [ -n "$postgres_volume_name" ] && podman volume inspect "$postgres_volume_name" >/dev/null 2>&1; then
			echo "PostgreSQL Migration Acceptance Runtime E2E: captured PostgreSQL volume survived cleanup" >&2
			cleanup_failed=1
		fi
		remaining_containers="$(podman ps -aq --filter "label=io.podman.compose.project=$PROJECT_NAME")"
		if [ -n "$remaining_containers" ]; then
			echo "PostgreSQL Migration Acceptance Runtime E2E: labelled PostgreSQL target survived cleanup" >&2
			cleanup_failed=1
		fi
	fi
	if [ "$cleanup_failed" = "1" ]; then
		echo "PostgreSQL Migration Acceptance Runtime E2E: cleanup failed; retain $WORKSPACE for inspection" >&2
		[ "$status" -ne 0 ] || status=1
	fi
	exit "$status"
}
trap cleanup EXIT
psql_in_container() {
	local login="$1"
	shift
	compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U "$login" "$@"
}
run_postgres_migration_acceptance_tool() { (cd "$WORKSPACE"; PLE_ACCEPTANCE_RUNTIME_MANIFEST="$POSTGRES_MIGRATION_ACCEPTANCE_RUNTIME_MANIFEST_PATH" cargo run --manifest-path "$REPO_ROOT/Cargo.toml" --quiet -p project-tools -- database "$@" --acceptance-runtime); }
expect_denied() {
	local label="$1"
	shift
	if "$@" >/dev/null 2>&1; then
		fail "$label unexpectedly succeeded"
	fi
}
assert_restricted_logins() {
	echo "PostgreSQL Migration Acceptance Runtime E2E: restricted LOGIN allow/deny probes"
	psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" <<'SQL'
CREATE ROLE ple_postgres_migration_acceptance_app_probe LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_postgres_migration_acceptance_auth_probe LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_postgres_migration_acceptance_student_probe LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
GRANT ple_app TO ple_postgres_migration_acceptance_app_probe WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT ple_auth TO ple_postgres_migration_acceptance_auth_probe WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT ple_student TO ple_postgres_migration_acceptance_student_probe WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT CONNECT ON DATABASE ple_e2e_baseline TO ple_postgres_migration_acceptance_app_probe, ple_postgres_migration_acceptance_auth_probe,
    ple_postgres_migration_acceptance_student_probe;
SQL
	psql_in_container ple_postgres_migration_acceptance_app_probe -d "$DATABASE_NAME" -c 'SET ROLE ple_app; SELECT version, success, checksum FROM ple_api.ple_migration_state LIMIT 1' >/dev/null
	expect_denied "ple_app direct migration-ledger read" psql_in_container ple_postgres_migration_acceptance_app_probe -d "$DATABASE_NAME" -c 'SET ROLE ple_app; SELECT 1 FROM public._sqlx_migrations LIMIT 1'
	expect_denied "ple_app data-schema usage" psql_in_container ple_postgres_migration_acceptance_app_probe -d "$DATABASE_NAME" -c 'SET ROLE ple_app; SELECT 1 FROM ple_data.postgres_migration_acceptance_probe'
	local probe capability
	for probe in ple_postgres_migration_acceptance_auth_probe ple_postgres_migration_acceptance_student_probe; do
		case "$probe" in
			ple_postgres_migration_acceptance_auth_probe) capability="ple_auth" ;;
			ple_postgres_migration_acceptance_student_probe) capability="ple_student" ;;
			*) fail "unknown restricted PostgreSQL Migration Acceptance Runtime probe $probe" ;;
		esac
			expect_denied "$probe API read" psql_in_container "$probe" -d "$DATABASE_NAME" -c "SET ROLE $capability; SELECT 1 FROM ple_api.ple_migration_state LIMIT 1"
			expect_denied "$probe data-schema usage" psql_in_container "$probe" -d "$DATABASE_NAME" -c "SET ROLE $capability; SELECT 1 FROM ple_data.postgres_migration_acceptance_probe"
			expect_denied "$probe owner SET ROLE" psql_in_container "$probe" -d "$DATABASE_NAME" -c 'SET ROLE ple_data_owner'
			expect_denied "$probe object creation" psql_in_container "$probe" -d "$DATABASE_NAME" -c "SET ROLE $capability; CREATE TABLE ple_api.postgres_migration_acceptance_probe_denied (id integer)"
	done
}
assert_imathas_question_backend_service_logins() {
	echo "PostgreSQL Migration Acceptance Runtime E2E: iMathAS Question Backend API/worker authority probes"
	expect_denied "API login cannot assume procedure owner" psql_in_container ple_api_login -d "$DATABASE_NAME" -c 'SET ROLE ple_api_owner'; expect_denied "worker login cannot assume procedure owner" psql_in_container ple_worker_login -d "$DATABASE_NAME" -c 'SET ROLE ple_api_owner'
	expect_denied "API login cannot assume grading worker" psql_in_container ple_api_login -d "$DATABASE_NAME" -c 'SET ROLE ple_imathas_question_backend_grading_worker'; expect_denied "grading worker cannot assume API capability" psql_in_container ple_worker_login -d "$DATABASE_NAME" -c 'SET ROLE ple_app'
	expect_denied "API login cannot read iMathAS Question Backend Sessions directly" psql_in_container ple_api_login -d "$DATABASE_NAME" -c 'SELECT 1 FROM ple_private.imathas_question_backend_session LIMIT 1'; expect_denied "grading worker cannot read iMathAS Question Backend Sessions directly" psql_in_container ple_worker_login -d "$DATABASE_NAME" -c 'SELECT 1 FROM ple_private.imathas_question_backend_session LIMIT 1'
}
cd "$REPO_ROOT"
require_command podman
require_command cargo
require_command python3
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
export PLE_ACCEPTANCE_RUNTIME_MANIFEST="$RUNTIME_MANIFEST_PATH"
echo "PostgreSQL Migration Acceptance Runtime E2E: starting isolated PostgreSQL 17 project $PROJECT_NAME"
compose_started=1
compose up -d postgres
capture_postgres_volume
ready=0
for _ in {1..30}; do
	if psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB" -c 'SELECT 1' >/dev/null 2>&1; then
		ready=1
		break
	fi
	sleep 1
done
[ "$ready" = "1" ] || fail "disposable PostgreSQL did not become ready"
version_major="$(psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB" -At -c "SELECT split_part(current_setting('server_version'), '.', 1)")"
[ "$version_major" = "17" ] || fail "disposable PostgreSQL is not major 17 (got $version_major)"
psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB" <<'SQL'
CREATE ROLE ple_database_owner NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_migrator LOGIN NOINHERIT NOSUPERUSER NOCREATEDB CREATEROLE NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 2;
GRANT ple_database_owner TO ple_migrator WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
CREATE DATABASE ple_e2e_baseline OWNER ple_database_owner;
SQL
(cd "$REPO_ROOT"; python3 -m local_stack_control.runtime_manifest --emit-migration-acceptance-bootstrap "$WORKSPACE") | psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB"
psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB" -c "REVOKE CONNECT, CREATE, TEMPORARY ON DATABASE $DATABASE_NAME FROM PUBLIC; GRANT CONNECT ON DATABASE $DATABASE_NAME TO ple_migrator"
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" -c 'REVOKE ALL ON SCHEMA public FROM PUBLIC; GRANT CREATE, USAGE ON SCHEMA public TO ple_migrator; GRANT USAGE ON SCHEMA pg_catalog TO ple_migrator'
echo "PostgreSQL Migration Acceptance Runtime E2E: Migration Check is pending before apply"
initial_status="$(run_postgres_migration_acceptance_tool migration-acceptance-status)"
printf '%s\n' "$initial_status"
printf '%s\n' "$initial_status" | grep -Eq "$POSTGRES_MIGRATION_ACCEPTANCE_MIGRATION.*pending" || fail "Migration Check did not report $POSTGRES_MIGRATION_ACCEPTANCE_MIGRATION as pending"
echo "PostgreSQL Migration Acceptance Runtime E2E: fresh apply and second-run no-op"
run_postgres_migration_acceptance_tool migration-acceptance-migrate
second_apply="$(run_postgres_migration_acceptance_tool migration-acceptance-migrate)"
printf '%s\n' "$second_apply"
printf '%s\n' "$second_apply" | grep -Eiq 'no.?op|already applied|complete' || fail "second PostgreSQL Migration did not report a no-op-compatible result"
run_postgres_migration_acceptance_tool migration-acceptance-verify
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" < "$REPO_ROOT/tests/e2e/question_records.sql"
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" < "$REPO_ROOT/tests/e2e/question_publication_credit_catalog.sql"
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" < "$REPO_ROOT/tests/e2e/assignment_revision_entry_snapshot_catalog.sql"
echo "PostgreSQL Migration Acceptance Runtime E2E: exact principal, schema, ACL, and membership catalog"
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" < "$REPO_ROOT/tests/e2e/postgres_migration_acceptance_catalog.sql"
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" < "$REPO_ROOT/tests/e2e/postgres_migration_acceptance_instructor_account_creation.sql"
assert_restricted_logins
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" < "$REPO_ROOT/tests/e2e/imathas_question_backend_session_postgres_oracle.sql"
assert_imathas_question_backend_service_logins
bash "$REPO_ROOT/tests/e2e/e2e_imathas_question_backend_session_postgres_oracle.sh" "$WORKSPACE"
echo "PostgreSQL Migration Acceptance Runtime E2E: PASS (fresh apply, no-op, PostgreSQL 17 catalog, restricted probes)"
