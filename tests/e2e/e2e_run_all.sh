#!/usr/bin/env bash
# e2e_run_all.sh - run every non-browser end-to-end check.
#
# Named with the e2e_ prefix because tests/test_test_naming_conventions.py
# requires it for every shell file under tests/e2e/, which takes precedence
# over the run_all.sh name suggested in docs/E2E_TESTS.md.
#
# These checks are deliberately outside `pytest tests/`: they need real build
# artifacts on disk, which makes them too slow for the fast lane. See
# docs/E2E_TESTS.md.
#
# Prerequisites are checked per test rather than up front, so a missing
# artifact reports which command to run rather than a generic failure.
#
# Run: bash tests/e2e/e2e_run_all.sh

set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

PASSED=0
FAILED=0
FAILED_NAMES=()

run_check() {
	local name="$1"
	shift
	echo "==> $name"
	if "$@"; then
		PASSED=$((PASSED + 1))
	else
		FAILED=$((FAILED + 1))
		FAILED_NAMES+=("$name")
	fi
}

# A Rust export is callable from Node through the wasm-bindgen glue.
run_check wasm_bridge node tests/e2e/e2e_wasm_bridge.mjs

# The processed Wasm artifact exposes only the explicitly reviewed bridge API.
run_check wasm_export_allowlist node tests/e2e/e2e_wasm_export_allowlist.mjs

# Production and local browser builds expose different authentication capabilities.
run_check browser_local_auth_build node tests/e2e/e2e_browser_local_development_build.mjs

# SQLx baseline, role grants, forced RLS, and disposable migration checksum proof.
run_check database_baseline bash tests/e2e/e2e_database_baseline.sh

# Fresh, retained, interrupted, concurrent, mixed, and regenerated live-demo baseline lifecycle.
run_check live_demo_baseline bash -c \
	'source source_me.sh && python3 tests/e2e/e2e_live_demo_baseline.py'

# A learner session and idempotent submission survive across two API replicas.
# A missing Podman machine is deliberately a failing BLOCKED prerequisite, not a skip.
run_check replica_restart node tests/e2e/e2e_replica_restart.mjs

echo
echo "Summary: $PASSED passed, $FAILED failed."
if [ "$FAILED" -gt 0 ]; then
	for name in "${FAILED_NAMES[@]}"; do
		echo "  FAIL $name"
	done
	exit 1
fi
exit 0
