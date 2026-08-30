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
script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$script_directory/../.." || exit 1

passed=0
failed=0
failed_names=()

run_check() {
	local name="$1"
	shift
	echo "==> $name"
	if "$@"; then
		passed=$((passed + 1))
	else
		failed=$((failed + 1))
		failed_names+=("$name")
	fi
}

# A Rust export is callable from Node through the wasm-bindgen glue.
run_check wasm_bridge node tests/e2e/e2e_wasm_bridge.mjs

# The processed Wasm artifact exposes only the explicitly reviewed bridge API.
run_check wasm_export_allowlist node tests/e2e/e2e_wasm_export_allowlist.mjs

# The shipped browser build has one production-authentication composition path.
run_check browser_production_build node tests/e2e/e2e_browser_production_build.mjs

# SQLx baseline, role grants, forced RLS, and disposable migration checksum proof.
run_check database_baseline bash tests/e2e/e2e_database_baseline.sh

# PostgreSQL current-pointer state and MinIO object cleanup agree across the durable boundary.
run_check course_appearance bash tests/e2e/e2e_course_appearance.sh

# The isolated upstream WebWork renderer honors PLE's authenticated render-and-grade contract.
run_check webwork_render_rpc bash tests/e2e/e2e_webwork_render_rpc.sh

# A Student session and idempotent submission survive across two API replicas.
# A missing Podman machine is deliberately a failing BLOCKED prerequisite, not a skip.
run_check replica_restart node tests/e2e/e2e_replica_restart.mjs

echo
echo "Summary: $passed passed, $failed failed."
if [ "$failed" -gt 0 ]; then
	for name in "${failed_names[@]}"; do
		echo "  FAIL $name"
	done
	exit 1
fi
exit 0
