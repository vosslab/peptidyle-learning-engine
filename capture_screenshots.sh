#!/usr/bin/env bash
# Capture the canonical production PLE visual corpus through the shared browser front door.
set -euo pipefail

print_usage() {
	printf '%s\n' \
		"Usage: ${BASH_SOURCE[0]##*/}" \
		"       ${BASH_SOURCE[0]##*/} --help" \
		"" \
		"With no options, publish the full real-stack screenshot corpus:" \
		"  source source_me.sh && ./${BASH_SOURCE[0]##*/}" \
		"" \
		"Normal invocation accepts no options."
}

if (( $# > 0 )); then
	case "$1" in
		-h|--help)
			if (( $# == 1 )); then
				print_usage
				exit 0
			fi
			;;
	esac
	printf 'Unsupported argument: %s\n' "$1" >&2
	printf 'Usage: %s [--help]\n' "${BASH_SOURCE[0]##*/}" >&2
	exit 2
fi

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$SCRIPT_DIRECTORY"
exec "$SCRIPT_DIRECTORY/run_playwright_tests.sh" --screenshots
