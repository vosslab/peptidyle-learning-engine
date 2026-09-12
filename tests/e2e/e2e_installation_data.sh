#!/usr/bin/env bash
# Connected acceptance for the explicit ordinary installation-data phase.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly manifest_path="local_stack_state/live_demo_browser/workspace/disposable.manifest"

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

started=0

cleanup() {
	local status="$?"
	if [ "$started" = "1" ]; then
		"$repository_root/launchers/run_live_demo.sh" stop >/dev/null 2>&1 || status=1
	fi
	exit "$status"
}
trap cleanup EXIT

start_default() {
	"$repository_root/launchers/run_live_demo.sh" --headless
	started=1
}

stop_current() {
	"$repository_root/launchers/run_live_demo.sh" stop
	started=0
}

start_without_live_demo() {
	python3 "$repository_root/local_stack.py" start --headless --without-live-demo
	started=1
}

start_default
bash "$repository_root/tests/e2e/e2e_live_demo_course_seed.sh" --state
python3 -m local_stack_control.disposable_stack_command replay-installation-data \
	--manifest "$manifest_path"
bash "$repository_root/tests/e2e/e2e_live_demo_course_seed.sh" --state
stop_current

start_without_live_demo
python3 -m local_stack_control.disposable_stack_command assert-live-demo-absent \
	--manifest "$manifest_path"

# The non-enumerating boundary remains public-facing even when the database is
# otherwise empty.  The fixed reference is never an ambient caller input.
source "$repository_root/tests/e2e/e2e_live_demo_assignment_helpers.sh"
assert_concealed "$(request '/api/course-instances/C-1')"

echo "installation-data E2E: PASS"
