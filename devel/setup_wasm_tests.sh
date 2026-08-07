#!/usr/bin/env bash
# setup_wasm_tests.sh - install the version-matched wasm-bindgen test runner.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

RUNNER_VERSION="0.2.126"
RUNNER_ROOT="target/tooling/wasm-bindgen-cli"
RUNNER="$RUNNER_ROOT/bin/wasm-bindgen-test-runner"

if [ -x "$RUNNER" ]; then
	ACTUAL_VERSION="$($RUNNER --version)"
	if [[ "$ACTUAL_VERSION" == *"$RUNNER_VERSION"* ]]; then
		echo "wasm-bindgen test runner $RUNNER_VERSION already installed in $RUNNER_ROOT"
		exit 0
	fi
	echo "ERROR: $RUNNER has the wrong version: $ACTUAL_VERSION" >&2
	echo "Remove $RUNNER_ROOT and rerun this setup command." >&2
	exit 1
fi

cargo install wasm-bindgen-cli \
	--version "$RUNNER_VERSION" \
	--locked \
	--root "$RUNNER_ROOT"

echo "Installed wasm-bindgen test runner $RUNNER_VERSION in $RUNNER_ROOT"
