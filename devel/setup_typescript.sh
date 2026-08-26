#!/bin/sh
# Compatibility delegate; the repository-root setup.sh is the public interface.

set -e

REPO_ROOT="$(git rev-parse --show-toplevel)"
exec "$REPO_ROOT/setup.sh" "$@"
