#!/usr/bin/env bash
# Run the complete offline validation subset without starting the live stack.
#
# This intentionally matches the offline gates in all_test.sh, plus schema style.
# Live PostgreSQL, object-store, browser, and deployment acceptance remains in
# launchers/all_test.sh.

set -euo pipefail

usage() {
	cat <<'USAGE'
Usage: launchers/run_fast_checks.sh [-h|--help]

Runs the repository's offline aggregate gates:
  1. Schema tables doc and schema style (fails on any finding)
  2. Rust checks and tests
  3. TypeScript checks and tests
  4. Python tests

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

source source_me.sh
./devel/generate_schema_tables_doc.py
./schema_style/check_schema_style.py

./check_rust.sh

./check_codebase.sh

python3 -m pytest tests/

echo "PASS: offline aggregate checks passed."
