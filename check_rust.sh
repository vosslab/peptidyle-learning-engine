#!/usr/bin/env bash
# Repository-owned Rust verification front door.
#
# Keep this separate from check_codebase.sh: that vendored script is replaced
# when the shared TypeScript codebase gate is refreshed.

set -euo pipefail

usage() {
	cat <<'USAGE'
Usage: check_rust.sh [-h|--help]

Runs the complete offline Rust gate:
  1. Rust-owned TypeScript contract generation
  2. tracked Rust-owned fixture verification
  3. rustfmt check
  4. default-feature workspace check
  5. all-target, all-feature workspace check
  6. default-feature Clippy with warnings denied
  7. all-target, all-feature Clippy with warnings denied
  8. default-feature workspace tests and doctests
  9. all-feature workspace tests and doctests
 10. wasm_bridge check for wasm32-unknown-unknown

Live PostgreSQL, MinIO, container, and deployment tests remain in their named
E2E gates and are not made trustworthy by this offline script.

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

cd "$(git rev-parse --show-toplevel)"

if ! command -v cargo >/dev/null 2>&1; then
	echo "ERROR: cargo not found on PATH." >&2
	echo "Install the rustup toolchain selected by rust-toolchain.toml." >&2
	exit 1
fi
if ! cargo fmt --version >/dev/null 2>&1; then
	echo "ERROR: rustfmt is unavailable. Run: rustup component add rustfmt" >&2
	exit 1
fi
if ! cargo clippy --version >/dev/null 2>&1; then
	echo "ERROR: Clippy is unavailable. Run: rustup component add clippy" >&2
	exit 1
fi

run_step() {
	local step_name="$1"
	shift
	echo
	echo "==> ${step_name}"
	"$@"
}

run_step "Rust-owned TypeScript contracts" cargo tools tsgen
run_step "Rust-owned fixture contracts" cargo tools fixtures --check
run_step "Rust formatting" cargo fmt --all -- --check
run_step "Default-feature workspace check" cargo check --workspace --locked
run_step "All-target, all-feature workspace check" \
	cargo check --workspace --all-targets --all-features --locked
run_step "Default-feature strict Clippy" \
	cargo clippy --workspace --all-targets --locked -- -D warnings
run_step "All-target, all-feature strict Clippy" \
	cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
run_step "Default-feature workspace tests and doctests" \
	cargo test --workspace --locked --no-fail-fast
run_step "All-feature workspace tests and doctests" \
	cargo test --workspace --all-features --locked --no-fail-fast
run_step "Browser WebAssembly target check" \
	cargo check -p wasm_bridge --target wasm32-unknown-unknown --locked

echo
echo "PASS: Rust workspace checks and tests completed."
