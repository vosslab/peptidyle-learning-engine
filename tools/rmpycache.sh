#!/bin/sh

find . -type d -name "__pycache__" -prune -exec rm -rf -- {} +
find . -type f -name ".DS_Store" -exec rm -f -- {} +
find . -type d -name ".pytest_cache" -prune -exec rm -rf -- {} +
