#!/bin/sh

rm -fr $(find . -name "__pycache__" -type d)
rm -f $(find . -name ".DS_Store" -type f)
rm -fr $(find . -name ".pytest_cache" -type d)
