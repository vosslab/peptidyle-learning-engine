#!/usr/bin/env python3
"""Send one attended batch of student signup emails through macOS Mail.app."""

# local repo modules
import invitation_mailer.cli


#============================================
def main() -> None:
	"""Delegate to the importable invitation-mailer command."""
	invitation_mailer.cli.main()


if __name__ == "__main__":
	main()
