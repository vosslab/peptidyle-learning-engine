#!/usr/bin/env bash
# e2e_ui_walkthrough.sh - stable shell entrypoint for the Python walkthrough runner.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT

# Repository Python commands run through the maintained local environment.
# shellcheck disable=SC1091
source "$REPO_ROOT/source_me.sh"
exec python3 "$REPO_ROOT/tests/e2e/e2e_ui_walkthrough.py" "$@"
