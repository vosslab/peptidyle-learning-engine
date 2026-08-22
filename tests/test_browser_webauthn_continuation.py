"""Offline adversarial tests for the private WebAuthn transition contract."""

import base64
import json
import os
import pathlib
import sys

import pytest

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as browser_scenario_contract
import e2e_browser_webauthn_continuation as browser_webauthn


#============================================
def private_file(path: pathlib.Path, contents: str) -> None:
	"""Exclusively write one exact mode-0600 ASCII private test fixture."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		output.write(contents.encode("ascii"))


#============================================
def continuation_contents(gateway_port: int = 55001) -> str:
	"""Return the closed, canonical resident-credential continuation fixture."""
	return json.dumps(
		{
			"version": 1,
			"origin": f"https://localhost:{gateway_port}",
			"rpId": "localhost",
			"credentials": [{
				"credentialId": "AA",
				"isResidentCredential": True,
				"rpId": "localhost",
				"privateKey": "AQI",
				"signCount": 0,
				"userHandle": "Aw",
				"backupEligibility": False,
				"backupState": False,
			}],
		},
		separators=(",", ":"),
	)


#============================================
def claimed_contract() -> browser_scenario_contract.ScenarioContract:
	"""Return the smallest claimed contract valid under the scenario boundary."""
	return browser_scenario_contract.ScenarioContract(
		scenario_id="claimed",
		spec_path="tests/playwright/e2e/claimed.spec.ts",
		personas=("morgan_sysadmin",),
		baseline_reads=("base_course",),
		ui_creates=("course",),
		sysadmin_requirement="claimed",
		visible_observation="visible_passkey_entry",
	)


#============================================
def acknowledgement_contents(
	scenario_id: str = "claimed",
	namespace: str = "bs1-0123456789ab-claimed",
	gateway_port: int = 55001,
) -> str:
	"""Return canonical evidence of a visible claimed Sysadmin passkey entry."""
	return json.dumps(
		{
			"event": "visible_sysadmin_passkey_sign_in",
			"namespace": namespace,
			"origin": f"https://localhost:{gateway_port}",
			"scenarioId": scenario_id,
			"schemaVersion": 1,
		},
		separators=(",", ":"),
	)


#============================================
def test_continuation_paths_are_private_and_claimed_acknowledgements_are_distinct(
	tmp_path: pathlib.Path,
) -> None:
	"""The owner selects fixed paths without letting a child choose either capability location."""
	private_directory = tmp_path.resolve()
	assert browser_webauthn.continuation_path(private_directory) == (
		private_directory / "webauthn-continuation.json"
	)
	assert browser_webauthn.acknowledgement_path(private_directory, "claimed") == (
		private_directory / "webauthn-continuation-ack-claimed.json"
	)
	assert browser_webauthn.acknowledgement_path(private_directory, "other") != (
		browser_webauthn.acknowledgement_path(private_directory, "claimed")
	)
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.acknowledgement_path(private_directory, "invalid-id!")


#============================================
def test_continuation_validator_rejects_adversarial_schema_metadata_and_user_handle(
	tmp_path: pathlib.Path,
) -> None:
	"""The owner accepts only canonical, bounded, current-user continuation data."""
	path = tmp_path / "webauthn-continuation.json"
	private_file(path, continuation_contents())
	browser_webauthn.validate_continuation(path, 55001)
	for change in (
		lambda value: value.update({"unexpected": True}),
		lambda value: value.update({"origin": "https://localhost:55002"}),
		lambda value: value.update({"rpId": "example.test"}),
		lambda value: value.update({"credentials": []}),
		lambda value: value.update({"credentials": value["credentials"] * 2}),
		lambda value: value["credentials"][0].update({"credentialId": "="}),
		lambda value: value["credentials"][0].update({"rpId": "example.test"}),
		lambda value: value["credentials"][0].update({"extra": True}),
		lambda value: value["credentials"][0].update({
			"userHandle": base64.urlsafe_b64encode(b"x" * 65).decode("ascii").rstrip("="),
		}),
	):
		value = json.loads(continuation_contents())
		change(value)
		path.unlink()
		private_file(path, json.dumps(value, separators=(",", ":")))
		with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
			browser_webauthn.validate_continuation(path, 55001)
	path.unlink()
	private_file(path, continuation_contents() + "\n")
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_continuation(path, 55001)
	path.chmod(0o644)
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_continuation(path, 55001)
	path.unlink()
	private_file(path, "x" * 16_385)
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_continuation(path, 55001)
	path.unlink()
	target = tmp_path / "continuation-target.json"
	private_file(target, continuation_contents())
	path.symlink_to(target)
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_continuation(path, 55001)


#============================================
def test_acknowledgement_validator_rejects_adversarial_metadata_and_bindings(
	tmp_path: pathlib.Path,
) -> None:
	"""Only a canonical post-passkey acknowledgement can consume a continuation."""
	contract = claimed_contract()
	namespace = "bs1-0123456789ab-claimed"
	path = tmp_path / "webauthn-continuation-ack-claimed.json"
	private_file(path, acknowledgement_contents())
	browser_webauthn.validate_acknowledgement(path, 55001, contract, namespace)
	for change in (
		lambda value: value.update({"unexpected": True}),
		lambda value: value.update({"event": "other"}),
		lambda value: value.update({"namespace": "bs1-0123456789ab-other"}),
		lambda value: value.update({"origin": "https://localhost:55002"}),
		lambda value: value.update({"scenarioId": "other"}),
		lambda value: value.update({"schemaVersion": 2}),
	):
		value = json.loads(acknowledgement_contents())
		change(value)
		path.unlink()
		private_file(path, json.dumps(value, separators=(",", ":")))
		with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
			browser_webauthn.validate_acknowledgement(path, 55001, contract, namespace)
	path.unlink()
	private_file(path, acknowledgement_contents() + "\n")
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_acknowledgement(path, 55001, contract, namespace)
	path.chmod(0o644)
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_acknowledgement(path, 55001, contract, namespace)
	path.unlink()
	private_file(path, "x" * 1_025)
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_acknowledgement(path, 55001, contract, namespace)
	path.unlink()
	target = tmp_path / "acknowledgement-target.json"
	private_file(target, acknowledgement_contents())
	path.symlink_to(target)
	with pytest.raises(browser_webauthn.BrowserWebAuthnContinuationError):
		browser_webauthn.validate_acknowledgement(path, 55001, contract, namespace)
