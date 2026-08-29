#!/usr/bin/env bash
# setup_wasm_tests.sh - install the version-matched wasm-bindgen test runner.

set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"

runner_package_id="$(cargo pkgid wasm-bindgen)"
runner_version="${runner_package_id##*@}"
runner_root="target/tooling/wasm-bindgen-cli"
runner="$runner_root/bin/wasm-bindgen-test-runner"

if [ -x "$runner" ]; then
	actual_version="$($runner --version)"
	if [[ "$actual_version" == *"$runner_version"* ]]; then
		echo "wasm-bindgen test runner $runner_version already installed in $runner_root"
		exit 0
	fi
	echo "ERROR: $runner has the wrong version: $actual_version" >&2
	echo "Remove $runner_root and rerun this setup command." >&2
	exit 1
fi

cargo install wasm-bindgen-cli \
	--version "$runner_version" \
	--locked \
	--root "$runner_root"

echo "Installed wasm-bindgen test runner $runner_version in $runner_root"
