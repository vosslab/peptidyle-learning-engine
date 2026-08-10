#!/usr/bin/env bash
# e2e_course_appearance.sh - isolated PostgreSQL and MinIO course-appearance oracle.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT
readonly COMPOSE_FILE="$REPO_ROOT/tests/e2e/compose.course-appearance.yaml"
readonly PROJECT_NAME="ple_course_appearance_$$"
readonly POSTGRES_USER="ple_course_appearance"
readonly POSTGRES_DB="ple_course_appearance"
readonly POSTGRES_PORT="${PLE_COURSE_APPEARANCE_POSTGRES_PORT:-$((49000 + RANDOM % 500))}"
readonly MINIO_PORT="${PLE_COURSE_APPEARANCE_MINIO_PORT:-$((50000 + RANDOM % 500))}"

COMPOSE_STARTED=0

fail() {
	echo "course appearance E2E: $*" >&2
	exit 1
}

require_command() {
	command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

compose() {
	env POSTGRES_USER="$POSTGRES_USER" POSTGRES_PASSWORD="$POSTGRES_PASSWORD" \
		POSTGRES_DB="$POSTGRES_DB" PLE_POSTGRES_HOST_PORT="$POSTGRES_PORT" \
		MINIO_ROOT_USER="$MINIO_ROOT_USER" MINIO_ROOT_PASSWORD="$MINIO_ROOT_PASSWORD" \
		PLE_MINIO_API_HOST_PORT="$MINIO_PORT" podman-compose \
		-p "$PROJECT_NAME" -f "$COMPOSE_FILE" "$@"
}

cleanup() {
	local status="$?"
	if [ "${PLE_E2E_KEEP:-0}" = "1" ]; then
		echo "course appearance E2E: preserving disposable project $PROJECT_NAME"
	elif [ "$COMPOSE_STARTED" = "1" ]; then
		compose down --volumes --remove-orphans >/dev/null 2>&1 || true
	fi
	exit "$status"
}
trap cleanup EXIT

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
require_command podman-compose
require_command python3
[ -f "$COMPOSE_FILE" ] || fail "missing Compose definition: $COMPOSE_FILE"

# Repository Python commands always run through the maintained environment.
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
POSTGRES_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
MINIO_ROOT_USER="ple-course-appearance"
MINIO_ROOT_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
export POSTGRES_PASSWORD MINIO_ROOT_USER MINIO_ROOT_PASSWORD

echo "course appearance E2E: starting isolated PostgreSQL and MinIO"
compose up -d postgres minio
COMPOSE_STARTED=1
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
