"""Behavior tests for invitation-mailer input boundaries."""

# Standard Library
import os
import json
import pathlib

# PIP3 modules
import pytest

# local repo modules
import invitation_mailer.input


#============================================
def _config() -> invitation_mailer.input.MailerConfig:
	"""Return the narrow test configuration."""
	return invitation_mailer.input.MailerConfig(
		allowed_recipient_domains=frozenset({"mail.roosevelt.edu"}),
		throttle_seconds=0,
	)


#============================================
def _write_export(path: pathlib.Path, payload: object) -> None:
	"""Write one JSON export fixture."""
	path.write_text(json.dumps(payload), encoding="utf-8")


#============================================
@pytest.mark.parametrize(
	("email", "signup_url"),
	(
		("student@example.edu", "https://example.edu/signup/token"),
		(
			"student@mail.roosevelt.edu.attacker.example",
			"https://example.edu/signup/token",
		),
		("student@mail.roosevelt.edu", "http://example.edu/signup/token"),
		("student@mail.roosevelt.edu", "https://user:secret@example.edu/token"),
	),
)
def test_read_export_rejects_unsafe_recipient_boundaries(
	tmp_path: pathlib.Path,
	email: str,
	signup_url: str,
) -> None:
	"""Reject recipients outside the domain and HTTPS URL contracts."""
	path = tmp_path / "roster.json"
	_write_export(
		path,
		{
			"course_name": "Genetics 301",
			"students": [{"email": email, "signup_url": signup_url}],
		},
	)

	with pytest.raises(invitation_mailer.input.InvitationInputError):
		invitation_mailer.input.read_export(path, _config())


#============================================
def test_read_export_rejects_normalized_duplicate_addresses(
	tmp_path: pathlib.Path,
) -> None:
	"""Refuse an export that could send the same student twice."""
	path = tmp_path / "roster.json"
	_write_export(
		path,
		{
			"course_name": "Genetics 301",
			"students": [
				{
					"email": "student@mail.roosevelt.edu",
					"signup_url": "https://example.edu/signup/one",
				},
				{
					"email": "STUDENT@mail.roosevelt.edu",
					"signup_url": "https://example.edu/signup/two",
				},
			],
		},
	)

	with pytest.raises(invitation_mailer.input.InvitationInputError, match="duplicate"):
		invitation_mailer.input.read_export(path, _config())


#============================================
def test_resolve_private_export_accepts_only_private_direct_json(
	tmp_path: pathlib.Path,
) -> None:
	"""Confine PII-bearing input to an owner-private output-email file."""
	working_directory = tmp_path / "output-email"
	working_directory.mkdir(mode=0o700)
	export_path = working_directory / "roster.json"
	export_path.write_text("{}", encoding="ascii")
	os.chmod(export_path, 0o600)

	resolved = invitation_mailer.input.resolve_private_export(
		tmp_path,
		"output-email/roster.json",
	)

	assert resolved == export_path.resolve()
	os.chmod(export_path, 0o644)
	with pytest.raises(invitation_mailer.input.InvitationInputError, match="group or other"):
		invitation_mailer.input.resolve_private_export(
			tmp_path,
			"output-email/roster.json",
		)
	outside_path = tmp_path / "outside.json"
	outside_path.write_text("{}", encoding="ascii")
	os.chmod(outside_path, 0o600)
	with pytest.raises(invitation_mailer.input.InvitationInputError):
		invitation_mailer.input.resolve_private_export(tmp_path, str(outside_path))
