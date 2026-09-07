#!/usr/bin/env bash
# Full local build entry point. It builds Rust, WASM, generated TypeScript
# definitions, fixtures, and the browser client in dependency order. The client
# needs the generated types and WASM bridge. Correctness gates live in
# ./check_codebase.sh; the per-stage timings below are diagnostic only.
#
# Flags: --release (optimized) or --debug (default).
#

set -euo pipefail
script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$script_directory"

profile="debug"
for arg in "$@"; do
	case "$arg" in
		--release)
			profile="release"
			;;
		--debug)
			profile="debug"
			;;
		-h|--help)
			echo "Usage: build.sh [--release|--debug]"
			exit 0
			;;
		*)
			echo "ERROR: unknown flag: $arg" >&2
			echo "Usage: build.sh [--release|--debug]" >&2
			exit 2
			;;
	esac
done

# Preflight. Fail before doing work rather than halfway through it.
if ! command -v cargo >/dev/null 2>&1; then
	echo "ERROR: cargo not found on PATH. Install Rust from https://rustup.rs" >&2
	exit 1
fi
if [ ! -d node_modules ]; then
	echo "ERROR: node_modules missing. Run ./devel/setup_typescript.sh first." >&2
	exit 1
fi

# Python provides sub-second timing on macOS Bash 3.2 without a new dependency.
now_seconds() {
	python3 -c 'import time; print(f"{time.time():.3f}")'
}

stage_names=()
stage_times=()
build_start="$(now_seconds)"

# run_stage <name> <command...>
run_stage() {
	local name="$1"
	shift
	echo "==> $name"
	local started
	started="$(now_seconds)"
	"$@"
	local finished
	finished="$(now_seconds)"
	stage_names+=("$name")
	stage_times+=("$(python3 -c "print(f'{${finished} - ${started}:.2f}')")")
}

cargo_profile_flag=""
wasm_profile_flag=""
if [ "$profile" = "release" ]; then
	cargo_profile_flag="--release"
else
	wasm_profile_flag="--debug"
fi

# shellcheck disable=SC2086
run_stage rust cargo build --workspace $cargo_profile_flag

# shellcheck disable=SC2086
run_stage wasm ./pipeline/build_wasm.sh $wasm_profile_flag

run_stage tsgen cargo tools tsgen
run_stage fixtures cargo tools fixtures --check
run_stage client node pipeline/build.mjs --skip-wasm

build_end="$(now_seconds)"
total="$(python3 -c "print(f'{${build_end} - ${build_start}:.2f}')")"

echo
echo "Build summary ($profile):"
index=0
while [ "$index" -lt "${#stage_names[@]}" ]; do
	printf '  %-8s %6ss\n' "${stage_names[$index]}" "${stage_times[$index]}"
	index=$((index + 1))
done
printf '  %-8s %6ss\n' "total" "$total"
echo
echo "Client bundle in dist/, WASM bridge in dist_wasm/."
