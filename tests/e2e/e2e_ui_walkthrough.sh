#!/usr/bin/env bash
# e2e_ui_walkthrough.sh - compatibility entry point for the dedicated walkthrough.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT

exec bash "$REPO_ROOT/tests/walkthrough/run_ui_walkthrough.sh" "$@"
