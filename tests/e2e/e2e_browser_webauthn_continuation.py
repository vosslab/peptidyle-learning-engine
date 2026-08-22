"""Private WebAuthn continuation and acknowledgement contract for browser-suite transitions."""

import base64
import json
import pathlib
import re

import local_stack_control.models
import local_stack_control.private_files

import e2e_browser_scenario_contract

WEBAUTHN_CONTINUATION_FILENAME = "webauthn-continuation.json"
WEBAUTHN_CONTINUATION_MAXIMUM_BYTES = 16_384
WEBAUTHN_CONTINUATION_ACKNOWLEDGEMENT_MAXIMUM_BYTES = 1_024
WEBAUTHN_CONTINUATION_ACKNOWLEDGEMENT_EVENT = "visible_sysadmin_passkey_sign_in"
WEBAUTHN_CONTINUATION_RP_ID = "localhost"
WEBAUTHN_CONTINUATION_BASE64URL_PATTERN = re.compile(r"^[A-Za-z0-9_-]+$")


class BrowserWebAuthnContinuationError(local_stack_control.models.ControllerError):
	"""A private WebAuthn continuation or acknowledgement is not trusted."""


#============================================
def continuation_path(directory: pathlib.Path) -> pathlib.Path:
	"""Return the sole owner-selected continuation path below one private run state."""
	result = directory / WEBAUTHN_CONTINUATION_FILENAME
	if not directory.is_absolute() or result.parent != directory:
		raise BrowserWebAuthnContinuationError("browser-suite continuation path is invalid")
	return result


#============================================
def acknowledgement_path(directory: pathlib.Path, scenario_id: str) -> pathlib.Path:
	"""Return one distinct owner-selected acknowledgement path for a claimed child."""
	if e2e_browser_scenario_contract.SCENARIO_PATTERN.fullmatch(scenario_id) is None:
		raise BrowserWebAuthnContinuationError("browser-suite acknowledgement path is invalid")
	result = directory / f"webauthn-continuation-ack-{scenario_id}.json"
	if not directory.is_absolute() or result.parent != directory:
		raise BrowserWebAuthnContinuationError("browser-suite acknowledgement path is invalid")
	return result


#============================================
def validate_continuation(path: pathlib.Path, gateway_port: int) -> None:
	"""Accept exactly one canonical private resident-credential continuation."""
	# ASVS 1.5.1, 2.2.1, 5.3.2, and 6.7.1: decode a bounded owner-only capability once.
	text = _private_ascii(path, WEBAUTHN_CONTINUATION_MAXIMUM_BYTES, "continuation")
	try:
		value = json.loads(text)
	except json.JSONDecodeError as error:
		raise BrowserWebAuthnContinuationError(
			"browser-suite WebAuthn continuation is invalid"
		) from error
	if not isinstance(value, dict) or set(value) != {"version", "origin", "rpId", "credentials"}:
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")
	expected_origin = f"https://localhost:{gateway_port}"
	if (
		value["version"] != 1
		or value["origin"] != expected_origin
		or value["rpId"] != WEBAUTHN_CONTINUATION_RP_ID
	):
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")
	credentials = value["credentials"]
	if not isinstance(credentials, list) or len(credentials) != 1:
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")
	credential = credentials[0]
	if not isinstance(credential, dict):
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")
	expected_credential_keys = {
		"credentialId",
		"isResidentCredential",
		"rpId",
		"privateKey",
		"signCount",
		"userHandle",
		"backupEligibility",
		"backupState",
	}
	if set(credential) != expected_credential_keys:
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")
	if (
		credential["isResidentCredential"] is not True
		or credential["rpId"] != WEBAUTHN_CONTINUATION_RP_ID
		or not isinstance(credential["signCount"], int)
		or isinstance(credential["signCount"], bool)
		or not 0 <= credential["signCount"] <= 0xFFFFFFFF
		or not isinstance(credential["backupEligibility"], bool)
		or not isinstance(credential["backupState"], bool)
		or (credential["backupState"] and not credential["backupEligibility"])
	):
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")
	for name, minimum_bytes, maximum_bytes in (
		("credentialId", 1, 1_024),
		("privateKey", 1, 4_096),
		("userHandle", 1, 64),
	):
		validate_base64url(credential[name], minimum_bytes, maximum_bytes)
	canonical_value = {
		"version": 1,
		"origin": expected_origin,
		"rpId": WEBAUTHN_CONTINUATION_RP_ID,
		"credentials": [{
			"credentialId": credential["credentialId"],
			"isResidentCredential": True,
			"rpId": WEBAUTHN_CONTINUATION_RP_ID,
			"privateKey": credential["privateKey"],
			"signCount": credential["signCount"],
			"userHandle": credential["userHandle"],
			"backupEligibility": credential["backupEligibility"],
			"backupState": credential["backupState"],
		}],
	}
	if text != json.dumps(canonical_value, separators=(",", ":"), ensure_ascii=True):
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")


#============================================
def validate_acknowledgement(
	path: pathlib.Path,
	gateway_port: int,
	contract: e2e_browser_scenario_contract.ScenarioContract,
	namespace: str,
) -> None:
	"""Accept one canonical owner-private acknowledgement from a claimed child."""
	# ASVS 1.5.1, 2.2.1, 2.3.1, and 5.3.2 bind a child completion to its visible passkey event.
	text = _private_ascii(path, WEBAUTHN_CONTINUATION_ACKNOWLEDGEMENT_MAXIMUM_BYTES, "acknowledgement")
	try:
		value = json.loads(text)
	except json.JSONDecodeError as error:
		raise BrowserWebAuthnContinuationError(
			"browser-suite WebAuthn acknowledgement is invalid"
		) from error
	expected_origin = f"https://localhost:{gateway_port}"
	canonical_value = {
		"event": WEBAUTHN_CONTINUATION_ACKNOWLEDGEMENT_EVENT,
		"namespace": namespace,
		"origin": expected_origin,
		"scenarioId": contract.scenario_id,
		"schemaVersion": 1,
	}
	if (
		not isinstance(value, dict)
		or set(value) != set(canonical_value)
		or value != canonical_value
		or text != json.dumps(canonical_value, separators=(",", ":"), ensure_ascii=True)
	):
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn acknowledgement is invalid")


#============================================
def validate_base64url(value: object, minimum_bytes: int, maximum_bytes: int) -> None:
	"""Require one canonical, bounded base64url CDP binary projection."""
	if (
		not isinstance(value, str)
		or WEBAUTHN_CONTINUATION_BASE64URL_PATTERN.fullmatch(value) is None
	):
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")
	try:
		decoded = base64.b64decode(
			value + "=" * (-len(value) % 4), altchars=b"-_", validate=True,
		)
	except ValueError as error:
		raise BrowserWebAuthnContinuationError(
			"browser-suite WebAuthn continuation is invalid"
		) from error
	if (
		not minimum_bytes <= len(decoded) <= maximum_bytes
		or base64.urlsafe_b64encode(decoded).decode("ascii").rstrip("=") != value
	):
		raise BrowserWebAuthnContinuationError("browser-suite WebAuthn continuation is invalid")


#============================================
def _private_ascii(path: pathlib.Path, maximum_bytes: int, kind: str) -> str:
	"""Read one bounded, current-user, mode-0600 private state file as ASCII."""
	try:
		contents = local_stack_control.private_files.read_current_user_private_file(
			path,
			maximum_bytes,
		)
		return contents.decode("ascii")
	except (local_stack_control.models.ControllerError, UnicodeDecodeError) as error:
		raise BrowserWebAuthnContinuationError(
			"browser-suite WebAuthn " + kind + " is invalid"
		) from error
