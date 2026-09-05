#!/usr/bin/env bash

set -euo pipefail

launcher_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "$launcher_directory" rev-parse --show-toplevel)"
cd "$repo_root"

# Aggregate Validation: run the four final gates in their authoritative order.
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
