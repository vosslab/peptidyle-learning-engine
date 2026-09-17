#!/usr/bin/env bash
# capture_screenshots.sh - publish or live-verify the manifest screenshot corpus.

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repository_root="$(dirname "$script_directory")"
runner="$repository_root/tests/playwright/capture_live_demo_screenshots.mjs"
mode="--publish"
headed=""
fresh=""
only_ids=""

for argument in "$@"; do
	case "$argument" in
	--verify)
		mode="--verify"
		;;
	--headed)
		headed="--headed"
		;;
	--fresh)
		fresh="yes"
		;;
	--only=*)
		mode="--only"
		only_ids="${argument#--only=}"
		;;
	-h | --help)
		printf '%s\n' \
			"Usage: ./devel/capture_screenshots.sh [--verify] [--headed] [--fresh]" \
			"Default: publish the complete corpus against the running Live Demo" \
			"(start one with ./launchers/run_live_demo.sh --headless)." \
			"--verify: validate tracked artifacts, replay live, and retain replay evidence." \
			"--headed: open Chromium; the local CLI authenticator still completes Sysadmin MFA." \
			"--fresh: stop, start, and afterwards stop an owned Live Demo instead of reusing one."
		exit 0
		;;
	*)
		printf 'Unsupported argument: %s\n' "$argument" >&2
		exit 2
		;;
	esac
done

cd "$repository_root"

if [[ "$mode" == "--verify" ]]; then
	DEBUG="" PWDEBUG="" node --import tsx "$runner" --verify-static
fi

"$repository_root/devel/setup_playwright.sh"

source "$repository_root/source_me.sh"

if [[ "$fresh" == "yes" ]]; then
	"$repository_root/launchers/run_live_demo.sh" stop
	cleanup() {
		"$repository_root/launchers/run_live_demo.sh" stop >/dev/null || true
	}
	trap cleanup EXIT
fi
# The launcher reuses a running suite (reporting its URL) or starts one when none is running.
live_demo_output="$("$repository_root/launchers/run_live_demo.sh" --headless | tee /dev/stderr)"

live_demo_entry="$(printf '%s\n' "$live_demo_output" | sed -n 's/^Live demo entry: //p')"
if [[ -z "$live_demo_entry" ]]; then
	printf 'The Live Demo did not report its entry URL.\n' >&2
	exit 1
fi

# Path-only handoff: the separate CLI child reads private setup material, not Chromium.
authenticator_output="$(python3 local_stack.py authenticator --env-file \
	local_stack_state/live_demo_browser/workspace/env.local)"
PLE_LOCAL_DEMO_TOTP_SETUP_FILE="$(printf '%s\n' "$authenticator_output" | \
	sed -n 's/^Local authenticator setup URI: //p')"
if [[ -z "$PLE_LOCAL_DEMO_TOTP_SETUP_FILE" ]]; then
	printf 'The local authenticator did not report its private setup path.\n' >&2
	exit 1
fi
export PLE_LOCAL_DEMO_TOTP_SETUP_FILE
export NODE_EXTRA_CA_CERTS="$repository_root/local_stack_state/live_demo_browser/workspace/gateway-root.crt"

if [[ "$mode" == "--only" ]]; then
	DEBUG="" PWDEBUG="" node --import tsx "$runner" --only "$live_demo_entry" "$only_ids"
elif [[ "$headed" == "--headed" ]]; then
	DEBUG="" PWDEBUG="" node --import tsx "$runner" "$mode" --headed "$live_demo_entry"
else
	DEBUG="" PWDEBUG="" node --import tsx "$runner" "$mode" "$live_demo_entry"
fi

if [[ "$fresh" != "yes" ]]; then
	printf 'Screenshot corpus complete; the existing Live Demo stack is still running.\n'
	exit 0
fi

"$repository_root/launchers/run_live_demo.sh" stop
trap - EXIT
printf 'Screenshot corpus complete; the owned Live Demo stack is clean.\n'
