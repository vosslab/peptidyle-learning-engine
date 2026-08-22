#!/usr/bin/env bash
# run_playwright_tests.sh - canonical production-browser suite front door.
#
# The shared owner builds the production dist/ bundle as part of its disposable
# lifecycle, starts the private HTTPS gateway and real services, then creates
# the fixed Playwright child command. Every accepted selection owns a fresh
# stack and has the same lifecycle cleanup boundary.

set -euo pipefail

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$SCRIPT_DIRECTORY"
source "$SCRIPT_DIRECTORY/source_me.sh"
exec python3 "$SCRIPT_DIRECTORY/tests/e2e/e2e_browser_suite_owner.py" "$@"
