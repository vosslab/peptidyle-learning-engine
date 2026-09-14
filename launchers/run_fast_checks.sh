#!/usr/bin/env bash
# Run the complete offline validation subset without starting the live stack.
#
# This intentionally matches the first three authoritative gates in all_test.sh.
# Live PostgreSQL, object-store, browser, and deployment acceptance remains in
# launchers/all_test.sh.

set -euo pipefail

usage() {
	cat <<'USAGE'
Usage: launchers/run_fast_checks.sh [-h|--help]

Runs the repository's offline aggregate gates:
  1. Rust checks and tests
  2. TypeScript checks and tests
  3. Python tests

The disposable live-stack acceptance gate is intentionally omitted.

  -h, --help  Print this help and exit 0.
USAGE
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "ERROR: unknown flag: $1" >&2
			usage >&2
			exit 2
			;;
	esac
done

launcher_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "$launcher_directory" rev-parse --show-toplevel)"
cd "$repo_root"

./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/

echo "PASS: offline aggregate checks passed."
