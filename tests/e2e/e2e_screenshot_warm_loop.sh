#!/usr/bin/env bash
# Prove a warm Live Demo rebuilds a stale client bundle and serves it for capture.

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$repository_root"
# shellcheck disable=SC1091
source "$repository_root/source_me.sh"

started="$(date +%s)"
control="$repository_root/local_stack_state/live_demo_browser/developer-control.json"
if [[ ! -f "$control" ]]; then
	echo "Starting Live Demo because none is running."
	./launchers/run_live_demo.sh --headless
fi

style="$repository_root/src/style.css"
dist="$repository_root/dist/index.html"
if [[ ! -f "$dist" ]]; then
	echo "dist/index.html is missing; a warm client rebuild cannot be proven." >&2
	exit 1
fi
before="$(stat -f %m "$dist" 2>/dev/null || stat -c %Y "$dist")"
touch "$style"
restore() {
	git checkout -- "$style" >/dev/null 2>&1 || true
}
trap restore EXIT

./devel/capture_screenshots.sh --verify
after="$(stat -f %m "$dist" 2>/dev/null || stat -c %Y "$dist")"
if ! [[ "$after" -gt "$before" ]]; then
	echo "dist/index.html was not newer after the warm capture run." >&2
	exit 1
fi
elapsed=$(( $(date +%s) - started ))
echo "Warm screenshot loop rebuilt the stale client bundle in ${elapsed}s."
echo "Replay verification passed."
