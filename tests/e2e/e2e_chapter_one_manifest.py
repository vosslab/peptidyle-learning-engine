#!/usr/bin/env python3
"""Validate the answer-free Chapter 1 release-seed manifest."""

from __future__ import annotations

import sys
import pathlib

# This executable lives below the repository import root, so direct E2E
# execution uses the same explicit package anchor as the publication runner.
SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.chapter_one_manifest
import local_stack_control.models

#============================================
def fail(message: str) -> None:
	raise SystemExit(f"Chapter 1 manifest E2E: {message}")


#============================================
def load_manifest(path: pathlib.Path) -> dict[str, object]:
	try:
		manifest_bytes = path.read_bytes()
	except OSError as error:
		fail(f"could not read {path}: {error}")
	try:
		manifest = local_stack_control.chapter_one_manifest.parse_manifest_bytes(manifest_bytes)
	except local_stack_control.models.ControllerError as error:
		fail(f"could not validate {path}: {error}")
	return manifest


#============================================
def main(argv: list[str]) -> None:
	if len(argv) != 3:
		fail("usage: e2e_chapter_one_manifest.py FIRST_MANIFEST SECOND_MANIFEST")
	first = load_manifest(pathlib.Path(argv[1]))
	second = load_manifest(pathlib.Path(argv[2]))
	if first != second:
		fail("the release seed did not produce the same manifest on rerun")


if __name__ == "__main__":
	main(sys.argv)
