#!/usr/bin/env bash
# run_ui_walkthrough.sh - canonical shell entry point for the UI walkthrough.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT

# Repository Python commands run through the maintained local environment.
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
exec python -B "$REPO_ROOT/tests/walkthrough/run_ui_walkthrough.py" "$@"
