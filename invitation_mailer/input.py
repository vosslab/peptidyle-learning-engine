"""Read and validate the invitation mailer's config and exported recipients."""

# Standard Library
import os
import json
import math
import stat
import pathlib
import dataclasses
import urllib.parse

# PIP3 modules
import yaml


#============================================
class InvitationInputError(ValueError):
	"""An invitation-mailer input does not match its documented contract."""


#============================================
@dataclasses.dataclass(frozen=True)
class MailerConfig:
	"""Validated operator configuration."""

	allowed_recipient_domains: frozenset[str]
	throttle_seconds: float


#============================================
@dataclasses.dataclass(frozen=True)
class Recipient:
	"""One validated exported recipient."""

	email: str
	signup_url: str
	display_name: str | None
	roster_id: str | None


#============================================
@dataclasses.dataclass(frozen=True)
class MailingExport:
	"""One course mailing list after boundary validation."""

	course_name: str
	recipients: tuple[Recipient, ...]


#============================================
def _required_mapping(value: object, label: str) -> dict:
	"""Return a mapping or raise a boundary-specific error."""
	if not isinstance(value, dict):
		raise InvitationInputError(f"{label} must be a mapping")
	return value


#============================================
def _required_text(value: object, label: str) -> str:
	"""Return trimmed non-empty text without control characters."""
	if not isinstance(value, str):
		raise InvitationInputError(f"{label} must be text")
	text = value.strip()
	if not text:
		raise InvitationInputError(f"{label} must not be empty")
	if any(ord(character) < 32 or ord(character) == 127 for character in text):
		raise InvitationInputError(f"{label} contains a control character")
	return text


#============================================
def _optional_text(value: object, label: str) -> str | None:
	"""Return optional trimmed text under the same text boundary."""
	if value is None:
		return None
	return _required_text(value, label)


#============================================
def normalize_address(address: str) -> str:
	"""Normalize one email address for matching and duplicate suppression."""
	# ASVS 2.2.1: validate decision-bearing input against the documented shape.
	text = _required_text(address, "email")
	normalized = text.lower()
	local_part, separator, domain = normalized.rpartition("@")
	if separator != "@" or not local_part or not domain or "@" in local_part:
		raise InvitationInputError("email must contain one local part and one domain")
	if any(character.isspace() for character in normalized):
		raise InvitationInputError("email must not contain whitespace")
	return normalized


#============================================
def validate_recipient_address(
	address: str,
	allowed_recipient_domains: frozenset[str],
) -> str:
	"""Return a normalized address whose complete domain is allowed."""
	normalized = normalize_address(address)
	domain = normalized.rpartition("@")[2]
	if domain not in allowed_recipient_domains:
		raise InvitationInputError(f"recipient domain is not allowed: {domain}")
	return normalized


#============================================
def validate_signup_url(value: object) -> str:
	"""Return an opaque absolute HTTPS signup URL."""
	text = _required_text(value, "signup_url")
	parsed = urllib.parse.urlsplit(text)
	# ASVS 1.2.2: accept only the intended safe URL protocol.
	if parsed.scheme.lower() != "https" or parsed.hostname is None:
		raise InvitationInputError("signup_url must be an absolute https URL")
	if parsed.username is not None or parsed.password is not None:
		raise InvitationInputError("signup_url must not contain URL credentials")
	return text


#============================================
def _allowed_domains(value: object) -> frozenset[str]:
	"""Validate the exact-domain recipient allowlist."""
	if not isinstance(value, list) or not value:
		raise InvitationInputError("allowed_recipient_domains must be a non-empty list")
	domains = set()
	for raw_domain in value:
		domain = _required_text(raw_domain, "allowed recipient domain").lower()
		if "@" in domain or domain.startswith(".") or domain.endswith("."):
			raise InvitationInputError(f"invalid allowed recipient domain: {domain}")
		if any(character.isspace() for character in domain):
			raise InvitationInputError(f"invalid allowed recipient domain: {domain}")
		domains.add(domain)
	return frozenset(domains)


#============================================
def _throttle_seconds(value: object) -> float:
	"""Validate the non-negative delay between attended sends."""
	if isinstance(value, bool) or not isinstance(value, (int, float)):
		raise InvitationInputError("throttle_seconds must be a non-negative number")
	seconds = float(value)
	if seconds < 0 or not math.isfinite(seconds):
		raise InvitationInputError("throttle_seconds must be a non-negative number")
	return seconds


#============================================
def load_config(path: pathlib.Path) -> MailerConfig:
	"""Load the required mailer configuration from YAML."""
	try:
		with path.open("r", encoding="ascii") as handle:
			loaded = yaml.safe_load(handle)
	except (UnicodeError, yaml.YAMLError) as error:
		raise InvitationInputError(f"invalid mailer YAML: {error}") from error
	data = _required_mapping(loaded, "mailer config")
	if "allowed_recipient_domains" not in data or "throttle_seconds" not in data:
		raise InvitationInputError(
			"mailer config requires allowed_recipient_domains and throttle_seconds"
		)
	config = MailerConfig(
		allowed_recipient_domains=_allowed_domains(data["allowed_recipient_domains"]),
		throttle_seconds=_throttle_seconds(data["throttle_seconds"]),
	)
	return config


#============================================
def _recipient_from_row(
	row: object,
	row_number: int,
	config: MailerConfig,
) -> Recipient:
	"""Validate one exported student row."""
	data = _required_mapping(row, f"students row {row_number}")
	if "email" not in data or "signup_url" not in data:
		raise InvitationInputError(
			f"students row {row_number} requires email and signup_url"
		)
	email_value = data["email"]
	if not isinstance(email_value, str):
		raise InvitationInputError(f"students row {row_number} email must be text")
	recipient = Recipient(
		email=validate_recipient_address(
			email_value,
			config.allowed_recipient_domains,
		),
		signup_url=validate_signup_url(data["signup_url"]),
		display_name=_optional_text(
			data.get("display_name"),
			f"students row {row_number} display_name",
		),
		roster_id=_optional_text(
			data.get("roster_id"),
			f"students row {row_number} roster_id",
		),
	)
	return recipient


#============================================
def read_export(path: pathlib.Path, config: MailerConfig) -> MailingExport:
	"""Read and validate one JSON mailing-list export."""
	try:
		with path.open("r", encoding="utf-8") as handle:
			loaded = json.load(handle)
	except (UnicodeError, json.JSONDecodeError) as error:
		raise InvitationInputError(f"invalid mailing-list JSON: {error}") from error
	data = _required_mapping(loaded, "mailing-list export")
	if "course_name" not in data or "students" not in data:
		raise InvitationInputError("mailing-list export requires course_name and students")
	course_name = _required_text(data["course_name"], "course_name")
	rows = data["students"]
	if not isinstance(rows, list):
		raise InvitationInputError("students must be a list")
	recipients = []
	seen_addresses = set()
	for index, row in enumerate(rows, start=1):
		recipient = _recipient_from_row(row, index, config)
		if recipient.email in seen_addresses:
			raise InvitationInputError(f"duplicate exported recipient: {recipient.email}")
		seen_addresses.add(recipient.email)
		recipients.append(recipient)
	mailing_export = MailingExport(course_name, tuple(recipients))
	return mailing_export


#============================================
def resolve_private_export(repo_root: pathlib.Path, value: str) -> pathlib.Path:
	"""Resolve a private regular JSON file directly inside output-email."""
	working_path = repo_root / "output-email"
	if working_path.is_symlink():
		raise InvitationInputError("output-email must not be a symbolic link")
	working_directory = working_path.resolve()
	candidate = pathlib.Path(value)
	if not candidate.is_absolute():
		candidate = repo_root / candidate
	if candidate.is_symlink():
		raise InvitationInputError("mailing-list export must not be a symbolic link")
	try:
		resolved = candidate.resolve(strict=True)
	except FileNotFoundError as error:
		raise InvitationInputError(f"mailing-list export does not exist: {candidate}") from error
	# ASVS 5.3.2: keep caller-selected files inside the tool's fixed private boundary.
	if resolved.parent != working_directory or resolved.suffix.lower() != ".json":
		raise InvitationInputError(
			"mailing-list export must be a JSON file directly inside output-email"
		)
	metadata = resolved.stat()
	if not stat.S_ISREG(metadata.st_mode):
		raise InvitationInputError("mailing-list export must be a regular file")
	if metadata.st_mode & (stat.S_IRWXG | stat.S_IRWXO):
		raise InvitationInputError("mailing-list export must not permit group or other access")
	if not os.access(resolved, os.R_OK):
		raise InvitationInputError("mailing-list export is not readable")
	return resolved
