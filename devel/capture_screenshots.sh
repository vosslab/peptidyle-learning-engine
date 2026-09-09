#!/usr/bin/env bash
# capture_screenshots.sh - publish or live-verify the manifest screenshot corpus.

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repository_root="$(dirname "$script_directory")"
runner="$repository_root/tests/playwright/capture_live_demo_screenshots.mjs"
mode="--publish"

case "$#" in
	0)
		;;
	1)
		case "$1" in
		--verify)
			mode="--verify"
			;;
		-h | --help)
			printf '%s\n' \
				"Usage: ./devel/capture_screenshots.sh [--verify]" \
				"" \
				"Default: capture and publish the complete manifest corpus." \
				"--verify: validate tracked artifacts, replay live, and retain replay evidence."
			exit 0
			;;
		*)
			printf 'Unsupported argument: %s\n' "$1" >&2
			exit 2
			;;
		esac
		;;
	*)
		printf 'Usage: ./devel/capture_screenshots.sh [--verify]\n' >&2
		exit 2
		;;
esac

cd "$repository_root"

if [[ "$mode" == "--verify" ]]; then
	node --import tsx "$runner" --verify-static
fi

"$repository_root/devel/setup_playwright.sh"
"$repository_root/launchers/run_live_demo.sh" stop

cleanup() {
	"$repository_root/launchers/run_live_demo.sh" stop >/dev/null || true
}
trap cleanup EXIT

live_demo_output="$("$repository_root/launchers/run_live_demo.sh" --headless | tee /dev/stderr)"
live_demo_entry="$(printf '%s\n' "$live_demo_output" | sed -n 's/^Live demo entry: //p')"
if [[ -z "$live_demo_entry" ]]; then
	printf 'The Live Demo did not report its entry URL.\n' >&2
	exit 1
fi

node --import tsx "$runner" "$mode" "$live_demo_entry"

"$repository_root/launchers/run_live_demo.sh" stop
trap - EXIT
printf 'Screenshot corpus complete; the owned Live Demo stack is clean.\n'
