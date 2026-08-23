#!/usr/bin/env bash
# run_live_demo.sh - concise front door for the canonical live demo.

set -euo pipefail

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

usage() {
  echo "Usage: ./run_live_demo.sh [--no-open|start [--no-open]|stop]"
}

COMMAND="start"
NO_OPEN="false"

case "$#" in
  0)
    ;;
  1)
    case "$1" in
      --help|-h)
        usage
        exit 0
        ;;
      --no-open)
        NO_OPEN="true"
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
    if [ "$1" = "start" ] && [ "$2" = "--no-open" ]; then
      NO_OPEN="true"
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

if [ "$COMMAND" = "stop" ]; then
  exec python3 "$SCRIPT_DIRECTORY/local_stack.py" stop
fi

if [ "$NO_OPEN" = "true" ]; then
  exec python3 "$SCRIPT_DIRECTORY/local_stack.py" start --no-open
fi

exec python3 "$SCRIPT_DIRECTORY/local_stack.py" start
