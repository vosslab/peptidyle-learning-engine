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
		stop_current || status=1
	fi
	exit "$status"
}
trap cleanup EXIT

start_default() {
	"$repository_root/launchers/run_live_demo.sh" --headless
	started=1
}

stop_current() {
	local attempt
	for attempt in 1 2 3 4 5 6 7 8; do
		if "$repository_root/launchers/run_live_demo.sh" stop; then
			started=0
			return 0
		fi
		echo "Live Demo stop attempt $attempt failed; retrying" >&2
		sleep 2
	done
	echo "Live Demo stop did not release its owned resources" >&2
	python3 -m local_stack_control.disposable_stack_command diagnostics \
		--manifest "$repository_root/$manifest_path" || true
	return 1
}

start_default
bash "$repository_root/tests/e2e/e2e_live_demo_course_seed.sh" --state
python3 -m local_stack_control.disposable_stack_command replay-installation-data \
	--manifest "$manifest_path"
bash "$repository_root/tests/e2e/e2e_live_demo_course_seed.sh" --state
stop_current
echo "installation-data E2E: Live Demo provision and replay PASS"
