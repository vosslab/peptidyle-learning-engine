#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$script_directory"

# The fixed repo-local environment owns Python dependencies for every final gate.
./devel/setup_python.sh

# Aggregate Validation: run the four final gates in their authoritative order.
./check_rust.sh
./check_codebase.sh
source source_me.sh && .venv/bin/python -m pytest tests/
source source_me.sh && .venv/bin/python local_stack.py acceptance
