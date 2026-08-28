#!/usr/bin/env bash
# run_live_demo.sh - concise front door for the canonical live demo.

set -euo pipefail

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

usage() {
  echo "Usage: ./run_live_demo.sh [--headless|start [--headless]|stop]"
}

COMMAND="start"
HEADLESS="false"

case "$#" in
  0)
    ;;
  1)
    case "$1" in
      --help|-h)
        usage
        exit 0
        ;;
      --headless)
        HEADLESS="true"
        ;;
      start|stop)
        COMMAND="$1"
        ;;
      *)
        usage >&2
        exit 2
        ;;
    esac
    ;;
  2)
    if [ "$1" = "start" ] && [ "$2" = "--headless" ]; then
      HEADLESS="true"
    else
      usage >&2
      exit 2
    fi
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

# shellcheck disable=SC1091
source "$SCRIPT_DIRECTORY/source_me.sh"

"$SCRIPT_DIRECTORY/devel/setup_python.sh"

if [ "$COMMAND" = "stop" ]; then
  exec "$SCRIPT_DIRECTORY/.venv/bin/python" "$SCRIPT_DIRECTORY/local_stack.py" stop
fi

if [ ! -d "$SCRIPT_DIRECTORY/node_modules" ]; then
  echo "==> First launch: installing repository dependencies"
  "$SCRIPT_DIRECTORY/devel/setup_typescript.sh"
fi

if [ "$HEADLESS" = "true" ]; then
  exec "$SCRIPT_DIRECTORY/.venv/bin/python" "$SCRIPT_DIRECTORY/local_stack.py" start --headless
fi

exec "$SCRIPT_DIRECTORY/.venv/bin/python" "$SCRIPT_DIRECTORY/local_stack.py" start
