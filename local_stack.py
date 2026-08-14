#!/usr/bin/env python3
"""Operator entry point for the local Podman stack controller."""

import sys
import pathlib

import local_stack_control.cli
import local_stack_control.compose
import local_stack_control.models
import local_stack_control.process


#============================================
def main() -> None:
	"""Run the repository-anchored local stack command line interface."""
	try:
		repo_root = local_stack_control.compose.repo_root_from_entrypoint(pathlib.Path(__file__))
	except local_stack_control.models.ControllerError as error:
		print(f"ERROR: {error}", file=sys.stderr)
		raise SystemExit(2) from error
	runner = local_stack_control.process.SubprocessRunner()
	result = local_stack_control.cli.run(sys.argv[1:], runner, repo_root)
	raise SystemExit(result)


if __name__ == "__main__":
	main()
