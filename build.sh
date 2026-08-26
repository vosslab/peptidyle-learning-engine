#!/usr/bin/env bash
# build.sh - master build for the whole repository.
#
# Front door: run this directly as ./build.sh (npm run build mirrors it). It is
# the one command that builds everything, in dependency order:
#
#   1. rust     cargo build for the workspace (the API server and every crate)
#   2. wasm     crates/wasm compiled to WebAssembly, plus generated JS glue
#   3. tsgen    TypeScript definitions generated from the Rust question model
#   4. fixtures verify tracked typed fixture evidence
#   5. client   the Solid browser client bundled into dist/
#
# The order is not arbitrary. The client imports the generated types and the
# WASM bridge, so both must exist before it bundles. Running a later stage
# alone is what produces the "my change did nothing" class of confusion.
#
# Timing: each stage is measured and reported at the end. The numbers are ours
# rather than cargo's because cargo only sees its own stage, and the useful
# question is where the whole build spends its time.
#
# Treat the timings as information. They report where time went so a stage that
# grows becomes visible and can be investigated. Build time varies with cache
# state, machine load, and what changed, so read the numbers as a trend across
# runs on one machine. Correctness gates live in ./check_codebase.sh.
#
# Flags:
#   --release    optimized build (slower to build, what you ship)
#   --debug      unoptimized build (default; faster iteration)
#
# Not a GitHub Pages build. This repository ships a server platform, so dist/
# is a client bundle the API serves, not a static site.

set -euo pipefail
SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$SCRIPT_DIRECTORY"

PROFILE="debug"
for arg in "$@"; do
	case "$arg" in
		--release)
			PROFILE="release"
			;;
		--debug)
			PROFILE="debug"
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

# Sub-second timing. bash 3.2 ships on macOS and has no EPOCHREALTIME, and
# `date +%s` only resolves to whole seconds, which is too coarse to tell a 0.4s
# stage from a 1.4s one. python3 is already a hard dependency of this repo (the
# pytest gate and devel/ scripts), so use it rather than adding a new one.
now_seconds() {
	python3 -c 'import time; print(f"{time.time():.3f}")'
}

STAGE_NAMES=()
STAGE_TIMES=()
BUILD_START="$(now_seconds)"

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
	STAGE_NAMES+=("$name")
	STAGE_TIMES+=("$(python3 -c "print(f'{${finished} - ${started}:.2f}')")")
}

CARGO_PROFILE_FLAG=""
WASM_PROFILE_FLAG=""
if [ "$PROFILE" = "release" ]; then
	CARGO_PROFILE_FLAG="--release"
else
	WASM_PROFILE_FLAG="--debug"
fi

# 1. The Rust workspace, including the API server binary.
# shellcheck disable=SC2086
run_stage rust cargo build --workspace $CARGO_PROFILE_FLAG

# 2. The WASM bridge. Always built before the client, which copies it.
# shellcheck disable=SC2086
run_stage wasm ./pipeline/build_wasm.sh $WASM_PROFILE_FLAG

# 3. TypeScript definitions generated from the Rust model, so the client cannot
#    compile against a stale shape.
run_stage tsgen cargo tools tsgen

# 4. Verify intentional fixture evidence.
run_stage fixtures cargo tools fixtures --check

# 5. The browser client. --skip-wasm because stage 2 already built the bridge.
run_stage client node pipeline/build.mjs --skip-wasm

BUILD_END="$(now_seconds)"
TOTAL="$(python3 -c "print(f'{${BUILD_END} - ${BUILD_START}:.2f}')")"

echo
echo "Build summary ($PROFILE):"
index=0
while [ "$index" -lt "${#STAGE_NAMES[@]}" ]; do
	printf '  %-8s %6ss\n' "${STAGE_NAMES[$index]}" "${STAGE_TIMES[$index]}"
	index=$((index + 1))
done
printf '  %-8s %6ss\n' "total" "$TOTAL"
echo
echo "Client bundle in dist/, WASM bridge in dist_wasm/."
