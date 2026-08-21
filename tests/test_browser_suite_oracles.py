"""Offline behavior checks for browser-suite origin and lifecycle evidence."""

import json
import pathlib
import sys

import pytest

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_suite_oracles as browser_suite_oracles


#============================================
def inventory(**changes: object) -> browser_suite_oracles.SuiteInventory:
	"""Build a minimal no-resource live-demo provider inventory."""
	value: dict[str, object] = {
		"project": "ple-live-demo-browser-0123456789ab",
		"containers": (),
		"volumes": (),
		"networks": (),
		"private_artifacts": (),
		"owner_processes": (),
		"provider": browser_suite_oracles.ProviderReceipt(
			"podman-compose", ("podman-compose", "--in-pod", "false"), False
		),
	}
	value.update(changes)
	return browser_suite_oracles.SuiteInventory(**value)  # type: ignore[arg-type]


#============================================
def test_origin_receipt_accepts_only_the_expected_https_gateway(tmp_path: pathlib.Path) -> None:
	"""Chromium evidence accepts the exact production HTTPS gateway for pages and requests."""
	path = tmp_path / "origin.json"
	path.write_text(
		json.dumps({"pageOrigins": ["https://localhost:55001"], "requestOrigins": ["https://localhost:55001"]}),
		encoding="ascii",
	)
	receipt = browser_suite_oracles.origin_receipt_from_file(path, "https://localhost:55001/")
	assert receipt.expected_origin == "https://localhost:55001"
	assert receipt.observed_page_origins == ("https://localhost:55001",)


#============================================
def test_origin_receipt_refuses_a_mixed_browser_origin(tmp_path: pathlib.Path) -> None:
	"""A page or request outside the gateway fails the visible-browser receipt."""
	path = tmp_path / "origin.json"
	path.write_text(
		json.dumps({"pageOrigins": ["https://localhost:55001"], "requestOrigins": ["https://example.test"]}),
		encoding="ascii",
	)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="outside"):
		browser_suite_oracles.origin_receipt_from_file(path, "https://localhost:55001/")


#============================================
def test_private_artifact_inventory_keeps_metadata_and_never_content(tmp_path: pathlib.Path) -> None:
	"""Receipt inventory exposes a leftover private path without serializing its secret bytes."""
	path = tmp_path / "secret"
	path.write_text("sensitive-value", encoding="ascii")
	path.chmod(0o600)
	items = browser_suite_oracles.private_artifacts(tmp_path)
	public = browser_suite_oracles.public_inventory(inventory(private_artifacts=items))
	assert items[0].path == "secret" and items[0].mode == 0o600
	assert "sensitive-value" not in json.dumps(public, sort_keys=True)


#============================================
def test_private_artifact_inventory_refuses_a_symlink(tmp_path: pathlib.Path) -> None:
	"""A surviving symlink cannot make a private cleanup receipt appear empty."""
	target = tmp_path / "target"
	target.write_text("content", encoding="ascii")
	link = tmp_path / "link"
	link.symlink_to(target)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="unexpected"):
		browser_suite_oracles.private_artifacts(tmp_path)


#============================================
def test_process_group_inventory_retains_a_reparented_owner_child() -> None:
	"""A typed PPID-one row remains owned because its recorded process group persists."""
	processes = browser_suite_oracles.processes_from_rows([(42, 1, 7102)], {7102}, 99, set())
	assert processes == (browser_suite_oracles.ProcessIdentity(42, 1, 7102),)


#============================================
def test_process_marker_detects_a_session_escape_without_retaining_command_text() -> None:
	"""A marker keeps an owner child visible after it creates a different session group."""
	processes = browser_suite_oracles.processes_from_rows([(43, 1, 8200)], {7102}, 99, {43})
	assert processes == (browser_suite_oracles.ProcessIdentity(43, 1, 8200),)


#============================================
@pytest.mark.parametrize(
	("change", "message"),
	[
		({"private_artifacts": (browser_suite_oracles.PrivateArtifact("leftover", 0o600, 1),)}, "private artifacts"),
		({"owner_processes": (browser_suite_oracles.ProcessIdentity(9, 1, 9),)}, "background processes"),
		({"provider": browser_suite_oracles.ProviderReceipt("podman-compose", ("podman-compose", "--in-pod", "true"), True)}, "pod ownership disabled"),
	],
)
def test_cleanup_oracle_refuses_remaining_owned_state(change: dict[str, object], message: str) -> None:
	"""The post-cleanup gate fails on any remaining owned resource class or pod policy change."""
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match=message):
		browser_suite_oracles.empty_after_cleanup(inventory(**change))
