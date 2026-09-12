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

echo "course appearance E2E: using the owner-prepared canonical database build"
compose_started=1
compose -- --profile course-appearance-initialization run --rm -T createbuckets

echo "course appearance E2E: MinIO object-store conformance"
run_live_cargo_test "MinIO object-store conformance" cargo test -p objects --features s3 \
	--test conformance minio_object_store_conforms -- --ignored --exact --test-threads=1

echo "course appearance E2E: typed banner object contract"
run_live_cargo_test "Course Banner PostgreSQL plus MinIO saga" cargo test \
	-p learning-data-access --features postgres --test course_banner_saga_postgres \
	course_banner_saga_is_durable_authorized_and_cross_store -- --ignored --exact --test-threads=1

echo "course appearance E2E: self-only Profile Thumbnail object contract"
run_live_cargo_test "Profile Thumbnail PostgreSQL plus MinIO saga" cargo test \
	-p learning-data-access --features postgres --test profile_thumbnail_saga_postgres \
	profile_thumbnail_saga_is_self_only_durable_and_cross_store -- --ignored --exact --test-threads=1

echo "course appearance E2E: PASS"
