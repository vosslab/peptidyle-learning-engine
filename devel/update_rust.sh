#!/usr/bin/env bash
# Update Rust toolchains and rustup itself.

set -euo pipefail

echo "=== updating rustup via homebrew ==="
brew upgrade rustup

echo "=== updating rust toolchains ==="
rustup update

echo "=== current versions ==="
rustc --version
cargo --version
rustup --version

echo "done"
