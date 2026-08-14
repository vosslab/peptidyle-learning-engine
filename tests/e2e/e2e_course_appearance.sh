#!/usr/bin/env bash
# e2e_course_appearance.sh - isolated PostgreSQL and MinIO course-appearance oracle.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT
readonly POSTGRES_USER="ple_course_appearance"
readonly POSTGRES_DB="ple_course_appearance"
readonly POSTGRES_PORT="${PLE_COURSE_APPEARANCE_POSTGRES_PORT:-$((49000 + RANDOM % 500))}"
readonly MINIO_PORT="${PLE_COURSE_APPEARANCE_MINIO_PORT:-$((50000 + RANDOM % 500))}"

COMPOSE_STARTED=0
ENV_FILE=""
MANIFEST_FILE=""
CAPABILITY_FILE=""
PROJECT_NAME=""

fail() {
	echo "course appearance E2E: $*" >&2
	exit 1
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
	if [ "${PLE_E2E_KEEP:-0}" = "1" ]; then
		echo "course appearance E2E: preserving disposable project $PROJECT_NAME (manifest $MANIFEST_FILE)"
	elif [ "$COMPOSE_STARTED" = "1" ]; then
		python3 -m local_stack_control._consumer_cli cleanup --manifest "$MANIFEST_FILE" \
			|| cleanup_failed=1
	fi
	if [ "${PLE_E2E_KEEP:-0}" != "1" ] && [ "$cleanup_failed" = "0" ]; then
		[ -n "$ENV_FILE" ] && rm -f -- "$ENV_FILE"
		[ -n "$MANIFEST_FILE" ] && rm -f -- "$MANIFEST_FILE"
		[ -n "$CAPABILITY_FILE" ] && rm -f -- "$CAPABILITY_FILE"
	fi
	if [ "$cleanup_failed" = "1" ]; then
		echo "course appearance E2E: cleanup failed; inspect project $PROJECT_NAME with manifest $MANIFEST_FILE" >&2
		[ "$status" -ne 0 ] || status=1
	fi
	exit "$status"
}
trap cleanup EXIT

write_private_target() {
	local project_token capability_digest
	project_token="$(python3 -c 'import secrets; print(secrets.token_hex(12))')"
	PROJECT_NAME="ple_course_appearance_${project_token}"
	ENV_FILE="$(mktemp "${TMPDIR:-/tmp}/ple-course-appearance.XXXXXX.env")"
	MANIFEST_FILE="$(mktemp "${TMPDIR:-/tmp}/ple-course-appearance.XXXXXX.manifest")"
	CAPABILITY_FILE="$(mktemp "${TMPDIR:-/tmp}/ple-course-appearance.XXXXXX.capability")"
	capability_digest="$(python3 -c 'import hashlib, os, secrets, sys; raw = secrets.token_bytes(32); fd = os.open(sys.argv[1], os.O_WRONLY | os.O_TRUNC, 0o600); os.write(fd, raw); os.close(fd); os.chmod(sys.argv[1], 0o600); print(hashlib.sha256(raw).hexdigest())' "$CAPABILITY_FILE")"
	chmod 600 "$ENV_FILE" "$MANIFEST_FILE" "$CAPABILITY_FILE"
	printf '%s\n' \
		"POSTGRES_USER=$POSTGRES_USER" \
		"POSTGRES_PASSWORD=$POSTGRES_PASSWORD" \
		"POSTGRES_DB=$POSTGRES_DB" \
		"PLE_POSTGRES_HOST_PORT=$POSTGRES_PORT" \
		"MINIO_ROOT_USER=$MINIO_ROOT_USER" \
		"MINIO_ROOT_PASSWORD=$MINIO_ROOT_PASSWORD" \
		"PLE_MINIO_API_HOST_PORT=$MINIO_PORT" \
		"PLE_DISPOSABLE_CAPABILITY_SHA256=$capability_digest" >"$ENV_FILE"
	printf '%s\n' \
		"OWNER=course-appearance" \
		"PROJECT=$PROJECT_NAME" \
		"ENV_FILE=$ENV_FILE" \
		"CAPABILITY_FILE=$CAPABILITY_FILE" >"$MANIFEST_FILE"
}

wait_for_postgres() {
	for _ in {1..30}; do
		if compose exec -T postgres pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
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

cd "$REPO_ROOT"
require_command cargo
require_command podman
require_command python3

# Repository Python commands always run through the maintained environment.
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
POSTGRES_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
MINIO_ROOT_USER="ple-course-appearance"
MINIO_ROOT_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
export POSTGRES_PASSWORD MINIO_ROOT_USER MINIO_ROOT_PASSWORD
write_private_target

echo "course appearance E2E: starting isolated PostgreSQL and MinIO"
COMPOSE_STARTED=1
compose up -d postgres minio
wait_for_postgres
wait_for_minio
compose run --rm createbuckets

DATABASE_URL="postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@127.0.0.1:$POSTGRES_PORT/$POSTGRES_DB"
export DATABASE_URL
echo "course appearance E2E: applying and verifying the accepted migration set"
PLE_MIGRATION_DATABASE_URL="$DATABASE_URL" cargo tools database migrate
PLE_MIGRATION_DATABASE_URL="$DATABASE_URL" cargo tools database verify

echo "course appearance E2E: PostgreSQL revision, role, RLS, and current-pointer oracle"
PLE_TEST_DATABASE_URL="$DATABASE_URL" cargo test -p learning-data-access --features postgres \
	--test postgres_course_appearance_live \
	postgres_course_appearance_is_revisioned_role_bound_and_current_only \
	-- --ignored --exact --test-threads=1

echo "course appearance E2E: MinIO object-store conformance"
PLE_S3_ENDPOINT="http://127.0.0.1:$MINIO_PORT" PLE_S3_REGION="us-east-1" \
	AWS_ACCESS_KEY_ID="$MINIO_ROOT_USER" AWS_SECRET_ACCESS_KEY="$MINIO_ROOT_PASSWORD" \
	cargo test -p objects --features s3 --test conformance minio_object_store_conforms \
	-- --ignored --exact --test-threads=1

echo "course appearance E2E: combined PostgreSQL claim, MinIO delete, and completion"
PLE_TEST_DATABASE_URL="$DATABASE_URL" PLE_S3_ENDPOINT="http://127.0.0.1:$MINIO_PORT" \
	PLE_S3_REGION="us-east-1" AWS_ACCESS_KEY_ID="$MINIO_ROOT_USER" \
	AWS_SECRET_ACCESS_KEY="$MINIO_ROOT_PASSWORD" cargo test -p server_core \
	course_appearance::tests::postgres_minio_cleanup_deletes_superseded_objects_and_preserves_current \
	--lib -- --ignored --exact --test-threads=1

echo "course appearance E2E: real MinIO upload, promotion, delivery, and supersession"
PLE_S3_ENDPOINT="http://127.0.0.1:$MINIO_PORT" PLE_S3_REGION="us-east-1" \
	AWS_ACCESS_KEY_ID="$MINIO_ROOT_USER" AWS_SECRET_ACCESS_KEY="$MINIO_ROOT_PASSWORD" \
	cargo test -p server_core \
	course_appearance::tests::minio_author_atomic_flow_student_read_and_current_only_delivery_conform \
	--lib -- --ignored --exact --test-threads=1

echo "course appearance E2E: PASS"
