#!/usr/bin/env bash

set -euo pipefail

# Aggregate Validation: fast checks first, then one suite-owned real-stack acceptance run.
source source_me.sh
pytest tests/

# Keep this explicit production-artifact receipt distinct from the disposable
# browser suite's fresh build of the target it owns.
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 local_stack.py acceptance
git diff --check
git diff --cached --check
