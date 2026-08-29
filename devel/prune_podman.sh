#!/usr/bin/env bash

podman df
echo ""
sleep 1
podman system prune
sleep 1
podman volume prune
sleep 1
echo ""
podman df
echo ""
