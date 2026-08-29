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
compose_started=0

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
		python3 -m local_stack_control._consumer_cli compose \
			--manifest "$runtime_manifest_path" "$@"
	)
}

cleanup() {
	local status="$?"
	local cleanup_failed=0
	if [ "$compose_started" = "1" ]; then
		(
			cd "$REPO_ROOT"
			python3 -m local_stack_control._consumer_cli cleanup \
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
		if compose exec -T postgres pg_isready -U ple_e2e_migrator -d postgres \
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
compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U ple_e2e_migrator -d postgres \
	-c 'CREATE DATABASE ple_e2e_baseline'

echo "course appearance E2E: applying and verifying the accepted migration set"
run_project_tools migrate
run_project_tools verify

echo "course appearance E2E: MinIO object-store conformance"
run_live_cargo_test "MinIO object-store conformance" cargo test -p objects --features s3 \
	--test conformance minio_object_store_conforms -- --ignored --exact --test-threads=1

echo "course appearance E2E: real MinIO upload, promotion, delivery, and supersession"
run_live_cargo_test "MinIO course-appearance flow" cargo test -p server_core \
	course_appearance::tests::minio_author_atomic_flow_student_read_and_current_only_delivery_conform \
	--lib -- --ignored --exact --test-threads=1

echo "course appearance E2E: PostgreSQL claim, MinIO delete, and completion"
run_live_cargo_test "PostgreSQL and MinIO cleanup" cargo test -p server_core \
	course_appearance::tests::postgres_minio_cleanup_deletes_superseded_objects_and_preserves_current \
	--lib -- --ignored --exact --test-threads=1

echo "course appearance E2E: PASS"
