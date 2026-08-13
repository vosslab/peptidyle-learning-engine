#!/usr/bin/env bash
# e2e_chapter_one_pilot.sh - isolated publication oracle for the first teaching corpus.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT
readonly COMPOSE_FILE="$REPO_ROOT/tests/e2e/compose.course-appearance.yaml"
readonly PROJECT_NAME="ple_chapter_one_pilot_$$"
readonly POSTGRES_USER="ple_chapter_one_pilot"
readonly POSTGRES_DB="ple_chapter_one_pilot"
readonly POSTGRES_PORT="${PLE_CHAPTER_ONE_POSTGRES_PORT:-$((50500 + RANDOM % 500))}"
readonly MINIO_PORT="${PLE_CHAPTER_ONE_MINIO_PORT:-$((51000 + RANDOM % 500))}"
readonly TENANT_ID="00000000-0000-0000-0000-000000000100"
readonly INSTRUCTOR_ID="00000000-0000-0000-0000-000000000101"
readonly STUDENT_ID="00000000-0000-0000-0000-000000000102"

COMPOSE_STARTED=0
TEMP_DIRECTORY=""

fail() {
	echo "Chapter 1 pilot E2E: $*" >&2
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
		echo "Chapter 1 pilot E2E: preserving disposable project $PROJECT_NAME"
	elif [ "$COMPOSE_STARTED" = "1" ]; then
		compose down --volumes --remove-orphans >/dev/null 2>&1 || true
	fi
	if [ -n "$TEMP_DIRECTORY" ] && [ -d "$TEMP_DIRECTORY" ]; then
		rm -rf -- "$TEMP_DIRECTORY"
	fi
	exit "$status"
}
trap cleanup EXIT

wait_for_service() {
	local service="$1"
	for _ in {1..30}; do
		if [ "$service" = "postgres" ]; then
			compose exec -T postgres pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
				>/dev/null 2>&1 && return 0
		else
			compose exec -T minio mc ready local >/dev/null 2>&1 && return 0
		fi
		sleep 1
	done
	fail "disposable $service did not become ready"
}

expect_query() {
	local expected="$1"
	local query="$2"
	local actual
	actual="$(compose exec -T postgres psql -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" \
		-d "$POSTGRES_DB" -Atc "$query")"
	[ "$actual" = "$expected" ] || fail "database evidence differed: expected '$expected', got '$actual'"
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
MINIO_ROOT_USER="ple-chapter-one-pilot"
MINIO_ROOT_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
export POSTGRES_PASSWORD MINIO_ROOT_USER MINIO_ROOT_PASSWORD
TEMP_DIRECTORY="$(mktemp -d)"

echo "Chapter 1 pilot E2E: validating the tracked source corpus"
cargo tools pilot-content

echo "Chapter 1 pilot E2E: starting isolated PostgreSQL and MinIO"
compose up -d postgres minio
COMPOSE_STARTED=1
wait_for_service postgres
wait_for_service minio
compose run --rm createbuckets

DATABASE_URL="postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@127.0.0.1:$POSTGRES_PORT/$POSTGRES_DB"
seed_chapters() {
	local manifest_file="$1"
	AWS_ACCESS_KEY_ID="$MINIO_ROOT_USER" AWS_SECRET_ACCESS_KEY="$MINIO_ROOT_PASSWORD" \
		cargo tools e2e-seed --chapter-one-pilot \
		--database-url "$DATABASE_URL" \
		--apply-migrations \
		--tenant "$TENANT_ID" \
		--instructor "$INSTRUCTOR_ID" \
		--student "$STUDENT_ID" \
		--s3-endpoint "http://127.0.0.1:$MINIO_PORT" \
		--s3-region "us-east-1" \
		--private-content-bucket private-content >"$manifest_file"
}

echo "Chapter 1 pilot E2E: publishing two assignments and eight immutable questions"
seed_chapters "$TEMP_DIRECTORY/first.json"
echo "Chapter 1 pilot E2E: verifying exact idempotent rerun"
seed_chapters "$TEMP_DIRECTORY/second.json"
python3 tests/e2e/e2e_chapter_one_manifest.py \
	"$TEMP_DIRECTORY/first.json" "$TEMP_DIRECTORY/second.json"

expect_query "2" "SELECT count(*) FROM course WHERE tenant_id = '$TENANT_ID';"
expect_query "2" "SELECT count(*) FROM assignment WHERE tenant_id = '$TENANT_ID';"
expect_query "8" "SELECT count(*) FROM assignment_item WHERE tenant_id = '$TENANT_ID' AND delivery_state = 'active';"
expect_query "4|4" "SELECT count(*) FILTER (WHERE backend = 'native') || '|' || count(*) FILTER (WHERE backend = 'webwork') FROM problem_version;"
expect_query "8" "SELECT count(*) FROM published_source_artifact;"
expect_query "0" "SELECT count(*) FROM problem_version WHERE version_number <> 1 OR previous_version_id IS NOT NULL;"
expect_query "2" "SELECT count(*) FROM enrollment WHERE tenant_id = '$TENANT_ID';"
expect_query "Biochemistry Chapter 1 Mastery|4
Genetics Chapter 1 Mastery|4" "SELECT assignment.title || '|' || count(assignment_item.assignment_item_id) FROM assignment JOIN assignment_item USING (tenant_id, assignment_id) WHERE assignment.tenant_id = '$TENANT_ID' GROUP BY assignment.title ORDER BY assignment.title;"

echo "Chapter 1 pilot E2E: PASS"
