#!/usr/bin/env bash
# Disposable PostgreSQL baseline lifecycle gate.
#
# The lease owner performs the complete ordinary lifecycle: fresh initialize,
# compatible replay, restricted application verification, and the stable
# security catalog. This public entry point deliberately accepts no database
# URL, migration path, or compatibility command.

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repository_root="$(cd "$script_directory/../.." && pwd -P)"

cd "$repository_root"
exec python3 -m local_stack_control.database_baseline_owner "$@"
