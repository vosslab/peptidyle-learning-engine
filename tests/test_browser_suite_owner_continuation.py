"""Offline contracts for the private WebAuthn continuation hand-off."""

import dataclasses
import pathlib

import pytest

from test_browser_suite_owner import (
	browser_scenario_contract,
	browser_suite_owner,
	offline_dependencies,
	scenario_contract,
)

#============================================
def test_claimed_child_stops_before_chromium_when_visible_setup_did_not_create_continuation(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A claimed target never receives a missing, substituted, or malformed continuation."""
	base = scenario_contract()
	claimed = dataclasses.replace(
		base,
		scenario_id="claimed",
		spec_path="tests/playwright/e2e/claimed.spec.ts",
		ui_creates=("question", "course"),
		sysadmin_requirement="claimed",
		exclusive_seed_mutations=(),
	)
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (base, claimed))
	dependencies, commands, receipts = offline_dependencies(
		tmp_path,
		produce_webauthn_continuation=False,
	)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="WebAuthn continuation"):
		browser_suite_owner.run_selection(
			browser_suite_owner.BrowserSuiteSelection("claimed", None, False), dependencies,
		)
	assert [command[3] for command in commands if command[0] == "npx"] == [base.spec_path]
	assert [item.child_succeeded for item in receipts[0].scenario_receipts] == [False]


#============================================
@pytest.mark.parametrize("acknowledgement", [None, "{}"])
def test_claimed_child_requires_a_valid_post_passkey_acknowledgement(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	acknowledgement: str | None,
) -> None:
	"""A claimed child reaches Chromium but earns no consumption receipt without its acknowledgement."""
	base = scenario_contract()
	claimed = dataclasses.replace(
		base,
		scenario_id="claimed",
		spec_path="tests/playwright/e2e/claimed.spec.ts",
		ui_creates=("question", "course"),
		sysadmin_requirement="claimed",
		exclusive_seed_mutations=(),
	)
	monkeypatch.setattr(browser_scenario_contract, "scenario_contracts", lambda: (base, claimed))
	dependencies, commands, receipts = offline_dependencies(
		tmp_path,
		produce_webauthn_acknowledgement=acknowledgement is not None,
		webauthn_acknowledgement_content=acknowledgement,
	)
	with pytest.raises(browser_suite_owner.BrowserSuiteError, match="WebAuthn acknowledgement"):
		browser_suite_owner.run_selection(
			browser_suite_owner.BrowserSuiteSelection("claimed", None, False), dependencies,
		)
	assert [command[3] for command in commands if command[0] == "npx"] == [
		base.spec_path,
		claimed.spec_path,
	]
	assert [item.child_succeeded for item in receipts[0].scenario_receipts] == [True, False]
	assert [item.webauthn_continuation_consumed for item in receipts[0].scenario_receipts] == [
		False,
		False,
	]
