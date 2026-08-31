#!/usr/bin/env bash
# e2e_sd1_staged_database.sh - disposable SD1 PostgreSQL staging acceptance.
#
# The public entry point delegates the lease, private manifest, and fixed
# Compose ownership to local_stack_control.sd1_staged_database_owner. The
# private child owns only this PostgreSQL 17 oracle.

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$script_directory/../.." && pwd -P)"
readonly REPO_ROOT

if [ "${1:-}" != "--owned-child" ]; then
	cd "$REPO_ROOT"
	exec python3 -m local_stack_control.sd1_staged_database_owner
fi
shift

[ "$#" -eq 2 ] && [ "$1" = "--runtime-manifest" ] && [ "$2" = "runtime.yaml" ] || {
	echo "SD1 staged database E2E: private child requires the owner-created runtime manifest" >&2
	exit 2
}
RUNTIME_MANIFEST="$2"
WORKSPACE="$(pwd -P)"
RUNTIME_MANIFEST_PATH="$WORKSPACE/$RUNTIME_MANIFEST"
SD1_RUNTIME_MANIFEST_PATH="$WORKSPACE/sd1/runtime.yaml"
readonly RUNTIME_MANIFEST WORKSPACE RUNTIME_MANIFEST_PATH SD1_RUNTIME_MANIFEST_PATH

readonly DATABASE_NAME="ple_e2e_baseline"
readonly BOOTSTRAP_USER="ple_e2e_migrator"
readonly POSTGRES_DB="postgres"
readonly PROJECT_NAME="ple-live-demo-browser"
readonly STAGED_MIGRATION="2026082901"

compose_started=0
postgres_volume_name=""

fail() {
	echo "SD1 staged database E2E: $*" >&2
	exit 1
}

require_command() {
	command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

compose() {
	(
		cd "$REPO_ROOT"
		python3 -m local_stack_control.disposable_stack_command compose --manifest "$RUNTIME_MANIFEST_PATH" "$@"
	)
}

capture_postgres_volume() {
	local container_ids container_id volume_projects
	container_ids="$(podman ps -aq \
		--filter "label=io.podman.compose.project=$PROJECT_NAME" \
		--filter 'label=io.podman.compose.service=postgres')"
	if [ "$(printf '%s\n' "$container_ids" | sed '/^$/d' | wc -l | tr -d ' ')" -ne 1 ]; then
		fail "could not resolve exactly one labelled PostgreSQL container"
	fi
	container_id="$container_ids"
	postgres_volume_name="$(podman inspect --format \
		'{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' \
		"$container_id")"
	[ -n "$postgres_volume_name" ] || fail "PostgreSQL container has no data volume"
	podman volume inspect "$postgres_volume_name" >/dev/null 2>&1 || \
		fail "captured PostgreSQL volume is unavailable"
	volume_projects="$(podman volume inspect "$postgres_volume_name" --format \
		'{{index .Labels "io.podman.compose.project"}}|{{index .Labels "com.docker.compose.project"}}')"
	case "$volume_projects" in
		*'|containers|'* | 'containers|'* | *'|containers')
			fail "refused to claim the ordinary containers volume"
			;;
	esac
	echo "SD1 staged database E2E: captured disposable PostgreSQL volume $postgres_volume_name"
}

cleanup() {
	local status="$?"
	local cleanup_failed=0
	if [ "$compose_started" = "1" ]; then
		(
			cd "$REPO_ROOT"
			python3 -m local_stack_control.disposable_stack_command cleanup --manifest "$RUNTIME_MANIFEST_PATH"
		) || cleanup_failed=1
	fi
	if [ "$cleanup_failed" = "0" ]; then
		if [ -n "$postgres_volume_name" ] && podman volume inspect "$postgres_volume_name" >/dev/null 2>&1; then
			echo "SD1 staged database E2E: captured PostgreSQL volume survived cleanup" >&2
			cleanup_failed=1
		fi
		remaining_containers="$(podman ps -aq --filter "label=io.podman.compose.project=$PROJECT_NAME")"
		if [ -n "$remaining_containers" ]; then
			echo "SD1 staged database E2E: labelled PostgreSQL target survived cleanup" >&2
			cleanup_failed=1
		fi
	fi
	if [ "$cleanup_failed" = "1" ]; then
		echo "SD1 staged database E2E: cleanup failed; retain $WORKSPACE for inspection" >&2
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

run_staged_tool() {
	(
		cd "$WORKSPACE"
		PLE_ACCEPTANCE_RUNTIME_MANIFEST="$SD1_RUNTIME_MANIFEST_PATH" \
		cargo run --manifest-path "$REPO_ROOT/Cargo.toml" --quiet -p project-tools -- \
			database "$@" --acceptance-runtime
	)
}

expect_denied() {
	local label="$1"
	shift
	if "$@" >/dev/null 2>&1; then
		fail "$label unexpectedly succeeded"
	fi
}

assert_catalog() {
	echo "SD1 staged database E2E: exact principal, schema, ACL, and membership catalog"
	psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" <<'SQL'
DO $$
DECLARE
	role_name text;
	owner_name text;
	role_count integer;
	membership_count integer;
	unexpected_membership_detail text;
	column_count integer;
BEGIN
	IF current_database() <> 'ple_e2e_baseline' THEN
		RAISE EXCEPTION 'oracle connected to an unexpected database';
	END IF;
	IF (SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname = current_database()) <> 'ple_database_owner' THEN
		RAISE EXCEPTION 'database owner is not ple_database_owner';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_roles WHERE rolname = 'ple_migrator' AND rolcanlogin
		AND NOT rolsuper AND NOT rolcreatedb AND rolcreaterole
		AND NOT rolreplication AND NOT rolbypassrls AND NOT rolinherit
		AND rolconnlimit = 2
	) THEN
		RAISE EXCEPTION 'ple_migrator attributes are not exact';
	END IF;
	FOREACH role_name IN ARRAY ARRAY[
		'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
		'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student'
	] LOOP
		IF NOT EXISTS (
			SELECT 1 FROM pg_roles WHERE rolname = role_name AND NOT rolcanlogin
			AND NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole
			AND NOT rolreplication AND NOT rolbypassrls AND NOT rolinherit
			AND rolconnlimit = -1
		) THEN
			RAISE EXCEPTION 'reserved role % has incorrect attributes', role_name;
		END IF;
	END LOOP;
	SELECT count(*)::integer INTO role_count
	FROM pg_roles
	WHERE rolname = ANY (ARRAY[
		'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
		'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student'
	]);
	IF role_count <> 8 THEN
		RAISE EXCEPTION 'reserved role set is incomplete or duplicated';
	END IF;
	SELECT count(*)::integer INTO membership_count
	FROM pg_auth_members membership
	JOIN pg_roles member ON member.oid = membership.member
	JOIN pg_roles parent ON parent.oid = membership.roleid
	WHERE member.rolname = 'ple_migrator'
	AND parent.rolname = ANY (ARRAY[
		'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
		'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student'
	]);
	IF membership_count <> 12 THEN
		RAISE EXCEPTION 'unexpected membership count: %', membership_count;
	END IF;
	SELECT string_agg(
		format(
			'%s -> %s (grantor=%s, inherit=%s, set=%s, admin=%s)',
			member.rolname,
			parent.rolname,
			grantor.rolname,
			membership.inherit_option,
			membership.set_option,
			membership.admin_option
		),
		'; ' ORDER BY parent.rolname, member.rolname
	) INTO unexpected_membership_detail
	FROM pg_auth_members membership
	JOIN pg_roles member ON member.oid = membership.member
	JOIN pg_roles parent ON parent.oid = membership.roleid
	JOIN pg_roles grantor ON grantor.oid = membership.grantor
	WHERE (
		member.rolname = ANY (ARRAY[
			'ple_migrator', 'ple_database_owner', 'ple_data_owner',
			'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app',
			'ple_auth', 'ple_student'
		])
		OR parent.rolname = ANY (ARRAY[
			'ple_migrator', 'ple_database_owner', 'ple_data_owner',
			'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app',
			'ple_auth', 'ple_student'
		])
	)
	AND NOT (
		member.rolname = 'ple_migrator'
		AND parent.rolname = ANY (ARRAY[
			'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
			'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student'
		])
		AND grantor.rolname = 'ple_e2e_migrator'
		AND NOT membership.inherit_option
		AND NOT membership.set_option AND membership.admin_option
	)
	AND NOT (
		member.rolname = 'ple_migrator'
		AND parent.rolname = 'ple_database_owner'
		AND grantor.rolname = 'ple_e2e_migrator'
		AND NOT membership.inherit_option AND membership.set_option AND NOT membership.admin_option
	)
	AND NOT (
		member.rolname = 'ple_migrator'
		AND parent.rolname = ANY (ARRAY[
			'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
			'ple_api_owner'
		])
		AND grantor.rolname = 'ple_migrator'
		AND NOT membership.inherit_option AND membership.set_option AND NOT membership.admin_option
	);
	IF unexpected_membership_detail IS NOT NULL THEN
		RAISE EXCEPTION 'membership graph has an unexpected edge, grantor, or option: %', unexpected_membership_detail;
	END IF;
	FOREACH owner_name IN ARRAY ARRAY[
		'ple_data', 'ple_private', 'ple_audit', 'ple_api'
	] LOOP
		IF NOT EXISTS (
			SELECT 1 FROM pg_namespace namespace
			JOIN pg_roles owner_role ON owner_role.oid = namespace.nspowner
			WHERE namespace.nspname = owner_name
			AND owner_role.rolname = owner_name || '_owner'
		) THEN
			RAISE EXCEPTION 'schema % has an unexpected owner', owner_name;
		END IF;
	END LOOP;
	IF NOT EXISTS (
		SELECT 1 FROM pg_class relation
		JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
		JOIN pg_roles owner_role ON owner_role.oid = relation.relowner
		WHERE namespace.nspname = 'public' AND relation.relname = '_sqlx_migrations'
		AND owner_role.rolname = 'ple_migrator'
	) THEN
		RAISE EXCEPTION 'SQLx migration ledger has an unexpected owner';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_class relation
		JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
		JOIN pg_roles owner_role ON owner_role.oid = relation.relowner
		WHERE namespace.nspname = 'ple_api' AND relation.relname = 'ple_migration_state'
		AND owner_role.rolname = 'ple_api_owner'
	) THEN
		RAISE EXCEPTION 'migration-state projection has an unexpected owner';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_namespace namespace
		CROSS JOIN LATERAL aclexplode(
			COALESCE(namespace.nspacl, acldefault('n', namespace.nspowner))
		) privilege
		WHERE namespace.nspname = ANY (ARRAY['public', 'ple_data', 'ple_private', 'ple_audit', 'ple_api'])
		AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'PUBLIC retains privilege on a PLE schema';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_database database_record
		CROSS JOIN LATERAL aclexplode(
			COALESCE(database_record.datacl, acldefault('c', database_record.datdba))
		) privilege
		WHERE database_record.datname = current_database() AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'PUBLIC retains target database privilege';
	END IF;
	IF NOT has_schema_privilege('ple_migrator', 'public', 'USAGE')
		OR NOT has_schema_privilege('ple_api_owner', 'public', 'USAGE')
		OR has_schema_privilege('ple_migrator', 'public', 'CREATE')
		OR has_schema_privilege('ple_api_owner', 'public', 'CREATE') THEN
		RAISE EXCEPTION 'migration or API owner public-schema privileges are not exact';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_roles owner_role
		CROSS JOIN (VALUES ('n'::"char"), ('r'::"char"), ('S'::"char"), ('f'::"char"), ('T'::"char")) object_kind(kind)
		LEFT JOIN pg_default_acl defaults
			ON defaults.defaclrole = owner_role.oid
			AND defaults.defaclobjtype = object_kind.kind
		CROSS JOIN LATERAL aclexplode(
			COALESCE(defaults.defaclacl, acldefault(object_kind.kind, owner_role.oid))
		) privilege
		WHERE owner_role.rolname = ANY (ARRAY[
			'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner', 'ple_api_owner'
		])
		AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'one or more owner default ACLs grant a privilege to PUBLIC';
	END IF;
	SELECT count(*)::integer INTO column_count
	FROM information_schema.columns
	WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state';
	IF column_count <> 3
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'version')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'success')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'checksum') THEN
		RAISE EXCEPTION 'migration-state projection does not have the exact safe columns';
	END IF;
	IF has_table_privilege('ple_app', 'public._sqlx_migrations', 'SELECT')
		OR NOT has_schema_privilege('ple_app', 'ple_api', 'USAGE')
		OR NOT has_table_privilege('ple_app', 'ple_api.ple_migration_state', 'SELECT') THEN
		RAISE EXCEPTION 'ple_app direct/projection privileges are not exact';
	END IF;
	SELECT count(*)::integer INTO column_count
	FROM information_schema.columns
	WHERE table_schema = 'ple_data' AND table_name = 'student_record';
	IF column_count <> 4
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'student_record_id')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'course_id')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'student_account_id')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'created_at')
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'membership_id')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_data.student_record'::regclass AND conname = 'student_record_account_course_is_unique')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_data.student_record'::regclass AND conname = 'student_record_course_reference_is_unique')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_data.course_membership'::regclass AND conname = 'course_membership_student_record_presence')
		OR NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ple_data.course_membership'::regclass AND tgname = 'course_membership_binds_exact_student_record' AND NOT tgisinternal) THEN
		RAISE EXCEPTION 'Student Record ownership is not exact and stable across membership episodes';
	END IF;
	IF NOT has_table_privilege('ple_api_owner', 'public._sqlx_migrations', 'SELECT') THEN
		RAISE EXCEPTION 'ple_api_owner cannot read the migration ledger for its projection';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_constraint
		WHERE conrelid = 'ple_private.assignment_attempt'::regclass
		AND conname = 'assignment_attempt_revision_belongs_to_assignment'
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.assignment_attempt'::regclass
		AND tgname = 'assignment_attempt_requires_published_revision' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM information_schema.columns
		WHERE table_schema = 'ple_private' AND table_name = 'issued_question'
		AND column_name = 'statistics_eligible' AND is_nullable = 'NO'
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_data.assignment_revision'::regclass
		AND tgname = 'assignment_revision_is_immutable' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.question_submission'::regclass
		AND tgname = 'question_submission_is_immutable' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.assignment_submission'::regclass
		AND tgname = 'assignment_submission_is_immutable' AND NOT tgisinternal
	) THEN
		RAISE EXCEPTION 'issued work does not retain its exact pin and Question Statistics Eligibility, or accepted submission evidence is not immutable';
	END IF;
	IF to_regclass('ple_data.course_schedule_revision') IS NULL
		OR (SELECT count(*) FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'course_schedule_revision') <> 7
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'course_instance'
			AND column_name = 'delivery_time_zone'
		) OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'assignment'
			AND column_name IN ('available_at', 'due_at', 'closes_at', 'local_override')
		) OR NOT EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
			AND column_name = 'course_schedule_revision_id' AND is_nullable = 'NO'
		) OR NOT EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
			AND column_name = 'assignment_lifecycle' AND is_nullable = 'NO'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.assignment_revision'::regclass
			AND conname = 'assignment_revision_course_matches_assignment'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.assignment_revision'::regclass
			AND conname = 'assignment_revision_schedule_matches_course'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.assignment_revision'::regclass
			AND conname = 'assignment_revision_schedule_is_ordered'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_schedule_revision'::regclass
			AND tgname = 'course_schedule_revision_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_class
			JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
			WHERE pg_namespace.nspname = 'ple_data'
			AND pg_class.relname = 'published_assignment_revision_availability_idx'
		) THEN
		RAISE EXCEPTION 'Course Schedule Revision and Assignment Revision do not own the exact durable delivery schedule';
	END IF;
	IF EXISTS (
		SELECT 1 FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
		AND column_name = 'course_delivery_settings'
	) OR NOT EXISTS (
		SELECT 1 FROM pg_constraint
		WHERE conrelid = 'ple_data.assignment_revision'::regclass
		AND conname = 'assignment_revision_title_is_valid'
	) OR (SELECT count(*) FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
		AND column_name IN (
			'assignment_title', 'assignment_lifecycle', 'assignment_instructions', 'late_work_rule',
			'assignment_deadline_rule', 'assignment_completion_rule',
			'assignment_attempt_grade_rule', 'assignment_attempt_continuation_rule',
			'question_variation_rule', 'assignment_attempt_resume_rule',
			'assignment_question_display_rule', 'assignment_navigation_rule',
			'assignment_question_order_rule'
		) AND is_nullable = 'NO') <> 13 OR (SELECT count(*) FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
		AND column_name IN (
			'assignment_attempt_time_limit_seconds', 'attempt_limit',
			'assignment_completion_score_threshold', 'max_additional_assignment_attempts'
		)) <> 4 THEN
		RAISE EXCEPTION 'Assignment Revision Definition is not stored as explicit immutable delivery fields';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.assignment_grade_calculation'::regclass
		AND tgname = 'assignment_grade_calculation_is_immutable' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM pg_constraint
		WHERE conrelid = 'ple_private.assignment_grade'::regclass
		AND conname = 'assignment_grade_selected_calculation_matches'
	) THEN
		RAISE EXCEPTION 'Gradebook does not preserve immutable calculations and an exact selected grade';
	END IF;
	IF to_regclass('ple_private.grading_result') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.grading_result'::regclass
			AND conname = 'grading_result_submission_matches_attempt'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.grading_result'::regclass
			AND conname = 'grading_result_matches_operation_submission'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_audit.automated_grading_receipt'::regclass
			AND conname = 'automated_grading_receipt_matches_result_operation'
		) THEN
		RAISE EXCEPTION 'Grading Result does not bind one Question Submission, automated operation, and receipt';
	END IF;
	IF to_regclass('ple_data.question_publication_event') IS NULL
		OR to_regclass('ple_data.question_version_availability_event') IS NULL
		OR to_regclass('ple_data.published_question_lifecycle_event') IS NOT NULL
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'published_question_version'
			AND column_name = 'lifecycle'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.question_publication_event'::regclass
			AND conname = 'question_publication_event_version_is_unique'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.question_version_availability_event'::regclass
			AND conname = 'question_version_availability_event_kind_is_unique'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.question_version_availability_event'::regclass
			AND tgname = 'question_version_availability_event_has_valid_transition' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Question publication and availability evidence remains conflated';
	END IF;
	IF to_regclass('ple_private.account_state_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.account'::regclass
			AND tgname = 'account_creation_records_active_state' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.account_state_event'::regclass
			AND tgname = 'account_restriction_revokes_sessions' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Account State does not govern authenticated-session access';
	END IF;
	IF to_regclass('ple_private.instructor_approval_event') IS NULL
		OR to_regclass('ple_private.instructor_approval') IS NOT NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.instructor_approval_event'::regclass
			AND tgname = 'instructor_approval_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Instructor Approval does not retain immutable approval and revocation evidence';
	END IF;
	IF to_regclass('ple_data.course_membership_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_membership'::regclass
			AND tgname = 'course_membership_creation_records_started_event' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_membership_event'::regclass
			AND tgname = 'course_membership_event_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_membership_event'::regclass
			AND tgname = 'course_membership_event_has_valid_transition' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Membership does not preserve immutable state history';
	END IF;
	IF to_regclass('ple_private.authoring_workspace_collaborator') IS NOT NULL
		OR to_regclass('ple_private.authoring_workspace_collaborator_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.authoring_workspace_collaborator_event'::regclass
			AND tgname = 'authoring_workspace_collaborator_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.authoring_workspace_collaborator_event'::regclass
			AND tgname = 'authoring_workspace_collaborator_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Workspace Collaborator does not preserve immutable start and end evidence';
	END IF;
	IF to_regclass('ple_private.course_observer_relationship') IS NOT NULL
		OR to_regclass('ple_private.course_observer_relationship_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_observer_relationship_event'::regclass
			AND tgname = 'course_observer_relationship_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_observer_relationship_event'::regclass
			AND tgname = 'course_observer_relationship_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Observer does not preserve immutable relationship evidence';
	END IF;
	IF to_regclass('ple_data.question_stewardship_event') IS NOT NULL
		OR to_regclass('ple_data.question_change_proposal_revision') IS NULL
		OR to_regclass('ple_data.question_change_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.question_change_proposal_revision'::regclass
			AND tgname = 'question_change_proposal_revision_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.question_change_event'::regclass
			AND tgname = 'question_change_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.question_change_event'::regclass
			AND tgname = 'question_change_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Question Change Proposals do not preserve immutable revisions and lifecycle evidence';
	END IF;
	IF to_regclass('ple_private.course_invitation_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.course_invitation_event'::regclass
			AND conname = 'course_invitation_event_invitation_id_key'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_invitation_event'::regclass
			AND tgname = 'course_invitation_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_invitation_event'::regclass
			AND tgname = 'course_invitation_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Invitation does not retain one valid immutable terminal Event';
	END IF;
	IF to_regclass('ple_data.blueprint_publication_event') IS NULL
		OR to_regclass('ple_data.blueprint_collaborator_event') IS NULL
		OR to_regclass('ple_data.blueprint_revision_availability_event') IS NULL
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'blueprint_course'
			AND column_name = 'archived_at'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.blueprint_publication_event'::regclass
			AND conname = 'blueprint_publication_event_revision_is_unique'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_collaborator_event'::regclass
			AND tgname = 'blueprint_collaborator_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_collaborator_event'::regclass
			AND tgname = 'blueprint_collaborator_event_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_revision_availability_event'::regclass
			AND tgname = 'blueprint_revision_availability_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_revision_availability_event'::regclass
			AND tgname = 'blueprint_revision_availability_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Blueprint publication, availability, and Draft Blueprint Revision collaboration evidence is incomplete';
	END IF;
	IF to_regclass('ple_data.course_origin') IS NULL
		OR to_regclass('ple_data.course_instance_blueprint_adoption') IS NOT NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_origin'::regclass
			AND tgname = 'course_origin_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Origin is not retained as immutable exact source evidence';
	END IF;
	IF to_regclass('ple_audit.forced_question_correction_assignment_target') IS NULL
		OR to_regclass('ple_audit.forced_question_correction_attempt_target') IS NULL
		OR to_regclass('ple_audit.forced_question_correction_issued_question_target') IS NULL
		OR to_regclass('ple_audit.forced_question_correction_grade_target') IS NULL
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'forced_question_correction'
			AND column_name = 'remediation'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_assignment_target'::regclass
			AND tgname = 'forced_question_correction_assignment_target_is_immutable' AND NOT tgisinternal
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_attempt_target'::regclass
			AND tgname = 'forced_question_correction_attempt_target_is_immutable' AND NOT tgisinternal
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_issued_question_target'::regclass
			AND tgname = 'forced_question_correction_issued_question_target_is_immutable' AND NOT tgisinternal
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_grade_target'::regclass
			AND tgname = 'forced_question_correction_grade_target_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Forced Question Correction Manifest lacks immutable exact teaching targets';
	END IF;
	IF to_regclass('ple_audit.assignment_grade_event') IS NULL
		OR to_regclass('ple_audit.grade_control_event') IS NOT NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.assignment_grade_event'::regclass
			AND tgname = 'assignment_grade_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Assignment Grade Event is not immutable exact calculation evidence';
	END IF;
	IF EXISTS (
		SELECT 1 FROM pg_roles capability
		WHERE capability.rolname = ANY (ARRAY['ple_app', 'ple_auth', 'ple_student'])
		AND (has_schema_privilege(capability.rolname, 'ple_data', 'USAGE')
			OR has_schema_privilege(capability.rolname, 'ple_private', 'USAGE')
			OR has_schema_privilege(capability.rolname, 'ple_audit', 'USAGE'))
	) THEN
		RAISE EXCEPTION 'capability role has ambient data-schema usage';
	END IF;
	END $$;
SQL
}

legacy_assert_catalog() {
	echo "SD1 staged database E2E: exact principal, schema, ACL, and membership catalog"
	psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" <<'SQL'
DO $$
DECLARE
	role_name text;
	owner_name text;
	role_count integer;
	membership_count integer;
	default_acl_count integer;
	column_count integer;
BEGIN
	IF current_database() <> 'ple_e2e_baseline' THEN
		RAISE EXCEPTION 'oracle connected to an unexpected database';
	END IF;
	IF (SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname = current_database()) <> 'ple_database_owner' THEN
		RAISE EXCEPTION 'database owner is not ple_database_owner';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_roles WHERE rolname = 'ple_migrator' AND rolcanlogin
		AND NOT rolsuper AND NOT rolcreatedb AND rolcreaterole
		AND NOT rolreplication AND NOT rolbypassrls AND NOT rolinherit
		AND rolconnlimit = 2
	) THEN
		RAISE EXCEPTION 'ple_migrator attributes are not exact';
	END IF;
	FOREACH role_name IN ARRAY ARRAY[
		'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
		'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student'
	] LOOP
		IF NOT EXISTS (
			SELECT 1 FROM pg_roles WHERE rolname = role_name AND NOT rolcanlogin
			AND NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole
			AND NOT rolreplication AND NOT rolbypassrls AND NOT rolinherit
			AND rolconnlimit = -1
		) THEN
			RAISE EXCEPTION 'reserved role % has incorrect attributes', role_name;
		END IF;
	END LOOP;
	SELECT count(*)::integer INTO role_count
	FROM pg_roles
	WHERE rolname = ANY (ARRAY[
		'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
		'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student'
	]);
	IF role_count <> 8 THEN
		RAISE EXCEPTION 'reserved role set is incomplete or duplicated';
	END IF;
	SELECT count(*)::integer INTO membership_count FROM pg_auth_members;
	IF membership_count <> 12 THEN
		RAISE EXCEPTION 'unexpected membership count: %', membership_count;
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_auth_members membership
		JOIN pg_roles member ON member.oid = membership.member
		JOIN pg_roles parent ON parent.oid = membership.roleid
		JOIN pg_roles grantor ON grantor.oid = membership.grantor
		WHERE NOT (
			member.rolname = 'ple_migrator'
			AND parent.rolname = ANY (ARRAY[
				'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
				'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student'
			])
			AND grantor.rolname = 'ple_e2e_migrator'
			AND NOT membership.inherit_option AND NOT membership.set_option AND membership.admin_option
		)
		AND NOT (
			parent.rolname = 'ple_migrator'
			AND member.rolname = ANY (ARRAY[
				'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
				'ple_api_owner'
			])
			AND grantor.rolname = 'ple_migrator'
			AND NOT membership.inherit_option AND membership.set_option AND NOT membership.admin_option
		)
	) THEN
		RAISE EXCEPTION 'membership graph has an unexpected edge, grantor, or option';
	END IF;
	FOREACH owner_name IN ARRAY ARRAY[
		'ple_data', 'ple_private', 'ple_audit', 'ple_api'
	] LOOP
		IF NOT EXISTS (
			SELECT 1 FROM pg_namespace namespace
			JOIN pg_roles owner_role ON owner_role.oid = namespace.nspowner
			WHERE namespace.nspname = owner_name
			AND owner_role.rolname = owner_name || '_owner'
		) THEN
			RAISE EXCEPTION 'schema % has an unexpected owner', owner_name;
		END IF;
	END LOOP;
	IF has_schema_privilege('public', 'public', 'USAGE') OR has_schema_privilege('public', 'public', 'CREATE') THEN
		RAISE EXCEPTION 'PUBLIC retains privilege on schema public';
	END IF;
	IF has_database_privilege('public', current_database(), 'CONNECT')
		OR has_database_privilege('public', current_database(), 'CREATE')
		OR has_database_privilege('public', current_database(), 'TEMPORARY') THEN
		RAISE EXCEPTION 'PUBLIC retains target database privilege';
	END IF;
	FOREACH owner_name IN ARRAY ARRAY[
		'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner', 'ple_api_owner'
	] LOOP
		SELECT count(*)::integer INTO default_acl_count
		FROM pg_default_acl defaults
		JOIN pg_roles owner_role ON owner_role.oid = defaults.defaclrole
		WHERE owner_role.rolname = owner_name
		AND defaults.defaclobjtype IN ('n', 'r', 'S', 'f', 'T')
		AND NOT EXISTS (
			SELECT 1 FROM aclexplode(defaults.defaclacl) privilege
			JOIN pg_roles grantee ON grantee.oid = privilege.grantee
			WHERE grantee.rolname = 'PUBLIC'
		);
		IF default_acl_count <> 5 THEN
			RAISE EXCEPTION 'default ACL closure is incomplete for %: %', owner_name, default_acl_count;
		END IF;
	END LOOP;
	SELECT count(*)::integer INTO column_count
	FROM information_schema.columns
	WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state';
	IF column_count <> 3
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'version')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'success')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'checksum') THEN
		RAISE EXCEPTION 'migration-state projection does not have the exact safe columns';
	END IF;
	IF has_table_privilege('ple_app', 'public._sqlx_migrations', 'SELECT')
		OR NOT has_schema_privilege('ple_app', 'ple_api', 'USAGE')
		OR NOT has_table_privilege('ple_app', 'ple_api.ple_migration_state', 'SELECT') THEN
		RAISE EXCEPTION 'ple_app direct/projection privileges are not exact';
	END IF;
	IF EXISTS (
		SELECT 1 FROM pg_roles capability
		WHERE capability.rolname = ANY (ARRAY['ple_app', 'ple_auth', 'ple_student'])
		AND (has_schema_privilege(capability.rolname, 'ple_data', 'USAGE')
			OR has_schema_privilege(capability.rolname, 'ple_private', 'USAGE')
			OR has_schema_privilege(capability.rolname, 'ple_audit', 'USAGE'))
	) THEN
		RAISE EXCEPTION 'capability role has ambient data-schema usage';
	END IF;
END $$;
SQL
}

assert_restricted_logins() {
	echo "SD1 staged database E2E: restricted LOGIN allow/deny probes"
	psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" <<'SQL'
CREATE ROLE ple_sd1_app_probe LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_sd1_auth_probe LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_sd1_student_probe LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
GRANT ple_app TO ple_sd1_app_probe WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT ple_auth TO ple_sd1_auth_probe WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT ple_student TO ple_sd1_student_probe WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT CONNECT ON DATABASE ple_e2e_baseline TO ple_sd1_app_probe, ple_sd1_auth_probe,
    ple_sd1_student_probe;
SQL
	psql_in_container ple_sd1_app_probe -d "$DATABASE_NAME" -c \
		'SET ROLE ple_app; SELECT version, success, checksum FROM ple_api.ple_migration_state LIMIT 1' >/dev/null
	expect_denied "ple_app direct migration-ledger read" psql_in_container ple_sd1_app_probe -d "$DATABASE_NAME" -c \
		'SET ROLE ple_app; SELECT 1 FROM public._sqlx_migrations LIMIT 1'
	expect_denied "ple_app data-schema usage" psql_in_container ple_sd1_app_probe -d "$DATABASE_NAME" -c \
		'SET ROLE ple_app; SELECT 1 FROM ple_data.sd1_probe'
	local probe capability
	for probe in ple_sd1_auth_probe ple_sd1_student_probe; do
		case "$probe" in
			ple_sd1_auth_probe) capability="ple_auth" ;;
			ple_sd1_student_probe) capability="ple_student" ;;
			*) fail "unknown restricted SD1 probe $probe" ;;
		esac
		expect_denied "$probe API read" psql_in_container "$probe" -d "$DATABASE_NAME" -c \
			"SET ROLE $capability; SELECT 1 FROM ple_api.ple_migration_state LIMIT 1"
		expect_denied "$probe data-schema usage" psql_in_container "$probe" -d "$DATABASE_NAME" -c \
			"SET ROLE $capability; SELECT 1 FROM ple_data.sd1_probe"
		expect_denied "$probe owner SET ROLE" psql_in_container "$probe" -d "$DATABASE_NAME" -c \
			'SET ROLE ple_data_owner'
		expect_denied "$probe object creation" psql_in_container "$probe" -d "$DATABASE_NAME" -c \
			"SET ROLE $capability; CREATE TABLE ple_api.sd1_probe_denied (id integer)"
	done
}

cd "$REPO_ROOT"
require_command podman
require_command cargo
require_command python3
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
export PLE_ACCEPTANCE_RUNTIME_MANIFEST="$RUNTIME_MANIFEST_PATH"

echo "SD1 staged database E2E: starting isolated PostgreSQL 17 project $PROJECT_NAME"
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
version_major="$(psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB" -At -c \
	"SELECT split_part(current_setting('server_version'), '.', 1)")"
[ "$version_major" = "17" ] || fail "disposable PostgreSQL is not major 17 (got $version_major)"

# The official image bootstrap login is used only to create the canonical
# migration login and empty target. The staged command then installs that
# role for SQLx; no bootstrap secret is copied into the child or SQL text.
psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB" <<'SQL'
CREATE ROLE ple_database_owner NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_migrator LOGIN NOINHERIT NOSUPERUSER NOCREATEDB CREATEROLE
    NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 2;
GRANT ple_database_owner TO ple_migrator WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
CREATE DATABASE ple_e2e_baseline OWNER ple_database_owner;
SQL
# The password remains in the mode-0600 runtime workspace. This dedicated
# emitter keeps it out of shell arguments and ordinary process environments.
(
	cd "$REPO_ROOT"
	python3 -m local_stack_control.runtime_manifest --emit-sd1-staged-bootstrap "$WORKSPACE"
) | psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB"
psql_in_container "$BOOTSTRAP_USER" -d "$POSTGRES_DB" -c \
	"REVOKE CONNECT, CREATE, TEMPORARY ON DATABASE $DATABASE_NAME FROM PUBLIC; GRANT CONNECT ON DATABASE $DATABASE_NAME TO ple_migrator"
psql_in_container "$BOOTSTRAP_USER" -d "$DATABASE_NAME" -c \
	'REVOKE ALL ON SCHEMA public FROM PUBLIC; GRANT CREATE, USAGE ON SCHEMA public TO ple_migrator; GRANT USAGE ON SCHEMA pg_catalog TO ple_migrator'

echo "SD1 staged database E2E: staged status is pending before apply"
initial_status="$(run_staged_tool sd1-staged-status)"
printf '%s\n' "$initial_status"
printf '%s\n' "$initial_status" | grep -Eq "$STAGED_MIGRATION.*pending" || \
	fail "staged status did not report $STAGED_MIGRATION as pending"

echo "SD1 staged database E2E: fresh apply and second-run no-op"
run_staged_tool sd1-staged-migrate
second_apply="$(run_staged_tool sd1-staged-migrate)"
printf '%s\n' "$second_apply"
printf '%s\n' "$second_apply" | grep -Eiq 'no.?op|already applied|complete' || \
	fail "second staged apply did not report a no-op-compatible result"
run_staged_tool sd1-staged-verify

assert_catalog
assert_restricted_logins

echo "SD1 staged database E2E: PASS (fresh apply, no-op, PostgreSQL 17 catalog, restricted probes)"
