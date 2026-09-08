#!/usr/bin/env bash

podman system df
echo ""
sleep 1
podman system prune
sleep 1
podman volume prune
sleep 1
echo ""
podman system df
echo ""
