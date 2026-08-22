#!/usr/bin/env bash
# Capture the canonical production PLE visual corpus through the shared browser front door.
set -euo pipefail

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$SCRIPT_DIRECTORY"
exec "$SCRIPT_DIRECTORY/run_playwright_tests.sh" --screenshots
