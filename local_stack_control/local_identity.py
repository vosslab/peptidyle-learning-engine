"""Private local login credentials and hash-only identity projection."""

import base64
import collections.abc
import dataclasses
import hashlib
import json
import os
import pathlib

import local_stack_control.models
import local_stack_control.private_files


JACK_FAKE_STUDENT_HASH = "1111111111111111111111111111111111111111111111111111111111111111"


@dataclasses.dataclass(frozen=True)
class LocalIdentityConfiguration:
	"""Fixed local identities and private file paths for one selected environment."""

	credential_file: pathlib.Path
	identity_file: pathlib.Path
	tenant_id: str
	instructor_id: str
	student_id: str


@dataclasses.dataclass(frozen=True)
class LocalCredentials:
	"""Validated private credentials, retained only by the local bootstrap owner."""

	instructor: bytes
	student: bytes


#============================================
def bootstrap_local_identities(
	configuration: LocalIdentityConfiguration,
	random_bytes: collections.abc.Callable[[int], bytes] = os.urandom,
	owner_id: int | None = None,
) -> None:
	"""Create absent local credentials once and atomically project their hashes."""
	if configuration.credential_file.exists() or configuration.credential_file.is_symlink():
		credentials = read_local_credentials(configuration.credential_file, owner_id)
	else:
		if configuration.identity_file.exists() or configuration.identity_file.is_symlink():
			raise local_stack_control.models.ControllerError(
				"local credentials are missing; refusing to rotate existing local identities"
			)
		credentials = generate_distinct_credentials(random_bytes)
		write_credentials(configuration.credential_file, credentials)
	write_identity_projection(configuration, credentials)


#============================================
def generate_distinct_credentials(
	random_bytes: collections.abc.Callable[[int], bytes],
) -> LocalCredentials:
	"""Generate two distinct canonical credential values from injected entropy."""
	instructor = encode_secret(random_bytes(32))
	student = encode_secret(random_bytes(32))
	if instructor == student:
		raise local_stack_control.models.ControllerError("local credential generator did not provide distinct credentials")
	result = LocalCredentials(instructor, student)
	return result


#============================================
def encode_secret(value: bytes) -> bytes:
	"""Encode exactly 32 random bytes as canonical base64url without padding."""
	if len(value) != 32:
		raise local_stack_control.models.ControllerError("local credential generator did not provide credential material")
	encoded = base64.urlsafe_b64encode(value).rstrip(b"=")
	return encoded


#============================================
def write_credentials(path: pathlib.Path, credentials: LocalCredentials) -> None:
	"""Atomically create a mode-0600 private credential record."""
	content = b"instructor=" + credentials.instructor + b"\nstudent=" + credentials.student + b"\n"
	local_stack_control.private_files.write_atomic_file(path, content, 0o600)


#============================================
def read_local_credentials(path: pathlib.Path, owner_id: int | None = None) -> LocalCredentials:
	"""Read the exact two-role credential record through a bounded descriptor."""
	content = local_stack_control.private_files.read_current_user_private_file(path, 128, owner_id)
	try:
		lines = content.decode("ascii").splitlines()
	except UnicodeDecodeError as error:
		raise local_stack_control.models.ControllerError("local credentials are invalid") from error
	values: dict[str, bytes] = {}
	for line in lines:
		if "=" not in line:
			raise local_stack_control.models.ControllerError("local credentials are invalid")
		role, credential = line.split("=", 1)
		if role not in ("instructor", "student") or role in values:
			raise local_stack_control.models.ControllerError("local credentials are invalid")
		values[role] = credential.encode("ascii")
	if set(values) != {"instructor", "student"}:
		raise local_stack_control.models.ControllerError("local credentials are invalid")
	instructor = validate_credential(values["instructor"])
	student = validate_credential(values["student"])
	if instructor == student:
		raise local_stack_control.models.ControllerError("local credentials must be distinct")
	result = LocalCredentials(instructor, student)
	return result


#============================================
def validate_credential(value: bytes) -> bytes:
	"""Require the shared canonical 32-byte local credential representation."""
	local_stack_control.private_files.canonical_secret32(value)
	return value


#============================================
def write_identity_projection(
	configuration: LocalIdentityConfiguration,
	credentials: LocalCredentials,
) -> None:
	"""Write the public hash-only identity projection after private validation."""
	instructor_secret = local_stack_control.private_files.canonical_secret32(credentials.instructor)
	student_secret = local_stack_control.private_files.canonical_secret32(credentials.student)
	instructor_hash = hashlib.sha256(instructor_secret).hexdigest()
	student_hash = hashlib.sha256(student_secret).hexdigest()
	projection = {
		"credentials": [
			{
				"credential_sha256": instructor_hash,
				"learner_alias": "instructor-local",
				"tenant_id": configuration.tenant_id,
				"user_id": configuration.instructor_id,
				"display_name": "Dr. Fake Professor",
				"roles": ["instructor", "sysadmin"],
			},
			{
				"credential_sha256": student_hash,
				"learner_alias": "student-local",
				"tenant_id": configuration.tenant_id,
				"user_id": configuration.student_id,
				"display_name": "Mary Fake Student",
				"roles": ["student"],
			},
			{
				"credential_sha256": JACK_FAKE_STUDENT_HASH,
				"learner_alias": "student-jack",
				"tenant_id": configuration.tenant_id,
				"user_id": "00000000-0000-0000-0000-000000000103",
				"display_name": "Jack Fake Student",
				"roles": ["student"],
			},
		],
	}
	content = json.dumps(projection, separators=(",", ":")).encode("ascii") + b"\n"
	local_stack_control.private_files.write_atomic_file(configuration.identity_file, content, 0o644)
