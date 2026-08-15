#!/usr/bin/env bash
# Compatibility entry point for the documented Chapter One publication command.
REPO_ROOT="$(git -C "$(dirname "$0")/../.." rev-parse --show-toplevel)"
cd "$REPO_ROOT"
source "$REPO_ROOT/source_me.sh"
exec python3 tests/e2e/e2e_chapter_one_pilot.py "$@"
