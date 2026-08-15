#!/usr/bin/env python3
"""Compatibility entry point for the dedicated UI walkthrough runner."""

import sys

import tests.walkthrough.walklib.runner as runner


if __name__ == "__main__":
	raise SystemExit(runner.main(sys.argv[1:]))
