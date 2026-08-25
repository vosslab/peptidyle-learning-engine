"""JSON-backed screenshot-corpus policy for the one production browser suite."""

import dataclasses
import json
import pathlib
import re
from collections.abc import Iterable, Sequence

import e2e_browser_scenario_contract

SCHEMA_VERSION = 2
CORPUS_DIRECTORY = pathlib.PurePosixPath("docs/screenshots")
_IDENTIFIER = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
_PATH_PART = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
_PRIVACY_CHECKS = frozenset({"no_private_material", "no_feedback", "email_masked"})
_VIEWPORTS = frozenset({"laptop", "tablet", "iphone_pro", "square"})


class ScreenshotContractError(ValueError):
	"""A screenshot request falls outside the checked-in visual corpus."""


@dataclasses.dataclass(frozen=True)
class ViewportProfile:
	"""One named browser viewport used for canonical visual evidence."""

	width: int
	height: int
	device_scale_factor: int


@dataclasses.dataclass(frozen=True)
class ScreenshotArtifact:
	"""One stable public PNG produced by one declared visible state."""

	artifact_id: str
	scenario_id: str
	state_id: str
	path: pathlib.PurePosixPath
	viewport: str
	role: str
	journey: str
	capture_order: int
	journey_step: int
	privacy_checks: tuple[str, ...]


def _corpus_source() -> pathlib.Path:
	return pathlib.Path(__file__).with_name("browser_screenshot_corpus.json")


def _decode_source() -> dict[str, object]:
	try:
		value = json.loads(_corpus_source().read_text(encoding="ascii"))
	except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
		raise ScreenshotContractError("screenshot corpus JSON is unavailable") from error
	if not isinstance(value, dict):
		raise ScreenshotContractError("screenshot corpus JSON is invalid")
	return value


def _required_mapping(value: object, label: str) -> dict[str, object]:
	if not isinstance(value, dict):
		raise ScreenshotContractError(f"{label} is invalid")
	return value


def _required_text(value: object, label: str) -> str:
	if not isinstance(value, str) or not value.isascii() or not value:
		raise ScreenshotContractError(f"{label} is invalid")
	return value


def _required_number(value: object, label: str) -> int:
	if isinstance(value, bool) or not isinstance(value, int) or value < 1:
		raise ScreenshotContractError(f"{label} is invalid")
	return value


def _profile(value: object, name: str) -> ViewportProfile:
	mapping = _required_mapping(value, "viewport profile")
	if set(mapping) != {"width", "height", "deviceScaleFactor"}:
		raise ScreenshotContractError("viewport profile fields are invalid")
	return ViewportProfile(
		_required_number(mapping["width"], "viewport width"),
		_required_number(mapping["height"], "viewport height"),
		_required_number(mapping["deviceScaleFactor"], "viewport scale factor"),
	)


def _artifact(value: object) -> ScreenshotArtifact:
	mapping = _required_mapping(value, "screenshot artifact")
	expected = {
		"artifactId", "scenarioId", "stateId", "role", "journey", "captureOrder",
		"journeyStep", "viewport", "path", "privacyChecks",
	}
	if set(mapping) != expected:
		raise ScreenshotContractError("screenshot artifact fields are invalid")
	privacy = mapping["privacyChecks"]
	if not isinstance(privacy, list):
		raise ScreenshotContractError("screenshot artifact lists are invalid")
	return ScreenshotArtifact(
		_required_text(mapping["artifactId"], "artifact ID"),
		_required_text(mapping["scenarioId"], "scenario ID"),
		_required_text(mapping["stateId"], "state ID"),
		pathlib.PurePosixPath(_required_text(mapping["path"], "artifact path")),
		_required_text(mapping["viewport"], "artifact viewport"),
		_required_text(mapping["role"], "artifact role"),
		_required_text(mapping["journey"], "artifact journey"),
		_required_number(mapping["captureOrder"], "capture order"),
		_required_number(mapping["journeyStep"], "journey step"),
		tuple(_required_text(item, "privacy check") for item in privacy),
	)


_SOURCE = _decode_source()
if _SOURCE.get("schemaVersion") != SCHEMA_VERSION:
	raise ScreenshotContractError("screenshot corpus schema version is invalid")
if _SOURCE.get("corpusDirectory") != str(CORPUS_DIRECTORY):
	raise ScreenshotContractError("screenshot corpus directory is invalid")
_RAW_PROFILES = _required_mapping(_SOURCE.get("viewportProfiles"), "viewport profiles")
if set(_RAW_PROFILES) != _VIEWPORTS:
	raise ScreenshotContractError("screenshot viewport profile names are invalid")
VIEWPORT_PROFILES = {name: _profile(_RAW_PROFILES[name], name) for name in sorted(_VIEWPORTS)}
_RAW_ARTIFACTS = _SOURCE.get("artifacts")
if not isinstance(_RAW_ARTIFACTS, list):
	raise ScreenshotContractError("screenshot artifacts are invalid")
ARTIFACTS = tuple(_artifact(item) for item in _RAW_ARTIFACTS)
SCENARIO_ORDER = tuple(dict.fromkeys(item.scenario_id for item in ARTIFACTS))


def _validate_identifier(value: str, label: str) -> None:
	if _IDENTIFIER.fullmatch(value) is None:
		raise ScreenshotContractError(f"{label} is invalid")


def _validate_path(path: pathlib.PurePosixPath, role: str, journey: str, step: int, state: str, viewport: str) -> None:
	if path.suffix != ".png" or not path.is_relative_to(CORPUS_DIRECTORY):
		raise ScreenshotContractError("screenshot path is outside the stable corpus")
	parts = path.relative_to(CORPUS_DIRECTORY).parts
	journey_parts = tuple(journey.split("/"))
	if len(parts) != len(journey_parts) + 2 or parts[:-1] != (role, *journey_parts):
		raise ScreenshotContractError("screenshot path hierarchy is invalid")
	for part in (role, *journey_parts):
		if _PATH_PART.fullmatch(part) is None:
			raise ScreenshotContractError("screenshot path hierarchy is invalid")
	variant_count = sum(item.state_id == state and item.scenario_id == _scenario_for_path(path) for item in ARTIFACTS)
	suffix = f"_{viewport}" if variant_count > 1 else ""
	expected = f"{step:02d}_{state}{suffix}.png"
	if path.name != expected:
		raise ScreenshotContractError("screenshot path name is invalid")


def _scenario_for_path(path: pathlib.PurePosixPath) -> str:
	"""Resolve a unique declared path to its scenario for viewport suffix validation."""
	for artifact in ARTIFACTS:
		if artifact.path == path:
			return artifact.scenario_id
	raise ScreenshotContractError("screenshot path is undeclared")


def viewport_profile(viewport: str) -> ViewportProfile:
	"""Return one checked-in browser viewport profile by name."""
	try:
		return VIEWPORT_PROFILES[viewport]
	except KeyError as error:
		raise ScreenshotContractError("screenshot viewport is invalid") from error


def validate() -> None:
	"""Confirm a nonempty, unique, contiguous ordered UI evidence corpus."""
	if not ARTIFACTS:
		raise ScreenshotContractError("screenshot corpus must not be empty")
	if not set(e2e_browser_scenario_contract.REQUIRED_ROLE_SECURITY_SCENARIOS).issubset(
		{item.scenario_id for item in ARTIFACTS}
	):
		raise ScreenshotContractError(
			"screenshot corpus requires both named role-security journeys"
		)
	if tuple(item.capture_order for item in ARTIFACTS) != tuple(range(1, len(ARTIFACTS) + 1)):
		raise ScreenshotContractError("screenshot capture order is invalid")
	if len({item.artifact_id for item in ARTIFACTS}) != len(ARTIFACTS):
		raise ScreenshotContractError("screenshot artifact IDs must be unique")
	if len({item.path for item in ARTIFACTS}) != len(ARTIFACTS):
		raise ScreenshotContractError("screenshot corpus paths must be unique")
	registry = {item.scenario_id: item for item in e2e_browser_scenario_contract.scenario_contracts()}
	last_steps: dict[tuple[str, str], int] = {}
	for artifact in ARTIFACTS:
		_validate_identifier(artifact.artifact_id, "screenshot artifact ID")
		_validate_identifier(artifact.scenario_id, "screenshot scenario ID")
		_validate_identifier(artifact.state_id, "screenshot state ID")
		_validate_identifier(artifact.role, "screenshot role")
		if artifact.viewport not in VIEWPORT_PROFILES:
			raise ScreenshotContractError("screenshot viewport is invalid")
		if artifact.privacy_checks[0:1] != ("no_private_material",) or len(artifact.privacy_checks) != len(set(artifact.privacy_checks)) or not set(artifact.privacy_checks).issubset(_PRIVACY_CHECKS):
			raise ScreenshotContractError("screenshot privacy checks are invalid")
		_validate_path(artifact.path, artifact.role, artifact.journey, artifact.journey_step, artifact.state_id, artifact.viewport)
		journey_key = (artifact.role, artifact.journey)
		if artifact.journey_step < last_steps.get(journey_key, 0):
			raise ScreenshotContractError("screenshot journey steps are invalid")
		last_steps[journey_key] = artifact.journey_step
		contract = registry.get(artifact.scenario_id)
		if contract is None or artifact.state_id not in contract.screenshot_states:
			raise ScreenshotContractError("screenshot state is not declared by its scenario")


def artifacts_for_scenario(scenario_id: str) -> tuple[ScreenshotArtifact, ...]:
	"""Return only the contract-approved capture moments for one selected child."""
	validate()
	return tuple(item for item in ARTIFACTS if item.scenario_id == scenario_id)


def input_value(scenario_id: str) -> dict[str, object]:
	"""Project no-path capture IDs into the private browser input ABI."""
	artifacts = artifacts_for_scenario(scenario_id)
	return {"version": 1, "artifacts": [{"artifactId": item.artifact_id, "stateId": item.state_id} for item in artifacts]}


def validate_input(value: object, scenario_id: str) -> None:
	"""Require the browser input to be exactly the owner-selected projection."""
	if value != input_value(scenario_id):
		raise ScreenshotContractError("browser screenshot input has an invalid shape")


def ordered_contracts(contracts: Sequence[e2e_browser_scenario_contract.ScenarioContract]) -> tuple[e2e_browser_scenario_contract.ScenarioContract, ...]:
	"""Resolve screenshot mode solely through the checked-in catalog order."""
	validate()
	registry = {item.scenario_id: item for item in contracts}
	try:
		return tuple(registry[scenario_id] for scenario_id in SCENARIO_ORDER)
	except KeyError as error:
		raise ScreenshotContractError("screenshot catalog is incomplete") from error


def artifact_paths(artifacts: Iterable[ScreenshotArtifact] = ARTIFACTS) -> tuple[str, ...]:
	"""Expose canonical repository-relative paths without caller-controlled output names."""
	return tuple(str(item.path) for item in artifacts)
