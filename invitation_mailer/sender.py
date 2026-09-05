"""Render and dispatch attended Mail.app invitation messages."""

# Standard Library
import time
import re
import pathlib
import string
import datetime
import dataclasses
import collections.abc

# local repo modules
import invitation_mailer.input
import invitation_mailer.status_log


MAIL_APP_SCRIPT = "\n".join(
	(
		"on run argv",
		"\tset recipientName to item 1 of argv",
		"\tset recipientAddress to item 2 of argv",
		"\tset messageSubject to item 3 of argv",
		"\tset messageContent to item 4 of argv",
		'\ttell application "Mail"',
		"\t\tset theMessage to make new outgoing message with properties "
		"{subject:messageSubject, content:messageContent, visible:true}",
		"\t\ttell theMessage",
		"\t\t\tmake new to recipient with properties "
		"{name:recipientName, address:recipientAddress}",
		"\t\t\tsend",
		"\t\tend tell",
		"\tend tell",
		"end run",
	)
)

TEMPLATE_FIELDS = frozenset({"course_name", "recipient_name", "signup_url"})
ERROR_MESSAGE_LIMIT = 200

SendFunction = collections.abc.Callable[[str, str, str, str], str | None]


#============================================
class InvitationSenderError(ValueError):
	"""The message template or sender result is invalid."""


#============================================
@dataclasses.dataclass(frozen=True)
class BatchSummary:
	"""Outcome counts for one attended batch."""

	sent: int = 0
	failed: int = 0
	dry_run: int = 0


#============================================
def load_template(path: pathlib.Path) -> str:
	"""Read the plain-text message template."""
	try:
		template = path.read_text(encoding="ascii")
	except UnicodeError as error:
		raise InvitationSenderError("invitation template must be ASCII text") from error
	if not template.strip():
		raise InvitationSenderError("invitation template must not be empty")
	return template


#============================================
def _template_fields(template: str) -> frozenset[str]:
	"""Return the exact replacement fields used by a template."""
	fields = set()
	formatter = string.Formatter()
	try:
		for _, field_name, format_spec, conversion in formatter.parse(template):
			if field_name is not None:
				if format_spec or conversion:
					raise InvitationSenderError(
						"invitation template fields cannot use conversions or format specs"
					)
				fields.add(field_name)
	except ValueError as error:
		raise InvitationSenderError(f"invalid invitation template: {error}") from error
	return frozenset(fields)


#============================================
def render_message(
	template: str,
	course_name: str,
	recipient: invitation_mailer.input.Recipient,
) -> tuple[str, str]:
	"""Render one subject and body without interpreting recipient text."""
	fields = _template_fields(template)
	unknown_fields = fields - TEMPLATE_FIELDS
	if unknown_fields:
		unknown = ", ".join(sorted(unknown_fields))
		raise InvitationSenderError(f"unknown invitation template field: {unknown}")
	missing_fields = TEMPLATE_FIELDS - fields
	if missing_fields:
		missing = ", ".join(sorted(missing_fields))
		raise InvitationSenderError(f"missing invitation template field: {missing}")
	recipient_name = recipient.display_name or "Student"
	values = {
		"course_name": course_name,
		"recipient_name": recipient_name,
		"signup_url": recipient.signup_url,
	}
	try:
		body = template.format_map(values)
	except (ValueError, IndexError) as error:
		raise InvitationSenderError(f"invalid invitation template: {error}") from error
	if body.splitlines().count(recipient.signup_url) != 1:
		raise InvitationSenderError("signup_url must appear exactly once on its own line")
	subject = f"PLE signup link for {course_name}"
	return subject, body


#============================================
def default_send_func(
	recipient_name: str,
	recipient_address: str,
	subject: str,
	body: str,
) -> str | None:
	"""Send one visible message through Mail.app."""
	# The optional native dependency is reached only for a real attended send.
	import applescript

	# ASVS 1.2.5: dynamic values are Apple-event arguments, never script source.
	script = applescript.AppleScript(MAIL_APP_SCRIPT)
	script.run(recipient_name, recipient_address, subject, body)
	return None


#============================================
def _attempted_at() -> str:
	"""Return one explicit UTC timestamp for local operator evidence."""
	value = datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds")
	timestamp = value.replace("+00:00", "Z")
	return timestamp


#============================================
def _bounded_error_message(error: object) -> str:
	"""Return a single-line bounded error without message content or URL data."""
	text = " ".join(str(error).split())
	if not text:
		text = type(error).__name__
	text = re.sub(r"https?://\S+", "[redacted URL]", text, flags=re.IGNORECASE)
	message = text[:ERROR_MESSAGE_LIMIT]
	return message


#============================================
def _send_error(
	send_func: SendFunction,
	recipient_name: str,
	recipient_email: str,
	subject: str,
	body: str,
) -> str | None:
	"""Return a bounded failure for one injected sender call."""
	try:
		result = send_func(recipient_name, recipient_email, subject, body)
	except Exception as error:
		return _bounded_error_message(error)
	if result is None:
		return None
	if not isinstance(result, str):
		raise InvitationSenderError("send_func must return an error string or None")
	message = _bounded_error_message(result)
	return message


#============================================
def process_batch(
	mailing_export: invitation_mailer.input.MailingExport,
	recipients: tuple[invitation_mailer.input.Recipient, ...],
	template: str,
	cells: dict[tuple[str, str], invitation_mailer.status_log.StatusCell],
	status_path: pathlib.Path,
	dry_run: bool,
	throttle_seconds: float,
	send_func: SendFunction,
) -> BatchSummary:
	"""Process one batch, recording each observation before continuing."""
	sent = 0
	failed = 0
	dry_run_count = 0
	for index, recipient in enumerate(recipients, start=1):
		subject, body = render_message(template, mailing_export.course_name, recipient)
		key = invitation_mailer.status_log.dedup_key(
			mailing_export.course_name,
			recipient.email,
		)
		previous = cells.get(key)
		deliberate_resend = previous is not None and previous.status in {
			"sent",
			"indeterminate",
		}
		attempted_at = _attempted_at()
		if dry_run:
			cell = invitation_mailer.status_log.StatusCell(
				course_name=mailing_export.course_name,
				email=recipient.email,
				status="dry_run",
				attempted_at=attempted_at,
			)
			invitation_mailer.status_log.set_status(cells, cell)
			invitation_mailer.status_log.save(status_path, cells)
			print(f"dry-run {index}/{len(recipients)} {recipient.email}")
			dry_run_count += 1
			continue
		# ASVS 2.3.3: persist ambiguity before the external side effect begins.
		indeterminate = invitation_mailer.status_log.StatusCell(
			course_name=mailing_export.course_name,
			email=recipient.email,
			status="indeterminate",
			attempted_at=attempted_at,
			deliberate_resend=deliberate_resend,
		)
		invitation_mailer.status_log.set_status(cells, indeterminate)
		invitation_mailer.status_log.save(status_path, cells)
		recipient_name = recipient.display_name or "Student"
		error_message = _send_error(
			send_func,
			recipient_name,
			recipient.email,
			subject,
			body,
		)
		if error_message is None:
			status = "sent"
			message = None
			sent += 1
		else:
			status = "failed"
			message = error_message
			failed += 1
		cell = invitation_mailer.status_log.StatusCell(
			course_name=mailing_export.course_name,
			email=recipient.email,
			status=status,
			attempted_at=attempted_at,
			message=message,
			deliberate_resend=deliberate_resend,
		)
		invitation_mailer.status_log.set_status(cells, cell)
		invitation_mailer.status_log.save(status_path, cells)
		# ASVS 16.2.5: progress omits the signup URL and message body.
		print(f"{status} {index}/{len(recipients)} {recipient.email}")
		if index < len(recipients) and throttle_seconds > 0:
			time.sleep(throttle_seconds)
	summary = BatchSummary(sent=sent, failed=failed, dry_run=dry_run_count)
	return summary
