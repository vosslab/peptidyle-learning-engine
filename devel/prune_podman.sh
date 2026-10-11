#!/usr/bin/env bash

set -e

podman system df
echo ""

podman stop -a
podman rm -a -f

podman system prune --all --force
podman volume prune --all --force

echo ""
podman system df
echo ""
