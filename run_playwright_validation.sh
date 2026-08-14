#!/usr/bin/env bash
# run_playwright_validation.sh - public front door for live browser validation.

set -euo pipefail

usage() {
	cat <<'USAGE'
Usage: run_playwright_validation.sh --live

Run the complete opt-in Playwright Validation suite.  It includes the ordinary
built mock-browser suite, visual evidence, the canonical instructor-to-student
walkthrough, the Chapter 1 real-browser journey, and WebWork browser
acceptance.

This command may start disposable local stacks and exercise the renderer.
--live is required so an ordinary browser run does not unexpectedly use local
credentials or Podman.  The shared controller checks for conflicting default
or walkthrough containers before any lane runs; it never stops or removes a
stack it did not create.
USAGE
}

if [ "$#" -eq 1 ] && { [ "$1" = "-h" ] || [ "$1" = "--help" ]; }; then
	usage
	exit 0
fi

if [ "$#" -ne 1 ] || [ "$1" != "--live" ]; then
	usage >&2
	exit 2
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT
cd "$REPO_ROOT"

# Keep the supported Python invocation and repository environment consistent
# with the other developer entry points.  The controller owns lifecycle
# configuration, conflict inspection, and child-environment sanitization.
# shellcheck disable=SC1091
source source_me.sh
exec python3 local_stack.py acceptance
