#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$script_directory/../.."
source source_me.sh
exec .venv/bin/python tests/e2e/e2e_live_demo_service_owner.py webwork_render_rpc
