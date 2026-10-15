#!/usr/bin/env python3
"""Import BPW curriculum through the ordinary Instructor authoring API."""

# local repo modules
import ple_import.import_bpw


#============================================
def main() -> None:
	"""Delegate to the shared curriculum importer."""
	ple_import.import_bpw.main()


if __name__ == "__main__":
	main()
