"""Safety tests for the invitation-mailer command arguments."""

# PIP3 modules
import pytest

# local repo modules
import invitation_mailer.cli


#============================================
def test_force_resend_requires_send_and_one_recipient() -> None:
	"""Reject broad or preview-only force-resend requests."""
	with pytest.raises(SystemExit):
		invitation_mailer.cli.parse_args(["output-email/roster.json", "--force-resend"])
	with pytest.raises(SystemExit):
		invitation_mailer.cli.parse_args(
			["output-email/roster.json", "--send", "--force-resend"]
		)
