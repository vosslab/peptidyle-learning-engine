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
def test_load_config_normalizes_recipient_domain(
	tmp_path: pathlib.Path,
) -> None:
	"""Normalize a configured recipient domain for exact matching."""
	path = tmp_path / "invitation_mailer.yaml"
	path.write_text(
		"allowed_recipient_domains:\n  - MAIL.ROOSEVELT.EDU\nthrottle_seconds: 1\n",
		encoding="ascii",
	)

	config = invitation_mailer.input.load_config(path)

	assert config.allowed_recipient_domains == frozenset({"mail.roosevelt.edu"})


#============================================
@pytest.mark.parametrize(
	"invalid_config",
	(
		"allowed_recipient_domains: []\n",
		"allowed_recipient_domains:\n  - mail.roosevelt.edu\nthrottle_seconds: .nan\n",
	),
)
def test_load_config_rejects_unusable_values(
	tmp_path: pathlib.Path,
	invalid_config: str,
) -> None:
	"""Reject a missing domain allowlist or non-finite throttle."""
	path = tmp_path / "invitation_mailer.yaml"
	path.write_text(invalid_config, encoding="ascii")

	with pytest.raises(invitation_mailer.input.InvitationInputError):
		invitation_mailer.input.load_config(path)


#============================================
def test_read_export_normalizes_and_preserves_recipient_text(
	tmp_path: pathlib.Path,
) -> None:
	"""Accept operator text while normalizing the email identity."""
	path = tmp_path / "roster.json"
	_write_export(
		path,
		{
			"course_name": "Genetics 301",
			"students": [
				{
					"email": "STUDENT@MAIL.ROOSEVELT.EDU",
					"signup_url": "https://example.edu/signup/token",
					"display_name": "D'Angelo \\\"DJ\\\" Jones",
				}
			],
		},
	)

	mailing_export = invitation_mailer.input.read_export(path, _config())

	recipient = mailing_export.recipients[0]
	assert recipient.email == "student@mail.roosevelt.edu"
	assert recipient.display_name == "D'Angelo \\\"DJ\\\" Jones"


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
@pytest.mark.parametrize("missing_field", ("course_name", "signup_url"))
def test_read_export_rejects_missing_required_content(
	tmp_path: pathlib.Path,
	missing_field: str,
) -> None:
	"""Reject an export without its course identity or per-student link."""
	payload = {
		"course_name": "Genetics 301",
		"students": [
			{
				"email": "student@mail.roosevelt.edu",
				"signup_url": "https://example.edu/signup/token",
			}
		],
	}
	if missing_field == "course_name":
		del payload["course_name"]
	else:
		del payload["students"][0]["signup_url"]
	path = tmp_path / "roster.json"
	_write_export(path, payload)

	with pytest.raises(invitation_mailer.input.InvitationInputError):
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
