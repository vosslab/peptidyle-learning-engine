#!/usr/bin/env bash
# Legacy compatibility entry point. The Python controller owns lane sequencing.

if [ "$#" -ne 0 ]; then
	echo "ERROR: use source source_me.sh && python3 local_stack.py acceptance" >&2
	exit 2
fi

exec python3 local_stack.py acceptance
