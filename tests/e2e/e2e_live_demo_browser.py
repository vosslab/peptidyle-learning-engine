#!/usr/bin/env python3
"""Compatibility entry point for the shared production browser-suite owner."""

import e2e_browser_suite_owner


#============================================
def main() -> None:
	"""Run the existing live-demo command through the H0 shared owner."""
	e2e_browser_suite_owner.main()


if __name__ == "__main__":
	main()
