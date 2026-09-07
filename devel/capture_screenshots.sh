#!/usr/bin/env bash
# capture_screenshots.sh - rebuild current PLE screenshots through seeded demo data.

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

case "$#" in
	0)
		;;
	1)
		case "$1" in
			--verify)
				cd "$repository_root"
				exec node "$repository_root/tests/playwright/capture_live_demo_screenshots.mjs" --verify
				;;
			-h|--help)
				printf '%s\n' \
					"Usage: ./devel/capture_screenshots.sh [--verify]" \
					"" \
					"Starts a fresh seeded PLE capture environment and rebuilds current application screenshots." \
					"--verify checks the declared current-artifact manifest without starting a stack."
				exit 0
				;;
		esac
		printf 'Unsupported argument: %s\n' "$1" >&2
		exit 2
		;;
	*)
		printf 'Usage: ./devel/capture_screenshots.sh\n' >&2
		exit 2
		;;
esac

cd "$repository_root"

live_demo_output="$("$repository_root/launchers/run_live_demo.sh" --headless | tee /dev/stderr)"
live_demo_entry="$(printf '%s\n' "$live_demo_output" | sed -n 's/^Live demo entry: //p')"

cleanup() {
	"$repository_root/launchers/run_live_demo.sh" stop >/dev/null || true
}
trap cleanup EXIT

"$repository_root/devel/setup_playwright.sh"

node "$repository_root/tests/playwright/capture_live_demo_screenshots.mjs" "$live_demo_entry"
