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
file_mtime() {
	local path="$1"
	local mtime
	mtime="$(stat -c %Y "$path" 2>/dev/null || stat -f %m "$path")"
	if ! [[ "$mtime" =~ ^[0-9]+$ ]]; then
		echo "Could not read a numeric modification time for $path." >&2
		return 1
	fi
	printf '%s\n' "$mtime"
}

before="$(file_mtime "$dist")"
temporary_directory="$(mktemp -d)"
style_mtime_reference="$temporary_directory/style.css"
restore() {
	local original_status=$?
	touch -r "$style_mtime_reference" "$style" || true
	rm -f "$style_mtime_reference" || true
	rmdir "$temporary_directory" || true
	exit "$original_status"
}
trap restore EXIT
touch -r "$style" "$style_mtime_reference"
touch "$style"

./devel/capture_screenshots.sh --verify
after="$(file_mtime "$dist")"
if ! [[ "$after" -gt "$before" ]]; then
	echo "dist/index.html was not newer after the warm capture run." >&2
	exit 1
fi
elapsed=$(( $(date +%s) - started ))
echo "Warm screenshot loop rebuilt the stale client bundle in ${elapsed}s."
echo "Replay verification passed."
