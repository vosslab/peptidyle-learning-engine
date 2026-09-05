"""Persist current invitation-delivery observations in private local files."""

# Standard Library
import os
import csv
import io
import json
import pathlib
import tempfile
import dataclasses
import collections.abc

# local repo modules
import invitation_mailer.input


VALID_STATUSES = frozenset({"sent", "failed", "dry_run", "indeterminate"})


#============================================
class StatusLogError(ValueError):
	"""The status log cannot be trusted for duplicate suppression."""


#============================================
@dataclasses.dataclass(frozen=True)
class StatusCell:
	"""The latest local observation for one course and recipient."""

	course_name: str
	email: str
	status: str
	attempted_at: str
	message: str | None = None
	deliberate_resend: bool = False


#============================================
@dataclasses.dataclass(frozen=True)
class RecipientSelection:
	"""Recipients selected for work plus duplicate-suppression counts."""

	recipients: tuple[invitation_mailer.input.Recipient, ...]
	already_sent: int
	held_indeterminate: int


#============================================
def dedup_key(course_name: str, normalized_email: str) -> tuple[str, str]:
	"""Return the status-cell identity for one course and recipient."""
	key = (course_name, normalized_email)
	return key


#============================================
def _cell_from_mapping(value: object, index: int) -> StatusCell:
	"""Validate one persisted status cell."""
	if not isinstance(value, dict):
		raise StatusLogError(f"status cell {index} must be a mapping")
	required = ("course_name", "email", "status", "attempted_at")
	if any(field not in value for field in required):
		raise StatusLogError(f"status cell {index} is missing a required field")
	course_name = value["course_name"]
	email = value["email"]
	status = value["status"]
	attempted_at = value["attempted_at"]
	message = value.get("message")
	deliberate_resend = value.get("deliberate_resend", False)
	if not isinstance(course_name, str) or not course_name.strip():
		raise StatusLogError(f"status cell {index} has an invalid course_name")
	if not isinstance(email, str):
		raise StatusLogError(f"status cell {index} has an invalid email")
	try:
		normalized_email = invitation_mailer.input.normalize_address(email)
	except invitation_mailer.input.InvitationInputError as error:
		raise StatusLogError(f"status cell {index} has an invalid email") from error
	if normalized_email != email:
		raise StatusLogError(f"status cell {index} email is not normalized")
	if not isinstance(status, str) or status not in VALID_STATUSES:
		raise StatusLogError(f"status cell {index} has an invalid status")
	if not isinstance(attempted_at, str) or not attempted_at:
		raise StatusLogError(f"status cell {index} has an invalid attempted_at")
	if message is not None and not isinstance(message, str):
		raise StatusLogError(f"status cell {index} has an invalid message")
	if not isinstance(deliberate_resend, bool):
		raise StatusLogError(f"status cell {index} has an invalid deliberate_resend")
	cell = StatusCell(
		course_name=course_name,
		email=email,
		status=status,
		attempted_at=attempted_at,
		message=message,
		deliberate_resend=deliberate_resend,
	)
	return cell


#============================================
def load(path: pathlib.Path) -> dict[tuple[str, str], StatusCell]:
	"""Load the current cells, refusing ambiguity that could cause a resend."""
	if not path.exists():
		return {}
	try:
		with path.open("r", encoding="ascii") as handle:
			loaded = json.load(handle)
	except (UnicodeError, json.JSONDecodeError) as error:
		raise StatusLogError(f"invalid status-log JSON: {error}") from error
	if not isinstance(loaded, dict) or loaded.get("version") != 1:
		raise StatusLogError("status log requires version 1")
	raw_cells = loaded.get("cells")
	if not isinstance(raw_cells, list):
		raise StatusLogError("status log cells must be a list")
	cells = {}
	for index, raw_cell in enumerate(raw_cells, start=1):
		cell = _cell_from_mapping(raw_cell, index)
		key = dedup_key(cell.course_name, cell.email)
		if key in cells:
			raise StatusLogError(f"duplicate status cell for {cell.email}")
		cells[key] = cell
	return cells


#============================================
def _cell_mapping(cell: StatusCell) -> dict:
	"""Build the stable persisted representation of one cell."""
	data = {
		"course_name": cell.course_name,
		"email": cell.email,
		"status": cell.status,
		"attempted_at": cell.attempted_at,
		"deliberate_resend": cell.deliberate_resend,
	}
	if cell.message is not None:
		data["message"] = cell.message
	return data


#============================================
def _atomic_write(
	path: pathlib.Path,
	write_func: collections.abc.Callable[[io.TextIOBase], None],
) -> None:
	"""Write a private sibling temporary file and replace the target."""
	path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
	fd, temporary_name = tempfile.mkstemp(
		prefix=f".{path.name}.",
		suffix=".tmp",
		dir=path.parent,
	)
	temporary_path = pathlib.Path(temporary_name)
	try:
		# ASVS 14.2.4: local PII observations use owner-only filesystem access.
		os.fchmod(fd, 0o600)
		with os.fdopen(fd, "w", encoding="ascii", newline="") as handle:
			write_func(handle)
		os.replace(temporary_path, path)
	finally:
		if temporary_path.exists():
			temporary_path.unlink()


#============================================
def save(path: pathlib.Path, cells: dict[tuple[str, str], StatusCell]) -> None:
	"""Atomically save the current status cells."""
	ordered_cells = [
		_cell_mapping(cells[key])
		for key in sorted(cells)
	]
	payload = {"version": 1, "cells": ordered_cells}

	def write_json(handle: io.TextIOBase) -> None:
		json.dump(payload, handle, indent=2, sort_keys=False, ensure_ascii=True)
		handle.write("\n")

	_atomic_write(path, write_json)


#============================================
def set_status(
	cells: dict[tuple[str, str], StatusCell],
	cell: StatusCell,
) -> None:
	"""Replace one current status cell after validating its closed status."""
	if cell.status not in VALID_STATUSES:
		raise StatusLogError(f"invalid status: {cell.status}")
	key = dedup_key(cell.course_name, cell.email)
	cells[key] = cell


#============================================
def pending_recipients(
	mailing_export: invitation_mailer.input.MailingExport,
	cells: dict[tuple[str, str], StatusCell],
	force_resend: bool = False,
) -> RecipientSelection:
	"""Select unsent recipients while holding ambiguous observations."""
	pending = []
	already_sent = 0
	held_indeterminate = 0
	for recipient in mailing_export.recipients:
		key = dedup_key(mailing_export.course_name, recipient.email)
		cell = cells.get(key)
		if cell is not None and cell.status == "sent" and not force_resend:
			already_sent += 1
			continue
		if cell is not None and cell.status == "indeterminate" and not force_resend:
			held_indeterminate += 1
			continue
		pending.append(recipient)
	selection = RecipientSelection(tuple(pending), already_sent, held_indeterminate)
	return selection


#============================================
def _spreadsheet_cell(value: str) -> str:
	"""Make a human-readable value inert when opened in a spreadsheet."""
	# ASVS 1.2.10: prevent CSV formula interpretation in the readable projection.
	if value.startswith(("=", "+", "-", "@", "\t", "\0")):
		return "'" + value
	return value


#============================================
def write_sent_log(
	path: pathlib.Path,
	cells: dict[tuple[str, str], StatusCell],
) -> None:
	"""Atomically project successful current cells to readable CSV."""
	sent_cells = [cell for cell in cells.values() if cell.status == "sent"]
	sent_cells.sort(key=lambda cell: (cell.course_name, cell.email))

	def write_csv(handle: io.TextIOBase) -> None:
		writer = csv.writer(handle)
		writer.writerow(("course_name", "email", "attempted_at", "deliberate_resend"))
		for cell in sent_cells:
			writer.writerow(
				(
					_spreadsheet_cell(cell.course_name),
					_spreadsheet_cell(cell.email),
					cell.attempted_at,
					"yes" if cell.deliberate_resend else "no",
				)
			)
	_atomic_write(path, write_csv)
