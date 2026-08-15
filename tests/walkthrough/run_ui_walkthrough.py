#!/usr/bin/env python3
"""Run the real-stack UI walkthrough through the importable walklib package."""

import sys

import tests.walkthrough.walklib.runner as runner


if __name__ == "__main__":
	raise SystemExit(runner.main(sys.argv[1:]))
