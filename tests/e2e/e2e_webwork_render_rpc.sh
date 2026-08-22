#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$SCRIPT_DIRECTORY/../.."
source source_me.sh
exec python3 tests/e2e/e2e_live_demo_service_owner.py webwork_render_rpc
