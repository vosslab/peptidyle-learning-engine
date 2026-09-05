"""Behavior tests for invitation rendering and dispatch observations."""

# Standard Library
import pathlib

# PIP3 modules
import pytest

# local repo modules
import invitation_mailer.input
import invitation_mailer.sender
import invitation_mailer.status_log


TEMPLATE = "Hello {recipient_name},\n\nCourse: {course_name}\n\n{signup_url}\n"


#============================================
def _recipient(address: str = "student@mail.roosevelt.edu") -> invitation_mailer.input.Recipient:
	"""Build one recipient for dispatch tests."""
	return invitation_mailer.input.Recipient(
		email=address,
		signup_url="https://example.edu/signup/token",
		display_name="D'Angelo \\\"DJ\\\" Jones",
		roster_id=None,
	)


#============================================
def _export(*recipients: invitation_mailer.input.Recipient) -> invitation_mailer.input.MailingExport:
	"""Build one course export for dispatch tests."""
	return invitation_mailer.input.MailingExport("Genetics 301", tuple(recipients))


#============================================
def test_render_message_keeps_signup_url_on_its_own_line() -> None:
	"""Render names literally and expose the opaque signup URL plainly."""
	recipient = _recipient()

	subject, body = invitation_mailer.sender.render_message(
		TEMPLATE,
		"Genetics 301",
		recipient,
	)

	assert "D'Angelo \\\"DJ\\\" Jones" in body
	assert "\nhttps://example.edu/signup/token\n" in body


#============================================
@pytest.mark.parametrize(
	"template",
	(
		"{",
		"{course_name} {recipient_name}",
		"{course_name} {recipient_name} {signup_url} {unknown}",
		"{course_name} {recipient_name} URL: {signup_url}",
	),
)
def test_render_message_rejects_template_contract_drift(template: str) -> None:
	"""Refuse templates that omit required content or add unknown fields."""
	with pytest.raises(invitation_mailer.sender.InvitationSenderError):
		invitation_mailer.sender.render_message(template, "Genetics 301", _recipient())


#============================================
@pytest.mark.parametrize("failure_mode", ("return", "raise"))
def test_process_batch_continues_after_one_recipient_failure(
	tmp_path: pathlib.Path,
	failure_mode: str,
) -> None:
	"""Record a per-recipient failure and continue the attended batch."""
	first = _recipient("first@mail.roosevelt.edu")
	second = _recipient("second@mail.roosevelt.edu")
	mailing_export = _export(first, second)
	cells = {}
	def send_func(name: str, address: str, subject: str, body: str) -> str | None:
		"""Fail the first address and accept the second."""
		if address != first.email:
			return None
		message = "Mail unavailable for https://example.edu/signup/private-token"
		if failure_mode == "raise":
			raise RuntimeError(message)
		return message

	summary = invitation_mailer.sender.process_batch(
		mailing_export=mailing_export,
		recipients=mailing_export.recipients,
		template=TEMPLATE,
		cells=cells,
		status_path=tmp_path / "status.json",
		dry_run=False,
		throttle_seconds=0,
		send_func=send_func,
	)

	assert (summary.failed, summary.sent) == (1, 1)
	assert cells[("Genetics 301", first.email)].message == (
		"Mail unavailable for [redacted URL]"
	)


#============================================
def test_interruption_leaves_indeterminate_status(tmp_path: pathlib.Path) -> None:
	"""Preserve a conservative observation when control stops during a send."""
	recipient = _recipient()
	mailing_export = _export(recipient)
	cells = {}
	status_path = tmp_path / "status.json"

	def interrupt(name: str, address: str, subject: str, body: str) -> str | None:
		"""Simulate an operator interruption after dispatch begins."""
		raise KeyboardInterrupt

	with pytest.raises(KeyboardInterrupt):
		invitation_mailer.sender.process_batch(
			mailing_export=mailing_export,
			recipients=mailing_export.recipients,
			template=TEMPLATE,
			cells=cells,
			status_path=status_path,
			dry_run=False,
			throttle_seconds=0,
			send_func=interrupt,
		)

	persisted = invitation_mailer.status_log.load(status_path)
	assert persisted[("Genetics 301", recipient.email)].status == "indeterminate"


#============================================
def test_dry_run_never_calls_sender(tmp_path: pathlib.Path) -> None:
	"""Keep the default preview path free of Mail.app side effects."""
	recipient = _recipient()
	mailing_export = _export(recipient)
	cells = {}

	def unexpected(name: str, address: str, subject: str, body: str) -> str | None:
		"""Fail if dry-run dispatch reaches the sender seam."""
		raise AssertionError("dry run called sender")

	summary = invitation_mailer.sender.process_batch(
		mailing_export=mailing_export,
		recipients=mailing_export.recipients,
		template=TEMPLATE,
		cells=cells,
		status_path=tmp_path / "status.json",
		dry_run=True,
		throttle_seconds=0,
		send_func=unexpected,
	)

	assert summary.dry_run == 1
	assert cells[("Genetics 301", recipient.email)].status == "dry_run"
