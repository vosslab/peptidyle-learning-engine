#!/usr/bin/env bash

set -euo pipefail

launcher_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "$launcher_directory" rev-parse --show-toplevel)"
cd "$repo_root"

# Aggregate Validation: run the final gates in their authoritative order.
source source_me.sh
./devel/generate_schema_tables_doc.py
./schema_style/check_schema_style.py
./check_rust.sh
./check_codebase.sh
python3 -m pytest tests/
python3 local_stack.py acceptance
