#!/usr/bin/env bash
# Run isolated current-source Chromium UI checks without the PLE service stack.

set -euo pipefail

usage() {
	cat <<'USAGE'
Usage: launchers/run_fast_ui_checks.sh [--headed --case <name>]

Without options, runs the established isolated headless Chromium checks. It does not
start PostgreSQL, Podman, authentication, Live Demo, or another PLE service.

  --headed --case <name>  Open exactly one registered current-source fixture in Chromium.
  -h, --help              Print this help and exit 0.
USAGE
}

launcher_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "$launcher_directory" rev-parse --show-toplevel)"
cd "$repo_root"

if [ "$#" -eq 0 ]; then
	for check in \
		tests/playwright/record_list_contracts.mjs \
		tests/playwright/provided_avatar_picker_presentation.mjs \
		tests/playwright/fast_ui_route_composition.mjs \
		tests/playwright/ribbon_shell_contract.mjs \
		tests/playwright/student_course_entry_m6_evidence.mjs; do
		node --import tsx "$check"
	done
	exit 0
fi

if [ "$#" -eq 1 ] && { [ "$1" = "-h" ] || [ "$1" = "--help" ]; }; then
	usage
	exit 0
fi

if [ "$#" -eq 3 ] && [ "$1" = "--headed" ] && [ "$2" = "--case" ]; then
	node --import tsx tests/playwright/fast_ui_headed_case.mjs --case "$3"
	exit 0
fi

usage >&2
exit 2
