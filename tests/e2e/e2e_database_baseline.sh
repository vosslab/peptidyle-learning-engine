#!/usr/bin/env bash
# Disposable PostgreSQL baseline acceptance entry point.
#
# The clean baseline owns one PostgreSQL oracle and its lifecycle lease in the
# staged database runner. Keeping this public entry point lets the aggregate
# E2E lane run that canonical baseline directly.

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
exec bash "$script_directory/e2e_sd1_staged_database.sh" "$@"
