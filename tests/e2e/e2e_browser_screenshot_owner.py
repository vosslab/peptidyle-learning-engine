"""Private capture preparation for the lease-owned browser-suite lifecycle."""

import json
import os
import pathlib

import e2e_browser_scenario_contract
import e2e_browser_screenshot_contract
import e2e_browser_screenshot_publisher


def prepare_staging(
	state_directory: pathlib.Path, screenshots: bool
) -> pathlib.Path | None:
	"""Create the one owner-private staging directory only for visual capture."""
	if not screenshots:
		return None
	staging = state_directory / "screenshots"
	staging.mkdir(mode=0o700)
	return staging


def add_capture_input(
	path: pathlib.Path,
	contract: e2e_browser_scenario_contract.ScenarioContract,
) -> None:
	"""Extend owner-created input with its closed no-path capture projection."""
	value = json.loads(path.read_text(encoding="ascii"))
	value["screenshotCapture"] = e2e_browser_screenshot_contract.input_value(
		contract.scenario_id
	)
	with path.open("w", encoding="ascii") as output:
		output.write(json.dumps(value, separators=(",", ":"), ensure_ascii=True))
	os.chmod(path, 0o600)


def capture_dist_digest(root: pathlib.Path, screenshots: bool) -> str | None:
	"""Bind screenshot mode to the ready production dist digest."""
	if not screenshots:
		return None
	return e2e_browser_screenshot_publisher.production_dist_digest(root)


def pending_after_capture(
	root: pathlib.Path,
	staging: pathlib.Path,
	origin: str,
	captured_dist_digest: str | None,
) -> e2e_browser_screenshot_publisher.PendingScreenshotPublication:
	"""Require the captured production dist digest before reading private artifacts."""
	if captured_dist_digest is None:
		raise e2e_browser_screenshot_publisher.ScreenshotPublicationError(
			"screenshot production dist digest is unavailable"
		)
	if captured_dist_digest != e2e_browser_screenshot_publisher.production_dist_digest(root):
		raise e2e_browser_screenshot_publisher.ScreenshotPublicationError(
			"production dist changed during screenshot capture"
		)
	return e2e_browser_screenshot_publisher.pending_from_staging(
		staging, origin, captured_dist_digest
	)


def artifact_evidence_for_scenario(
	pending: e2e_browser_screenshot_publisher.PendingScreenshotPublication,
	scenario_id: str,
) -> tuple[e2e_browser_screenshot_publisher.ScreenshotArtifactEvidence, ...]:
	"""Project safe artifact facts for one child without exposing image bytes."""
	return tuple(
		item[2] for item in pending.artifacts if item[0].scenario_id == scenario_id
	)
