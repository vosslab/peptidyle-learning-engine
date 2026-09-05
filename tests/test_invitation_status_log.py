"""Behavior tests for local invitation status and duplicate suppression."""

# Standard Library
import csv
import stat
import pathlib

# PIP3 modules
import pytest

# local repo modules
import invitation_mailer.input
import invitation_mailer.status_log


#============================================
def _recipient(address: str) -> invitation_mailer.input.Recipient:
	"""Build one validated-shape recipient for status tests."""
	return invitation_mailer.input.Recipient(
		email=address,
		signup_url="https://example.edu/signup/token",
		display_name=None,
		roster_id=None,
	)


#============================================
def _cell(address: str, status_name: str) -> invitation_mailer.status_log.StatusCell:
	"""Build one current status observation."""
	return invitation_mailer.status_log.StatusCell(
		course_name="Genetics 301",
		email=address,
		status=status_name,
		attempted_at="2026-09-05T12:00:00Z",
	)


#============================================
def test_status_log_round_trip_is_owner_private(tmp_path: pathlib.Path) -> None:
	"""Persist current cells without granting group or other access."""
	path = tmp_path / "output-email" / "invitation_status.json"
	cell = _cell("student@mail.roosevelt.edu", "sent")
	cells = {invitation_mailer.status_log.dedup_key(cell.course_name, cell.email): cell}

	invitation_mailer.status_log.save(path, cells)

	assert invitation_mailer.status_log.load(path) == cells
	assert stat.S_IMODE(path.stat().st_mode) & 0o077 == 0


#============================================
def test_pending_recipients_holds_sent_and_indeterminate_observations() -> None:
	"""Retry failures but suppress confirmed and ambiguous prior sends."""
	recipients = (
		_recipient("sent@mail.roosevelt.edu"),
		_recipient("unknown@mail.roosevelt.edu"),
		_recipient("failed@mail.roosevelt.edu"),
		_recipient("new@mail.roosevelt.edu"),
	)
	mailing_export = invitation_mailer.input.MailingExport("Genetics 301", recipients)
	cells = {}
	for address, status_name in (
		("sent@mail.roosevelt.edu", "sent"),
		("unknown@mail.roosevelt.edu", "indeterminate"),
		("failed@mail.roosevelt.edu", "failed"),
	):
		cell = _cell(address, status_name)
		cells[invitation_mailer.status_log.dedup_key(cell.course_name, cell.email)] = cell

	selection = invitation_mailer.status_log.pending_recipients(mailing_export, cells)

	assert tuple(recipient.email for recipient in selection.recipients) == (
		"failed@mail.roosevelt.edu",
		"new@mail.roosevelt.edu",
	)
	assert (selection.already_sent, selection.held_indeterminate) == (1, 1)


#============================================
def test_force_resend_releases_held_recipients() -> None:
	"""Release a closing observation only for an explicit resend."""
	recipients = (
		_recipient("sent@mail.roosevelt.edu"),
		_recipient("unknown@mail.roosevelt.edu"),
	)
	mailing_export = invitation_mailer.input.MailingExport("Genetics 301", recipients)
	cells = {}
	for address, status_name in (
		("sent@mail.roosevelt.edu", "sent"),
		("unknown@mail.roosevelt.edu", "indeterminate"),
	):
		cell = _cell(address, status_name)
		cells[invitation_mailer.status_log.dedup_key(cell.course_name, cell.email)] = cell

	forced = invitation_mailer.status_log.pending_recipients(
		mailing_export,
		cells,
		force_resend=True,
	)
	assert forced.recipients == recipients


#============================================
def test_pending_recipients_scopes_status_to_course() -> None:
	"""Do not suppress the same address in a different course."""
	recipient = _recipient("student@mail.roosevelt.edu")
	cell = _cell(recipient.email, "sent")
	cells = {
		invitation_mailer.status_log.dedup_key(cell.course_name, cell.email): cell,
	}
	other_course = invitation_mailer.input.MailingExport(
		"Biochemistry 301",
		(recipient,),
	)
	assert invitation_mailer.status_log.pending_recipients(
		other_course,
		cells,
	).recipients == (recipient,)


#============================================
def test_status_log_refuses_ambiguous_duplicate_cells(tmp_path: pathlib.Path) -> None:
	"""Fail closed when persisted state cannot support safe suppression."""
	path = tmp_path / "status.json"
	path.write_text(
		'{"version":1,"cells":['
		'{"course_name":"Genetics 301",'
		'"email":"student@mail.roosevelt.edu","status":"sent",'
		'"attempted_at":"2026-09-05T12:00:00Z"},'
		'{"course_name":"Genetics 301",'
		'"email":"student@mail.roosevelt.edu","status":"failed",'
		'"attempted_at":"2026-09-05T12:01:00Z"}]}',
		encoding="ascii",
	)

	with pytest.raises(invitation_mailer.status_log.StatusLogError, match="duplicate"):
		invitation_mailer.status_log.load(path)


#============================================
def test_sent_log_neutralizes_spreadsheet_formulas(tmp_path: pathlib.Path) -> None:
	"""Keep readable CSV status text inert in spreadsheet applications."""
	path = tmp_path / "sent_log.csv"
	cell = invitation_mailer.status_log.StatusCell(
		course_name="=unsafe",
		email="student@mail.roosevelt.edu",
		status="sent",
		attempted_at="2026-09-05T12:00:00Z",
	)
	cells = {invitation_mailer.status_log.dedup_key(cell.course_name, cell.email): cell}

	invitation_mailer.status_log.write_sent_log(path, cells)

	with path.open("r", encoding="ascii", newline="") as handle:
		rows = list(csv.reader(handle))
	assert rows[1][0] == "'=unsafe"
