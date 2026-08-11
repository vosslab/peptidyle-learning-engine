#!/usr/bin/env python3
"""Compatibility entry point for the dedicated UI walkthrough runner."""

import pathlib
import sys


WALKTHROUGH_DIRECTORY = pathlib.Path(__file__).resolve().parents[1] / "walkthrough"
sys.path.insert(0, str(WALKTHROUGH_DIRECTORY))

import walklib.runner


if __name__ == "__main__":
	raise SystemExit(walklib.runner.main(sys.argv[1:]))
