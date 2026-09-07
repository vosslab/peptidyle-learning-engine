#!/usr/bin/env bash
# Run the serial controller-managed production-browser owner.

set -euo pipefail

usage() {
	cat <<'USAGE'
Usage: ./devel/run_playwright_tests.sh [--build] [--grep recovery]

Runs real HTTPS browser scenarios through the fixed local-stack controller.
Only the declared recovery selector is accepted.
USAGE
}

build=false
recovery=false
while [ "$#" -gt 0 ]; do
	case "$1" in
		-h|--help) usage; exit 0 ;;
		--build) build=true ;;
		--grep)
			shift
			[ "${1:-}" = "recovery" ] || { usage >&2; exit 2; }
			recovery=true
			;;
		*) usage >&2; exit 2 ;;
	esac
	shift
done

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repository_root"
[ -d node_modules ] || { echo "ERROR: node_modules/ missing. Run npm install first." >&2; exit 1; }

if [ "$build" = true ]; then
	echo "==> rebuilding the production browser bundle"
	npm run build
fi

# shellcheck disable=SC1091
source source_me.sh
arguments=()
if [ "$recovery" = true ]; then arguments+=(--recovery); fi
exec python3 tests/e2e/e2e_live_demo_production_browser.py ${arguments[@]+"${arguments[@]}"}
