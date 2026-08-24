#!/usr/bin/env bash
# e2e_database_baseline.sh - disposable live PostgreSQL baseline acceptance gate.
#
# The public entry delegates lifecycle ownership to the same fixed live-demo
# lease used by browser acceptance. Its private child owns only the PostgreSQL
# oracle and cannot select a Compose project.
#
# Run: bash tests/e2e/e2e_database_baseline.sh

set -euo pipefail

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIRECTORY/../.." && pwd -P)"
readonly REPO_ROOT

if [ "${1:-}" != "--owned-child" ]; then
	cd "$REPO_ROOT"
	exec python3 -m local_stack_control.database_baseline_owner
fi
shift
[ "$#" -eq 0 ] || {
	echo "database baseline E2E: private child received unexpected arguments" >&2
	exit 2
}
OWNER_INPUT="${PLE_DATABASE_BASELINE_OWNER_INPUT:-}"
WORKSPACE="${PLE_DATABASE_BASELINE_WORKSPACE:-}"
E2E_PORT="${PLE_DATABASE_BASELINE_PORT:-}"
[ -n "$OWNER_INPUT" ] && [ -n "$WORKSPACE" ] && [ -n "$E2E_PORT" ] || {
	echo "database baseline E2E: private child requires owner-created input" >&2
	exit 2
}
if ! python3 - "$OWNER_INPUT" "$WORKSPACE" "$E2E_PORT" <<'PY'
import os
import pathlib
import stat
import sys

input_path = pathlib.Path(sys.argv[1])
workspace = pathlib.Path(sys.argv[2])
port_text = sys.argv[3]
metadata = input_path.lstat()
workspace_metadata = workspace.lstat()
if (
    not input_path.is_file()
    or input_path.is_symlink()
    or input_path.parent != workspace
    or metadata.st_uid != os.getuid()
    or stat.S_IMODE(metadata.st_mode) != 0o600
    or workspace.is_symlink()
    or not workspace.is_dir()
    or workspace_metadata.st_uid != os.getuid()
    or stat.S_IMODE(workspace_metadata.st_mode) != 0o700
    or not port_text.isdecimal()
    or not 1024 <= int(port_text) <= 65535
):
    raise SystemExit(1)
PY
then
	echo "database baseline E2E: private child owner input is invalid" >&2
	exit 2
fi
unset PLE_DATABASE_BASELINE_OWNER_INPUT PLE_DATABASE_BASELINE_WORKSPACE PLE_DATABASE_BASELINE_PORT

readonly DATABASE_NAME="ple_e2e_baseline"
readonly TENANT_A="00000000-0000-4000-8000-0000000000a1"
readonly TENANT_B="00000000-0000-4000-8000-0000000000b2"
readonly E2E_PORT
readonly POSTGRES_USER="ple_e2e_migrator"
readonly POSTGRES_DB="postgres"

TEMP_DIR=""
COMPOSE_STARTED=0
GATE_FAILURES=0
ENV_FILE=""
MANIFEST_FILE=""
CAPABILITY_FILE=""
PROJECT_NAME=""
POSTGRES_VOLUME_NAME=""

fail() {
	echo "database baseline E2E: $*" >&2
	exit 1
}

record_failure() {
	echo "database baseline E2E: FAIL: $*" >&2
	GATE_FAILURES=$((GATE_FAILURES + 1))
}

capture_postgres_volume() {
	local container_ids container_id volume_projects
	container_ids="$(podman ps -aq \
		--filter "label=io.podman.compose.project=$PROJECT_NAME" \
		--filter 'label=io.podman.compose.service=postgres')"
	if [ "$(printf '%s\n' "$container_ids" | sed '/^$/d' | wc -l | tr -d ' ')" -ne 1 ]; then
		echo "database baseline E2E: could not resolve exactly one labelled postgres container" >&2
		return 1
	fi
	container_id="$container_ids"
	POSTGRES_VOLUME_NAME="$(podman inspect --format \
		'{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' \
		"$container_id")"
	if [ -z "$POSTGRES_VOLUME_NAME" ]; then
		echo "database baseline E2E: postgres container has no data volume" >&2
		return 1
	fi
	if ! podman volume inspect "$POSTGRES_VOLUME_NAME" >/dev/null 2>&1; then
		echo "database baseline E2E: inspected postgres data volume is unavailable" >&2
		return 1
	fi
	volume_projects="$(podman volume inspect "$POSTGRES_VOLUME_NAME" --format \
		'{{index .Labels "io.podman.compose.project"}}|{{index .Labels "com.docker.compose.project"}}')"
	case "$POSTGRES_VOLUME_NAME" in
		containers_ple_pgdata|containers_ple_miniodata|containers_ple_identity_runtime)
			echo "database baseline E2E: refused to claim an ordinary containers volume" >&2
			return 1
			;;
	esac
	case "$volume_projects" in
		*'|containers|'*)
			echo "database baseline E2E: refused to claim an ordinary containers volume" >&2
			return 1
			;;
	esac
	echo "database baseline E2E: captured disposable postgres volume $POSTGRES_VOLUME_NAME"
}

remove_postgres_volume() {
	if [ -z "$POSTGRES_VOLUME_NAME" ]; then
		return 0
	fi
	if podman volume inspect "$POSTGRES_VOLUME_NAME" >/dev/null 2>&1; then
		podman volume rm "$POSTGRES_VOLUME_NAME" >/dev/null || {
			echo "database baseline E2E: could not remove disposable postgres volume $POSTGRES_VOLUME_NAME" >&2
			return 1
		}
	fi
}

require_command() {
	command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

compose() {
	python3 -m local_stack_control._consumer_cli compose --manifest "$MANIFEST_FILE" "$@"
}

cleanup() {
	local status="$?"
	local cleanup_failed=0
	if [ "$COMPOSE_STARTED" = "1" ]; then
		python3 -m local_stack_control._consumer_cli cleanup --manifest "$MANIFEST_FILE" \
			|| cleanup_failed=1
	fi
	if [ "$cleanup_failed" = "0" ] && [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
		rm -rf -- "$TEMP_DIR"
	fi
	if [ "$cleanup_failed" = "0" ]; then
		[ -n "$ENV_FILE" ] && rm -f -- "$ENV_FILE"
		[ -n "$MANIFEST_FILE" ] && rm -f -- "$MANIFEST_FILE"
		[ -n "$CAPABILITY_FILE" ] && rm -f -- "$CAPABILITY_FILE"
	fi
	if [ "$cleanup_failed" = "1" ]; then
		echo "database baseline E2E: cleanup failed; inspect project $PROJECT_NAME with manifest $MANIFEST_FILE" >&2
		[ "$status" -ne 0 ] || status=1
	fi
	exit "$status"
}
trap cleanup EXIT

psql_in_container() {
	compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" "$@"
}

write_private_target() {
	local capability_digest
	PROJECT_NAME="ple-live-demo-browser"
	ENV_FILE="$WORKSPACE/database-baseline.env"
	MANIFEST_FILE="$WORKSPACE/database-baseline.manifest"
	CAPABILITY_FILE="$WORKSPACE/database-baseline.capability"
	capability_digest="$(python3 -c 'import hashlib, os, secrets, sys; raw = secrets.token_bytes(32); fd = os.open(sys.argv[1], os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600); os.write(fd, raw); os.close(fd); print(hashlib.sha256(raw).hexdigest())' "$CAPABILITY_FILE")"
	umask 077
	printf '%s\n' \
		"POSTGRES_USER=$POSTGRES_USER" \
		"POSTGRES_PASSWORD=$POSTGRES_PASSWORD" \
		"POSTGRES_DB=$POSTGRES_DB" \
		"PLE_POSTGRES_HOST_PORT=$E2E_PORT" \
		"PLE_E2E_OWNER=live-demo-browser" \
		"PLE_DISPOSABLE_CAPABILITY_SHA256=$capability_digest" >"$ENV_FILE"
	printf '%s\n' \
		"OWNER=live-demo-browser" \
		"PROJECT=$PROJECT_NAME" \
		"PROFILE=database_baseline" \
		"ENV_FILE=$ENV_FILE" \
		"CAPABILITY_FILE=$CAPABILITY_FILE" >"$MANIFEST_FILE"
}

wait_for_postgres() {
	for _ in {1..30}; do
		if psql_in_container -d "$POSTGRES_DB" -c 'SELECT 1' >/dev/null 2>&1; then
			return 0
		fi
		sleep 1
	done
	fail "disposable PostgreSQL did not become ready; inspect project $PROJECT_NAME"
}

database_url() {
	python3 - "$POSTGRES_USER" "$POSTGRES_PASSWORD" "$E2E_PORT" "$DATABASE_NAME" <<'PY'
from sys import argv
from urllib.parse import quote

user, password, port, database = argv[1:]
print(
    "postgres://{}:{}@127.0.0.1:{}/{}".format(
        quote(user, safe=""), quote(password, safe=""), port, quote(database, safe="")
    )
)
PY
}

grader_database_url() {
	python3 - "$GRADER_PASSWORD" "$E2E_PORT" "$DATABASE_NAME" <<'PY'
from sys import argv
from urllib.parse import quote

password, port, database = argv[1:]
print(
    "postgres://ple_grading_reader:{}@127.0.0.1:{}/{}".format(
        quote(password, safe=""), port, quote(database, safe="")
    )
)
PY
}

run_project_tools() {
	env DATABASE_URL="$DATABASE_URL" PLE_MIGRATION_DATABASE_URL="$DATABASE_URL" \
		cargo tools database "$@"
}

# Cargo exits successfully when a filter selects zero tests.  This acceptance
# runner names individual live contracts, so a green zero-test invocation is
# evidence of nothing and must fail closed.
run_live_cargo_test() {
	local label="$1"
	shift
	local output
	if ! output="$("$@" 2>&1)"; then
		printf '%s\n' "$output" >&2
		fail "$label cargo test command failed"
	fi
	printf '%s\n' "$output"
	if ! grep -Eq 'test result: ok\. [1-9][0-9]* passed;' <<<"$output"; then
		fail "$label selected no live tests; update its exact test target"
	fi
}

run_role_matrix() {
	local role="$1"
	echo "database baseline E2E: RLS denial matrix for $role"
	psql_in_container -d "$DATABASE_NAME" -v role="$role" -v tenant_a="$TENANT_A" -v tenant_b="$TENANT_B" <<'SQL'
BEGIN;
SET LOCAL ROLE :"role";
SELECT set_config('ple.tenant_id', :'tenant_a', true);
SELECT set_config('ple.e2e_tenant_b', :'tenant_b', true);
SELECT current_user AS current_role,
       session_user AS session_role,
       current_setting('ple.tenant_id', true) AS tenant_context;

DO $$
DECLARE
    relation_name text;
    foreign_visible boolean;
    tenant_b uuid := current_setting('ple.e2e_tenant_b')::uuid;
BEGIN
    BEGIN
        PERFORM 1 FROM public.course WHERE tenant_id = tenant_b;
        IF FOUND THEN
            RAISE EXCEPTION 'RLS leak: role % exposed the tenant-B course sentinel', current_user;
        END IF;
        RAISE NOTICE 'RLS_EXERCISED_DENIED role=% relation=public.course', current_user;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'RLS_EXERCISED_NO_SELECT_GRANT role=% relation=public.course', current_user;
    END;

    FOR relation_name IN
        SELECT format('%I.%I', namespace.nspname, relation.relname)
        FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relkind IN ('r', 'p')
          AND relation.relrowsecurity
          AND EXISTS (
              SELECT 1
              FROM pg_attribute AS attribute
              WHERE attribute.attrelid = relation.oid
                AND attribute.attname = 'tenant_id'
                AND attribute.attnum > 0
                AND NOT attribute.attisdropped
          )
        ORDER BY namespace.nspname, relation.relname
    LOOP
		IF relation_name = 'public.course' THEN
			CONTINUE;
		END IF;
        BEGIN
            EXECUTE format(
                'SELECT EXISTS (SELECT 1 FROM %s WHERE tenant_id = %L::uuid)',
                relation_name,
                tenant_b
            ) INTO foreign_visible;
            IF foreign_visible THEN
                RAISE EXCEPTION 'RLS leak: role %, relation % exposed tenant B',
                    current_user, relation_name;
            END IF;
            RAISE NOTICE 'RLS_UNEXERCISED_QUERY_DENIED role=% relation=%', current_user, relation_name;
        EXCEPTION WHEN insufficient_privilege THEN
            RAISE NOTICE 'RLS_UNEXERCISED_NO_SELECT_GRANT role=% relation=%',
                current_user, relation_name;
        END;
    END LOOP;
END $$;
ROLLBACK;
SQL
}

cd "$REPO_ROOT"
require_command podman
require_command cargo
require_command python3
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
POSTGRES_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
GRADER_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
case "$DATABASE_NAME" in
	*[!a-z0-9_]* | '') fail "internal disposable database name is invalid" ;;
esac
write_private_target

echo "database baseline E2E: starting isolated project $PROJECT_NAME on loopback port $E2E_PORT"
COMPOSE_STARTED=1
compose up -d postgres
capture_postgres_volume || fail "could not resolve the disposable PostgreSQL data volume"
wait_for_postgres

psql_in_container -d postgres -c "CREATE DATABASE $DATABASE_NAME"
DATABASE_URL="$(database_url)"
EXPECTED_MIGRATION_COUNT="$(find "$REPO_ROOT/schemas/migrations" -maxdepth 1 -type f -name '*.sql' | wc -l | tr -d ' ')"
[ "$EXPECTED_MIGRATION_COUNT" -gt 0 ] || fail "migration inventory is empty"

initial_status="$(run_project_tools status)"
printf '%s\n' "$initial_status"
[ "$(printf '%s\n' "$initial_status" | grep -c ': pending')" -eq "$EXPECTED_MIGRATION_COUNT" ] || \
	fail "empty database did not report every tracked migration as pending"

run_project_tools migrate
run_project_tools migrate
final_status="$(run_project_tools status)"
printf '%s\n' "$final_status"
[ "$(printf '%s\n' "$final_status" | grep -c ': applied')" -eq "$EXPECTED_MIGRATION_COUNT" ] || \
	fail "migrated database did not report every tracked migration as applied"
run_project_tools verify
psql_in_container -d "$DATABASE_NAME" -c \
	"ALTER ROLE ple_grading_reader PASSWORD '$GRADER_PASSWORD'"

echo "database baseline E2E: bounded SQLx serialization retry"
run_live_cargo_test "bounded SQLx serialization retry" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	postgres::connection::tests::concurrent_serialization_failure_is_retried_and_commits \
	--lib -- --ignored --exact --test-threads=1

echo "database baseline E2E: passwordless account, roster, and role separation"
run_live_cargo_test "passwordless account, roster, and role separation" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_enrollment_live \
	postgres_enrollment_capability_is_locked_unique_and_role_separated \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: derived entitlement, materialized receipt, and forced RLS"
run_live_cargo_test "derived entitlement, materialized receipt, and forced RLS" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_entitlement_membership_live \
	postgres_entitlement_membership_is_derived_materialized_and_rls_enforced \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: family-filtered concurrent worker claims"
run_live_cargo_test "family-filtered concurrent worker claims" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_worker_filter_live \
	postgres_worker_claim_filter_is_concurrent_and_leaves_reserved_work_untouched \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: ranked catalog Store cursor behavior"
run_live_cargo_test "ranked catalog Store cursor behavior" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_catalog_search_live \
	postgres_catalog_search_store_preserves_ranked_cursor_behavior \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: ranked catalog continuation snapshot visibility"
run_live_cargo_test "ranked catalog continuation snapshot visibility" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_catalog_search_live \
	postgres_catalog_search_continuation_preserves_snapshot_visibility_boundaries \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: ranked catalog operator capability"
run_live_cargo_test "ranked catalog operator capability" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_catalog_search_plan_live \
	postgres_catalog_discovery_predicates_have_index_capability_evidence \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: catalog statistics disclosure broker boundary"
run_live_cargo_test "catalog statistics disclosure broker boundary" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_catalog_disclosure_live \
	postgres_catalog_statistics_disclosure_is_brokered_and_visibility_bound \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: course appearance revision, role, and current-pointer policy"
run_live_cargo_test "course appearance revision, role, and current-pointer policy" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_course_appearance_live \
	postgres_course_appearance_is_revisioned_role_bound_and_current_only \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: mandatory course term constraints, round-trip, and RLS"
run_live_cargo_test "mandatory course term constraints, round-trip, and RLS" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_course_term_live \
	postgres_course_terms_round_trip_enforce_constraints_and_remain_tenant_isolated \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: typed public references, RLS, and immutable bylines"
run_live_cargo_test "typed public references, RLS, and immutable bylines" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_public_references_byline_live \
	postgres_public_references_and_bylines_are_normalized_authorized_and_immutable \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: normalized effective policy, provenance, and RLS"
run_live_cargo_test "normalized effective policy, provenance, and RLS" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_effective_policy_live \
	postgres_effective_policy_is_normalized_precedence_bound_and_rls_enforced \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: assignment disclosure policy, current S3 timing, and RLS"
run_live_cargo_test "assignment disclosure policy, current S3 timing, and RLS" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_disclosure_policy_live \
	postgres_assignment_disclosure_policy_is_closed_revisioned_current_and_rls_bound \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: concurrent prefetch preserves immutable submission replay"
run_live_cargo_test "concurrent prefetch preserves immutable submission replay" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_submission_replay_live \
	postgres_submission_replay_preserves_its_immutable_receipt_during_concurrent_prefetch \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: immutable private flat-question image registry"
run_live_cargo_test "immutable private flat-question image registry" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_flat_question_assets_live \
	postgres_flat_question_asset_registry_is_immutable_private_and_checksum_bound \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: activity partition pruning and bounded gradebook summaries"
psql_in_container -d "$DATABASE_NAME" < \
	"$REPO_ROOT/tests/e2e/postgres_partition_pruning.sql"

echo "database baseline E2E: QTI partial import, provenance, and tenant isolation"
run_live_cargo_test "QTI partial import, provenance, and tenant isolation" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_qti_import_live \
	postgres_qti_import_preserves_partial_results_provenance_and_rls \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: QTI profile provenance, pinning, and RLS oracle"
psql_in_container -d "$DATABASE_NAME" < \
	"$REPO_ROOT/tests/e2e/postgres_qti_provenance.sql"

echo "database baseline E2E: flat-question current grading persistence oracle"
psql_in_container -d "$DATABASE_NAME" < \
	"$REPO_ROOT/tests/e2e/postgres_flat_question_current_grading.sql"

echo "database baseline E2E: account-presentation session broker authority"
psql_in_container -d "$DATABASE_NAME" < \
	"$REPO_ROOT/tests/e2e/postgres_account_presentation_authority.sql"

echo "database baseline E2E: recognized QTI profile full authoring and grading path"
run_live_cargo_test "recognized QTI profile full authoring and grading path" env \
	PLE_TEST_DATABASE_URL="$DATABASE_URL" \
	PLE_TEST_GRADER_DATABASE_URL="$(grader_database_url)" \
	cargo test -p server_core \
	qti_profile_postgres_live::postgres_profile_upload_worker_conversion_publication_and_grading_are_complete \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: QTI-profile-to-flat atomic conversion and private provenance"
run_live_cargo_test "QTI-profile-to-flat atomic conversion and private provenance" env \
	PLE_TEST_DATABASE_URL="$DATABASE_URL" \
	PLE_TEST_GRADER_DATABASE_URL="$(grader_database_url)" \
	cargo test -p learning-data-access --features postgres \
	--test postgres_flat_import_provenance_live \
	-- --ignored --test-threads=1

echo "database baseline E2E: flat-question private grading boundary"
run_live_cargo_test "flat-question private grading boundary" env \
	PLE_TEST_DATABASE_URL="$DATABASE_URL" \
	PLE_TEST_GRADER_DATABASE_URL="$(grader_database_url)" \
	cargo test -p learning-data-access --features postgres \
	--test postgres_flat_question_live \
	postgres_flat_question_publication_preserves_private_grading_boundary \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: course-local item-analysis generation fence and privacy"
run_live_cargo_test "course-local item-analysis generation fence and privacy" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_item_analysis_live \
	postgres_item_analysis_is_current_private_and_generation_fenced \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: mixed automatic/manual generation fence"
run_live_cargo_test "mixed automatic/manual generation fence" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_manual_grading_live \
	postgres_mixed_automatic_and_manual_grading_is_generation_fenced \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: course-grade scheme, compact totals, export audit, and RLS"
run_live_cargo_test "course-grade scheme, compact totals, export audit, and RLS" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_course_grade_scheme_live \
	postgres_course_grade_scheme_is_migrated_defaulted_revisioned_bounded_and_rls_fenced \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: course-grade upgrade backfill and retention wrapper"
run_live_cargo_test "course-grade upgrade backfill and retention wrapper" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_course_grade_upgrade_retention_live \
	postgres_course_grade_upgrade_backfill_and_retention_wrapper_are_lifecycle_safe \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: assignment teaching lifecycle, policy, receipt, and RLS"
run_live_cargo_test "assignment teaching lifecycle, policy, receipt, and RLS" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_assignment_teaching_projection_live \
	postgres_assignment_teaching_projection_is_atomic_current_and_rls_bound \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: T2 teaching-operations upgrade and schema"
run_live_cargo_test "T2 teaching-operations upgrade and schema" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_teaching_operations_upgrade_live \
	-- --ignored --test-threads=1

echo "database baseline E2E: T2 course groups and current policy receipts"
run_live_cargo_test "T2 course groups and current policy receipts" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_course_group_live_oracle \
	-- --ignored --test-threads=1

echo "database baseline E2E: T2 teaching authority, approval, invitation, and concurrency"
run_live_cargo_test "T2 teaching authority, approval, invitation, and concurrency" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_teaching_authority_live \
	-- --ignored --test-threads=1

echo "database baseline E2E: seeded Sysadmin ownership is atomic and irreversible"
run_live_cargo_test "seeded Sysadmin ownership is atomic and irreversible" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_seeded_sysadmin_ownership_live \
	postgres_seeded_sysadmin_ownership_is_atomic_and_irreversible \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: T3 preview plane authorization, atomic audit, and identity-free subject"
run_live_cargo_test "T3 preview plane authorization, atomic audit, and identity-free subject" env PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_preview_plane_live \
	postgres_preview_plane_live_oracle_is_authorized_atomic_and_identity_free \
	-- --ignored --exact --test-threads=1

echo "database baseline E2E: T4 live rehearsal persistence, progression, replay, and authority"
run_live_cargo_test "T4 live rehearsal persistence, progression, replay, and authority" env \
	PLE_TEST_DATABASE_URL="$DATABASE_URL" \
	PLE_TEST_GRADER_DATABASE_URL="$(grader_database_url)" \
	cargo test -p learning-data-access --features postgres \
	--test postgres_rehearsal_migration_live \
	-- --ignored --test-threads=1

TEMP_DIR="$WORKSPACE/migration-checksum"
mkdir "$TEMP_DIR"
cp -R "$REPO_ROOT/schemas/migrations" "$TEMP_DIR/migrations"
first_migration="$(find "$TEMP_DIR/migrations" -maxdepth 1 -type f -name '*.sql' | sort | head -n 1)"
[ -n "$first_migration" ] || fail "copied migration directory is empty"
printf '\n-- disposable E2E checksum mutation\n' >> "$first_migration"
mutated_status="$(run_project_tools status --migrations-dir "$TEMP_DIR/migrations")"
printf '%s\n' "$mutated_status"
printf '%s\n' "$mutated_status" | grep -q ': modified' || \
	fail "copied mutated migration was not reported as modified"

# The sentinel is a valid tenant-B row created through the migration/admin
# connection. It makes every role's course denial a real data observation,
# while the coverage inventory below keeps empty RLS relations visibly
# unexercised instead of treating their zero-row query as a pass.
psql_in_container -d "$DATABASE_NAME" -c \
	"INSERT INTO public.course (tenant_id, course_id, title, term_start_date, term_end_date, time_zone) VALUES ('$TENANT_B'::uuid, '00000000-0000-4000-8000-0000000000c1'::uuid, 'Tenant B RLS probe', DATE '2026-08-24', DATE '2026-12-18', 'America/Chicago')"

# Catalog fixtures exercise the two non-tenant-column policies: a public key,
# a tenant-A grant, and a tenant-B-only key. Values are written as the isolated
# database owner, then read as ple_grader under tenant-A context below.
psql_in_container -d "$DATABASE_NAME" <<SQL
INSERT INTO public.problem
    (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license)
VALUES
    ('00000000-0000-4000-8000-000000000101', 'C9V2R74', '$TENANT_B'::uuid, '00000000-0000-4000-8000-000000000201', 'public', 'CC0-1.0'),
    ('00000000-0000-4000-8000-000000000102', 'H5Q8X32', '$TENANT_B'::uuid, '00000000-0000-4000-8000-000000000202', 'institution', 'CC0-1.0'),
    ('00000000-0000-4000-8000-000000000103', 'N7P4Y98', '$TENANT_B'::uuid, '00000000-0000-4000-8000-000000000203', 'institution', 'CC0-1.0');
INSERT INTO public.problem_version
    (problem_id, version_id, content_sha256, workspace_id, title, publication_scope, author_ids, public_byline)
VALUES
    ('00000000-0000-4000-8000-000000000101', '00000000-0000-4000-8000-000000000111', repeat('a', 64), '00000000-0000-4000-8000-000000000211', 'public grader probe', 'public', '["E2E"]'::jsonb, ARRAY['E2E fixture']),
    ('00000000-0000-4000-8000-000000000102', '00000000-0000-4000-8000-000000000112', repeat('b', 64), '00000000-0000-4000-8000-000000000212', 'granted grader probe', 'institution', '["E2E"]'::jsonb, ARRAY['E2E fixture']),
    ('00000000-0000-4000-8000-000000000103', '00000000-0000-4000-8000-000000000113', repeat('c', 64), '00000000-0000-4000-8000-000000000213', 'private grader probe', 'institution', '["E2E"]'::jsonb, ARRAY['E2E fixture']);
INSERT INTO public.catalog_tenant_grant (tenant_id, problem_id, version_id)
VALUES ('$TENANT_A'::uuid, '00000000-0000-4000-8000-000000000102', '00000000-0000-4000-8000-000000000112');
INSERT INTO public.answer_key (problem_id, version_id, key_payload, key_sha256)
VALUES
    ('00000000-0000-4000-8000-000000000101', '00000000-0000-4000-8000-000000000111', '{}'::jsonb, repeat('d', 64)),
    ('00000000-0000-4000-8000-000000000102', '00000000-0000-4000-8000-000000000112', '{}'::jsonb, repeat('e', 64)),
    ('00000000-0000-4000-8000-000000000103', '00000000-0000-4000-8000-000000000113', '{}'::jsonb, repeat('f', 64));
SQL

psql_in_container -d "$DATABASE_NAME" -v tenant_a="$TENANT_A" <<'SQL'
BEGIN;
SET LOCAL ROLE ple_grader;
SELECT set_config('ple.tenant_id', :'tenant_a', true);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM public.answer_key WHERE problem_id = '00000000-0000-4000-8000-000000000101') THEN
        RAISE EXCEPTION 'ple_grader cannot read a public answer key';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM public.answer_key WHERE problem_id = '00000000-0000-4000-8000-000000000102') THEN
        RAISE EXCEPTION 'ple_grader cannot read a tenant-granted answer key';
    END IF;
    IF EXISTS (SELECT 1 FROM public.answer_key WHERE problem_id = '00000000-0000-4000-8000-000000000103') THEN
        RAISE EXCEPTION 'ple_grader read an ungranted tenant-B answer key';
    END IF;
END $$;
ROLLBACK;
SQL

echo "database baseline E2E: constraint validation inventory"
psql_in_container -d "$DATABASE_NAME" -c \
	"SELECT conrelid::regclass AS relation, conname FROM pg_constraint WHERE NOT convalidated ORDER BY 1, 2"
unvalidated_count="$(psql_in_container -d "$DATABASE_NAME" -At -c 'SELECT count(*) FROM pg_constraint WHERE NOT convalidated')"
if [ "$unvalidated_count" -gt 0 ]; then
	record_failure "$unvalidated_count constraints remain NOT VALID"
fi
echo "database baseline E2E: forced-RLS inventory"
psql_in_container -d "$DATABASE_NAME" -c \
	"SELECT relname, relrowsecurity, relforcerowsecurity FROM pg_class WHERE relkind IN ('r', 'p') AND relnamespace = 'public'::regnamespace AND NOT relispartition ORDER BY relname"
echo "database baseline E2E: explicitly unprotected public tables"
psql_in_container -d "$DATABASE_NAME" -c \
	"SELECT relname FROM pg_class WHERE relkind IN ('r', 'p') AND relnamespace = 'public'::regnamespace AND NOT relispartition AND NOT relrowsecurity ORDER BY relname"
unexpected_unprotected="$(psql_in_container -d "$DATABASE_NAME" -At -c "
-- _sqlx_migrations is the migration ledger; instructor_approval is global operator-owned eligibility.
-- live_demo_install_state is global install metadata, reachable only through its execute-only broker.
-- question_statistics_aggregate is identity-free.
WITH allowed(relname) AS (
    VALUES
        ('_sqlx_migrations'),
        ('instructor_approval'),
        ('live_demo_install_state'),
        ('question_statistics_aggregate')
)
SELECT count(*)
FROM pg_class AS relation
LEFT JOIN allowed ON allowed.relname = relation.relname
WHERE relation.relkind IN ('r', 'p')
  AND relation.relnamespace = 'public'::regnamespace
  AND NOT relation.relispartition
  AND NOT relation.relrowsecurity
  AND allowed.relname IS NULL")"
if [ "$unexpected_unprotected" -gt 0 ]; then
	record_failure "$unexpected_unprotected public tables lack RLS outside the explicit global/ledger allowlist"
fi
echo "database baseline E2E: default-partition row counts"
psql_in_container -d "$DATABASE_NAME" <<'SQL'
SELECT format('SELECT %L AS partition_name, count(*) AS row_count FROM %I.%I',
              child.relname, namespace.nspname, child.relname)
FROM pg_inherits AS inheritance
JOIN pg_class AS child ON child.oid = inheritance.inhrelid
JOIN pg_namespace AS namespace ON namespace.oid = child.relnamespace
WHERE child.relname LIKE '%_default'
ORDER BY child.relname
\gexec
SQL
psql_in_container -d "$DATABASE_NAME" <<'SQL'
DO $$
DECLARE
    child record;
    row_count bigint;
BEGIN
    FOR child IN
        SELECT namespace.nspname, relation.relname
          FROM pg_inherits AS inheritance
          JOIN pg_class AS relation ON relation.oid = inheritance.inhrelid
          JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
         WHERE pg_get_expr(relation.relpartbound, relation.oid) = 'DEFAULT'
    LOOP
        EXECUTE format('SELECT count(*) FROM %I.%I', child.nspname, child.relname)
            INTO row_count;
        IF row_count <> 0 THEN
            RAISE EXCEPTION 'default partition %.% contains % rows',
                child.nspname, child.relname, row_count;
        END IF;
    END LOOP;
END $$;
SQL
deterministic_partition_count="$(psql_in_container -d "$DATABASE_NAME" -At -c "
SELECT count(*)
  FROM pg_inherits AS inheritance
  JOIN pg_class AS parent ON parent.oid = inheritance.inhparent
  JOIN pg_class AS child ON child.oid = inheritance.inhrelid
 WHERE parent.relname IN ('question_attempt', 'submission', 'record_access_log', 'audit_event')
   AND child.relname ~ '_(2026_(0[89]|1[0-2])|2027_(0[1-9]|1[0-2])|2028_0[1-9])$'")"
if [ "$deterministic_partition_count" -ne 104 ]; then

	record_failure "activity partition epoch is not the fixed 2026-08 through 2028-09 set"
fi
echo "database baseline E2E: tenant-B RLS coverage inventory"
psql_in_container -d "$DATABASE_NAME" <<'SQL'
DO $$
DECLARE
    relation_name text;
    foreign_rows bigint;
    tenant_b uuid := '00000000-0000-4000-8000-0000000000b2'::uuid;
BEGIN
    FOR relation_name IN
        SELECT format('%I.%I', namespace.nspname, relation.relname)
        FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relkind IN ('r', 'p')
          AND relation.relrowsecurity
          AND EXISTS (
              SELECT 1 FROM pg_attribute AS attribute
              WHERE attribute.attrelid = relation.oid
                AND attribute.attname = 'tenant_id'
                AND attribute.attnum > 0
                AND NOT attribute.attisdropped
          )
        ORDER BY namespace.nspname, relation.relname
    LOOP
        EXECUTE format('SELECT count(*) FROM %s WHERE tenant_id = %L::uuid', relation_name, tenant_b)
            INTO foreign_rows;
        IF foreign_rows = 0 THEN
            RAISE NOTICE 'RLS_UNEXERCISED relation=%', relation_name;
        ELSE
            RAISE NOTICE 'RLS_EXERCISED relation=% tenant_b_rows=%', relation_name, foreign_rows;
        END IF;
    END LOOP;
END $$;
SQL
echo "database baseline E2E: role and grant inventory"
psql_in_container -d "$DATABASE_NAME" -c \
	"SELECT rolname, rolcanlogin, rolsuper, rolbypassrls FROM pg_roles WHERE rolname IN ('ple_app', 'ple_student', 'ple_grader', 'ple_grading_reader', 'ple_public_asset_publisher') ORDER BY rolname"

for role in ple_app ple_student ple_grader ple_grading_reader ple_public_asset_publisher; do
	run_role_matrix "$role"
done

if [ "$GATE_FAILURES" -gt 0 ]; then
	fail "$GATE_FAILURES actionable schema inventory check(s) failed"
fi
echo "database baseline E2E: PASS ($EXPECTED_MIGRATION_COUNT tracked migrations and representative role denial)"
