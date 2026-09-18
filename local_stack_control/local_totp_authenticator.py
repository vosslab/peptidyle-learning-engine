"""Private local setup material for Morgan's independent TOTP authenticator.

The controller owns the seed and wrapping key lifecycle.  The browser gets
neither value.  The path-only operator artifact lets a separate local
authenticator setup tool locate the private seed without turning the artifact
itself into an authentication credential.
"""

import base64
import json
import os
import pathlib
import urllib.parse

import local_stack_control.models
import local_stack_control.private_files


MORGAN_TOTP_SEED_FILE = "morgan-totp-seed"
MORGAN_TOTP_SEED_KEY_FILE = "morgan-totp-seed-wrapping-key"
MORGAN_TOTP_ARTIFACT_FILE = "morgan-totp-authenticator.json"
MORGAN_TOTP_SETUP_URI_FILE = "morgan-totp-setup-uri"
MORGAN_TOTP_ACCOUNT_LABEL = "Morgan Delgado"
MORGAN_TOTP_ISSUER = "Peptidyle Learning Engine local demo"
_TOTP_SECRET_BYTES = 32


#============================================
def _read_exact_private_bytes(path: pathlib.Path, size: int) -> bytes:
	"""Read one controller-owned exact-size secret without exposing its value."""
	value = local_stack_control.private_files.read_current_user_private_file(path, size)
	if len(value) != size:
		raise local_stack_control.models.ControllerError("local TOTP secret has invalid length")
	return value


#============================================
def _write_missing_private_bytes(path: pathlib.Path, size: int) -> None:
	"""Create a CSPRNG secret once, preserving a validated existing value."""
	if path.exists() or path.is_symlink():
		_read_exact_private_bytes(path, size)
		return
	local_stack_control.private_files.write_atomic_file(path, os.urandom(size), 0o600)
	_read_exact_private_bytes(path, size)


#============================================
def _artifact_content(seed_path: pathlib.Path, setup_uri_path: pathlib.Path) -> bytes:
	"""Return the non-secret authenticator handoff descriptor."""
	content = {
		"accountLabel": MORGAN_TOTP_ACCOUNT_LABEL,
		"algorithm": "SHA1",
		"digits": 6,
		"format": "ple-local-totp-authenticator-v1",
		"issuer": MORGAN_TOTP_ISSUER,
		"periodSeconds": 30,
		"seedFile": str(seed_path),
		"setupUriFile": str(setup_uri_path),
	}
	return (json.dumps(content, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")


#============================================
def bootstrap_local_totp_material(directory: pathlib.Path) -> pathlib.Path:
	"""Create Morgan's ignored private seed/key/artifact set and return artifact path."""
	local_stack_control.private_files.require_private_directory(directory)
	seed_path = directory / MORGAN_TOTP_SEED_FILE
	key_path = directory / MORGAN_TOTP_SEED_KEY_FILE
	artifact_path = directory / MORGAN_TOTP_ARTIFACT_FILE
	setup_uri_path = directory / MORGAN_TOTP_SETUP_URI_FILE
	_write_missing_private_bytes(seed_path, _TOTP_SECRET_BYTES)
	_write_missing_private_bytes(key_path, _TOTP_SECRET_BYTES)
	artifact = _artifact_content(seed_path, setup_uri_path)
	if artifact_path.exists() or artifact_path.is_symlink():
		current = local_stack_control.private_files.read_current_user_private_file(
			artifact_path, 2_048
		)
		if current != artifact:
			raise local_stack_control.models.ControllerError("local TOTP artifact is invalid")
	else:
		local_stack_control.private_files.write_atomic_file(artifact_path, artifact, 0o600)
	return artifact_path


#============================================
def require_local_totp_material(
	seed_path: pathlib.Path,
	key_path: pathlib.Path,
	artifact_path: pathlib.Path,
) -> None:
	"""Validate the three controller-owned local TOTP inputs without rewriting them."""
	_read_exact_private_bytes(seed_path, _TOTP_SECRET_BYTES)
	_read_exact_private_bytes(key_path, _TOTP_SECRET_BYTES)
	expected = _artifact_content(
		seed_path, artifact_path.parent / MORGAN_TOTP_SETUP_URI_FILE
	)
	actual = local_stack_control.private_files.read_current_user_private_file(artifact_path, 2_048)
	if actual != expected:
		raise local_stack_control.models.ControllerError("local TOTP artifact is invalid")


#============================================
def write_authenticator_setup_uri(artifact_path: pathlib.Path) -> pathlib.Path:
	"""Create a separate mode-0600 otpauth URI file from a valid path-only artifact.

	The caller may import this file with a local authenticator.  Neither this
	function nor its caller logs the URI, seed, or a generated time-based code.
	"""
	artifact_bytes = local_stack_control.private_files.read_current_user_private_file(
		artifact_path, 2_048
	)
	try:
		artifact = json.loads(artifact_bytes.decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise local_stack_control.models.ControllerError("local TOTP artifact is invalid") from error
	if not isinstance(artifact, dict) or set(artifact) != {
		"accountLabel", "algorithm", "digits", "format", "issuer", "periodSeconds", "seedFile", "setupUriFile"
	}:
		raise local_stack_control.models.ControllerError("local TOTP artifact is invalid")
	if not all(isinstance(value, str) for value in (
		artifact["accountLabel"], artifact["algorithm"], artifact["format"], artifact["issuer"],
		artifact["seedFile"], artifact["setupUriFile"],
	)) or not isinstance(artifact["digits"], int) or not isinstance(artifact["periodSeconds"], int):
		raise local_stack_control.models.ControllerError("local TOTP artifact is invalid")
	if (
		artifact["format"] != "ple-local-totp-authenticator-v1"
		or artifact["accountLabel"] != MORGAN_TOTP_ACCOUNT_LABEL
		or artifact["issuer"] != MORGAN_TOTP_ISSUER
		or artifact["algorithm"] != "SHA1"
		or artifact["digits"] != 6
		or artifact["periodSeconds"] != 30
	):
		raise local_stack_control.models.ControllerError("local TOTP artifact is invalid")
	seed_path = pathlib.Path(artifact["seedFile"])
	setup_uri_path = pathlib.Path(artifact["setupUriFile"])
	if (
		not seed_path.is_absolute()
		or not setup_uri_path.is_absolute()
		or seed_path.parent != artifact_path.parent
		or setup_uri_path.parent != artifact_path.parent
		or seed_path.name != MORGAN_TOTP_SEED_FILE
		or setup_uri_path.name != MORGAN_TOTP_SETUP_URI_FILE
	):
		raise local_stack_control.models.ControllerError("local TOTP artifact is invalid")
	secret = _read_exact_private_bytes(seed_path, _TOTP_SECRET_BYTES)
	encoded = base64.b32encode(secret).decode("ascii").rstrip("=")
	uri = "otpauth://totp/" + urllib.parse.quote(
		f"{MORGAN_TOTP_ISSUER}:{MORGAN_TOTP_ACCOUNT_LABEL}", safe=""
	) + "?" + urllib.parse.urlencode({
		"secret": encoded,
		"issuer": MORGAN_TOTP_ISSUER,
		"algorithm": "SHA1",
		"digits": "6",
		"period": "30",
	}) + "\n"
	local_stack_control.private_files.write_atomic_file(setup_uri_path, uri.encode("ascii"), 0o600)
	return setup_uri_path
