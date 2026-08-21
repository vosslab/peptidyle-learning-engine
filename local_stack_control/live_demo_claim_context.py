"""Private, generation-bound Sysadmin ownership context for the local live demo."""

import base64
import dataclasses
import json
import os
import pathlib
import uuid

import local_stack_control.models
import local_stack_control.private_files


MAXIMUM_CONTEXT_BYTES = 256


@dataclasses.dataclass(frozen=True)
class ClaimContext:
	"""One canonical private proof bound to one installed demo generation."""

	installation_generation: str
	sysadmin_user_id: str
	ownership_proof: str


#============================================
def context_content(context: ClaimContext) -> bytes:
	"""Encode the closed private context shape without whitespace or extensions."""
	value = {
		"installationGeneration": context.installation_generation,
		"sysadminUserId": context.sysadmin_user_id,
		"ownershipProof": context.ownership_proof,
	}
	return json.dumps(value, separators=(",", ":"), ensure_ascii=True).encode("ascii")


#============================================
def decode_context(content: bytes) -> ClaimContext:
	"""Decode one bounded canonical local context without exposing its proof."""
	try:
		text = content.decode("ascii")
		value = json.loads(text)
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise local_stack_control.models.ControllerError("live-demo Sysadmin claim context is invalid") from error
	if not isinstance(value, dict) or set(value) != {
		"installationGeneration", "sysadminUserId", "ownershipProof",
	}:
		raise local_stack_control.models.ControllerError("live-demo Sysadmin claim context is invalid")
	if not all(isinstance(value.get(name), str) for name in value):
		raise local_stack_control.models.ControllerError("live-demo Sysadmin claim context is invalid")
	try:
		generation = str(uuid.UUID(value["installationGeneration"]))
		user_id = str(uuid.UUID(value["sysadminUserId"]))
	except ValueError as error:
		raise local_stack_control.models.ControllerError("live-demo Sysadmin claim context is invalid") from error
	proof = value["ownershipProof"]
	try:
		decoded_proof = base64.urlsafe_b64decode(proof + "=")
	except ValueError as error:
		raise local_stack_control.models.ControllerError("live-demo Sysadmin claim context is invalid") from error
	context = ClaimContext(generation, user_id, proof)
	if (
		len(proof) != 43
		or base64.urlsafe_b64encode(decoded_proof).decode("ascii").rstrip("=") != proof
		or len(decoded_proof) != 32
		or context_content(context) != content
	):
		raise local_stack_control.models.ControllerError("live-demo Sysadmin claim context is invalid")
	return context


#============================================
def read_context(path: pathlib.Path) -> ClaimContext:
	"""Read the one private context through the strict current-user file boundary."""
	return decode_context(local_stack_control.private_files.read_current_user_private_file(path, MAXIMUM_CONTEXT_BYTES))


#============================================
def validate_bind_source(path: pathlib.Path) -> None:
	"""Accept only a private pending source or one canonical installed context."""
	content = local_stack_control.private_files.read_current_user_private_file(path, MAXIMUM_CONTEXT_BYTES)
	if content != b"":
		decode_context(content)


#============================================
def ensure_bind_source(path: pathlib.Path) -> bool:
	"""Create the empty private source Compose needs before installation has a generation."""
	if path.exists() or path.is_symlink():
		validate_bind_source(path)
		return False
	local_stack_control.private_files.write_atomic_file(path, b"", 0o600)
	validate_bind_source(path)
	return True


#============================================
def ensure_context(
	path: pathlib.Path,
	installation_generation: str,
	sysadmin_user_id: str,
	random_bytes: object = os.urandom,
) -> ClaimContext:
	"""Preserve one matching context or atomically create a fresh generation proof."""
	try:
		expected_generation = str(uuid.UUID(installation_generation))
		expected_user_id = str(uuid.UUID(sysadmin_user_id))
	except ValueError as error:
		raise local_stack_control.models.ControllerError("live-demo claim context inputs are invalid") from error
	if expected_generation != installation_generation or expected_user_id != sysadmin_user_id:
		raise local_stack_control.models.ControllerError("live-demo claim context inputs are invalid")
	if path.exists() or path.is_symlink():
		content = local_stack_control.private_files.read_current_user_private_file(path, MAXIMUM_CONTEXT_BYTES)
		if content != b"":
			existing = decode_context(content)
			if existing.sysadmin_user_id != expected_user_id:
				raise local_stack_control.models.ControllerError("live-demo Sysadmin claim context does not match the configured account")
			if existing.installation_generation == expected_generation:
				return existing
	secret = random_bytes(32)
	if not isinstance(secret, bytes) or len(secret) != 32:
		raise local_stack_control.models.ControllerError("claim proof generator did not provide private secret material")
	proof = base64.urlsafe_b64encode(secret).decode("ascii").rstrip("=")
	context = ClaimContext(expected_generation, expected_user_id, proof)
	local_stack_control.private_files.write_atomic_file(path, context_content(context), 0o600)
	return read_context(path)
